use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

pub const DEFAULT_PLAYBACK_POSITION_HISTORY_LIMIT: u32 = 50;
pub const MAX_PLAYBACK_POSITION_HISTORY_LIMIT: u32 = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackPositionEntry {
    /// Original playable local path or remote URL. Matching uses a normalized
    /// identity at runtime, while this value remains suitable for inspection.
    pub target: String,
    pub position_seconds: f64,
    pub updated_at_unix_ms: u64,
}

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
    Toggle,
    #[default]
    Hold,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NonUserControlVisibility {
    Hidden,
    #[default]
    Dimmed,
    Shown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(default)]
pub struct KeyChord {
    /// Portable physical-key name using the keyboard-types/USB code vocabulary
    /// (for example `KeyR`, `Digit5`, `F8`, or `Space`).
    pub key: String,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl Default for KeyChord {
    fn default() -> Self {
        Self {
            key: "KeyA".to_string(),
            control: false,
            alt: false,
            shift: false,
            super_key: false,
        }
    }
}

impl KeyChord {
    pub fn native_hotkey_string(&self) -> String {
        let mut parts = Vec::new();
        if self.shift {
            parts.push("shift".to_string());
        }
        if self.control {
            parts.push("control".to_string());
        }
        if self.alt {
            parts.push("alt".to_string());
        }
        if self.super_key {
            parts.push("super".to_string());
        }
        parts.push(self.key.clone());
        parts.join("+")
    }

