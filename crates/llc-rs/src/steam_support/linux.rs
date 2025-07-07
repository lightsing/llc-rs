use std::{io, path::PathBuf};

/// Retrieves the Steam installation root directory from the Windows registry.
pub fn get_steam_root() -> io::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME environment variable not set"))?;
    let home = PathBuf::from(home).canonicalize()?;
    let candidates = [
        ".steam/steam",
        ".local/share/Steam",
        ".var/app/com.valvesoftware.Steam/.steam/steam"
    ];
    for path in candidates.iter() {
        let full = home.join(path);
        if full.exists() {
            return Ok(full)
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Steam installation root not found",
    ))
}

/// Launch game via steam url
pub fn launch_game_via_steam(app_id: u32) -> io::Result<()> {
    let steam_root = get_steam_root()?;
    let steam_sh = steam_root.join("steam.sh");

    if !steam_sh.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Steam launcher not found",
        ));
    }

    let steam_url = format!("steam://rungameid/{app_id}");
    std::process::Command::new("sh")
        .arg(steam_sh)
        .arg(steam_url)
        .spawn()?;

    Ok(())
}
