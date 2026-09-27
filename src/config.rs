use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppLanguage {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "fa")]
    Persian,
}

impl AppLanguage {
    pub fn is_rtl(self) -> bool {
        matches!(self, Self::Persian)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub volume: f64,
    pub is_muted: bool,
    pub pin_controls: bool,
    pub show_remaining_time: bool,
    pub recent_media: Vec<PathBuf>,
    pub app_name: Option<String>,
    pub app_icon: Option<PathBuf>,
    pub theme: AppTheme,
    pub language: AppLanguage,
    pub direction: AppDirection,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            volume: 100.0,
            is_muted: false,
            pin_controls: false,
            show_remaining_time: false,
            recent_media: Vec::new(),
            app_name: None,
            app_icon: None,
            theme: AppTheme::System,
            language: AppLanguage::System,
            direction: AppDirection::Auto,
        }
    }
}

fn parse_language_tag(value: &str) -> Option<AppLanguage> {
    let normalized = value.trim().replace('_', "-").to_ascii_lowercase();
    match normalized.as_str() {
        "system" | "auto" => Some(AppLanguage::System),
        "en" | "english" => Some(AppLanguage::English),
        "fa" | "fa-ir" | "persian" | "farsi" => Some(AppLanguage::Persian),
        _ if normalized.starts_with("fa-") => Some(AppLanguage::Persian),
        _ if normalized.starts_with("en-") => Some(AppLanguage::English),
        _ => None,
    }
}

fn system_language() -> AppLanguage {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if let Some(language) = parse_language_tag(&value) {
                if language != AppLanguage::System {
                    return language;
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        if let Ok(international) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Control Panel\International")
        {
            if let Ok(locale_name) = international.get_value::<String, _>("LocaleName") {
                if let Some(language) = parse_language_tag(&locale_name) {
                    if language != AppLanguage::System {
                        return language;
                    }
                }
            }
        }
    }

    AppLanguage::English
}

pub fn resolved_language_preference(config: &AppConfig) -> AppLanguage {
    std::env::var("APP_LOCALE")
        .ok()
        .and_then(|value| parse_language_tag(&value))
        .unwrap_or(config.language)
}

pub fn resolve_language(preference: AppLanguage) -> AppLanguage {
    match preference {
        AppLanguage::System => system_language(),
        language => language,
    }
}

pub fn resolved_direction_preference(config: &AppConfig) -> AppDirection {
    match std::env::var("APP_DIRECTION")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "ltr" => AppDirection::Ltr,
        "rtl" => AppDirection::Rtl,
        "auto" => AppDirection::Auto,
        _ => config.direction,
    }
}

pub fn resolve_rtl(preference: AppDirection, language: AppLanguage) -> bool {
    match preference {
        AppDirection::Auto => language.is_rtl(),
        AppDirection::Ltr => false,
        AppDirection::Rtl => true,
    }
}

pub fn resolved_app_name(config: &AppConfig) -> String {
    std::env::var("APP_NAME")
        .ok()
        .or_else(|| config.app_name.clone())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Pealayer".to_string())
}

pub fn resolved_app_icon(config: &AppConfig) -> Option<PathBuf> {
    std::env::var_os("APP_ICON")
        .map(PathBuf::from)
        .or_else(|| config.app_icon.clone())
}

pub fn resolved_theme(config: &AppConfig) -> AppTheme {
    match std::env::var("APP_THEME")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "light" => AppTheme::Light,
        "dark" => AppTheme::Dark,
        "system" => AppTheme::System,
        _ => config.theme,
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
        assert!(cfg.app_name.is_none());
        assert!(cfg.app_icon.is_none());
        assert_eq!(cfg.theme, AppTheme::System);
        assert_eq!(cfg.language, AppLanguage::System);
        assert_eq!(cfg.direction, AppDirection::Auto);
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
    fn empty_configured_name_uses_product_default() {
        let mut cfg = AppConfig::default();
        cfg.app_name = Some("   ".to_string());
        if std::env::var_os("APP_NAME").is_none() {
            assert_eq!(resolved_app_name(&cfg), "Pealayer");
        }
    }

    #[test]
    fn language_tags_support_web_contract_values() {
        assert_eq!(parse_language_tag("en"), Some(AppLanguage::English));
        assert_eq!(parse_language_tag("fa-IR"), Some(AppLanguage::Persian));
        assert_eq!(parse_language_tag("farsi"), Some(AppLanguage::Persian));
        assert_eq!(parse_language_tag("system"), Some(AppLanguage::System));
        assert_eq!(parse_language_tag("de"), None);

        let json = serde_json::to_string(&AppLanguage::Persian).unwrap();
        assert_eq!(json, "\"fa\"");
        assert_eq!(serde_json::from_str::<AppLanguage>("\"en\"").unwrap(), AppLanguage::English);
        assert!(resolve_rtl(AppDirection::Auto, AppLanguage::Persian));
        assert!(!resolve_rtl(AppDirection::Auto, AppLanguage::English));
        assert!(resolve_rtl(AppDirection::Rtl, AppLanguage::English));
        assert!(!resolve_rtl(AppDirection::Ltr, AppLanguage::Persian));
    }
}
