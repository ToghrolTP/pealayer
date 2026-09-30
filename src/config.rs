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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OsdPosition {
    #[default]
    TopLeft,
    Center,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerDragAction {
    #[default]
    MoveWindow,
    Seek,
    TemporaryFastForward,
    None,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoBackground {
    #[default]
    Black,
    DarkGray,
    Theme,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct StatusBarConfig {
    pub media_rate: bool,
    pub hardware: bool,
    pub telemetry: bool,
    pub workspace: bool,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            media_rate: true,
            hardware: true,
            telemetry: true,
            workspace: true,
        }
    }
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
    pub app_publisher: Option<String>,
    pub app_copyright: Option<String>,
    pub theme: AppTheme,
    pub language: AppLanguage,
    pub direction: AppDirection,
    pub hardware_endpoint: Option<String>,
    pub auto_connect_hardware: bool,
    pub pause_on_hardware_disconnect: bool,
    pub click_player_to_toggle: bool,
    pub show_subseconds: bool,
    pub wheel_seek_seconds: f64,
    pub osd_position: OsdPosition,
    pub osd_timeout_seconds: f32,
    pub paused_drag_action: PlayerDragAction,
    pub playing_drag_action: PlayerDragAction,
    pub fullscreen_video_background: VideoBackground,
    pub status_bar: StatusBarConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_dock_layout: Option<String>,
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
            app_publisher: None,
            app_copyright: None,
            theme: AppTheme::System,
            language: AppLanguage::System,
            direction: AppDirection::Auto,
            hardware_endpoint: None,
            auto_connect_hardware: true,
            pause_on_hardware_disconnect: true,
            click_player_to_toggle: true,
            show_subseconds: true,
            wheel_seek_seconds: 5.0,
            osd_position: OsdPosition::TopLeft,
            osd_timeout_seconds: 3.5,
            paused_drag_action: PlayerDragAction::MoveWindow,
            playing_drag_action: PlayerDragAction::TemporaryFastForward,
            fullscreen_video_background: VideoBackground::Black,
            status_bar: StatusBarConfig::default(),
            workspace_dock_layout: None,
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
        .or_else(|| application_brand_string("applicationName"))
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Pealayer".to_string())
}

pub fn resolved_app_icon(config: &AppConfig) -> Option<PathBuf> {
    std::env::var_os("APP_ICON")
        .map(PathBuf::from)
        .or_else(|| config.app_icon.clone())
        .or_else(application_brand_app_icon)
}

fn application_brand() -> Option<(PathBuf, serde_json::Value)> {
    let path = std::env::var_os("APPLICATION_BRAND").map(PathBuf::from)?;
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    if value
        .get("format")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|format| format != "application-brand")
    {
        return None;
    }
    Some((path, value))
}

fn application_brand_string(name: &str) -> Option<String> {
    application_brand()?
        .1
        .get(name)?
        .as_str()
        .map(str::to_string)
}

fn application_brand_app_icon() -> Option<PathBuf> {
    let (path, value) = application_brand()?;
    let relative = value.get("windowsIcons")?.get("APP")?.as_str()?;
    Some(path.parent().unwrap_or_else(|| std::path::Path::new(".")).join(relative))
}

fn resolved_optional_branding(env_name: &str, configured: Option<&str>) -> Option<String> {
    std::env::var(env_name)
        .ok()
        .or_else(|| configured.map(str::to_string))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn resolved_app_publisher(config: &AppConfig) -> Option<String> {
    resolved_optional_branding("APP_PUBLISHER", config.app_publisher.as_deref())
        .or_else(|| application_brand_string("companyName"))
}

pub fn resolved_app_copyright(config: &AppConfig) -> Option<String> {
    resolved_optional_branding("APP_COPYRIGHT", config.app_copyright.as_deref())
        .or_else(|| application_brand_string("legalCopyright"))
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

pub fn runtime_port(env_name: &str, default: u16) -> u16 {
    std::env::var(env_name)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(default)
}

/// The single TCP port used by Pealayer's HTTP, WebSocket, and local IPC APIs.
/// `PEALAYER_HTTP_PORT` remains a compatibility fallback for existing launchers.
pub fn control_port() -> u16 {
    runtime_port("PEALAYER_PORT", runtime_port("PEALAYER_HTTP_PORT", 8080))
}

impl AppConfig {
    pub fn get_config_path() -> PathBuf {
        if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
            return PathBuf::from(path);
        }
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
                // Read system configuration file (contains full structured state including workspace_dock_layout)
                #[allow(unused_mut)]
                let mut config = {
                    let path = resolve_system_config_path();
                    if path.exists() {
                        if let Ok(data) = std::fs::read_to_string(&path) {
                            serde_json::from_str::<AppConfig>(&data).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                .unwrap_or_else(|| {
                    // Transparent Migration from legacy recent.json if present
                    let legacy_path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                        .join(".config")
                        .join("pealayer")
                        .join("recent.json");

                    let mut cfg = Self::default();
                    if legacy_path.exists() {
                        if let Ok(data) = std::fs::read_to_string(&legacy_path) {
                            if let Ok(list) = serde_json::from_str::<Vec<PathBuf>>(&data) {
                                cfg.recent_media = list;
                            }
                        }
                    }
                    cfg
                });

                // On Windows, overlay settings from Registry if present
                #[cfg(target_os = "windows")]
                {
                    if let Ok(Some(reg_cfg)) = crate::platform::registry::load_settings_from_registry() {
                        config.volume = reg_cfg.volume;
                        config.is_muted = reg_cfg.is_muted;
                        config.pin_controls = reg_cfg.pin_controls;
                        config.show_remaining_time = reg_cfg.show_remaining_time;
                        config.recent_media = reg_cfg.recent_media;
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
        if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
            let path = PathBuf::from(path);
            if path.exists() {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                        return cfg;
                    }
                }
            }
            return Self::default();
        }
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        Self::load_with_mode(mode, &exe_dir)
    }

    pub fn save(&self) {
        if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
            return;
        }
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
        assert!(cfg.app_name.is_none());
        assert!(cfg.app_icon.is_none());
        assert!(cfg.app_publisher.is_none());
        assert!(cfg.app_copyright.is_none());
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

    #[test]
    fn empty_configured_name_uses_product_default() {
        let mut cfg = AppConfig::default();
        cfg.app_name = Some("   ".to_string());
        if std::env::var_os("APP_NAME").is_none() {
            assert_eq!(resolved_app_name(&cfg), "Pealayer");
        }
    }

    #[test]
    fn configured_branding_metadata_is_trimmed_and_optional() {
        let mut cfg = AppConfig::default();
        cfg.app_publisher = Some("  Example Studio  ".to_string());
        cfg.app_copyright = Some("   ".to_string());
        if std::env::var_os("APP_PUBLISHER").is_none()
            && std::env::var_os("APP_COPYRIGHT").is_none()
        {
            assert_eq!(resolved_app_publisher(&cfg).as_deref(), Some("Example Studio"));
            assert_eq!(resolved_app_copyright(&cfg), None);
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
