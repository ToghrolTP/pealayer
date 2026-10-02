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
#[serde(rename_all = "snake_case")]
pub enum AccentColor {
    #[default]
    System,
    PealayerGreen,
    WindowsBlue,
    MacosBlue,
    Custom,
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MotionControlMode {
    #[default]
    Toggle,
    Hold,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct StatusBarConfig {
    pub media_rate: bool,
    pub hardware: bool,
    pub telemetry: bool,
    pub status_rgb: bool,
    pub warnings: bool,
    pub workspace: bool,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            media_rate: true,
            hardware: true,
            telemetry: true,
            status_rgb: true,
            warnings: true,
            workspace: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WindowGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

impl WindowGeometry {
    pub fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && (420.0..=16_384.0).contains(&self.width)
            && (300.0..=16_384.0).contains(&self.height)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub volume: f64,
    pub is_muted: bool,
    pub pin_controls: bool,
    pub show_remaining_time: bool,
    pub open_url_multiline: bool,
    pub open_url_history_expanded: bool,
    pub open_url_recent_click_edits: bool,
    pub open_url_fetch_remote_info: bool,
    pub open_url_fetch_remote_thumbnail: bool,
    pub open_url_use_proxy: bool,
    pub open_url_proxy_url: Option<String>,
    pub recent_media: Vec<PathBuf>,
    pub app_name: Option<String>,
    pub app_icon: Option<PathBuf>,
    pub app_publisher: Option<String>,
    pub app_copyright: Option<String>,
    pub theme: AppTheme,
    pub accent_color: AccentColor,
    pub custom_accent_color: Option<String>,
    pub language: AppLanguage,
    pub direction: AppDirection,
    pub hardware_endpoint: Option<String>,
    pub auto_connect_hardware: bool,
    pub pause_on_hardware_disconnect: bool,
    pub click_player_to_toggle: bool,
    pub show_subseconds: bool,
    pub quick_seek_seconds: f64,
    pub frame_step_count: u32,
    pub wheel_seek_seconds: f64,
    pub osd_position: OsdPosition,
    pub osd_timeout_seconds: f32,
    pub paused_drag_action: PlayerDragAction,
    pub playing_drag_action: PlayerDragAction,
    pub fullscreen_video_background: VideoBackground,
    pub motion_control_mode: MotionControlMode,
    pub compact_hardware_controls: bool,
    #[serde(alias = "show_raw_motion_relays")]
    pub show_raw_relays: bool,
    #[serde(alias = "prefix_relay_numbers")]
    pub prefix_relay_identifiers: bool,
    pub live_pwm_updates: bool,
    pub hardware_actions_on_press: bool,
    pub single_instance: bool,
    pub windows_mica_backdrop: bool,
    pub windows_dwm_theming: bool,
    pub opengl_vsync: bool,
    pub native_dialog_windows: bool,
    pub status_bar: StatusBarConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_geometry: Option<WindowGeometry>,
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
            open_url_multiline: true,
            open_url_history_expanded: true,
            open_url_recent_click_edits: true,
            open_url_fetch_remote_info: true,
            open_url_fetch_remote_thumbnail: true,
            open_url_use_proxy: true,
            open_url_proxy_url: None,
            recent_media: Vec::new(),
            app_name: None,
            app_icon: None,
            app_publisher: None,
            app_copyright: None,
            theme: AppTheme::System,
            accent_color: AccentColor::System,
            custom_accent_color: None,
            language: AppLanguage::System,
            direction: AppDirection::Auto,
            hardware_endpoint: None,
            auto_connect_hardware: true,
            pause_on_hardware_disconnect: true,
            click_player_to_toggle: true,
            show_subseconds: true,
            quick_seek_seconds: 10.0,
            frame_step_count: 1,
            wheel_seek_seconds: 5.0,
            osd_position: OsdPosition::TopLeft,
            osd_timeout_seconds: 3.5,
            paused_drag_action: PlayerDragAction::MoveWindow,
            playing_drag_action: PlayerDragAction::TemporaryFastForward,
            fullscreen_video_background: VideoBackground::Black,
            motion_control_mode: MotionControlMode::Hold,
            compact_hardware_controls: false,
            show_raw_relays: true,
            prefix_relay_identifiers: true,
            live_pwm_updates: true,
            hardware_actions_on_press: true,
            single_instance: true,
            windows_mica_backdrop: false,
            windows_dwm_theming: true,
            // Reactive egui rendering does not require a continuously synced
            // swap loop. Some Windows OpenGL drivers flicker with V-Sync, so
            // keep it opt-in while retaining the persisted preference.
            opengl_vsync: false,
            // Preferences is an owned native tool window on Windows. Keep the
            // embedded implementation as a runtime fallback when process or
            // window creation is unavailable.
            native_dialog_windows: true,
            status_bar: StatusBarConfig::default(),
            window_geometry: None,
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
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(international) =
            RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Control Panel\International")
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
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
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
    Some(
        path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(relative),
    )
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
pub fn control_port() -> u16 {
    runtime_port("PEALAYER_PORT", 8080)
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
                if let Ok(cfg) = Self::load_from_path(&path) {
                    return cfg;
                }
                Self::default()
            }
            StorageMode::System | StorageMode::Auto => {
                // The complete JSON document is authoritative and watchable on
                // every OS. Windows Registry values remain a native mirror and
                // a migration fallback, never a reason to discard newer fields.
                let path = resolve_system_config_path();
                if let Ok(cfg) = Self::load_from_path(&path) {
                    return cfg;
                }

                #[cfg(target_os = "windows")]
                {
                    if let Ok(Some(reg_cfg)) =
                        crate::platform::registry::load_settings_from_registry()
                    {
                        if reg_cfg.validate().is_ok() {
                            return reg_cfg;
                        }
                    }
                }

                // Transparent Migration from legacy recent.json if present
                let legacy_path =
                    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
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

    pub fn save_with_mode(
        &self,
        mode: StorageMode,
        exe_dir: &std::path::Path,
    ) -> Result<(), String> {
        self.validate()?;
        match mode {
            StorageMode::Portable => {
                let path = resolve_portable_config_path(exe_dir);
                self.save_to_path(&path)?;
            }
            StorageMode::System | StorageMode::Auto => {
                // Keep the full JSON file as the cross-platform, externally
                // watchable source. The Windows Registry mirrors the same
                // complete document for native tooling and migration.
                let path = resolve_system_config_path();
                self.save_to_path(&path)?;

                #[cfg(target_os = "windows")]
                {
                    crate::platform::registry::save_settings_to_registry(self)?;
                }
            }
        }
        Ok(())
    }

    pub fn load() -> Self {
        if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
            let path = PathBuf::from(path);
            if let Ok(cfg) = Self::load_from_path(&path) {
                return cfg;
            }
            return Self::default();
        }
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        Self::load_with_mode(mode, &exe_dir)
    }

    pub fn save(&self) -> Result<(), String> {
        self.validate()?;
        if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
            let path = PathBuf::from(path);
            return self.save_to_path(&path);
        }
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        self.save_with_mode(mode, &exe_dir)
    }

    pub fn load_from_path(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|error| format!("read configuration {}: {error}", path.display()))?;
        let config = serde_json::from_str::<Self>(&data)
            .map_err(|error| format!("parse configuration {}: {error}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .ok_or_else(|| format!("configuration path has no parent: {}", path.display()))?;
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create configuration directory {}: {error}",
                parent.display()
            )
        })?;
        let json = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("serialize configuration: {error}"))?;
        let sequence = CONFIG_TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let file_name = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("config.json");
        let temporary = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let write_result = (|| {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| {
                    format!(
                        "create temporary configuration {}: {error}",
                        temporary.display()
                    )
                })?;
            file.write_all(&json)
                .and_then(|_| file.write_all(b"\n"))
                .and_then(|_| file.sync_all())
                .map_err(|error| {
                    format!(
                        "write temporary configuration {}: {error}",
                        temporary.display()
                    )
                })?;
            replace_file(&temporary, path)
        })();
        if write_result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        write_result
    }

    pub fn fingerprint(path: &std::path::Path) -> Result<u64, String> {
        use std::hash::{Hash, Hasher};
        let bytes = std::fs::read(path)
            .map_err(|error| format!("read configuration {}: {error}", path.display()))?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        Ok(hasher.finish())
    }

    pub fn apply_patch(&self, patch: &serde_json::Value) -> Result<Self, String> {
        let patch = patch
            .as_object()
            .ok_or_else(|| "configuration update must be a JSON object".to_string())?;
        let mut value = serde_json::to_value(self)
            .map_err(|error| format!("serialize current configuration: {error}"))?;
        let target = value
            .as_object_mut()
            .ok_or_else(|| "current configuration is not an object".to_string())?;
        for (key, replacement) in patch {
            if !target.contains_key(key) {
                return Err(format!("unknown configuration setting: {key}"));
            }
            target.insert(key.clone(), replacement.clone());
        }
        let updated = serde_json::from_value::<Self>(value)
            .map_err(|error| format!("invalid configuration update: {error}"))?;
        updated.validate()?;
        Ok(updated)
    }

    pub fn validate_patch_shape(patch: &serde_json::Value) -> Result<(), String> {
        let patch = patch
            .as_object()
            .ok_or_else(|| "configuration update must be a JSON object".to_string())?;
        let known =
            serde_json::to_value(Self::default()).expect("default configuration serializes");
        let known = known
            .as_object()
            .expect("default configuration is an object");
        if let Some(key) = patch.keys().find(|key| !known.contains_key(*key)) {
            return Err(format!("unknown configuration setting: {key}"));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.volume.is_finite() || !(0.0..=130.0).contains(&self.volume) {
            return Err("volume must be between 0 and 130".to_string());
        }
        if !self.wheel_seek_seconds.is_finite() || !(0.1..=60.0).contains(&self.wheel_seek_seconds)
        {
            return Err("wheel_seek_seconds must be between 0.1 and 60".to_string());
        }
        if !self.quick_seek_seconds.is_finite() || !(0.1..=600.0).contains(&self.quick_seek_seconds)
        {
            return Err("quick_seek_seconds must be between 0.1 and 600".to_string());
        }
        if !(1..=120).contains(&self.frame_step_count) {
            return Err("frame_step_count must be between 1 and 120".to_string());
        }
        if !self.osd_timeout_seconds.is_finite()
            || !(1.0..=60.0).contains(&self.osd_timeout_seconds)
        {
            return Err("osd_timeout_seconds must be between 1 and 60".to_string());
        }
        if self.accent_color == AccentColor::Custom
            && self
                .custom_accent_color
                .as_deref()
                .and_then(parse_rgb_hex)
                .is_none()
        {
            return Err("custom_accent_color must be a color such as #0078d4".to_string());
        }
        if self
            .hardware_endpoint
            .as_deref()
            .is_some_and(|value| value.len() > 1024)
        {
            return Err("hardware_endpoint is too long".to_string());
        }
        if let Some(proxy) = self
            .open_url_proxy_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let parsed = url::Url::parse(proxy)
                .map_err(|_| "open_url_proxy_url must be a complete URL".to_string())?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err(
                    "open_url_proxy_url must use HTTP or HTTPS and include a host".to_string(),
                );
            }
        }
        if self.recent_media.len() > 100 {
            return Err("recent_media contains too many entries".to_string());
        }
        if self
            .window_geometry
            .is_some_and(|geometry| !geometry.is_valid())
        {
            return Err("window_geometry contains invalid coordinates or dimensions".to_string());
        }
        Ok(())
    }
}

