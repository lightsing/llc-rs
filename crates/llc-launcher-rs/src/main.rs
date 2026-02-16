#![feature(exit_status_error)]
#![feature(formatting_options)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

#[macro_use]
extern crate tracing;

use crate::config::LauncherConfig;
use directories::ProjectDirs;
use eframe::egui;
use eyre::{Context, ContextCompat};
use llc_rs::LLCConfig;
use std::{fs, path::PathBuf, process::exit};

const ORGANIZATION: &str = "lightsing";
const APP_NAME: &str = "llc-launcher-rs";

mod config;
mod llc;
mod logging;
mod self_update;
mod splash;
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

fn main() {
    utils::install_eyre_hook().expect("Failed to install eyre");

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_visible(false)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_always_on_top()
            .with_inner_size([1280.0, 800.0]),
        centered: true,
        ..Default::default()
    };

    let init_res = match init() {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{e}");
            shutdown_tx.send(()).ok();
            eframe::run_native(
                "Limbus Company Launcher",
                options,
                Box::new(|cc| Ok(Box::new(splash::SplashScreen::new(cc, false, shutdown_rx)))),
            )
            .expect("Failed to run the launcher splash screen");
            exit(-1);
        }
    };

    let is_tool = init_res.is_tool;
    let _shutdown_rx = shutdown_tx.subscribe();

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
            .block_on(main_inner(init_res, shutdown_tx, shutdown_rx))
    });

    eframe::run_native(
        "Limbus Company Launcher",
        options,
        Box::new(|cc| {
            Ok(Box::new(splash::SplashScreen::new(
                cc,
                is_tool,
                _shutdown_rx,
            )))
        }),
    )
    .expect("Failed to run the launcher splash screen");
}

async fn main_inner(
    InitResources {
        dirs,
        launcher_config,
        llc_config,
        self_path,
        is_tool,
    }: InitResources,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let logging_guard = logging::init(&dirs, &launcher_config, shutdown_rx).await;

    if is_tool {
        info!("Running as tool, path: {}", self_path.display());
    } else {
        info!("Running as launcher, path: {}", self_path.display());
    }

    if let Err(e) = {
        if is_tool {
            llc::run(llc_config.clone()).await
        } else {
            self_update::run(&dirs, &self_path, &llc_config).await
        }
    } {
        error!("{e:?}");
    }

    // for migration
    config::save(&dirs, &launcher_config, &llc_config)
        .inspect_err(|e| warn!("无法保存配置：{e}"))
        .ok();

    shutdown_tx.send(()).ok();
    if let Some(reporter) = logging_guard.sls_reporter {
        reporter.await.ok();
    }
}

#[allow(unused)]
pub struct InitResources {
    dirs: ProjectDirs,
    self_path: PathBuf,
    is_tool: bool,
    launcher_config: LauncherConfig,
    llc_config: LLCConfig,
}

fn init() -> eyre::Result<InitResources> {
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

    let is_tool = self_path.starts_with(&cache_dir) || cfg!(debug_assertions);

    let (launcher_config, llc_config) = config::load(&dirs);

    Ok(InitResources {
        dirs,
        self_path,
        is_tool,
        launcher_config,
        llc_config,
    })
}
