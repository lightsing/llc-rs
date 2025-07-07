//! Run self-update logic for the launcher.

use crate::config::LauncherConfig;
use directories::ProjectDirs;
use eyre::{Context, ContextCompat};
use flate2::read::GzDecoder;
use futures::{FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use reqwest::{
    Client, ClientBuilder, Response, header,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use semver::Version;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    future::ready,
    path::{Path, PathBuf},
    process::{Command, exit},
    sync::LazyLock,
};
use tokio::fs;
use url::Url;

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
    let self_version = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let (latest_version, tarball_url) = get_latest_version(&config)
        .await
        .inspect_err(|e| error!("Failed to get latest version: {e}"))
        .context("无法获取最新版本信息，请检查网络连接。")?;

    let tool_path = dirs.cache_dir().join(EXECUTABLE_NAME);

    if self_version > latest_version {
        fs::copy(&self_path, &tool_path)
            .await
            .inspect_err(|e| error!("Failed to copy self to tool path: {e}"))
            .context("无法更新启动器可执行文件")?;
        launch_tool(&tool_path, &self_path)
    }

    download_and_extract_update(dirs, tarball_url).await?;
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

async fn download_and_extract_update(dirs: &ProjectDirs, url: Url) -> eyre::Result<()> {
    let res = NPM_CLIENT
        .get(url)
        .send()
        .map(|r| r.and_then(|res| res.error_for_status()))
        .and_then(|res| res.bytes())
        .await
        .inspect_err(|e| error!("failed to download update package: {e}"))
        .context("无法下载更新包")?;

    let tar = GzDecoder::new(res.as_ref());
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
            file.unpack(dirs.cache_dir().join(EXECUTABLE_NAME))
                .inspect_err(|e| error!("Failed to unpack entry: {e}"))
                .context("无法解压更新包条目")?;
        }
    }
    Ok(())
}

#[instrument(skip(config), ret)]
async fn get_latest_version(config: &LauncherConfig) -> eyre::Result<(Version, Url)> {
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
    let mut res: Metadata = request_npm(config, PKG_NAME).await?.json().await?;
    let latest = res.dist_tags.latest;
    let tarball_url = res
        .versions
        .remove(&latest)
        .map(|v| v.dist.tarball)
        .context("NPM 中未找到最新版本的 tarball URL")
        .inspect_err(|e| error!("Failed to get tarball url: {e}"))?;
    Ok((latest, tarball_url))
}

async fn request_npm(config: &LauncherConfig, path: &str) -> eyre::Result<Response> {
    let registries = config.npm_registries();

    let (_, res) = registries
        .iter()
        .map(|base_url| base_url.join(path).expect("infallible"))
        .enumerate()
        .map(|(idx, url)| {
            NPM_CLIENT
                .get(url.clone())
                .send()
                .map(|r| r.and_then(|res| res.error_for_status()))
                .map(move |res| (idx, res.map(|v| (url, v))))
        })
        .collect::<FuturesUnordered<_>>()
        .skip_while(|(idx, res)| {
            ready(
                *idx != registries.len() - 1 // keep trying until the last node
                    && res.as_ref().inspect_err(|e| error!("{e}")).is_err(),
            )
        })
        .next()
        .await
        .expect("unreachable: should always yield at least one item");

    let (url, res) = res?;
    info!("request npm from {url}");
    Ok(res)
}

static NPM_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::default()
        .user_agent("npm/10.2.3 node/v20.11.1 win32 x64 workspaces/false ci/false")
        .default_headers(HeaderMap::from_iter([
            (
                header::ACCEPT,
                HeaderValue::from_static(
                    "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*",
                ),
            ),
            (
                header::ACCEPT_ENCODING,
                HeaderValue::from_static("gzip, deflate, br"),
            ),
            (
                header::ACCEPT_CHARSET,
                HeaderValue::from_static("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7"),
            ),
            (
                HeaderName::from_static("npm-in-ci"),
                HeaderValue::from_static("false"),
            ),
            (
                HeaderName::from_static("npm-scope"),
                HeaderValue::from_static("@lightsing"),
            ),
        ]))
        .brotli(true)
        .deflate(true)
        .gzip(true)
        .https_only(true)
        .build()
        .expect("Building NPM client failed")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_latest_version() {
        let config = LauncherConfig::default();

        let version = get_latest_version(&config).await;
        println!(
            "[test_get_latest_version] Latest version: {:?}",
            version.unwrap()
        );
    }

    #[tokio::test]
    async fn test_download_update() {
        let dirs = ProjectDirs::from("com", "lightsing", "llc-launcher-rs").unwrap();
        let config = LauncherConfig::default();

        let (_version, url) = get_latest_version(&config).await.unwrap();
        let result = download_and_extract_update(&dirs, url).await;
        assert!(result.is_ok());
    }
}
