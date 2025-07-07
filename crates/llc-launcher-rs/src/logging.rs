use crate::{config::LauncherConfig, utils};
use directories::ProjectDirs;
use eyre::Context;
use std::fs;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{self, Rotation},
};
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub struct LoggingGuard {
    _file_appender_guard: WorkerGuard,
    _stderr_appender_guard: WorkerGuard,
}

pub fn init(dirs: &ProjectDirs, config: &LauncherConfig) -> Option<LoggingGuard> {
    match init_inner(dirs, config) {
        Ok(guard) => Some(guard),
        Err(e) => {
            eprintln!("{e}");
            utils::create_msgbox(
                "启动器出错了！",
                &format!(
                    "无法初始化日志系统：{e}。\n启动器仍然会继续运行，但日志将会无法记录，如果后续发生错误，将无法提供帮助。"
                ),
                utils::IconType::Error,
            );
            None
        }
    }
}

fn init_inner(dirs: &ProjectDirs, config: &LauncherConfig) -> eyre::Result<LoggingGuard> {
    let log_dir = dirs.data_dir().join("logs");
    fs::create_dir_all(&log_dir)
        .inspect_err(|e| eprintln!("failed to create log directory: {e}"))
        .context("无法创建日志目录")?;
    eprintln!("logging to {}", log_dir.display());

    let file_appender = rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .max_log_files(10)
        .filename_suffix("log")
        .build(&log_dir)
        .inspect_err(|e| eprintln!("failed to create file appender: {e}"))
        .context("无法创建日志输出器")?;

    let (non_blocking_file_appender, file_appender_guard) =
        tracing_appender::non_blocking(file_appender);
    let (non_blocking_stderr_appender, stderr_appender_guard) =
        tracing_appender::non_blocking(std::io::stderr());

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_writer(non_blocking_file_appender.with_max_level(config.log_level())),
        )
        .with(
            fmt::Layer::new()
                .with_writer(non_blocking_stderr_appender.with_max_level(config.log_level())),
        )
        .init();

    Ok(LoggingGuard {
        _file_appender_guard: file_appender_guard,
        _stderr_appender_guard: stderr_appender_guard,
    })
}
