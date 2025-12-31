use crate::{config::LauncherConfig, utils};
use aho_corasick::{AhoCorasick, AhoCorasickKind, Anchored, Input, StartKind};
use directories::ProjectDirs;
use eyre::Context;
use std::fs;
use tokio::task::JoinHandle;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{self, Rotation},
};
use tracing_subscriber::{
    filter::filter_fn, fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt,
    util::SubscriberInitExt,
};
#[derive(Default)]
pub(crate) struct LoggingGuard {
    _file_appender_guard: Option<WorkerGuard>,
    _stderr_appender_guard: Option<WorkerGuard>,
    pub(crate) sls_reporter: Option<JoinHandle<()>>,
}

pub async fn init(
    dirs: &ProjectDirs,
    config: &LauncherConfig,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> LoggingGuard {
    init_inner(dirs, config, shutdown_rx).await.unwrap_or_else(|e| {
        eprintln!("{e}");
        utils::create_msgbox(
            "启动器出错了！",
            &format!(
                "无法初始化日志系统：{e}。\n启动器仍然会继续运行，但日志将会无法记录，如果后续发生错误，将无法提供帮助。"
            ),
            utils::IconType::Error,
        );
        LoggingGuard::default()
    })
}

async fn init_inner(
    dirs: &ProjectDirs,
    config: &LauncherConfig,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> eyre::Result<LoggingGuard> {
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

    let nosie_targets = AhoCorasick::builder()
        .kind(Some(AhoCorasickKind::DFA))
        .start_kind(StartKind::Anchored)
        .build(&[
            "async_io",
            "polling",
            "react",
            "hyper",
            "aliyun_sls",
            "winit",
            "zbus",
        ])?;
    let nosie_filter = filter_fn(move |metadata| {
        let target = metadata.target();
        let input = Input::new(target).anchored(Anchored::Yes);
        if nosie_targets.find(input).is_some() && metadata.level() <= &tracing::Level::INFO {
            return false;
        }
        true
    });

    let layered = tracing_subscriber::registry()
        .with(nosie_filter)
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
        );

    #[cfg(not(debug_assertions))]
    let sls_reporter = if config.telemetry() {
        let client = tracing_aliyun_sls::SlsClient::builder()
            .endpoint(env!("ALIYUN_SLS_ENDPOINT"))
            .access_key(env!("ALIYUN_SLS_ACCESS_KEY"))
            .access_secret(env!("ALIYUN_SLS_ACCESS_SECRET"))?
            .project(env!("ALIYUN_SLS_PROJECT"))
            .logstore(env!("ALIYUN_SLS_LOGSTORE"))
            .enable_trace(true)
            .build()
            .inspect_err(|e| eprintln!("failed to create sls client: {e}"))
            .context("无法创建 SLS 客户端")?;
        let reporter = tracing_aliyun_sls::reporter::Reporter::from_client(client);

        let sls_reporter = tokio::spawn(
            reporter
                .clone()
                .reporting(|| async {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                })
                .await
                .unwrap()
                .with_graceful_shutdown(async move {
                    let mut shutdown_rx = shutdown_rx;
                    shutdown_rx.recv().await.ok();
                })
                .with_vec_pool_capacity(128)
                .with_log_group_capacity(128)
                .with_log_vec_capacity(128)
                .start(),
        );

        layered
            .with(tracing_aliyun_sls::layer(reporter).with_instance_id(config.uuid().to_string()))
            .init();
        Some(sls_reporter)
    } else {
        layered.init();
        None
    };

    #[cfg(debug_assertions)]
    let sls_reporter = {
        let _ = shutdown_rx;
        layered.init();
        None
    };

    Ok(LoggingGuard {
        _file_appender_guard: Some(file_appender_guard),
        _stderr_appender_guard: Some(stderr_appender_guard),
        sls_reporter,
    })
}