pub fn parse_rgb_hex(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    (value.len() == 6).then_some(())?;
    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}

static CONFIG_TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[cfg(target_os = "windows")]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;
    let destination_display = destination.display().to_string();
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|error| format!("replace configuration {destination_display}: {error}"))
}

#[cfg(not(target_os = "windows"))]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> Result<(), String> {
    std::fs::rename(source, destination)
        .map_err(|error| format!("replace configuration {}: {error}", destination.display()))
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
        assert!(cfg.open_url_multiline);
        assert!(cfg.open_url_history_expanded);
        assert!(cfg.open_url_recent_click_edits);
        assert!(cfg.open_url_fetch_remote_info);
        assert!(cfg.open_url_fetch_remote_thumbnail);
        assert!(cfg.open_url_use_proxy);
        assert!(cfg.open_url_proxy_url.is_none());
        assert!(cfg.native_dialog_windows);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut cfg = AppConfig::default();
        cfg.volume = 85.0;
        cfg.pin_controls = true;
        cfg.open_url_multiline = false;
        cfg.open_url_history_expanded = false;
        cfg.open_url_recent_click_edits = false;
        cfg.open_url_fetch_remote_info = false;
        cfg.open_url_fetch_remote_thumbnail = false;
        cfg.open_url_use_proxy = false;
        cfg.open_url_proxy_url = Some("http://127.0.0.1:8080".to_string());
        cfg.native_dialog_windows = true;
        cfg.recent_media.push(PathBuf::from("/test/file.mp4"));

        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.volume, 85.0);
        assert!(loaded.pin_controls);
        assert!(!loaded.open_url_multiline);
        assert!(!loaded.open_url_history_expanded);
        assert!(!loaded.open_url_recent_click_edits);
        assert!(!loaded.open_url_fetch_remote_info);
        assert!(!loaded.open_url_fetch_remote_thumbnail);
        assert!(!loaded.open_url_use_proxy);
        assert_eq!(
            loaded.open_url_proxy_url.as_deref(),
            Some("http://127.0.0.1:8080")
        );
        assert_eq!(loaded.recent_media.len(), 1);
        assert_eq!(loaded.recent_media[0], PathBuf::from("/test/file.mp4"));
        assert!(loaded.native_dialog_windows);
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
        let temp_dir =
            std::env::temp_dir().join(format!("pealayer_test_sys_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::System);

        let sys_path = resolve_system_config_path();
        assert!(sys_path.ends_with("config.json"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_pealayer_json() {
        let temp_dir =
            std::env::temp_dir().join(format!("pealayer_test_json_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("pealayer.json"), "{}").unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_portable_dat() {
        let temp_dir =
            std::env::temp_dir().join(format!("pealayer_test_dat_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("portable.dat"), "").unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_env_var() {
        let temp_dir =
            std::env::temp_dir().join(format!("pealayer_test_env_{}", uuid::Uuid::new_v4()));
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
    fn accent_presets_and_custom_color_are_validated() {
        assert_eq!(parse_rgb_hex("#38d27a"), Some([56, 210, 122]));
        assert_eq!(parse_rgb_hex("0078D4"), Some([0, 120, 212]));
        assert_eq!(parse_rgb_hex("not-a-color"), None);

        let mut config = AppConfig {
            accent_color: AccentColor::Custom,
            custom_accent_color: Some("#0a84ff".to_string()),
            ..AppConfig::default()
        };
        assert!(config.validate().is_ok());
        config.custom_accent_color = Some("blue".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_portable_save_and_load_roundtrip() {
        let temp_dir =
            std::env::temp_dir().join(format!("pealayer_port_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut cfg = AppConfig::default();
        cfg.volume = 77.0;
        cfg.is_muted = true;
        cfg.pin_controls = true;
        cfg.show_remaining_time = true;
        cfg.recent_media.push(PathBuf::from("/media/video.mp4"));

        cfg.save_with_mode(StorageMode::Portable, &temp_dir)
            .unwrap();

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
    fn config_patch_rejects_unknown_keys_and_persists_complete_settings() {
        let temp_dir =
            std::env::temp_dir().join(format!("pealayer_config_patch_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let original = AppConfig::default();
        let updated = original
            .apply_patch(&serde_json::json!({
                "theme": "dark",
                "hardware_endpoint": "pccontroller://cafe-pc:8787",
                "pause_on_hardware_disconnect": false,
                "motion_control_mode": "hold",
                "compact_hardware_controls": true,
                "show_raw_relays": false,
                "prefix_relay_identifiers": false,
                "status_bar": {
                    "media_rate": false,
                    "hardware": true,
                    "telemetry": false,
                    "status_rgb": true,
                    "warnings": true,
                    "workspace": false
                }
            }))
            .unwrap();
        assert_eq!(updated.theme, AppTheme::Dark);
        assert_eq!(
            updated.hardware_endpoint.as_deref(),
            Some("pccontroller://cafe-pc:8787")
        );
        assert!(!updated.pause_on_hardware_disconnect);
        assert_eq!(updated.motion_control_mode, MotionControlMode::Hold);
        assert!(updated.compact_hardware_controls);
        assert!(!updated.show_raw_relays);
        assert!(!updated.prefix_relay_identifiers);
        assert!(!updated.status_bar.media_rate);
        assert!(!updated.status_bar.workspace);
        assert!(
            updated
                .apply_patch(&serde_json::json!({"hardawre_endpoint": "typo"}))
                .unwrap_err()
                .contains("unknown configuration setting")
        );

        updated
            .save_with_mode(StorageMode::Portable, &temp_dir)
            .unwrap();
        let restored = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
        assert_eq!(restored, updated);
        let leftovers = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn renamed_relay_presentation_settings_migrate_from_existing_config() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("show_raw_relays");
        object.remove("prefix_relay_identifiers");
        object.insert(
            "show_raw_motion_relays".to_string(),
            serde_json::json!(false),
        );
        object.insert("prefix_relay_numbers".to_string(), serde_json::json!(false));

        let migrated: AppConfig = serde_json::from_value(value).unwrap();
        assert!(!migrated.show_raw_relays);
        assert!(!migrated.prefix_relay_identifiers);

        let serialized = serde_json::to_value(migrated).unwrap();
        assert_eq!(
            serialized.get("show_raw_relays"),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            serialized.get("prefix_relay_identifiers"),
            Some(&serde_json::json!(false))
        );
        assert!(serialized.get("show_raw_motion_relays").is_none());
        assert!(serialized.get("prefix_relay_numbers").is_none());
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
            assert_eq!(
                resolved_app_publisher(&cfg).as_deref(),
                Some("Example Studio")
            );
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
        assert_eq!(
            serde_json::from_str::<AppLanguage>("\"en\"").unwrap(),
            AppLanguage::English
        );
        assert!(resolve_rtl(AppDirection::Auto, AppLanguage::Persian));
        assert!(!resolve_rtl(AppDirection::Auto, AppLanguage::English));
        assert!(resolve_rtl(AppDirection::Rtl, AppLanguage::English));
        assert!(!resolve_rtl(AppDirection::Ltr, AppLanguage::Persian));
    }
}
