#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(clippy::unwrap_used)]

#[macro_use]
extern crate tracing;

use std::path::PathBuf;

mod config;
pub use config::LLCConfig;

mod steam_support;
pub mod utils;
pub mod zeroasso;

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

    #[cfg(target_os = "linux")]
    use nyquest_backend_curl as nyquest_backend;
    #[cfg(target_os = "windows")]
    use nyquest_backend_winrt as nyquest_backend;

    nyquest_backend::register();
}
