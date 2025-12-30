#![feature(exit_status_error)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

#[macro_use]
extern crate tracing;

use crate::{config::LauncherConfig, logging::LoggingGuard};
use directories::ProjectDirs;
use eyre::{Context, ContextCompat};
use llc_rs::LLCConfig;
use std::{fs, path::PathBuf, process::exit};

const ORGANIZATION: &str = "lightsing";
const APP_NAME: &str = "llc-launcher-rs";

mod config;
mod llc;
mod logging;
mod self_update;
mod utils;

#[cfg(test)]
#[ctor::ctor]
fn setup_test() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")),
        )
        .with_file(true)
        .with_line_number(true)
        .init();
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let init_res = match init().await {
        Ok(res) => res,
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

    main_inner(&init_res).await;

    // for migration
    config::save(
        &init_res.dirs,
        &init_res.launcher_config,
        &init_res.llc_config,
    )
    .inspect_err(|e| warn!("无法保存配置：{e}"))
    .ok();

    init_res.shutdown_tx.send(()).ok();
    if let Some(reporter) = init_res.logging_guard.sls_reporter {
        reporter.await.unwrap();
    }
}

async fn main_inner(init_res: &InitResources) {
    if let Err(e) = {
        if init_res.is_tool {
            llc::run(&init_res.dirs, init_res.llc_config.clone()).await
        } else {
            self_update::run(&init_res.dirs, &init_res.self_path, &init_res.llc_config).await
        }
    } {
        error!("{e:?}");
        utils::create_msgbox(
            "启动器崩溃了！",
            &format!(
                "{e}\n请检查日志文件（位于 {}）以获取更多信息。",
                init_res.dirs.data_dir().join("logs").display()
            ),
            utils::IconType::Error,
        );
    }
}

pub struct InitResources {
    dirs: ProjectDirs,
    self_path: PathBuf,
    is_tool: bool,
    launcher_config: LauncherConfig,
    llc_config: LLCConfig,
    logging_guard: LoggingGuard,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

async fn init() -> eyre::Result<InitResources> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
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

    let (launcher_config, llc_config) = config::load(&dirs);
    let logging_guard = logging::init(&dirs, &launcher_config, shutdown_rx).await;

    if is_tool {
        info!("Running as tool, path: {}", self_path.display());
    } else {
        info!("Running as launcher, path: {}", self_path.display());
    }

    Ok(InitResources {
        dirs,
        self_path,
        is_tool,
        launcher_config,
        llc_config,
        logging_guard,
        shutdown_tx,
    })
}
