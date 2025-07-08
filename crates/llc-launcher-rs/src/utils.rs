#[derive(Debug, Copy, Clone)]
pub enum IconType {
    Error,
    Info,
}

#[cfg(target_os = "windows")]
pub fn create_msgbox(title: &str, content: &str, icon_type: IconType) {
    let icon_type = match icon_type {
        IconType::Error => msgbox::IconType::Error,
        IconType::Info => msgbox::IconType::Info,
    };
    if let Err(e) = msgbox::create(title, content, icon_type) {
        eprintln!("Failed to create message box: {e}");
    }
}

#[cfg(not(target_os = "windows"))]
pub fn create_msgbox(_title: &str, _content: &str, _icon_type: IconType) {}
