#[cfg(target_os = "windows")]
fn main() {
    let package_version = env!("CARGO_PKG_VERSION");
    let mut version_parts = package_version
        .split_once('-')
        .map_or(package_version, |(core, _)| core)
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .collect::<Vec<_>>();
    version_parts.resize(4, 0);
    let windows_version = version_parts
        .iter()
        .take(4)
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(".");

    let mut res = winres::WindowsResource::new();
    res.set("FileVersion", &windows_version);
    res.set("ProductVersion", &windows_version);
    res.set("FileDescription", "Pealayer 4D Video & Haptic Player");
    res.set("ProductName", "Pealayer");
    res.set("InternalName", "pealayer");
    res.set("OriginalFilename", "pealayer.exe");
    res.set("CompanyName", "Pealayer");
    res.set("LegalCopyright", "Copyright © 2026 Pealayer Team");
    if std::path::Path::new("assets/icon.ico").exists() {
        res.set_icon("assets/icon.ico");
    }
    res.compile()
        .expect("failed to compile Pealayer Windows resources");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
