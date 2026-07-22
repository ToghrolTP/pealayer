use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub volume: f64,
    pub is_muted: bool,
    pub pin_controls: bool,
    pub show_remaining_time: bool,
    pub recent_media: Vec<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            volume: 100.0,
            is_muted: false,
            pin_controls: false,
            show_remaining_time: false,
            recent_media: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn get_config_path() -> PathBuf {
        // 1. Portable Mode check (local executable folder flag/file)
        if PathBuf::from("portable.flag").exists() || PathBuf::from("pealayer.json").exists() {
            return PathBuf::from("config").join("settings.json");
        }

        // 2. Windows vs Linux standard AppData / XDG config path
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

        // Linux / Unix XDG fallback
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("pealayer").join("config.json");
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("pealayer").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                    return cfg;
                }
            }
        }

        // Transparent Migration from legacy recent.json if present
        let legacy_path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
            .join(".config")
            .join("pealayer")
            .join("recent.json");

        let mut config = Self::default();
        if legacy_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&legacy_path) {
                if let Ok(list) = serde_json::from_str::<Vec<PathBuf>>(&data) {
                    config.recent_media = list;
                }
            }
        }

        config
    }

    pub fn save(&self) {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.volume, 100.0);
        assert!(!cfg.is_muted);
        assert!(!cfg.pin_controls);
        assert!(!cfg.show_remaining_time);
        assert!(cfg.recent_media.is_empty());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut cfg = AppConfig::default();
        cfg.volume = 85.0;
        cfg.pin_controls = true;
        cfg.recent_media.push(PathBuf::from("/test/file.mp4"));

        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.volume, 85.0);
        assert!(loaded.pin_controls);
        assert_eq!(loaded.recent_media.len(), 1);
        assert_eq!(loaded.recent_media[0], PathBuf::from("/test/file.mp4"));
    }
}