    pub fn display_name(&self) -> String {
        let mut parts = Vec::new();
        if self.control {
            parts.push(
                if cfg!(target_os = "macos") {
                    "⌃"
                } else {
                    "Ctrl"
                }
                .to_string(),
            );
        }
        if self.alt {
            parts.push(
                if cfg!(target_os = "macos") {
                    "⌥"
                } else {
                    "Alt"
                }
                .to_string(),
            );
        }
        if self.shift {
            parts.push(
                if cfg!(target_os = "macos") {
                    "⇧"
                } else {
                    "Shift"
                }
                .to_string(),
            );
        }
        if self.super_key {
            parts.push(
                if cfg!(target_os = "macos") {
                    "⌘"
                } else {
                    "Super"
                }
                .to_string(),
            );
        }
        let key = self
            .key
            .strip_prefix("Key")
            .or_else(|| self.key.strip_prefix("Digit"))
            .unwrap_or(&self.key);
        parts.push(key.to_string());
        parts.join(if cfg!(target_os = "macos") { "" } else { "+" })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HardwareKeyBindingAction {
    Invoke {
        action_id: String,
        label: String,
    },
    Toggle {
        on_action_id: String,
        off_action_id: String,
    },
    SetPwm {
        percent: f64,
    },
    Hold {
        press_action_id: String,
        release_action_id: String,
        press_label: String,
        release_label: String,
    },
}

impl Default for HardwareKeyBindingAction {
    fn default() -> Self {
        Self::Invoke {
            action_id: String::new(),
            label: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HardwareKeyBinding {
    pub id: String,
    pub channel_key: String,
    pub chord: KeyChord,
    pub action: HardwareKeyBindingAction,
    pub global: bool,
    pub enabled: bool,
}

impl Default for HardwareKeyBinding {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel_key: String::new(),
            chord: KeyChord::default(),
            action: HardwareKeyBindingAction::default(),
            global: false,
            enabled: true,
        }
    }
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

/// User-facing workspace state that is meaningful independently of egui's
/// internal widget memory. Window/dialog positions and scroll offsets live in
/// the serialized egui memory; these fields restore which surfaces were open
/// and which tabs were active inside them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct WorkspaceDialogs {
    pub subtitles: bool,
    pub audio: bool,
    pub open_location: bool,
    pub shortcuts: bool,
    pub about: bool,
    pub about_tab: usize,
    pub preferences: bool,
    pub preferences_tab: usize,
    pub board_information: bool,
    pub board_information_tab: usize,
    pub channel_manager: bool,
    pub hardware_control_key: Option<String>,
    pub hardware_channel_detail_active: bool,
    pub effects_manager: bool,
    pub effects_selection: Option<String>,
    pub workspace_profiles: bool,
}

/// A complete reusable workspace snapshot. The same structure backs the
/// automatic last-session restore and user-named profiles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WorkspaceProfile {
    /// Stable machine-facing identity. The map key remains authoritative; this
    /// human-facing caption can be renamed freely without breaking RPC callers.
    pub name: String,
    /// Stable Phosphor icon name shared by the native and web renderers.
    pub icon: String,
    /// Explicit user-defined ordering. UI surfaces never sort by caption.
    pub order: i32,
    pub nle: bool,
    pub window_geometry: Option<WindowGeometry>,
    pub dock_layout: Option<String>,
    pub dialogs: WorkspaceDialogs,
    /// Serialized egui memory contains movable window rectangles, active dock
    /// leaves, and every stable ScrollArea offset.
    pub egui_memory: Option<String>,
}

impl Default for WorkspaceProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            icon: String::new(),
            order: 0,
            nle: true,
            window_geometry: None,
            dock_layout: None,
            dialogs: WorkspaceDialogs::default(),
            egui_memory: None,
        }
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
    pub last_media_target: Option<PathBuf>,
    pub last_media_paused: bool,
    pub restore_last_media_on_startup: bool,
    pub remember_playback_position: bool,
    pub playback_position_history_limit: u32,
    pub playback_positions: Vec<PlaybackPositionEntry>,
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
    pub playback_speed: f64,
    pub temporary_fast_forward_speed: f64,
    pub subtitle_direction: crate::subtitle::SubtitleDirection,
    pub subtitle_text_replacements: Vec<crate::subtitle::SubtitleReplacement>,
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
    pub non_user_control_visibility: NonUserControlVisibility,
    #[serde(alias = "prefix_relay_numbers")]
    pub prefix_relay_identifiers: bool,
    pub live_pwm_updates: bool,
    pub hardware_actions_on_press: bool,
    pub hardware_key_bindings: Vec<HardwareKeyBinding>,
    pub show_estop_control: bool,
    pub confirm_estop_release: bool,
    pub single_instance: bool,
    pub windows_mica_backdrop: bool,
    pub windows_dwm_theming: bool,
    pub opengl_vsync: bool,
    pub native_dialog_windows: bool,
    pub auto_reload_config: bool,
    pub status_bar: StatusBarConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_geometry: Option<WindowGeometry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_dock_layout: Option<String>,
    pub workspace_session: WorkspaceProfile,
    pub workspace_profiles: BTreeMap<String, WorkspaceProfile>,
    #[serde(default)]
    pub workspace_profiles_initialized: bool,
    #[serde(default)]
    pub workspace_profiles_revision: u32,
    #[serde(default)]
    pub active_workspace_profile: Option<String>,
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
            last_media_target: None,
            last_media_paused: false,
            restore_last_media_on_startup: true,
            remember_playback_position: true,
            playback_position_history_limit: DEFAULT_PLAYBACK_POSITION_HISTORY_LIMIT,
            playback_positions: Vec::new(),
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
            playback_speed: 1.0,
            temporary_fast_forward_speed: 2.0,
            subtitle_direction: crate::subtitle::SubtitleDirection::Auto,
            subtitle_text_replacements: crate::subtitle::default_text_replacements(),
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
            non_user_control_visibility: NonUserControlVisibility::Dimmed,
            prefix_relay_identifiers: true,
            live_pwm_updates: true,
            hardware_actions_on_press: true,
            hardware_key_bindings: Vec::new(),
            show_estop_control: true,
            confirm_estop_release: true,
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
            auto_reload_config: true,
            status_bar: StatusBarConfig::default(),
            window_geometry: None,
            workspace_dock_layout: None,
            workspace_session: WorkspaceProfile::default(),
            workspace_profiles: default_workspace_profiles(),
            workspace_profiles_initialized: true,
            workspace_profiles_revision: 1,
            active_workspace_profile: Some("nle".to_string()),
        }
    }
}

