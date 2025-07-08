use crate::steam_support::SteamSupportError;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Finds the installation path for a game given its app ID.
pub fn find_game_path_for_app(
    steam_root: impl AsRef<Path>,
    app_id: u32,
) -> Result<PathBuf, SteamSupportError> {
    // Get Steam Libraries
    #[derive(Debug, Deserialize)]
    struct LibraryFolders {
        libraryfolders: Vec<Library>,
    }
    #[derive(Debug, Deserialize)]
    struct Library {
        path: String,
        apps: BTreeMap<u32, usize>,
    }

    let steam_root = steam_root
        .as_ref()
        .join("steamapps")
        .join("libraryfolders.vdf");
    let vdf_content = std::fs::read_to_string(steam_root)?;
    let libraries: LibraryFolders = vdf_reader::from_str(&vdf_content)?;

    // Find the library path for the given app ID
    let library_path = libraries
        .libraryfolders
        .iter()
        .find_map(|library| {
            if library.apps.contains_key(&app_id) {
                Some(Path::new(&library.path))
            } else {
                None
            }
        })
        .ok_or(SteamSupportError::AppNotFound(app_id))?;

    // Check if the app manifest exists in the library path
    #[derive(Debug, Deserialize)]
    struct AppSateDe {
        #[serde(rename = "AppState")]
        app_state: AppSate,
    }
    #[derive(Debug, Deserialize)]
    struct AppSate {
        installdir: String,
    }

    let steam_apps = library_path.join("steamapps");
    let app_state_path = steam_apps.join(format!("appmanifest_{app_id}.acf"));

    if !app_state_path.exists() {
        return Err(SteamSupportError::AppNotFound(app_id));
    }

    // Read the app state file
    let app_state_content = std::fs::read_to_string(app_state_path)?;
    let app_state: AppSateDe = vdf_reader::from_str(&app_state_content)?;

    let game_path = steam_apps
        .join("common")
        .join(app_state.app_state.installdir);
    if !game_path.exists() {
        return Err(SteamSupportError::AppNotFound(app_id));
    }

    Ok(game_path)
}

#[cfg(test)]
mod tests {
    use super::{super::imp, *};
    use crate::LIMBUS_COMPANY_STEAM_APP_ID;

    #[test]
    fn test_find_game_path_for_app() {
        let steam_root = imp::get_steam_root().unwrap();
        let game_path = find_game_path_for_app(&steam_root, LIMBUS_COMPANY_STEAM_APP_ID).unwrap();
        assert!(game_path.exists());
        println!("game_path: {:?}", game_path);
    }
}
