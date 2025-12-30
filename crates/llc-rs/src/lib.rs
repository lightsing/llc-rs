#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(clippy::unwrap_used)]

#[macro_use]
extern crate tracing;

use crate::utils::OptionExt;
use nyquest::{AsyncClient, ClientBuilder};
use std::{
    path::PathBuf,
    sync::{LazyLock, OnceLock},
};

mod config;
pub use config::LLCConfig;

mod steam_support;
pub mod utils;
// pub mod zeroasso;
pub mod npm;

pub use steam_support::{
    SteamSupportError, find_game_path_for_app, get_steam_root, launch_game_via_steam,
};

/// The Steam App ID for the Limbus Company.
pub const LIMBUS_COMPANY_STEAM_APP_ID: u32 = 1973530;

/// Get the installation path for Limbus Company, resolved at runtime.
pub fn get_limbus_company_install_path() -> Result<PathBuf, SteamSupportError> {
    let steam_root = get_steam_root()?;
    find_game_path_for_app(&steam_root, LIMBUS_COMPANY_STEAM_APP_ID)
}

/// Launch Limbus Company via Steam.
pub fn launch_limbus_company() -> Result<(), SteamSupportError> {
    launch_game_via_steam(LIMBUS_COMPANY_STEAM_APP_ID)?;
    Ok(())
}

pub static USER_AGENT: LazyLock<&str> = LazyLock::new(|| {
    let os_info = os_info::get();
    Box::leak(format!(
        "{pkg}/{ver} ({os} {os_ver}; {rustc}; {arch}; +{homepage}) nyquest/0.4 (+https://github.com/bdbai/nyquest)",
        pkg = env!("CARGO_PKG_NAME"),
        ver = env!("CARGO_PKG_VERSION"),
        os = os_info.os_type(),
        os_ver = os_info.version(),
        rustc = env!("RUSTC_VERSION"),
        arch = std::env::consts::ARCH,
        homepage = env!("CARGO_PKG_HOMEPAGE"),
    ).into_boxed_str())
});

pub async fn get_client() -> nyquest::Result<&'static AsyncClient> {
    static CLIENT: OnceLock<AsyncClient> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }

    let client = ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .with_header("FROM", "ligh.tsing@gmail.com")
        .no_caching()
        .build_async()
        .await
        .inspect_err(|e| error!("Failed to initialize client: {e}"))?;

    CLIENT.set(client).ok();
    Ok(CLIENT.get().infallible())
}

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

    nyquest_preset::register();
}
