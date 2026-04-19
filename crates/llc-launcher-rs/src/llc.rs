use bytes::Bytes;
use eyre::{Context, ContextCompat};
use flate2::read::GzDecoder;
use llc_rs::{
    LLCConfig, get_limbus_company_install_path, launch_limbus_company,
    npm::{DistInfo, NpmClient},
    utils::OptionExt,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

const PKG_NAME: &str = "@lightsing/llc-zh-cn";

pub async fn run(llc_config: LLCConfig) -> eyre::Result<()> {
    install_or_update_llc(llc_config)
        .await
        .inspect_err(|e| error!("Failed to install or update LLC: {e}"))
        .context("无法安装或更新 LLC")?;

    info!("LLC installation or update completed successfully.");

    launch_limbus_company()
        .inspect_err(|e| error!("cannot start Limbus Company: {e}"))
        .context("无法启动 Limbus Company")?;

    info!("Limbus Company launched successfully.");

    #[cfg(not(debug_assertions))]
    {
        copy_self_to_launcher()
            .await
            .inspect_err(|e| error!("Failed to copy self to launcher: {e}"))
            .context("无法更新启动器可执行文件")?;

        info!("Launcher executable updated successfully.");
    }

    Ok(())
}

/// Reverse update the launcher executable.
#[cfg(not(debug_assertions))]
async fn copy_self_to_launcher() -> eyre::Result<()> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await; // Give some time for parent process to finish
    let launcher_path = PathBuf::from(
        std::env::var_os("LLC_LAUNCHER_PATH")
            .context("请勿直接运行本目录中的 llc-launcher-rs 可执行文件")
            .inspect_err(|_e| error!("LLC_LAUNCHER_PATH unset"))?,
    );
    let current_exe = std::env::current_exe()
        .inspect_err(|e| error!("Failed to get current executable path: {e}"))
        .context("无法获取当前可执行文件路径")?;

    tokio::fs::copy(current_exe, launcher_path)
        .await
        .inspect_err(|e| error!("Failed to copy executable to launcher path: {e}"))?;
    Ok(())
}

async fn install_or_update_llc(llc_config: LLCConfig) -> eyre::Result<()> {
    let game_root = get_limbus_company_install_path()
        .inspect_err(|e| error!("failed to get Limbus Company install path: {e}"))
        .context("无法获取 Limbus Company 安装路径")?;
    info!("Limbus Company install path: {}", game_root.display());

    tokio::fs::create_dir_all(game_root.join("LimbusCompany_Data").join("Lang"))
        .await
        .inspect_err(|e| error!("Failed to create LLC directory: {e}"))
        .context("无法创建语言目录")?;

    let font_installer = tokio::spawn(install_font_if_needed(game_root.clone()));

    let installed_tag = match get_version_installed(&game_root) {
        Ok(Some(version)) => version,
        Ok(None) => {
            info!("No version installed, proceeding with installation.");
            String::new()
        }
        Err(e) => {
            warn!("Failed to get installed version: {e}, proceeding with installation.");
            String::new()
        }
    };

    let latest_version = NpmClient::new(llc_config.npm_registries())
        .get_lastest_version(PKG_NAME)
        .await
        .inspect_err(|e| error!("Failed to get latest LLC version: {e}"))
        .context("无法获取最新 LLC 版本")?;
    let tag = latest_version
        .github_tag
        .context("无法获取最新 LLC 版本的发布标签")?;
    info!("Latest version available: {tag}");

    if installed_tag == tag {
        info!("LLC is already up to date (version {}).", installed_tag);
        return Ok(());
    }

    let cleaner = tokio::spawn(cleanup_installed_llc(game_root.clone()));
    let downloader = tokio::spawn(download_release(llc_config, latest_version.dist));

    info!("Updating LLC from version {installed_tag} to {tag}.",);

    cleaner
        .await
        .inspect_err(|e| error!("Failed to clean up installed files: {e}"))
        .context("无法清理已安装的文件")?;

    let tarball = downloader
        .await
        .map_err(|e| e.into())
        .and_then(|res| res)
        .inspect_err(|e| error!("Failed to download LLC: {e}"))
        .context("无法下载 LLC 文件")?;
    extract_apply_release(tarball, game_root.clone())
        .await
        .inspect_err(|e| error!("Failed to extract and apply LLC update: {e}"))
        .context("无法解压并应用 LLC 更新")?;

    font_installer
        .await
        .map_err(|e| e.into())
        .and_then(|res| res)
        .inspect_err(|e| error!("Failed to install font: {e}"))
        .context("无法安装字体")?;

    Ok(())
}

