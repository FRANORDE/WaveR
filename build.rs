// Embeds the icon (and version info taken from Cargo.toml) into the .exe
#[cfg(windows)] // Only on Windows
fn main() {
    // Re-run this script whenever the icon changes
    println!("cargo:rerun-if-changed=assets/WaveR.ico");

    // Only runs when targeting Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/WaveR.ico");
        res.compile().expect("failed to embed the icon");
    }
}
