use std::{ffi::OsString, io, path::PathBuf};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

/// Retrieves the Steam installation root directory from the Windows registry.
pub fn get_steam_root() -> io::Result<PathBuf> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let steam = hkcu.open_subkey(r"Software\Valve\Steam")?;
    let path_str: OsString = steam.get_value("SteamPath")?;
    Ok(PathBuf::from(path_str))
}

/// Launch game via steam url
pub fn launch_game_via_steam(app_id: u32) -> io::Result<()> {
    let steam_root = get_steam_root()?;
    let steam_exe = steam_root.join("steam.exe");

    if !steam_exe.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Steam executable not found",
        ));
    }

    let steam_url = format!("steam://rungameid/{app_id}");
    std::process::Command::new(steam_exe)
        .arg(steam_url)
        .spawn()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_steam_root() {
        println!("{:?}", get_steam_root().unwrap());
    }
}
