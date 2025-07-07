#[cfg(target_os = "windows")]
#[cfg_attr(docsrs, doc(cfg(target_os = "windows")))]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[cfg(target_os = "linux")]
#[cfg_attr(docsrs, doc(cfg(target_os = "windows")))]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

pub use imp::{get_steam_root, launch_game_via_steam};

mod common;
pub use common::find_game_path_for_app;

/// Steam support errors
#[derive(Debug, thiserror::Error)]
pub enum SteamSupportError {
    /// Io error
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Error reading vdf
    #[error("parse vdf: {0}")]
    Vdf(#[from] vdf_reader::error::VdfError),
    /// Given app ID not found in the Steam library
    #[error("steam app ({0}) not found")]
    AppNotFound(u32),
}
