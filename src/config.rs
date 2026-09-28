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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMode {
    Auto,
    Portable,
    System,
}

pub fn detect_executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn detect_storage_mode(exe_dir: &std::path::Path) -> StorageMode {
    if std::env::var("PEALAYER_PORTABLE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || exe_dir.join("portable.flag").exists()
        || exe_dir.join("pealayer.json").exists()
        || exe_dir.join("portable.dat").exists()
    {
        StorageMode::Portable
    } else {
        StorageMode::System
    }
}

pub fn resolve_portable_config_path(exe_dir: &std::path::Path) -> PathBuf {
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

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("pealayer").join("config.json");
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("pealayer").join("config.json")
}

impl AppConfig {
    pub fn get_config_path() -> PathBuf {
        let exe_dir = detect_executable_dir();
        match detect_storage_mode(&exe_dir) {
            StorageMode::Portable => resolve_portable_config_path(&exe_dir),
            StorageMode::System | StorageMode::Auto => resolve_system_config_path(),
        }
    }

    pub fn load_with_mode(mode: StorageMode, exe_dir: &std::path::Path) -> Self {
        match mode {
            StorageMode::Portable => {
                let path = resolve_portable_config_path(exe_dir);
                if path.exists() {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                            return cfg;
                        }
                    }
                }
                Self::default()
            }
            StorageMode::System | StorageMode::Auto => {
                // On Windows, try reading from Registry first
                #[cfg(target_os = "windows")]
                {
                    if let Ok(Some(reg_cfg)) = crate::platform::registry::load_settings_from_registry() {
                        return reg_cfg;
                    }
                }

                // Fall back to system configuration file
                let path = resolve_system_config_path();
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
        }
    }

    pub fn save_with_mode(&self, mode: StorageMode, exe_dir: &std::path::Path) {
        match mode {
            StorageMode::Portable => {
                let path = resolve_portable_config_path(exe_dir);
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(json) = serde_json::to_string_pretty(self) {
                    let _ = std::fs::write(path, json);
                }
            }
            StorageMode::System | StorageMode::Auto => {
                // On Windows, save to Registry
                #[cfg(target_os = "windows")]
                {
                    let _ = crate::platform::registry::save_settings_to_registry(self);
                }

                // Save to system config file (as shadow backup / cross-platform standard)
                let path = resolve_system_config_path();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(json) = serde_json::to_string_pretty(self) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
    }

    pub fn load() -> Self {
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        Self::load_with_mode(mode, &exe_dir)
    }

    pub fn save(&self) {
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        self.save_with_mode(mode, &exe_dir);
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

    #[test]
    fn test_detect_storage_mode_portable_flag() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("portable.flag"), "").unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        let portable_path = resolve_portable_config_path(&temp_dir);
        assert_eq!(portable_path, temp_dir.join("pealayer.json"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_system_default() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_sys_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::System);

        let sys_path = resolve_system_config_path();
        assert!(sys_path.ends_with("config.json"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_pealayer_json() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_json_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("pealayer.json"), "{}").unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_portable_dat() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_dat_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("portable.dat"), "").unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_env_var() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_env_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        unsafe {
            std::env::set_var("PEALAYER_PORTABLE", "1");
        }
        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        unsafe {
            std::env::remove_var("PEALAYER_PORTABLE");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_executable_dir() {
        let exe_dir = detect_executable_dir();
        assert!(exe_dir.exists());
    }

    #[test]
    fn test_portable_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_port_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut cfg = AppConfig::default();
        cfg.volume = 77.0;
        cfg.is_muted = true;
        cfg.pin_controls = true;
        cfg.show_remaining_time = true;
        cfg.recent_media.push(PathBuf::from("/media/video.mp4"));

        cfg.save_with_mode(StorageMode::Portable, &temp_dir);

        let portable_file = temp_dir.join("pealayer.json");
        assert!(portable_file.exists());

        let loaded = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
        assert_eq!(loaded.volume, 77.0);
        assert!(loaded.is_muted);
        assert!(loaded.pin_controls);
        assert!(loaded.show_remaining_time);
        assert_eq!(loaded.recent_media, vec![PathBuf::from("/media/video.mp4")]);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

