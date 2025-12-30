//! Run self-update logic for the launcher.

use directories::ProjectDirs;
use eyre::Context;
use flate2::read::GzDecoder;

use llc_rs::{LLCConfig, npm::NpmClient};
use semver::Version;
use smol::fs;
use std::{
    path::Path,
    process::{Command, exit},
};

#[cfg(target_os = "windows")]
const PKG_NAME: &str = "@lightsing/llc-launcher-rs-win32";
#[cfg(target_os = "linux")]
const PKG_NAME: &str = "@lightsing/llc-launcher-rs-linux";

#[cfg(target_os = "windows")]
const EXECUTABLE_NAME: &str = "llc-launcher-rs.exe";
#[cfg(target_os = "linux")]
const EXECUTABLE_NAME: &str = "llc-launcher-rs";

pub async fn run(dirs: &ProjectDirs, self_path: &Path, config: &LLCConfig) -> eyre::Result<()> {
    let client = NpmClient::new(config.npm_registries());

    let self_version = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let latest = client
        .get_lastest_version(PKG_NAME)
        .await
        .inspect_err(|e| error!("Failed to get latest version: {e}"))
        .context("无法获取最新版本信息，请检查网络连接。")?;

    let tool_path = dirs.cache_dir().join(EXECUTABLE_NAME);

    if self_version >= latest.version {
        info!("Current version is up-to-date: {}", self_version);
        fs::copy(&self_path, &tool_path)
            .await
            .inspect_err(|e| error!("Failed to copy self to tool path: {e}"))
            .context("无法更新启动器可执行文件")?;
        info!(
            "Copied self({}) to tool path: {}",
            self_path.display(),
            tool_path.display()
        );
        launch_tool(&tool_path, &self_path)
    }

    info!(
        "Current version: {}, Latest version: {}",
        self_version, latest.version
    );
    let tarball = client
        .download_dist(latest.dist)
        .await
        .inspect_err(|e| error!("failed to download tarball: {e}"))
        .context("无法下载更新包")?;

    extract_update(tarball, dirs).await?;
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

#[instrument(skip(tarball, dirs))]
async fn extract_update(tarball: Vec<u8>, dirs: &ProjectDirs) -> eyre::Result<()> {
    let tar = GzDecoder::new(tarball.as_slice());
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
            filetime::set_file_atime(&path, file_time)
                .and_then(|_| filetime::set_file_mtime(&path, file_time))
                .inspect_err(|e| error!("Failed to set file modification time: {e}"))
                .context("无法设置文件时间")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_macros::test;

    test! {
        async fn test_get_latest_version() {
            let config = LLCConfig::default();
            let client = NpmClient::new(&config.npm_registries());

            let version = client.get_lastest_version(PKG_NAME).await.unwrap();
            println!("[test_get_latest_version] Latest version: {version:?}");
        }
    }

    test! {
        async fn test_download_update() {
            let config = LLCConfig::default();
            let client = NpmClient::new(&config.npm_registries());
            let dirs = ProjectDirs::from("com", "lightsing", "llc-launcher-rs").unwrap();

            let version = client.get_lastest_version(PKG_NAME).await.unwrap();
            let tarball = client.download_dist(version.dist).await.unwrap();
            extract_update(tarball, &dirs).await.unwrap();
        }
    }
}
