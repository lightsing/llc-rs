//! Run self-update logic for the launcher.

use crate::config::LauncherConfig;
use directories::ProjectDirs;
use eyre::{Context, ContextCompat};
use flate2::read::GzDecoder;
use futures::{AsyncReadExt, FutureExt, StreamExt, stream::FuturesUnordered};
use nyquest::{AsyncClient, ClientBuilder, Request, r#async::Response};
use semver::Version;
use serde::Deserialize;
use std::{collections::BTreeMap, future::ready, path::Path, process::{Command, exit}};
use std::path::PathBuf;
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

pub async fn run(dirs: &ProjectDirs, self_path: PathBuf, config: LauncherConfig) -> eyre::Result<()> {
    let client = npm_client().await?;

    let self_version = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let (latest_version, tarball_url) = get_latest_version(&client, &config)
        .await
        .inspect_err(|e| error!("Failed to get latest version: {e}"))
        .context("无法获取最新版本信息，请检查网络连接。")?;

    let tool_path = dirs.cache_dir().join(EXECUTABLE_NAME);

    if self_version > latest_version {
        fs::copy(&self_path, &tool_path).await
            .inspect_err(|e| error!("Failed to copy self to tool path: {e}"))
            .context("无法更新启动器可执行文件")?;
        launch_tool(&tool_path, &self_path)
    }

    download_update(&client, &dirs, tarball_url).await?;
    launch_tool(&tool_path, &self_path)
}

fn launch_tool(tool_path: &Path, self_path: &Path) -> ! {
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    info!("Launching tool at: {}", tool_path.display());
    Command::new(tool_path)
        .args(args)
        .env(
            "LLC_LAUNCHER_PATH",
            self_path,
        )
        .spawn()
        .inspect_err(|e| error!("Failed to launch tool: {e}"))
        .ok();

    exit(0);
}

async fn download_update(client: &AsyncClient, dirs: &ProjectDirs, url: Url) -> eyre::Result<()> {
    let res = client
        .request(Request::get(url.to_string()))
        .await
        .inspect_err(|e| error!("Failed to download update package: {e}"))
        .context("无法下载更新包")?;

    let buffer_size = res.content_length().unwrap_or(1024 * 1024 * 2); // default to 2 MiB buffer
    let mut buffer = Vec::with_capacity(buffer_size as usize);
    res.into_async_read()
        .read_to_end(&mut buffer)
        .await
        .context("无法读取更新包内容")?;
    let tar = GzDecoder::new(buffer.as_slice());
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

#[instrument(skip(client, config), ret)]
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
    let mut res: Metadata = request_npm(client, config, PKG_NAME).await?.json().await?;
    let latest = res.dist_tags.latest;
    let tarball_url = res
        .versions
        .remove(&latest)
        .map(|v| v.dist.tarball)
        .context("NPM 中未找到最新版本的 tarball URL")
        .inspect_err(|e| error!("Failed to get tarball url: {e}"))?;
    Ok((latest, tarball_url))
}

async fn request_npm(
    client: &AsyncClient,
    config: &LauncherConfig,
    path: &str,
) -> eyre::Result<Response> {
    let registries = config.npm_registries();

    let (_, res) = registries
        .iter()
        .map(|base_url| base_url.join(path).expect("infallible"))
        .enumerate()
        .map(|(idx, url)| {
            client
                .request(Request::get(url.to_string()))
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

async fn npm_client() -> eyre::Result<AsyncClient> {
    ClientBuilder::default()
        .user_agent("npm/10.2.3 node/v20.11.1 win32 x64 workspaces/false ci/false")
        .with_header(
            "Accept",
            "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*",
        )
        .with_header("Accept-Encoding", "gzip, deflate, br")
        .with_header("Accept-Language", "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7")
        .with_header("npm-in-ci", "false")
        .with_header("npm-scope", "@lightsing")
        .no_caching()
        .build_async()
        .await
        .context("无法创建 npm 客户端")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_latest_version() {
        let config = LauncherConfig::default();
        let client = npm_client().await.unwrap();

        let version = get_latest_version(&client, &config).await;
        println!(
            "[test_get_latest_version] Latest version: {:?}",
            version.unwrap()
        );
    }

    #[tokio::test]
    async fn test_download_update() {
        let dirs = ProjectDirs::from("com", "lightsing", "llc-launcher-rs").unwrap();
        let config = LauncherConfig::default();
        let client = npm_client().await.unwrap();

        let (_version, url) = get_latest_version(&client, &config).await.unwrap();
        // // Ensure the tool path is clean before testing
        // if tool_path.exists() {
        //     std::fs::remove_file(&tool_path).unwrap();
        // }

        let result = download_update(&client, &dirs, url).await;
        assert!(result.is_ok());
        // assert!(tool_path.exists());
    }
}