pub fn default_workspace_profiles() -> BTreeMap<String, WorkspaceProfile> {
    BTreeMap::from([
        (
            "simple".to_string(),
            WorkspaceProfile {
                name: "Simple".to_string(),
                icon: "monitor".to_string(),
                order: 0,
                nle: false,
                ..Default::default()
            },
        ),
        (
            "nle".to_string(),
            WorkspaceProfile {
                name: "NLE".to_string(),
                icon: "timeline".to_string(),
                order: 1,
                nle: true,
                ..Default::default()
            },
        ),
    ])
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

/// A compact, privacy-preserving location for UI surfaces. Configuration
/// remains addressable without exposing a full user-profile path in the app.
pub fn display_config_path(path: &std::path::Path) -> String {
    let roots = [
        ("APPDATA", "%APPDATA%"),
        ("XDG_CONFIG_HOME", "$XDG_CONFIG_HOME"),
        ("HOME", "~"),
    ];
    for (variable, label) in roots {
        if let Some(root) = std::env::var_os(variable).map(PathBuf::from)
            && let Ok(relative) = path.strip_prefix(root)
        {
            return format!("{label}{}{}", std::path::MAIN_SEPARATOR, relative.display());
        }
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
        && let Ok(relative) = path.strip_prefix(directory)
    {
        return format!(".{}{}", std::path::MAIN_SEPARATOR, relative.display());
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.json".to_string())
}

/// Native filesystem notifications for the authoritative JSON configuration.
/// The parent directory is watched so atomic file replacement remains visible.
pub struct ConfigFileWatcher {
    _watcher: notify::RecommendedWatcher,
    events: Receiver<notify::Result<notify::Event>>,
}

impl ConfigFileWatcher {
    pub fn new(
        path: &std::path::Path,
        wake_ui: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        use notify::Watcher;

        let parent = path
            .parent()
            .ok_or_else(|| format!("configuration path has no parent: {}", path.display()))?;
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create configuration directory {}: {error}",
                parent.display()
            )
        })?;
        let (sender, events) = channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = sender.send(event);
                wake_ui();
            })
            .map_err(|error| format!("create configuration file watcher: {error}"))?;
        watcher
            .watch(parent, notify::RecursiveMode::NonRecursive)
            .map_err(|error| {
                format!(
                    "watch configuration directory {}: {error}",
                    parent.display()
                )
            })?;
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }

    pub fn take_changed(&self) -> Result<bool, String> {
        let mut changed = false;
        let mut last_error = None;
        while let Ok(event) = self.events.try_recv() {
            match event {
                Ok(event) if !matches!(event.kind, notify::EventKind::Access(_)) => changed = true,
                Ok(_) => {}
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        last_error.map_or(Ok(changed), |error| {
            Err(format!("configuration file watcher: {error}"))
        })
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
    pub(crate) fn normalize_playback_positions(&mut self) {
        self.playback_position_history_limit = self
            .playback_position_history_limit
            .clamp(1, MAX_PLAYBACK_POSITION_HISTORY_LIMIT);
        self.playback_positions
            .truncate(self.playback_position_history_limit as usize);
    }

    fn normalize_workspace_profiles(&mut self) {
        self.normalize_playback_positions();
        let migrating_profile_metadata = self.workspace_profiles_revision < 1;
        if !self.workspace_profiles_initialized || migrating_profile_metadata {
            for (id, profile) in default_workspace_profiles() {
                self.workspace_profiles.entry(id).or_insert(profile);
            }
            self.workspace_profiles_initialized = true;
        }

        let mut next_order = self
            .workspace_profiles
            .values()
            .map(|profile| profile.order)
            .max()
            .unwrap_or(-1)
            + 1;
        for (id, profile) in &mut self.workspace_profiles {
            if profile.name.trim().is_empty() {
                profile.name = id.clone();
            }
            if migrating_profile_metadata {
                if id == "simple" && profile.name == "simple" {
                    profile.name = "Simple".to_string();
                } else if id == "nle" && profile.name == "nle" {
                    profile.name = "NLE".to_string();
                }
            }
            if profile.icon.trim().is_empty() {
                profile.icon = if profile.nle { "timeline" } else { "monitor" }.to_string();
            }
            if profile.order < 0 {
                profile.order = next_order;
                next_order += 1;
            }
        }
        let mut ordered_ids = self.workspace_profiles.keys().cloned().collect::<Vec<_>>();
        ordered_ids.sort_by(|left_id, right_id| {
            let left = &self.workspace_profiles[left_id];
            let right = &self.workspace_profiles[right_id];
            let default_rank = |id: &str| match id {
                "simple" => 0,
                "nle" => 1,
                _ => 2,
            };
            left.order
                .cmp(&right.order)
                .then_with(|| default_rank(left_id).cmp(&default_rank(right_id)))
                .then_with(|| left_id.cmp(right_id))
        });
        for (order, id) in ordered_ids.into_iter().enumerate() {
            if let Some(profile) = self.workspace_profiles.get_mut(&id) {
                profile.order = order as i32;
            }
        }
        self.workspace_profiles_revision = 1;
        if self
            .active_workspace_profile
            .as_ref()
            .is_none_or(|id| !self.workspace_profiles.contains_key(id))
        {
            self.active_workspace_profile = self
                .workspace_profiles
                .iter()
                .filter(|(_, profile)| profile.nle == self.workspace_session.nle)
                .min_by_key(|(_, profile)| profile.order)
                .map(|(id, _)| id.clone())
                .or_else(|| self.workspace_profiles.keys().next().cloned());
        }
    }

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
                    if let Ok(Some(mut reg_cfg)) =
                        crate::platform::registry::load_settings_from_registry()
                    {
                        reg_cfg.normalize_workspace_profiles();
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
                if let Err(error) = crate::platform::windows::configure_config_directory(
                    &path,
                    &resolved_app_name(self),
                ) {
                    log::warn!("Could not apply native configuration-folder metadata: {error}");
                }

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
            self.save_to_path(&path)?;
            if let Err(error) = crate::platform::windows::configure_config_directory(
                &path,
                &resolved_app_name(self),
            ) {
                log::warn!("Could not apply native configuration-folder metadata: {error}");
            }
            return Ok(());
        }
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        self.save_with_mode(mode, &exe_dir)
    }

    pub fn load_from_path(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|error| format!("read configuration {}: {error}", path.display()))?;
        let mut config = serde_json::from_str::<Self>(&data)
            .map_err(|error| format!("parse configuration {}: {error}", path.display()))?;
        config.normalize_workspace_profiles();
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
        let mut updated = serde_json::from_value::<Self>(value)
            .map_err(|error| format!("invalid configuration update: {error}"))?;
        updated.normalize_workspace_profiles();
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
        if !self.playback_speed.is_finite() || !(0.25..=4.0).contains(&self.playback_speed) {
            return Err("playback_speed must be between 0.25 and 4".to_string());
        }
        if !self.temporary_fast_forward_speed.is_finite()
            || !(1.0..=16.0).contains(&self.temporary_fast_forward_speed)
        {
            return Err("temporary_fast_forward_speed must be between 1 and 16".to_string());
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
        if self.subtitle_text_replacements.len() > 128 {
            return Err("subtitle_text_replacements contains more than 128 entries".to_string());
        }
        let mut subtitle_sources = std::collections::BTreeSet::new();
        for replacement in &self.subtitle_text_replacements {
            if replacement.from.chars().count() > 64 || replacement.to.chars().count() > 256 {
                return Err(
                    "subtitle_text_replacements contains an invalid source or replacement"
                        .to_string(),
                );
            }
            if !replacement.from.is_empty() && !subtitle_sources.insert(replacement.from.as_str()) {
                return Err(format!(
                    "subtitle_text_replacements contains duplicate source {:?}",
                    replacement.from
                ));
            }
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
            .last_media_target
            .as_ref()
            .is_some_and(|target| target.to_string_lossy().len() > 8_192)
        {
            return Err("last_media_target is too long".to_string());
        }
        if !(1..=MAX_PLAYBACK_POSITION_HISTORY_LIMIT)
            .contains(&self.playback_position_history_limit)
        {
            return Err(format!(
                "playback_position_history_limit must be between 1 and {MAX_PLAYBACK_POSITION_HISTORY_LIMIT}"
            ));
        }
        if self.playback_positions.len() > self.playback_position_history_limit as usize {
            return Err("playback_positions exceeds playback_position_history_limit".to_string());
        }
        for entry in &self.playback_positions {
            if entry.target.trim().is_empty() || entry.target.len() > 8_192 {
                return Err("playback_positions contains an invalid target".to_string());
            }
            if !entry.position_seconds.is_finite() || entry.position_seconds < 0.0 {
                return Err("playback_positions contains an invalid position".to_string());
            }
        }
        if self
            .window_geometry
            .is_some_and(|geometry| !geometry.is_valid())
        {
            return Err("window_geometry contains invalid coordinates or dimensions".to_string());
        }
        if self.workspace_profiles.len() > 64 {
            return Err("workspace_profiles contains more than 64 profiles".to_string());
        }
        for (id, profile) in &self.workspace_profiles {
            let valid_id = !id.is_empty()
                && id.len() <= 64
                && id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                });
            if !valid_id {
                return Err(format!("workspace profile ID is invalid: {id}"));
            }
            let name = profile.name.trim();
            if name.is_empty() || name.chars().count() > 64 {
                return Err(format!("workspace profile name is invalid: {id}"));
            }
            let icon = profile.icon.trim();
            if icon.is_empty()
                || icon.len() > 64
                || !icon.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
            {
                return Err(format!("workspace profile icon is invalid: {id}"));
            }
        }
        if self.hardware_key_bindings.len() > 512 {
            return Err("hardware_key_bindings contains more than 512 bindings".to_string());
        }
        let mut binding_ids = std::collections::BTreeSet::new();
        for binding in &self.hardware_key_bindings {
            if binding.id.trim().is_empty() || !binding_ids.insert(binding.id.as_str()) {
                return Err("hardware_key_bindings contains an empty or duplicate ID".to_string());
            }
            if binding.channel_key.trim().is_empty() || binding.channel_key.len() > 256 {
                return Err(format!(
                    "hardware binding {} has an invalid channel key",
                    binding.id
                ));
            }
            binding
                .chord
                .native_hotkey_string()
                .parse::<global_hotkey::hotkey::HotKey>()
                .map_err(|error| format!("hardware binding {}: {error}", binding.id))?;
            match &binding.action {
                HardwareKeyBindingAction::Invoke { action_id, .. } => {
                    if action_id.trim().is_empty() {
                        return Err(format!("hardware binding {} has no action", binding.id));
                    }
                }
                HardwareKeyBindingAction::Toggle {
                    on_action_id,
                    off_action_id,
                } => {
                    if on_action_id.trim().is_empty() || off_action_id.trim().is_empty() {
                        return Err(format!(
                            "hardware binding {} has an invalid toggle",
                            binding.id
                        ));
                    }
                }
                HardwareKeyBindingAction::SetPwm { percent } => {
                    if !percent.is_finite() || !(0.0..=100.0).contains(percent) {
                        return Err(format!(
                            "hardware binding {} PWM percentage must be between 0 and 100",
                            binding.id
                        ));
                    }
                }
                HardwareKeyBindingAction::Hold {
                    press_action_id,
                    release_action_id,
                    ..
                } => {
                    if press_action_id.trim().is_empty() || release_action_id.trim().is_empty() {
                        return Err(format!(
                            "hardware binding {} has an invalid hold pair",
                            binding.id
                        ));
                    }
                }
            }
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
    fn displayed_config_path_does_not_expose_an_absolute_profile_path() {
        let path = if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata).join("pealayer").join("config.json")
        } else {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/home/test".into()))
                .join(".config")
                .join("pealayer")
                .join("config.json")
        };
        let displayed = display_config_path(&path);
        assert!(!std::path::Path::new(&displayed).is_absolute());
        assert!(displayed.ends_with("config.json"));
    }

    #[test]
    fn config_watcher_observes_external_file_changes() {
        let directory =
            std::env::temp_dir().join(format!("pealayer_watch_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.json");
        std::fs::write(&path, b"{}\n").unwrap();
        let watcher = ConfigFileWatcher::new(&path, || {}).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(60));
        std::fs::write(&path, b"{\"volume\":75}\n").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut changed = false;
        while std::time::Instant::now() < deadline && !changed {
            changed = watcher.take_changed().unwrap();
            if !changed {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        assert!(
            changed,
            "native configuration watcher did not report the write"
        );
        drop(watcher);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn test_default_config_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.volume, 100.0);
        assert!(!cfg.is_muted);
        assert!(!cfg.pin_controls);
        assert!(!cfg.show_remaining_time);
        assert!(cfg.recent_media.is_empty());
        assert!(cfg.last_media_target.is_none());
        assert!(!cfg.last_media_paused);
        assert!(cfg.restore_last_media_on_startup);
        assert!(cfg.remember_playback_position);
        assert_eq!(
            cfg.playback_position_history_limit,
            DEFAULT_PLAYBACK_POSITION_HISTORY_LIMIT
        );
        assert!(cfg.playback_positions.is_empty());
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
        assert_eq!(cfg.playback_speed, 1.0);
        assert_eq!(cfg.temporary_fast_forward_speed, 2.0);
        assert!(cfg.native_dialog_windows);
        assert!(cfg.auto_reload_config);
        assert_eq!(cfg.active_workspace_profile.as_deref(), Some("nle"));
        assert_eq!(cfg.workspace_profiles["simple"].name, "Simple");
        assert_eq!(cfg.workspace_profiles["nle"].name, "NLE");
    }

    #[test]
    fn playback_speeds_reject_non_finite_and_out_of_range_values() {
        let mut config = AppConfig::default();
        config.playback_speed = 0.0;
        assert!(config.validate().unwrap_err().contains("playback_speed"));

        config.playback_speed = 1.0;
        config.temporary_fast_forward_speed = f64::NAN;
        assert!(
            config
                .validate()
                .unwrap_err()
                .contains("temporary_fast_forward_speed")
        );
    }

    #[test]
    fn old_workspace_storage_is_seeded_once_but_user_deletions_stay_deleted() {
        let mut migrated: AppConfig = serde_json::from_str(
            r#"{"workspace_profiles":{},"workspace_profiles_initialized":false}"#,
        )
        .unwrap();
        assert!(!migrated.workspace_profiles_initialized);
        migrated.normalize_workspace_profiles();
        assert!(migrated.workspace_profiles_initialized);
        assert_eq!(migrated.workspace_profiles_revision, 1);
        assert_eq!(
            migrated
                .workspace_profiles
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["nle".to_string(), "simple".to_string()]
        );

        migrated.workspace_profiles.remove("simple");
        let json = serde_json::to_string(&migrated).unwrap();
        let mut reloaded: AppConfig = serde_json::from_str(&json).unwrap();
        reloaded.normalize_workspace_profiles();
        assert!(!reloaded.workspace_profiles.contains_key("simple"));
        assert!(reloaded.workspace_profiles.contains_key("nle"));
    }

    #[test]
    fn playback_position_history_is_trimmed_to_the_configured_limit() {
        let mut config = AppConfig::default();
        config.playback_position_history_limit = 2;
        config.playback_positions = (0..4)
            .map(|index| PlaybackPositionEntry {
                target: format!("/media/{index}.mp4"),
                position_seconds: f64::from(index),
                updated_at_unix_ms: index as u64,
            })
            .collect();

        config.normalize_playback_positions();

        assert_eq!(config.playback_positions.len(), 2);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn first_profile_metadata_migration_repairs_seed_captions_and_order_once() {
        let mut migrated = AppConfig::default();
        migrated.workspace_profiles_revision = 0;
        {
            let simple = migrated.workspace_profiles.get_mut("simple").unwrap();
            simple.name = "simple".to_string();
            simple.order = 0;
        }
        {
            let nle = migrated.workspace_profiles.get_mut("nle").unwrap();
            nle.name = "nle".to_string();
            nle.order = 0;
        }
        migrated.normalize_workspace_profiles();

        assert_eq!(migrated.workspace_profiles["simple"].name, "Simple");
        assert_eq!(migrated.workspace_profiles["simple"].order, 0);
        assert_eq!(migrated.workspace_profiles["nle"].name, "NLE");
        assert_eq!(migrated.workspace_profiles["nle"].order, 1);

        {
            let simple = migrated.workspace_profiles.get_mut("simple").unwrap();
            simple.name = "Cinema".to_string();
            simple.order = 1;
        }
        migrated.workspace_profiles.get_mut("nle").unwrap().order = 0;
        migrated.normalize_workspace_profiles();
        assert_eq!(migrated.workspace_profiles["simple"].name, "Cinema");
        assert_eq!(migrated.workspace_profiles["nle"].order, 0);
        assert_eq!(migrated.workspace_profiles["simple"].order, 1);
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
        cfg.last_media_target = Some(PathBuf::from("/test/file.mp4"));
        cfg.last_media_paused = true;
        cfg.workspace_session = WorkspaceProfile {
            name: String::new(),
            icon: String::new(),
            order: 0,
            nle: false,
            window_geometry: Some(WindowGeometry {
                x: 20.0,
                y: 30.0,
                width: 960.0,
                height: 640.0,
                maximized: false,
            }),
            dock_layout: Some("{\"surface\":\"hardware\"}".to_string()),
            dialogs: WorkspaceDialogs {
                board_information: true,
                board_information_tab: 2,
                channel_manager: true,
                hardware_control_key: Some("relay.5".to_string()),
                hardware_channel_detail_active: true,
                ..Default::default()
            },
            egui_memory: None,
        };
        cfg.workspace_profiles.insert(
            "hardware-review".to_string(),
            WorkspaceProfile {
                name: "Hardware review".to_string(),
                icon: "hardware".to_string(),
                order: 2,
                ..cfg.workspace_session.clone()
            },
        );
        cfg.active_workspace_profile = Some("hardware-review".to_string());

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
        assert_eq!(
            loaded.last_media_target,
            Some(PathBuf::from("/test/file.mp4"))
        );
        assert!(loaded.last_media_paused);
        assert!(loaded.native_dialog_windows);
        assert!(!loaded.workspace_session.nle);
        assert_eq!(
            loaded.workspace_session.window_geometry.unwrap().width,
            960.0
        );
        assert!(loaded.workspace_session.dialogs.board_information);
        assert_eq!(loaded.workspace_session.dialogs.board_information_tab, 2);
        assert!(loaded.workspace_session.dialogs.channel_manager);
        assert_eq!(
            loaded
                .workspace_session
                .dialogs
                .hardware_control_key
                .as_deref(),
            Some("relay.5")
        );
        assert!(
            loaded
                .workspace_session
                .dialogs
                .hardware_channel_detail_active
        );
        assert!(loaded.workspace_profiles.contains_key("hardware-review"));
        assert_eq!(
            loaded.active_workspace_profile.as_deref(),
            Some("hardware-review")
        );
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
                "non_user_control_visibility": "hidden",
                "prefix_relay_identifiers": false,
                "show_estop_control": false,
                "confirm_estop_release": false,
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
        assert_eq!(
            updated.non_user_control_visibility,
            NonUserControlVisibility::Hidden
        );
        assert!(!updated.prefix_relay_identifiers);
        assert!(!updated.show_estop_control);
        assert!(!updated.confirm_estop_release);
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
    fn emergency_stop_ui_is_visible_and_release_is_guarded_by_default() {
        let config = AppConfig::default();
        assert!(config.show_estop_control);
        assert!(config.confirm_estop_release);
    }

    #[test]
    fn motion_controls_default_to_push_behavior() {
        assert_eq!(MotionControlMode::default(), MotionControlMode::Hold);
        assert_eq!(
            AppConfig::default().motion_control_mode,
            MotionControlMode::Hold
        );
    }

    #[test]
    fn non_user_control_visibility_round_trips_as_one_policy() {
        let config = AppConfig {
            non_user_control_visibility: NonUserControlVisibility::Shown,
            ..AppConfig::default()
        };
        let serialized = serde_json::to_value(config).unwrap();
        assert_eq!(
            serialized.get("non_user_control_visibility"),
            Some(&serde_json::json!("shown"))
        );
        let restored: AppConfig = serde_json::from_value(serialized).unwrap();
        assert_eq!(
            restored.non_user_control_visibility,
            NonUserControlVisibility::Shown
        );
    }

    #[test]
    fn hardware_key_bindings_round_trip_and_reject_unsafe_values() {
        let binding = HardwareKeyBinding {
            id: "seat-a-hold".to_string(),
            channel_key: "seat.a".to_string(),
            chord: KeyChord {
                key: "KeyU".to_string(),
                control: true,
                ..Default::default()
            },
            action: HardwareKeyBindingAction::Hold {
                press_action_id: "seat.a.up".to_string(),
                release_action_id: "seat.a.stop".to_string(),
                press_label: "Up".to_string(),
                release_label: "Stop".to_string(),
            },
            global: true,
            enabled: true,
        };
        let config = AppConfig {
            hardware_key_bindings: vec![binding.clone()],
            ..Default::default()
        };
        assert!(config.validate().is_ok());
        let restored: AppConfig = serde_json::from_value(serde_json::to_value(&config).unwrap())
            .expect("binding configuration round trip");
        assert_eq!(restored.hardware_key_bindings, vec![binding]);

        let mut invalid = config;
        invalid.hardware_key_bindings.push(HardwareKeyBinding {
            id: "bad-level".to_string(),
            channel_key: "pwm.0".to_string(),
            action: HardwareKeyBindingAction::SetPwm { percent: 101.0 },
            ..Default::default()
        });
        assert!(
            invalid
                .validate()
                .unwrap_err()
                .contains("between 0 and 100")
        );
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
