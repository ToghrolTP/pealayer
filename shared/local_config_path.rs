use std::path::{Path, PathBuf};

pub fn is_portable(exe_dir: &Path, portable: Option<&str>) -> bool {
    portable.is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        || exe_dir.join("portable.flag").exists()
        || exe_dir.join("pealayer.json").exists()
        || exe_dir.join("portable.dat").exists()
}

pub fn resolve_portable_config_path(exe_dir: &Path) -> PathBuf {
    exe_dir.join("pealayer.json")
}

pub fn resolve_system_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("pealayer").join("config.json");
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("pealayer")
                .join("config.json");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Pealayer")
                .join("config.json");
        }
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("pealayer").join("config.json");
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("pealayer")
        .join("config.json")
}
