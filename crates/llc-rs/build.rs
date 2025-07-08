fn main() {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("Failed to run rustc");

    println!(
        "cargo:rustc-env=RUSTC_VERSION={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
