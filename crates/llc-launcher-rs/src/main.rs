#![feature(once_cell_try)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![deny(clippy::unwrap_used)]

#[macro_use]
extern crate tracing;

use directories::ProjectDirs;
use eyre::Context;
use llc_rs::{LLCConfig, launch_limbus_company};
use std::process::exit;

const ORGANIZATION: &str = "lightsing";
const APP_NAME: &str = "llc-launcher-rs";

mod config;
mod installer;
mod logging;
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
    let Some(dirs) = ProjectDirs::from("me", ORGANIZATION, APP_NAME) else {
        eprintln!("无法初始化项目目录，无法侦测用户目录");
        msgbox::create(
            "启动器崩溃了！",
            "无法初始化项目目录，无法侦测用户目录。",
            msgbox::IconType::Error,
        )
        .ok();
        exit(-1);
    };

    let (config, llc_config) = config::load(&dirs);
    let _logging_guard = logging::init(&dirs, &config);

    if let Err(e) = inner(llc_config).await {
        error!("{e:?}");
        msgbox::create(
            "启动器崩溃了！",
            &format!(
                "无法启动 Limbus Company：{e}。\n请检查日志文件（位于 {}）以获取更多信息。",
                dirs.data_dir().join("logs").display()
            ),
            msgbox::IconType::Error,
        )
        .ok();
    }
}

async fn inner(llc_config: LLCConfig) -> eyre::Result<()> {
    installer::install_or_update_llc(llc_config).await?;
    launch_limbus_company()
        .inspect_err(|e| error!("cannot start Limbus Company: {e}"))
        .context("无法启动 Limbus Company")?;
    Ok(())
}
