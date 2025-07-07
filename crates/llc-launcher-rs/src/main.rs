#![feature(exit_status_error)]
#![cfg_attr(
    all(target_os = "windows", debug_assertions),
    windows_subsystem = "windows"
)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

#[macro_use]
extern crate tracing;

use crate::config::LauncherConfig;
use directories::ProjectDirs;
use eyre::{Context, ContextCompat};
use llc_rs::LLCConfig;
use std::{fs, process::exit};

const ORGANIZATION: &str = "lightsing";
const APP_NAME: &str = "llc-launcher-rs";

mod config;
mod llc;
mod logging;
mod self_update;
mod utils;
pub mod zeroasso;

#[ctor::ctor]
fn setup() {
    nyquest_preset::register();
}

#[cfg(test)]
#[ctor::ctor]
fn setup_test() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")),
        )
        .init();
}

#[tokio::main]
async fn main() {
    let (dirs, is_tool, (config, llc_config), _logging_guard) = match init() {
        Ok((dirs, is_tool, (config, llc_config), logging_guard)) => {
            (dirs, is_tool, (config, llc_config), logging_guard)
        }
        Err(e) => {
            eprintln!("{e}");
            utils::create_msgbox(
                "启动器出错了！",
                &format!("无法初始化：{e}。"),
                utils::IconType::Error,
            );
            exit(-1);
        }
    };

    if let Err(e) = if is_tool {
        llc::run(llc_config).await
    } else {
        self_update::run(&dirs, config).await
    } {
        error!("{e:?}");
        utils::create_msgbox(
            "启动器崩溃了！",
            &format!(
                "{e}\n请检查日志文件（位于 {}）以获取更多信息。",
                dirs.data_dir().join("logs").display()
            ),
            utils::IconType::Error,
        );
    }
}

fn init() -> eyre::Result<(
    ProjectDirs,
    bool,
    (LauncherConfig, LLCConfig),
    Option<logging::LoggingGuard>,
)> {
    let dirs = ProjectDirs::from("me", ORGANIZATION, APP_NAME).context("无法侦测用户目录")?;

    fs::create_dir_all(dirs.cache_dir()).context("无法创建缓存目录")?;
    fs::create_dir_all(dirs.config_dir()).context("无法创建配置目录")?;
    fs::create_dir_all(dirs.data_dir()).context("无法创建数据目录")?;

    let self_path = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .context("无法获取当前可执行文件路径")?;
    let cache_dir = dirs
        .cache_dir()
        .canonicalize()
        .context("无法获取缓存目录路径")?;

    let is_tool = self_path.starts_with(&cache_dir);

    let (config, llc_config) = config::load(&dirs);
    let logging_guard = logging::init(&dirs, &config);

    config::save(&dirs, &config, &llc_config)
        .inspect_err(|e| warn!("无法保存配置：{e}"))
        .context("无法保存配置")?;

    Ok((dirs, is_tool, (config, llc_config), logging_guard))
}
