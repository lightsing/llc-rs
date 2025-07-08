use crate::{utils};
use eyre::{Context, ContextCompat};
use llc_rs::{LLCConfig, get_limbus_company_install_path, launch_limbus_company, zeroasso};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use smol::stream::StreamExt;

#[derive(Deserialize)]
struct Version {
    version: u64,
}

pub async fn run(llc_config: LLCConfig) -> eyre::Result<()> {
    let job = smol::spawn(copy_self_to_launcher());

    install_or_update_llc(llc_config).await?;

    launch_limbus_company()
        .inspect_err(|e| error!("cannot start Limbus Company: {e}"))
        .context("无法启动 Limbus Company")?;

    job.await
        .inspect_err(|e| error!("Failed to copy self to launcher: {e}"))
        .context("无法更新启动器可执行文件")?;
    Ok(())
}

/// Reverse update the launcher executable.
async fn copy_self_to_launcher() -> eyre::Result<()> {
    smol::Timer::after(std::time::Duration::from_secs(1)).await;  // Give some time for parent process to finish
    let launcher_path = PathBuf::from(
        std::env::var_os("LLC_LAUNCHER_PATH")
            .context("请勿直接运行本目录中的 llc-launcher-rs 可执行文件")
            .inspect_err(|_e| error!("LLC_LAUNCHER_PATH unset"))?,
    );
    let current_exe = std::env::current_exe()
        .inspect_err(|e| error!("Failed to get current executable path: {e}"))
        .context("无法获取当前可执行文件路径")?;

    smol::fs::copy(current_exe, launcher_path)
        .await
        .inspect_err(|e| error!("Failed to copy executable to launcher path: {e}"))?;
    Ok(())
}

async fn install_or_update_llc(llc_config: LLCConfig) -> eyre::Result<()> {
    let llc_config = Arc::new(llc_config);
    let game_root = get_limbus_company_install_path()
        .inspect_err(|e| error!("failed to get Limbus Company install path: {e}"))
        .context("无法获取 Limbus Company 安装路径")?;
    info!("Limbus Company install path: {}", game_root.display());

    smol::fs::create_dir_all(game_root.join("LimbusCompany_Data").join("Lang"))
        .await
        .inspect_err(|e| error!("Failed to create LLC directory: {e}"))
        .context("无法创建语言目录")?;

    let installed_version = match get_version_installed(&game_root) {
        Ok(Some(version)) => version,
        Ok(None) => {
            info!("No version installed, proceeding with installation.");
            0
        }
        Err(e) => {
            warn!("Failed to get installed version: {e}, proceeding with installation.");
            0
        }
    };

    let hashes = smol::spawn(zeroasso::get_hash::run(llc_config.clone()));
    let latest_version = zeroasso::get_version::run(llc_config.clone())
        .await
        .inspect_err(|e| error!("Failed to get latest version: {e}"))
        .context("无法获取最新版本")?;

    info!("Latest version available: {}", latest_version);

    if installed_version >= latest_version {
        info!("LLC is already up to date (version {}).", installed_version);
        return Ok(());
    }

    let hashes = hashes
        .await
        .inspect_err(|e| error!("Failed to get hashes: {e}"))
        .context("无法获取文件哈希")?;

    let font_installer = smol::spawn(install_font_if_needed(
        llc_config.clone(),
        game_root.clone(),
        hashes.font_hash,
    ));
    let cleaner = smol::spawn(cleanup_installed_llc(game_root.clone()));
    let downloader = smol::spawn(zeroasso::download_file::run(
        llc_config.clone(),
        format!("LimbusLocalize_{latest_version}.7z"),
        Some(hashes.main_hash),
    ));

    info!(
        "Updating LLC from version {} to {}.",
        installed_version, latest_version
    );

    utils::create_msgbox(
        "更新 LLC",
        &format!("将会更新 LLC 到版本 {latest_version}"),
        utils::IconType::Info,
    );

    cleaner
        .await
        .inspect_err(|e| error!("Failed to clean up installed files: {e}"))
        .context("无法清理已安装的文件")?;

    let buffer = downloader
        .await
        .inspect_err(|e| error!("Failed to download LLC: {e}"))
        .context("无法下载 LLC 文件")?;
    let reader = std::io::Cursor::new(buffer);
    sevenz_rust::decompress(reader, &game_root)
        .inspect_err(|e| error!("Failed to extract LLC files: {e}"))
        .context("无法解压 LLC 文件")?;

    font_installer
        .await
        .inspect_err(|e| error!("Failed to install font: {e}"))
        .context("无法安装字体")?;

    Ok(())
}

fn get_version_installed(game_root: &Path) -> eyre::Result<Option<u64>> {
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
    let version = serde_json::from_reader::<_, Version>(std::fs::File::open(version_file)?)
        .inspect_err(|e| error!("Failed to parse version file: {e}"))
        .context("无法解析版本文件")?
        .version;
    info!("Installed version: {version}");
    Ok(Some(version))
}

async fn cleanup_installed_llc(game_root: PathBuf) -> eyre::Result<()> {
    let llc_dir = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN");
    if !llc_dir.exists() {
        return Ok(());
    }
    let mut read_dir = smol::fs::read_dir(llc_dir).await?;
    while let Some(entry) = read_dir.try_next().await? {
        if entry.file_name() == "Font" {
            continue; // Skip the Font directory
        }
        let path = entry.path();
        if path.is_dir() {
            smol::fs::remove_dir_all(path).await?;
        } else if path.is_file() {
            smol::fs::remove_file(path).await?;
        } else {
            warn!("Found suspicious path: {:?}", path.display());
        }
    }
    Ok(())
}

async fn install_font_if_needed(
    lc_config: Arc<LLCConfig>,
    game_root: PathBuf,
    font_hash: [u8; 32],
) -> eyre::Result<()> {
    let font_file = game_root
        .join("LimbusCompany_Data")
        .join("Lang")
        .join("LLC_zh-CN")
        .join("Font")
        .join("Context")
        .join("ChineseFont.ttf");
    if font_file.exists() {
        info!("Font file is already installed and valid.");
            return Ok(());
    } else {
        info!("Font file does not exist, installing...");
    }

    let font_file =
        zeroasso::download_file::run(lc_config, "LLCCN-Font.7z".to_string(), Some(font_hash))
            .await?;
    let reader = std::io::Cursor::new(font_file);

    sevenz_rust::decompress(reader, &game_root)?;

    Ok(())
}
