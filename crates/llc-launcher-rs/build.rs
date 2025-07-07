#[cfg(target_os = "windows")]
fn main() {
    set_rustc_version_env();

    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.compile().unwrap();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    set_rustc_version_env();
}

fn set_rustc_version_env() {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("Failed to run rustc");

    println!(
        "cargo:rustc-env=RUSTC_VERSION={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
