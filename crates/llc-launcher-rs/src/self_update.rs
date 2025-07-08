//! Run self-update logic for the launcher.

use crate::{config::LauncherConfig};
use directories::ProjectDirs;
use eyre::{Context, ContextCompat};
use flate2::read::GzDecoder;
use nyquest::{AsyncClient, ClientBuilder};
use semver::Version;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, exit},
};
use smol::fs;
use url::Url;
use llc_rs::utils::{ClientExt, ResultExt};

#[cfg(target_os = "windows")]
const PKG_NAME: &str = "@lightsing/llc-launcher-rs-win32";
#[cfg(target_os = "linux")]
const PKG_NAME: &str = "@lightsing/llc-launcher-rs-linux";

#[cfg(target_os = "windows")]
const EXECUTABLE_NAME: &str = "llc-launcher-rs.exe";
#[cfg(target_os = "linux")]
const EXECUTABLE_NAME: &str = "llc-launcher-rs";

pub async fn run(
    dirs: &ProjectDirs,
    self_path: PathBuf,
    config: LauncherConfig,
) -> eyre::Result<()> {
    let client = create_client().await?;

    let self_version = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let (latest_version, tarball_url) = get_latest_version(&client, &config)
        .await
        .inspect_err(|e| error!("Failed to get latest version: {e}"))
        .context("无法获取最新版本信息，请检查网络连接。")?;

    let tool_path = dirs.cache_dir().join(EXECUTABLE_NAME);

    if self_version >= latest_version {
        fs::copy(&self_path, &tool_path)
            .await
            .inspect_err(|e| error!("Failed to copy self to tool path: {e}"))
            .context("无法更新启动器可执行文件")?;
        launch_tool(&tool_path, &self_path)
    }

    download_and_extract_update(&client, dirs, tarball_url).await?;
    launch_tool(&tool_path, &self_path)
}

fn launch_tool(tool_path: &Path, self_path: &Path) -> ! {
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    info!("Launching tool at: {}", tool_path.display());
    Command::new(tool_path)
        .args(args)
        .env("LLC_LAUNCHER_PATH", self_path)
        .spawn()
        .inspect_err(|e| error!("Failed to launch tool: {e}"))
        .ok();

    exit(0);
}

#[instrument(skip(client, dirs))]
async fn download_and_extract_update(
    client: &AsyncClient,
    dirs: &ProjectDirs,
    url: Url,
) -> eyre::Result<()> {
    let res = client
        .download(url)
        .await
        .inspect_err(|e| error!("failed to download update package: {e}"))
        .context("无法下载更新包")?;

    let tar = GzDecoder::new(res.as_slice());
    let mut archive = tar::Archive::new(tar);
    for file in archive
        .entries()
        .inspect_err(|e| error!("Failed to read archive: {e}"))
        .context("无法读取更新包条目")?
    {
        let mut file = file
            .inspect_err(|e| error!("Failed to read archive entry: {e}"))
            .context("无法获取更新包条目")?;
        if file
            .path()
            .inspect_err(|e| error!("Failed to get entry path: {e}"))
            .context("无法获取更新包条目路径")?
            .ends_with(EXECUTABLE_NAME)
        {
            let path = dirs.cache_dir().join(EXECUTABLE_NAME);
            file.unpack(&path)
                .inspect_err(|e| error!("Failed to unpack entry: {e}"))
                .context("无法解压更新包条目")?;
            let file_time = filetime::FileTime::now();
            filetime::set_file_mtime(&path, file_time)
                .and_then(|_| filetime::set_file_mtime(&path, file_time))
                .inspect_err(|e| error!("Failed to set file modification time: {e}"))
                .context("无法设置文件时间")?;
        }
    }
    Ok(())
}

#[instrument(skip(config), ret)]
async fn get_latest_version(
    client: &AsyncClient,
    config: &LauncherConfig,
) -> eyre::Result<(Version, Url)> {
    #[derive(Deserialize)]
    struct Metadata {
        #[serde(rename = "dist-tags")]
        dist_tags: DistTags,
        versions: BTreeMap<Version, VersionMetadata>,
    }
    #[derive(Deserialize)]
    struct DistTags {
        latest: Version,
    }
    #[derive(Deserialize)]
    struct VersionMetadata {
        dist: DistInfo,
    }
    #[derive(Deserialize)]
    struct DistInfo {
        tarball: Url,
    }

    let registries = config.npm_registries();

    let mut res: Metadata = client
        .get_json(
            registries
                .iter()
                .map(|base_url| base_url.join(PKG_NAME).infallible()),
        )
        .await?;
    let latest = res.dist_tags.latest;
    let tarball_url = res
        .versions
        .remove(&latest)
        .map(|v| v.dist.tarball)
        .context("NPM 中未找到最新版本的 tarball URL")
        .inspect_err(|e| error!("Failed to get tarball url: {e}"))?;
    Ok((latest, tarball_url))
}

async fn create_client() -> eyre::Result<AsyncClient> {
    ClientBuilder::default()
        .user_agent("npm/10.2.3 node/v20.11.1 win32 x64 workspaces/false ci/false")
        .with_header(
            "Accept",
            "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*",
        )
        .with_header("Accept-Charset", "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7")
        .with_header("npm-in-ci", "false")
        .with_header("npm-scope", "@lightsing")
        .build_async()
        .await
        .inspect_err(|e| error!("Failed to create NPM client: {e}"))
        .context("无法创建 NPM 客户端")
}

#[cfg(test)]
mod tests {
    use smol_macros::test;
    use super::*;

    test! {
        async fn test_get_latest_version() {
            let client = create_client().await.unwrap();
            let config = LauncherConfig::default();

            let version = get_latest_version(&client, &config).await;
            println!(
                "[test_get_latest_version] Latest version: {:?}",
                version.unwrap()
            );
        }
    }


    test! {
        async fn test_download_update() {
            let client = create_client().await.unwrap();
            let dirs = ProjectDirs::from("com", "lightsing", "llc-launcher-rs").unwrap();
            let config = LauncherConfig::default();

            let (_version, url) = get_latest_version(&client, &config).await.unwrap();
            let result = download_and_extract_update(&client, &dirs, url).await;
            assert!(result.is_ok());
        }
    }
}
