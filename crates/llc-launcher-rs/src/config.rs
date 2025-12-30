use crate::utils;
use directories::ProjectDirs;
use eyre::Context;
use llc_rs::{LLCConfig, utils::ResultExt};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::{fs, path::Path};
use uuid::Uuid;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default = "Uuid::new_v4")]
    uuid: Uuid,
    #[serde_as(as = "DisplayFromStr")]
    log_level: tracing::Level,
    #[serde(default = "default_true")]
    telemetry: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            log_level: tracing::Level::INFO,
            telemetry: true,
        }
    }
}

impl LauncherConfig {
    #[inline]
    #[allow(dead_code)]
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    #[inline]
    #[allow(dead_code)]
    pub fn log_level(&self) -> tracing::Level {
        self.log_level
    }

    #[inline]
    #[allow(dead_code)]
    pub fn telemetry(&self) -> bool {
        self.telemetry
    }
}

pub fn load(dirs: &ProjectDirs) -> (LauncherConfig, LLCConfig) {
    match load_inner(dirs.config_dir()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{e}");
            utils::create_msgbox(
                "启动器出错了！",
                &format!(
                    "无法加载配置文件：{e}。\n启动器将会退出。\n如果不知道如何解决，请删除如下目录：{}",
                    dirs.config_dir().display()
                ),
                utils::IconType::Error,
            );
            std::process::exit(-1);
        }
    }
}

pub fn save(
    dirs: &ProjectDirs,
    config: &LauncherConfig,
    llc_config: &LLCConfig,
) -> eyre::Result<()> {
    let config_dir = dirs.config_dir();
    fs::create_dir_all(config_dir)
        .inspect_err(|e| eprintln!("failed to create config dir: {e}"))
        .context("无法创建配置目录")?;

    save_config(&config_dir.join("config.toml"), config).context("无法保存启动器配置文件")?;
    save_config(&config_dir.join("llc_config.toml"), llc_config)
        .context("无法保存 LLC 配置文件")?;

    Ok(())
}

fn load_inner(config_dir: &Path) -> eyre::Result<(LauncherConfig, LLCConfig)> {
    fs::create_dir_all(config_dir)
        .inspect_err(|e| eprintln!("failed to create config dir: {e}"))
        .context("无法创建配置目录")?;

    let config: LauncherConfig = load_config_or_default(&config_dir.join("config.toml"))
        .context("无法加载或创建启动器配置文件")?;
    let llc_config: LLCConfig = load_config_or_default(&config_dir.join("llc_config.toml"))
        .context("无法加载或创建 LLC 配置文件")?;

    Ok((config, llc_config))
}

fn load_config_or_default<T: Default + Serialize + for<'de> Deserialize<'de>>(
    path: &Path,
) -> eyre::Result<T> {
    if !path.exists() {
        let config = T::default();
        fs::write(path, toml::to_string_pretty(&config).infallible())
            .inspect_err(|e| eprintln!("failed to write config file: {e}"))
            .context("无法写入配置文件")?;
        return Ok(config);
    }

    let content = fs::read_to_string(path)
        .inspect_err(|e| eprintln!("failed to read config file: {e}"))
        .context("无法读取配置文件")?;
    let config: T = toml::from_str(&content)
        .inspect_err(|e| eprintln!("failed to parse config file: {e}"))
        .context("无法解析配置文件")?;
    Ok(config)
}

fn save_config<T: Serialize>(path: &Path, config: &T) -> eyre::Result<()> {
    fs::write(path, toml::to_string_pretty(config).infallible())
        .inspect_err(|e| eprintln!("failed to write config file: {e}"))
        .context("无法写入配置文件")
}

const fn default_true() -> bool {
    true
}