fn get_version_installed(game_root: &Path) -> eyre::Result<Option<String>> {
    let version_file = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN")
        .join("Info")
        .join("version.json");
    if !version_file.exists() {
        info!("Version file does not exist at {}", version_file.display());
        return Ok(None);
    }
    let version = serde_json::from_reader::<_, Value>(std::fs::File::open(version_file)?)
        .inspect_err(|e| error!("Failed to parse version file: {e}"))?;
    let Some(version) = version.get("version") else {
        info!("Version field not found in version file");
        return Ok(None);
    };
    match version {
        Value::String(s) => Ok(Some(s.clone())),
        Value::Number(n) => Ok(Some(n.to_string())),
        _ => {
            info!("Version field is neither string nor number");
            Ok(None)
        }
    }
}

async fn cleanup_installed_llc(game_root: PathBuf) {
    let llc_dir = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN");
    if !llc_dir.exists() {
        return;
    }
    let Ok(mut read_dir) = tokio::fs::read_dir(llc_dir).await else {
        warn!("Failed to read LLC directory for cleanup");
        return;
    };
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        if entry.file_name() == "Font" {
            continue; // Skip the Font directory
        }
        let path = entry.path();
        if path.is_dir() {
            tokio::fs::remove_dir_all(path).await.ok();
        } else if path.is_file() {
            tokio::fs::remove_file(path).await.ok();
        } else {
            warn!("Found suspicious path: {:?}", path.display());
        }
    }
}

async fn install_font_if_needed(game_root: PathBuf) -> eyre::Result<()> {
    static FONT_FILE: &[u8] = include_bytes!("../../../assets/SarasaGothicSC-Bold.ttf");

    let font_dir = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN")
        .join("Font");

    tokio::fs::create_dir_all(&font_dir).await?;
    tokio::fs::create_dir_all(font_dir.join("Context")).await?;
    tokio::fs::create_dir_all(font_dir.join("Title")).await?;

    let font_file = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN")
        .join("Font")
        .join("Context")
        .join("ChineseFont.ttf");

    let mut font_valid = false;

    if let Ok(font_data) = tokio::fs::read(&font_file).await {
        if ttf_parser::Face::parse(&font_data, 0).is_ok() {
            font_valid = true;
        }
    }

    if font_valid {
        info!("Font file is already installed.");
        return Ok(());
    } else {
        info!("Font file does not exist, installing...");
    }

    tokio::fs::create_dir_all(font_file.parent().infallible()).await?;
    tokio::fs::write(&font_file, FONT_FILE).await?;
    info!(
        "Font file installed successfully at {}",
        font_file.display()
    );
    Ok(())
}

async fn download_release(llc_config: LLCConfig, dist: DistInfo) -> eyre::Result<Bytes> {
    let client = NpmClient::new(llc_config.npm_registries());
    let buffer = client.download_dist(dist).await?;
    Ok(buffer)
}

async fn extract_apply_release(tarball: Bytes, game_root: PathBuf) -> eyre::Result<()> {
    let tar = GzDecoder::new(tarball.as_ref());
    let mut archive = tar::Archive::new(tar);

    let dst_dir = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN");

    for file in archive.entries()? {
        let mut file = file?;
        let path = file.path()?.to_path_buf();
        let Ok(path) = path.strip_prefix("package/LimbusCompany_Data/Lang/LLC_zh-CN") else {
            continue;
        };
        let dest_path = dst_dir.join(path);
        if let Some(parent) = dest_path.parent()
            && !parent.exists()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        file.unpack(&dest_path)?;
        let file_time = filetime::FileTime::now();
        filetime::set_file_atime(&dest_path, file_time)?;
        filetime::set_file_mtime(&dest_path, file_time)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_download_extract_release() {
        let game_root = get_limbus_company_install_path().unwrap();
        let llc_config = LLCConfig::default();
        let npm_client = NpmClient::new(llc_config.npm_registries());
        let dist = npm_client.get_lastest_version(PKG_NAME).await.unwrap().dist;

        let tarball = download_release(llc_config, dist).await.unwrap();
        extract_apply_release(tarball, game_root).await.unwrap();
    }
}
