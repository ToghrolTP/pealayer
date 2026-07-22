#[cfg(target_os = "windows")]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set("FileVersion", "0.1.0.0");
    res.set("ProductVersion", "0.1.0.0");
    res.set("FileDescription", "Pealayer 4D Video & Haptic Player");
    res.set("ProductName", "Pealayer");
    res.set("LegalCopyright", "Copyright © 2026 Pealayer Team");
    if std::path::Path::new("assets/icon.ico").exists() {
        res.set_icon("assets/icon.ico");
    }
    let _ = res.compile();
}

#[cfg(not(target_os = "windows"))]
fn main() {}
