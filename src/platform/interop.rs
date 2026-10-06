use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
#[cfg(any(unix, windows))]
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub operation_id: String,
    pub application_identity: String,
    pub sender_session_id: Option<u32>,
    pub sender_working_directory: Option<String>,
    pub target: Option<String>,
    pub fullscreen: bool,
    pub volume: Option<f64>,
    pub activate: bool,
    #[serde(default)]
    pub commands: Vec<InteropCommand>,
}

impl LaunchRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.operation_id.trim().is_empty() || self.operation_id.len() > 128 {
            return Err("launch operation_id must contain 1 to 128 bytes".to_string());
        }
        if self.application_identity.trim().is_empty() || self.application_identity.len() > 256 {
            return Err("launch application_identity must contain 1 to 256 bytes".to_string());
        }
        if self
            .sender_working_directory
            .as_ref()
            .is_some_and(|value| value.len() > 32_768)
            || self
                .target
                .as_ref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 32_768)
        {
            return Err(
                "launch target must be non-empty and path fields must not exceed 32768 bytes"
                    .to_string(),
            );
        }
        if self
            .volume
            .is_some_and(|value| !value.is_finite() || !(0.0..=130.0).contains(&value))
        {
            return Err("launch volume must be a finite value from 0 to 130".to_string());
        }
        if self.commands.len() > 64 {
            return Err("launch request must not contain more than 64 commands".to_string());
        }
        for command in &self.commands {
            if matches!(command, InteropCommand::Launch { .. }) {
                return Err("launch request must not contain a nested launch command".to_string());
            }
            command.validate()?;
        }
        Ok(())
    }
}

#[cfg(any(unix, windows, test))]
fn normalized_application_identity(identity: &str) -> String {
    identity.trim().to_lowercase()
}

#[cfg(any(unix, windows, test))]
fn validate_launch_destination(
    request: &LaunchRequest,
    expected_identity: &str,
    _expected_session_id: Option<u32>,
) -> Result<(), String> {
    if normalized_application_identity(&request.application_identity)
        != normalized_application_identity(expected_identity)
    {
        return Err("launch request targets a different application identity".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        if request.sender_session_id != _expected_session_id {
            return Err("launch request targets a different Windows session".to_string());
        }
    }
    Ok(())
}

#[cfg(any(unix, windows, test))]
#[derive(Default)]
struct LaunchReceiptCache {
    operation_ids: std::collections::VecDeque<String>,
}

#[cfg(any(unix, windows, test))]
impl LaunchReceiptCache {
    const CAPACITY: usize = 1024;

    fn claim(&mut self, operation_id: &str) -> bool {
        if self.operation_ids.iter().any(|known| known == operation_id) {
            return false;
        }
        if self.operation_ids.len() == Self::CAPACITY {
            self.operation_ids.pop_front();
        }
        self.operation_ids.push_back(operation_id.to_string());
        true
    }

    fn release(&mut self, operation_id: &str) {
        self.operation_ids.retain(|known| known != operation_id);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum InteropCommand {
    RfControl { operation: String, #[serde(default)] params: Value },
    OpenRfManager,
    Launch {
        request: LaunchRequest,
    },
    Play,
    Pause,
    TogglePause,
    Stop,
    Next,
    Previous,
    PreviousChapter,
    NextChapter,
    SetChapter {
        index: i64,
    },
    Seek {
        seconds: f64,
    },
    SeekTo {
        seconds: f64,
    },
    SeekAbs {
        percentage: f64,
    },
    #[serde(alias = "volume")]
    SetVolume {
        #[serde(alias = "level")]
        value: f64,
    },
    SetMute {
        muted: bool,
    },
    ToggleMute,
    SetRate {
        rate: f64,
    },
    #[serde(alias = "open_video")]
    Open {
        #[serde(alias = "path")]
        target: String,
    },
    BrowseRemote { #[serde(default)] target: String, #[serde(default)] use_proxy: Option<bool> },
    SelectRemote { target: String, #[serde(default)] play: bool },
    SortRemote { by: crate::remote_location::SortBy, #[serde(default)] descending: bool },
    CloseRemoteBrowser,
    SetFullscreen {
        enabled: bool,
    },
    ToggleFullscreen,
    Activate,
    Minimize,
    Maximize,
    Restore,
    OpenPreferences,
    OpenMediaInformation,
    OpenMediaFolder,
    EditConfiguration,
    OpenBoardInformation {
        tab: usize,
    },
    ShowMessage {
        message: String,
    },
    PublishToast {
        #[serde(flatten)]
        toast: crate::messaging::ToastRequest,
    },
    DismissToast { id: String },
    ShowOsd {
        message: String,
        #[serde(default)]
        options: OsdOptions,
    },
    HideOsd,
    Quit,
    SetWorkspace {
        profile: String,
    },
    CreateWorkspaceProfile {
        name: String,
        icon: String,
    },
    UpdateWorkspaceProfile {
        id: String,
        name: String,
        icon: String,
        capture: bool,
    },
    DeleteWorkspaceProfile {
        id: String,
    },
    MoveWorkspaceProfile {
        id: String,
        direction: i32,
    },
    AddEffectCue {
        effect_id: String,
        start_time_ms: u64,
    },
    RemoveEffectCue {
        instance_id: String,
    },
    UpdateEffectCue {
        instance_id: String,
        start_time_ms: u64,
        duration_ms: u64,
    },
    AddControllerEffectCue {
        reference: String,
        start_time_ms: u64,
    },
    PlayControllerEffect {
        reference: String,
    },
    StopControllerEffect,
    CreateControllerEffectGroup {
        name: String,
        icon: String,
    },
    DeleteControllerEffect {
        reference: String,
    },
    SaveControllerEffect {
        effect: WebControllerEffectDraft,
    },
    StartControllerEffectRecording {
        name: String,
        category: String,
        color: String,
        mode: String,
        #[serde(default)]
        effect: Option<WebControllerEffectDraft>,
    },
    RefreshControllerEffectRecording,
    SaveControllerEffectRecording,
    DiscardControllerEffectRecording,
    SetRecording {
        enabled: bool,
    },
    SetEmergencyStop {
        active: bool,
    },
    InvokeHardwareAction {
        action_id: String,
    },
    SetHardwarePwm {
        channel: u8,
        percent: f64,
    },
    UpdateHardwarePresentation {
        key: String,
        fields: Value,
    },
    ConfigureAddressableStrip {
        pixels: u16,
    },
    FillAddressableStrip {
        red: u8,
        green: u8,
        blue: u8,
        brightness: u8,
    },
    ClearAddressableStrip,
    PressFrontPanelKey {
        key: String,
    },
    UpdateConfig {
        values: Value,
    },
    PreviewConfig {
        config: Box<crate::config::AppConfig>,
    },
    CommitPreviewConfig {
        config: Box<crate::config::AppConfig>,
    },
    CancelPreviewConfig,
    ReloadConfig,
    GetStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsdAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Optional per-message OSD presentation overrides. Omitted values inherit
/// the user's application preferences, while X/Y percentages override the
/// named anchor and place the overlay around that point in the video surface.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OsdOptions {
    pub position: Option<OsdAnchor>,
    pub x_percent: Option<f32>,
    pub y_percent: Option<f32>,
    pub font_size: Option<f32>,
    pub icon: Option<String>,
    pub text_color: Option<String>,
    pub background_color: Option<String>,
    pub timeout_seconds: Option<f32>,
    pub padding_x: Option<f32>,
    pub padding_y: Option<f32>,
    pub corner_radius: Option<f32>,
}

fn valid_osd_color(value: &str) -> bool {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl OsdOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .x_percent
            .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || self
                .y_percent
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
        {
            return Err("OSD X/Y percentages must be finite values from 0 to 100".to_string());
        }
        if self
            .font_size
            .is_some_and(|value| !value.is_finite() || !(8.0..=128.0).contains(&value))
        {
            return Err("OSD font size must be a finite value from 8 to 128".to_string());
        }
        if self
            .timeout_seconds
            .is_some_and(|value| !value.is_finite() || !(0.25..=300.0).contains(&value))
        {
            return Err("OSD timeout must be a finite value from 0.25 to 300 seconds".to_string());
        }
        if self
            .padding_x
            .is_some_and(|value| !value.is_finite() || !(0.0..=96.0).contains(&value))
            || self
                .padding_y
                .is_some_and(|value| !value.is_finite() || !(0.0..=96.0).contains(&value))
            || self
                .corner_radius
                .is_some_and(|value| !value.is_finite() || !(0.0..=64.0).contains(&value))
        {
            return Err("OSD padding/radius values are outside their supported range".to_string());
        }
        if self
            .icon
            .as_ref()
            .is_some_and(|value| value.chars().count() > 64 || value.chars().any(char::is_control))
        {
            return Err("OSD icon name must not exceed 64 printable characters".to_string());
        }
        for color in [&self.text_color, &self.background_color]
            .into_iter()
            .flatten()
        {
            if !valid_osd_color(color) {
                return Err(
                    "OSD colors must use #RGB, #RGBA, #RRGGBB, or #RRGGBBAA notation".to_string(),
                );
            }
        }
        Ok(())
    }
}

fn valid_workspace_profile_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn valid_workspace_profile_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().count() <= 64 && !value.chars().any(char::is_control)
}

fn valid_workspace_profile_icon(value: &str) -> bool {
    valid_workspace_profile_id(value)
}

impl InteropCommand {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::RfControl { operation, params } => {
                if !crate::ui::rf::OPERATIONS.contains(&operation.as_str()) || !params.is_object() || params.to_string().len() > 32768 {
                    Err("Invalid RF operation or parameters".to_string())
                } else { Ok(()) }
            },
            Self::Launch { request } => request.validate(),
            Self::Seek { seconds } if !seconds.is_finite() => {
                Err("seek value must be finite".to_string())
            }
            Self::SeekTo { seconds } if !seconds.is_finite() || *seconds < 0.0 => {
                Err("absolute seek time must be a finite non-negative value".to_string())
            }
            Self::SeekAbs { percentage }
                if !percentage.is_finite() || !(0.0..=100.0).contains(percentage) =>
            {
                Err("seek percentage must be a finite value from 0 to 100".to_string())
            }
            Self::SetChapter { index } if *index < 0 => {
                Err("chapter index must be zero or greater".to_string())
            }
            Self::SetVolume { value } if !value.is_finite() || !(0.0..=130.0).contains(value) => {
                Err("volume must be a finite value from 0 to 130".to_string())
            }
            Self::SetRate { rate } if !rate.is_finite() || !(0.05..=16.0).contains(rate) => {
                Err("playback rate must be a finite value from 0.05 to 16".to_string())
            }
            Self::SetHardwarePwm { percent, .. }
                if !percent.is_finite() || !(0.0..=100.0).contains(percent) =>
            {
                Err("PWM percent must be a finite value from 0 to 100".to_string())
            }
            Self::ConfigureAddressableStrip { pixels } if *pixels == 0 => {
                Err("addressable strip pixel count must be greater than zero".to_string())
            }
            Self::InvokeHardwareAction { action_id }
                if action_id.trim().is_empty()
                    || action_id.len() > 128
                    || !action_id.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
                    }) =>
            {
                Err("hardware action ID is invalid".to_string())
            }
            Self::UpdateHardwarePresentation { key, fields }
                if key.trim().is_empty()
                    || key.len() > 128
                    || !key.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
                    })
                    || !fields.is_object() =>
            {
                Err("hardware presentation update is invalid".to_string())
            }
            Self::PressFrontPanelKey { key }
                if !matches!(key.to_ascii_uppercase().as_str(), "K1" | "K2" | "K3" | "K4") =>
            {
                Err("front-panel key must be K1, K2, K3, or K4".to_string())
            }
            Self::OpenBoardInformation { tab } if *tab > 3 => {
                Err("board information tab must be between 0 and 3".to_string())
            }
            Self::Open { target } if target.trim().is_empty() || target.len() > 32_768 => {
                Err("media target must contain 1 to 32768 bytes".to_string())
            }
            Self::AddEffectCue { effect_id, .. }
                if uuid::Uuid::parse_str(effect_id.trim()).is_err() =>
            {
                Err("effect_id must be a valid effect UUID".to_string())
            }
            Self::RemoveEffectCue { instance_id }
                if uuid::Uuid::parse_str(instance_id.trim()).is_err() =>
            {
                Err("instance_id must be a valid cue UUID".to_string())
            }
            Self::UpdateEffectCue {
                instance_id,
                duration_ms,
                ..
            } if uuid::Uuid::parse_str(instance_id.trim()).is_err()
                || *duration_ms == 0
                || *duration_ms > 86_400_000 =>
            {
                Err(
                    "cue update requires a valid UUID and duration from 1 ms to 24 hours"
                        .to_string(),
                )
            }
            Self::AddControllerEffectCue { reference, .. }
            | Self::PlayControllerEffect { reference }
            | Self::DeleteControllerEffect { reference }
                if !valid_controller_effect_reference(reference) =>
            {
                Err("controller effect reference is invalid".to_string())
            }
            Self::SaveControllerEffect { effect } => effect.validate(),
            Self::CreateControllerEffectGroup { name, icon }
                if name.trim().is_empty()
                    || name.len() > 64
                    || icon.len() > 64
                    || name.chars().any(char::is_control)
                    || icon.chars().any(char::is_control) =>
            {
                Err("Group name and icon must be bounded printable values".to_owned())
            }
            Self::StartControllerEffectRecording {
                name,
                category,
                color,
                mode,
                ..
            } if name.trim().is_empty()
                || name.len() > 64
                || category.trim().is_empty()
                || category.len() > 64
                || !matches!(
                    color.trim().to_ascii_lowercase().as_str(),
                    "red" | "blue" | "violet" | "purple" | "green" | "white"
                )
                || !matches!(
                    mode.as_str(),
                    "automatic" | "device-clock" | "board-retained"
                ) =>
            {
                Err(
                    "effect recording name, category, color, or capture mode is invalid"
                        .to_string(),
                )
            }
            Self::StartControllerEffectRecording { effect: Some(effect), .. } => {
                if effect.kind != "sequence" {
                    return Err("Capture requires a sequence effect".into());
                }
                effect.validate()
            }
            Self::ShowMessage { message } if message.chars().count() > 2_048 => {
                Err("message must not exceed 2048 characters".to_string())
            }
            Self::ShowOsd { message, .. } if message.chars().count() > 2_048 => {
                Err("OSD message must not exceed 2048 characters".to_string())
            }
            Self::ShowOsd { options, .. } => options.validate(),
            Self::PublishToast { toast } => toast.validate(),
            Self::BrowseRemote { target, .. } if !target.is_empty() => crate::remote_location::normalize(target).map(|_| ()),
            Self::SelectRemote { target, .. } => crate::remote_location::normalize(target).map(|_| ()),
            Self::DismissToast { id } if !crate::messaging::valid_id(id) => Err("invalid toast ID".into()),
            Self::SetWorkspace { profile }
            | Self::DeleteWorkspaceProfile { id: profile }
            | Self::MoveWorkspaceProfile { id: profile, .. }
                if !valid_workspace_profile_id(profile) =>
            {
                Err("workspace profile ID is invalid".to_string())
            }
            Self::CreateWorkspaceProfile { name, icon }
                if !valid_workspace_profile_name(name) || !valid_workspace_profile_icon(icon) =>
            {
                Err("workspace profile name or icon is invalid".to_string())
            }
            Self::UpdateWorkspaceProfile { id, name, icon, .. }
                if !valid_workspace_profile_id(id)
                    || !valid_workspace_profile_name(name)
                    || !valid_workspace_profile_icon(icon) =>
            {
                Err("workspace profile update is invalid".to_string())
            }
            Self::MoveWorkspaceProfile { direction, .. } if !matches!(direction, -1 | 1) => {
                Err("workspace profile direction must be -1 or 1".to_string())
            }
            Self::UpdateConfig { values } => crate::config::AppConfig::validate_patch_shape(values),
            Self::PreviewConfig { config } | Self::CommitPreviewConfig { config } => {
                config.validate()
            }
            _ => Ok(()),
        }
    }
}

pub fn command_catalog() -> Value {
    serde_json::json!({
        "contract": "pealayer.control",
        "transports": ["native", "http", "json-rpc", "websocket"],
        "messaging": { "contract": "pealayer.messages.v1", "snapshot": "/api/messages", "subscription": "/ws", "publish": "pealayer.toast.show", "dismiss": "pealayer.toast.dismiss", "state": "pealayer.messages.state", "surfaces": ["egui", "web", "terminal"], "persistent_timeout_ms": 0 },
        "remote_folders": { "browse": "pealayer.remote.browse", "select": "pealayer.remote.select", "sort": "pealayer.remote.sort", "close": "pealayer.remote.close", "state": "/api/remote/state", "thumbnail": "/api/remote/thumbnail", "subscription": "/ws" },
        "commands": [
            "open", "play", "pause", "toggle_pause", "stop", "next", "previous",
            "browse_remote", "select_remote", "sort_remote", "close_remote_browser",
            "chapter_previous", "chapter_next", "set_chapter",
            "seek", "seek_to", "seek_abs", "set_volume", "set_mute", "toggle_mute",
            "set_rate", "set_fullscreen", "toggle_fullscreen", "activate", "minimize",
            "maximize", "restore", "open_preferences", "open_media_information", "open_media_folder", "edit_configuration", "open_board_information", "show_message", "show_osd", "hide_osd", "set_workspace",
            "create_workspace_profile", "update_workspace_profile", "delete_workspace_profile",
            "move_workspace_profile", "update_config",
            "reload_config", "add_effect_cue", "update_effect_cue", "remove_effect_cue", "set_recording",
            "get_status", "publish_toast", "dismiss_toast", "quit", "controller_effect_cue.add", "controller_effect.play",
            "controller_effect.stop", "controller_effect.save", "controller_effect.delete",
            "controller_effect.group.create",
            "controller_effect.record.start", "controller_effect.record.status",
            "controller_effect.record.save", "controller_effect.record.discard",
            "set_emergency_stop", "invoke_hardware_action", "set_hardware_pwm",
            "configure_addressable_strip", "fill_addressable_strip", "clear_addressable_strip",
            "press_front_panel_key", "board_information", "rf_control", "open_rf_manager"
        ],
        "json_rpc_prefix": "pealayer",
        "discovery": "/api/player/commands"
    })
}

pub fn parse_text_command(input: &str) -> Result<InteropCommand, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("command must not be empty".to_string());
    }
    if trimmed.starts_with('{') {
        return parse_interop_request(trimmed).map(|(_, command)| command);
    }
    let (name, argument) = trimmed
        .split_once(char::is_whitespace)
        .map(|(name, argument)| (name, argument.trim()))
        .unwrap_or((trimmed, ""));
    let number = |label: &str| {
        argument
            .parse::<f64>()
            .map_err(|_| format!("{label} requires a numeric value"))
    };
    let boolean = || match argument.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        _ => Err("expected on/off, true/false, or 1/0".to_string()),
    };
    let effect_cue = || {
        let mut values = argument.split_whitespace();
        let effect_id = values
            .next()
            .ok_or_else(|| "add-effect-cue requires an effect UUID".to_string())?;
        let start_time_ms = values
            .next()
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|_| "add-effect-cue start time must be milliseconds".to_string())
            })
            .transpose()?
            .unwrap_or_default();
        if values.next().is_some() {
            return Err(
                "add-effect-cue accepts an effect UUID and optional start time".to_string(),
            );
        }
        Ok(InteropCommand::AddEffectCue {
            effect_id: effect_id.to_string(),
            start_time_ms,
        })
    };
    let command = match name.to_ascii_lowercase().as_str() {
        "play" => InteropCommand::Play,
        "pause" => InteropCommand::Pause,
        "toggle" | "toggle_pause" | "toggle-pause" => InteropCommand::TogglePause,
        "stop" => InteropCommand::Stop,
        "next" => InteropCommand::Next,
        "previous" | "prev" => InteropCommand::Previous,
        "chapter_previous" | "chapter-previous" | "previous_chapter" | "previous-chapter" => {
            InteropCommand::PreviousChapter
        }
        "chapter_next" | "chapter-next" | "next_chapter" | "next-chapter" => {
            InteropCommand::NextChapter
        }
        "chapter" | "set_chapter" | "set-chapter" => InteropCommand::SetChapter {
            index: argument
                .parse::<i64>()
                .map_err(|_| "chapter requires a zero-based chapter index".to_string())?,
        },
        "seek" => InteropCommand::Seek {
            seconds: number("seek")?,
        },
        "seek_to" | "seek-to" => InteropCommand::SeekTo {
            seconds: number("seek-to")?,
        },
        "seek_abs" | "seek-abs" => InteropCommand::SeekAbs {
            percentage: number("seek-abs")?,
        },
        "volume" | "set_volume" | "set-volume" => InteropCommand::SetVolume {
            value: number("volume")?,
        },
        "mute" if argument.is_empty() => InteropCommand::SetMute { muted: true },
        "unmute" => InteropCommand::SetMute { muted: false },
        "mute" | "set_mute" | "set-mute" => InteropCommand::SetMute { muted: boolean()? },
        "toggle_mute" | "toggle-mute" => InteropCommand::ToggleMute,
        "rate" | "set_rate" | "set-rate" => InteropCommand::SetRate {
            rate: number("rate")?,
        },
        "open" => InteropCommand::Open {
            target: argument.to_string(),
        },
        "browse_remote" => InteropCommand::BrowseRemote { target: argument.into(), use_proxy: None },
        "close_remote_browser" => InteropCommand::CloseRemoteBrowser,
        "fullscreen" | "set_fullscreen" | "set-fullscreen" => InteropCommand::SetFullscreen {
            enabled: boolean()?,
        },
        "toggle_fullscreen" | "toggle-fullscreen" => InteropCommand::ToggleFullscreen,
        "activate" | "focus" => InteropCommand::Activate,
        "minimize" => InteropCommand::Minimize,
        "maximize" => InteropCommand::Maximize,
        "restore" => InteropCommand::Restore,
        "preferences" | "open_preferences" | "open-preferences" => InteropCommand::OpenPreferences,
        "media_information" | "open_media_information" => InteropCommand::OpenMediaInformation,
        "media_folder" | "open_media_folder" => InteropCommand::OpenMediaFolder,
        "edit_configuration" | "edit_config" => InteropCommand::EditConfiguration,
        "message" | "show_message" | "show-message" => InteropCommand::ShowMessage {
            message: argument.to_string(),
        },
        "toast" => InteropCommand::PublishToast { toast: crate::messaging::ToastRequest {
            id: None, title: String::new(), message: argument.to_string(),
            severity: crate::messaging::Severity::Info, timeout_ms: 5000,
        } },
        "dismiss_toast" => InteropCommand::DismissToast { id: argument.into() },
        "osd" | "show_osd" | "show-osd" => InteropCommand::ShowOsd {
            message: argument.to_string(),
            options: OsdOptions::default(),
        },
        "hide_osd" | "hide-osd" | "clear_osd" | "clear-osd" => InteropCommand::HideOsd,
        "workspace" | "set_workspace" | "set-workspace" => InteropCommand::SetWorkspace {
            profile: argument.to_string(),
        },
        "add_effect_cue" | "add-effect-cue" => effect_cue()?,
        "remove_effect_cue" | "remove-effect-cue" => InteropCommand::RemoveEffectCue {
            instance_id: argument.to_string(),
        },
        "recording" | "set_recording" | "set-recording" => InteropCommand::SetRecording {
            enabled: boolean()?,
        },
        "estop" | "e-stop" | "emergency_stop" | "emergency-stop" => {
            InteropCommand::SetEmergencyStop { active: boolean()? }
        }
        "status" | "get_status" | "get-status" => InteropCommand::GetStatus,
        "reload_config" | "reload-config" => InteropCommand::ReloadConfig,
        "quit" | "exit" => InteropCommand::Quit,
        _ => return Err(format!("unknown Pealayer command: {name}")),
    };
    command.validate()?;
    Ok(command)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppearanceState {
    pub theme: crate::config::AppTheme,
    pub color_palette: crate::config::ColorPalette,
    pub accent_color: crate::config::AccentColor,
    pub custom_accent_color: Option<String>,
    /// Resolved by the host, not the browser's potentially different OS theme.
    pub resolved_theme: crate::config::AppTheme,
    pub resolved_accent: String,
}

impl AppearanceState {
    pub fn new(config: &crate::config::AppConfig, dark: bool) -> Self {
        let [red, green, blue] = crate::ui::platform_accent_rgb(config);
        Self {
            theme: crate::config::resolved_theme(config),
            color_palette: config.color_palette,
            accent_color: config.accent_color,
            custom_accent_color: config.custom_accent_color.clone(),
            resolved_theme: if dark {
                crate::config::AppTheme::Dark
            } else {
                crate::config::AppTheme::Light
            },
            resolved_accent: format!("#{red:02x}{green:02x}{blue:02x}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatusResponse {
    #[serde(default)] pub rf: Value,
    #[serde(default)]
    pub remote_browser: crate::remote_location::BrowserState,
    #[serde(default)]
    pub messages: crate::messaging::MessageSnapshot,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<AppearanceState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_wheel_preferences: Option<crate::config::TimelineWheelPreferences>,
    pub playing: bool,
    pub volume: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default = "default_playback_rate")]
    pub playback_rate: f64,
    pub playback_time: f64,
    pub duration: f64,
    pub current_video: Option<String>,
    #[serde(default)]
    pub chapters: Vec<WebMediaChapter>,
    #[serde(default)]
    pub current_chapter_index: Option<i64>,
    #[serde(default)]
    pub seekable: bool,
    #[serde(default)]
    pub live: bool,
    #[serde(default)]
    pub buffered_until: Option<f64>,
    #[serde(default)]
    pub buffering_percent: Option<f64>,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub active_workspace_profile: Option<String>,
    #[serde(default)]
    pub workspace_profiles: Vec<WebWorkspaceProfile>,
    #[serde(default)]
    pub controller_connected: bool,
    #[serde(default)]
    pub hardware_connection_requested: bool,
    #[serde(default)]
    pub hardware_endpoint: String,
    #[serde(default)]
    pub hardware_transport: Option<String>,
    #[serde(default)]
    pub hardware_connected: bool,
    /// Last coordinator/board transport error. Local authenticated clients use
    /// this to present an actionable reconnecting state instead of a generic
    /// offline label.
    #[serde(default)]
    pub hardware_error: Option<String>,
    #[serde(default)]
    pub hardware_sync: serde_json::Value,
    #[serde(default)]
    pub estop_active: bool,
    #[serde(default)]
    pub hardware: Option<HardwareStatusSummary>,
    #[serde(default)]
    pub recording: bool,
    #[serde(default)]
    pub recording_armed: bool,
    #[serde(default)]
    pub recordable_track_count: usize,
    #[serde(default)]
    pub effects: Vec<WebEffectProfile>,
    #[serde(default)]
    pub controller_effects: Vec<WebControllerEffect>,
    #[serde(default)]
    pub controller_effect_groups: Vec<crate::four_d::controller::HardwareEffectGroup>,
    #[serde(default)]
    pub effect_recording: WebEffectRecording,
    #[serde(default)]
    pub cues: Vec<WebEffectCue>,
    #[serde(default)]
    pub hardware_details: Option<Value>,
    #[serde(default)]
    pub update: crate::update::UpdateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebMediaChapter {
    pub index: i64,
    pub title: String,
    pub time_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebEffectProfile {
    pub id: String,
    pub name: String,
    pub duration_ms: u64,
    pub duration_display: String,
    pub action_count: usize,
    pub target: String,
    #[serde(default)]
    pub lane: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebWorkspaceProfile {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub order: i32,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebEffectCue {
    pub id: String,
    pub effect_id: String,
    pub name: String,
    pub start_time_ms: u64,
    pub duration_ms: u64,
    pub duration_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebControllerEffect {
    pub reference: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    pub category: String,
    pub description: String,
    pub kind: String,
    pub duration_ms: u64,
    pub duration_display: String,
    pub action_count: usize,
    pub editable: bool,
    #[serde(default)]
    pub lane: String,
    pub program: Value,
    pub default_fps: Option<u8>,
    pub default_pixels: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebEffectRecording {
    pub active: bool,
    pub id: u8,
    pub name: String,
    pub mode: String,
    pub category: String,
    pub color: String,
    pub steps: usize,
    #[serde(default)]
    pub preview: Vec<crate::four_d::controller::HardwareMacroStep>,
    pub device_retained: bool,
    pub overwritten: usize,
    pub started_at: String,
    pub last_error: String,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WebControllerEffectDraft {
    #[serde(default)]
    pub reference: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    pub category: String,
    #[serde(default)]
    pub description: String,
    pub kind: String,
    #[serde(default)]
    pub program: Value,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub default_fps: u8,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub default_pixels: u16,
    #[serde(default)]
    pub is_new: bool,
}

fn valid_controller_effect_reference(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ':' | '.' | '-' | '_')
        })
}

impl WebControllerEffectDraft {
    fn validate(&self) -> Result<(), String> {
        let field = |label: &str, value: &str| {
            let value = value.trim();
            (!value.is_empty()
                && value.len() <= 64
                && value
                    .chars()
                    .all(|character| character.is_alphanumeric() || " -_".contains(character)))
            .then_some(())
            .ok_or_else(|| format!("{label} must contain 1 to 64 safe characters"))
        };
        field("effect name", &self.name)?;
        field("effect category", &self.category)?;
        if !self.icon.trim().is_empty() {
            field("effect icon", &self.icon)?;
        }
        match self.kind.as_str() {
            "sequence" => self
                .id
                .parse::<u8>()
                .map(|_| ())
                .map_err(|_| "sequence effect id must be from 0 to 255".to_string()),
            "strip-stream" => {
                if !valid_controller_effect_reference(&self.id) || !self.program.is_object() {
                    return Err(
                        "lighting effect requires a safe id and a program object".to_string()
                    );
                }
                Ok(())
            }
            _ => Err("effect kind must be sequence or strip-stream".to_string()),
        }
    }
}

fn default_playback_rate() -> f64 {
    1.0
}

impl Default for PlayerStatusResponse {
    fn default() -> Self {
        Self {
            rf: Value::Null,
            status: String::new(),
            messages: crate::messaging::MessageSnapshot::default(),
            remote_browser: crate::remote_location::BrowserState::default(),
            appearance: None,
            timeline_wheel_preferences: None,
            playing: false,
            volume: 0.0,
            muted: false,
            playback_rate: default_playback_rate(),
            playback_time: 0.0,
            duration: 0.0,
            current_video: None,
            chapters: Vec::new(),
            current_chapter_index: None,
            seekable: false,
            live: false,
            buffered_until: None,
            buffering_percent: None,
            fullscreen: false,
            workspace: String::new(),
            active_workspace_profile: None,
            workspace_profiles: Vec::new(),
            controller_connected: false,
            hardware_connection_requested: false,
            hardware_endpoint: String::new(),
            hardware_transport: None,
            hardware_connected: false,
            hardware_error: None,
            hardware_sync: serde_json::Value::Null,
            estop_active: false,
            hardware: None,
            recording: false,
            recording_armed: false,
            recordable_track_count: 0,
            effects: Vec::new(),
            controller_effects: Vec::new(),
            controller_effect_groups: Vec::new(),
            effect_recording: WebEffectRecording::default(),
            cues: Vec::new(),
            hardware_details: None,
            update: crate::update::UpdateStatus::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareStatusSummary {
    pub board_name: String,
    pub relay_count: usize,
    pub pwm_count: usize,
    pub supports_rf_transmit: bool,
    pub supports_addressable_led: bool,
    pub supports_segment_display: bool,
    pub supports_lcd_display: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

pub fn command_from_json_rpc(request: &JsonRpcRequest) -> Result<Option<InteropCommand>, String> {
    if request
        .jsonrpc
        .as_deref()
        .is_some_and(|version| version != "2.0")
    {
        return Err("unsupported JSON-RPC version".to_string());
    }
    let number = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| request.params.get(*name).and_then(Value::as_f64))
            .ok_or_else(|| format!("missing numeric parameter: {}", names.join(" or ")))
    };
    let string = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| request.params.get(*name).and_then(Value::as_str))
            .map(str::to_string)
            .ok_or_else(|| format!("missing string parameter: {}", names.join(" or ")))
    };
    let command = match request.method.as_str() {
        "pealayer.rf" | "rf_control" => Some(InteropCommand::RfControl { operation: string(&["operation"])?, params: request.params.get("params").cloned().unwrap_or_else(||serde_json::json!({})) }),
        "pealayer.rf.open" => Some(InteropCommand::OpenRfManager),
        "play" | "pealayer.play" | "pealayer.player.play" => Some(InteropCommand::Play),
        "pause" | "pealayer.pause" | "pealayer.player.pause" => Some(InteropCommand::Pause),
        "toggle" | "toggle_pause" | "pealayer.toggle" | "pealayer.player.toggle" => {
            Some(InteropCommand::TogglePause)
        }
        "stop" | "pealayer.stop" | "pealayer.player.stop" => Some(InteropCommand::Stop),
        "next" | "pealayer.next" | "pealayer.player.next" => Some(InteropCommand::Next),
        "previous" | "prev" | "pealayer.previous" | "pealayer.player.previous" => {
            Some(InteropCommand::Previous)
        }
        "chapter_previous"
        | "previous_chapter"
        | "pealayer.chapter.previous"
        | "pealayer.player.chapter.previous" => Some(InteropCommand::PreviousChapter),
        "chapter_next"
        | "next_chapter"
        | "pealayer.chapter.next"
        | "pealayer.player.chapter.next" => Some(InteropCommand::NextChapter),
        "chapter" | "set_chapter" | "pealayer.chapter.set" | "pealayer.player.chapter.set" => {
            let index = request
                .params
                .get("index")
                .or_else(|| request.params.get("chapter"))
                .and_then(Value::as_i64)
                .ok_or_else(|| "missing integer parameter: index".to_string())?;
            Some(InteropCommand::SetChapter { index })
        }
        "seek" | "pealayer.seek" | "pealayer.player.seek" => Some(InteropCommand::Seek {
            seconds: number(&["seconds"])?,
        }),
        "seek_to" | "pealayer.seek_to" | "pealayer.player.seek_to" => {
            Some(InteropCommand::SeekTo {
                seconds: number(&["seconds", "position"])?,
            })
        }
        "seek_abs" | "pealayer.seek_absolute" | "pealayer.player.seek_absolute" => {
            Some(InteropCommand::SeekAbs {
                percentage: number(&["percentage"])?,
            })
        }
        "volume" | "set_volume" | "pealayer.volume.set" | "pealayer.player.volume.set" => {
            Some(InteropCommand::SetVolume {
                value: number(&["value", "level"])?,
            })
        }
        "mute" | "set_mute" | "pealayer.mute.set" | "pealayer.player.mute.set" => {
            let muted = request
                .params
                .get("muted")
                .or_else(|| request.params.get("enabled"))
                .and_then(Value::as_bool)
                .ok_or_else(|| "missing boolean parameter: muted".to_string())?;
            Some(InteropCommand::SetMute { muted })
        }
        "toggle_mute" | "pealayer.mute.toggle" | "pealayer.player.mute.toggle" => {
            Some(InteropCommand::ToggleMute)
        }
        "rate" | "set_rate" | "pealayer.rate.set" | "pealayer.player.rate.set" => {
            Some(InteropCommand::SetRate {
                rate: number(&["rate", "value"])?,
            })
        }
        "open" | "open_video" | "pealayer.open" | "pealayer.player.open" => {
            Some(InteropCommand::Open {
                target: string(&["target", "path"])?,
            })
        }
        "browse_remote" | "pealayer.remote.browse" => Some(InteropCommand::BrowseRemote { target: request.params.get("target").and_then(Value::as_str).unwrap_or_default().into(), use_proxy: request.params.get("use_proxy").and_then(Value::as_bool) }),
        "select_remote" | "pealayer.remote.select" => Some(InteropCommand::SelectRemote { target: string(&["target"])? , play: request.params.get("play").and_then(Value::as_bool).unwrap_or(false) }),
        "sort_remote" | "pealayer.remote.sort" => Some(InteropCommand::SortRemote { by: serde_json::from_value(request.params.get("by").cloned().unwrap_or(Value::String("name".into()))).map_err(|_| "sort must be name, date or size".to_string())?, descending: request.params.get("descending").and_then(Value::as_bool).unwrap_or(false) }),
        "close_remote_browser" | "pealayer.remote.close" => Some(InteropCommand::CloseRemoteBrowser),
        "fullscreen"
        | "set_fullscreen"
        | "pealayer.fullscreen.set"
        | "pealayer.player.fullscreen.set" => {
            let enabled = request
                .params
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| "missing boolean parameter: enabled".to_string())?;
            Some(InteropCommand::SetFullscreen { enabled })
        }
        "toggle_fullscreen" | "pealayer.fullscreen.toggle" => {
            Some(InteropCommand::ToggleFullscreen)
        }
        "activate" | "focus" | "pealayer.window.activate" => Some(InteropCommand::Activate),
        "minimize" | "pealayer.window.minimize" => Some(InteropCommand::Minimize),
        "maximize" | "pealayer.window.maximize" => Some(InteropCommand::Maximize),
        "restore" | "pealayer.window.restore" => Some(InteropCommand::Restore),
        "preferences" | "open_preferences" | "pealayer.window.preferences" => {
            Some(InteropCommand::OpenPreferences)
        }
        "media_information" | "open_media_information" | "pealayer.media.information" => {
            Some(InteropCommand::OpenMediaInformation)
        }
        "media_folder" | "open_media_folder" | "pealayer.media.folder" => {
            Some(InteropCommand::OpenMediaFolder)
        }
        "edit_configuration" | "edit_config" | "pealayer.config.edit" => {
            Some(InteropCommand::EditConfiguration)
        }
        "board_information" | "board.info.open" | "pealayer.board.info.open" => {
            let tab = request
                .params
                .get("tab")
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|value| usize::try_from(value).ok())
                        .or_else(|| {
                            value.as_str().and_then(|value| {
                                match value.trim().to_ascii_lowercase().as_str() {
                                    "overview" => Some(0),
                                    "capabilities" => Some(1),
                                    "front-panel" | "front_panel" | "front panel" => Some(2),
                                    "settings" | "board-settings" | "board_settings" => Some(3),
                                    _ => None,
                                }
                            })
                        })
                })
                .flatten()
                .unwrap_or(0);
            Some(InteropCommand::OpenBoardInformation { tab })
        }
        "publish_toast" | "toast.show" | "pealayer.toast.show" => Some(InteropCommand::PublishToast {
            toast: serde_json::from_value(request.params.clone()).map_err(|e| format!("invalid toast: {e}"))?,
        }),
        "dismiss_toast" | "toast.dismiss" | "pealayer.toast.dismiss" => Some(InteropCommand::DismissToast { id: string(&["id"])? }),
        "message" | "show_message" | "pealayer.message.show" => Some(InteropCommand::ShowMessage {
            message: request
                .params
                .get("message")
                .or_else(|| request.params.get("text"))
                .or_else(|| request.params.get("value"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "osd.show" | "show_osd" | "pealayer.osd.show" => {
            let options_value = request
                .params
                .get("options")
                .or_else(|| request.params.get("style"))
                .cloned()
                .unwrap_or_else(|| request.params.clone());
            let options = serde_json::from_value::<OsdOptions>(options_value)
                .map_err(|error| format!("invalid OSD options: {error}"))?;
            Some(InteropCommand::ShowOsd {
                message: request
                    .params
                    .get("message")
                    .or_else(|| request.params.get("text"))
                    .or_else(|| request.params.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                options,
            })
        }
        "osd.hide" | "hide_osd" | "clear_osd" | "pealayer.osd.hide" | "pealayer.message.hide" => {
            Some(InteropCommand::HideOsd)
        }
        "quit" | "exit" | "pealayer.quit" => Some(InteropCommand::Quit),
        "workspace" | "set_workspace" | "pealayer.workspace.set" | "pealayer.workspace.restore" => {
            Some(InteropCommand::SetWorkspace {
                profile: string(&["profile", "workspace", "id", "value"])?
                    .trim()
                    .to_string(),
            })
        }
        "workspace.create" | "pealayer.workspace.create" => {
            Some(InteropCommand::CreateWorkspaceProfile {
                name: string(&["name"])?,
                icon: string(&["icon"])?,
            })
        }
        "workspace.update" | "pealayer.workspace.update" => {
            Some(InteropCommand::UpdateWorkspaceProfile {
                id: string(&["id", "profile"])?,
                name: string(&["name"])?,
                icon: string(&["icon"])?,
                capture: request
                    .params
                    .get("capture")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        }
        "workspace.delete" | "pealayer.workspace.delete" => {
            Some(InteropCommand::DeleteWorkspaceProfile {
                id: string(&["id", "profile"])?,
            })
        }
        "workspace.move" | "pealayer.workspace.move" => {
            Some(InteropCommand::MoveWorkspaceProfile {
                id: string(&["id", "profile"])?,
                direction: request
                    .params
                    .get("direction")
                    .and_then(Value::as_i64)
                    .and_then(|value| i32::try_from(value).ok())
                    .ok_or_else(|| "missing workspace move direction".to_string())?,
            })
        }
        "effect_cue.add" | "pealayer.effect_cue.add" | "pealayer.timeline.effect.add" => {
            Some(InteropCommand::AddEffectCue {
                effect_id: string(&["effect_id", "effect"])?,
                start_time_ms: request
                    .params
                    .get("start_time_ms")
                    .or_else(|| request.params.get("time_ms"))
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
            })
        }
        "effect_cue.remove" | "pealayer.effect_cue.remove" | "pealayer.timeline.effect.remove" => {
            Some(InteropCommand::RemoveEffectCue {
                instance_id: string(&["instance_id", "cue_id"])?,
            })
        }
        "effect_cue.update" | "pealayer.effect_cue.update" | "pealayer.timeline.effect.update" => {
            Some(InteropCommand::UpdateEffectCue {
                instance_id: string(&["instance_id", "cue_id"])?,
                start_time_ms: request
                    .params
                    .get("start_time_ms")
                    .or_else(|| request.params.get("time_ms"))
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing cue start_time_ms".to_string())?,
                duration_ms: request
                    .params
                    .get("duration_ms")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing cue duration_ms".to_string())?,
            })
        }
        "controller_effect_cue.add" | "pealayer.controller_effect_cue.add" => {
            Some(InteropCommand::AddControllerEffectCue {
                reference: string(&["reference", "effect"])?,
                start_time_ms: request
                    .params
                    .get("start_time_ms")
                    .or_else(|| request.params.get("time_ms"))
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
            })
        }
        "controller_effect.play" | "pealayer.controller_effect.play" => {
            Some(InteropCommand::PlayControllerEffect {
                reference: string(&["reference", "effect"])?,
            })
        }
        "controller_effect.stop" | "pealayer.controller_effect.stop" => {
            Some(InteropCommand::StopControllerEffect)
        }
        "controller_effect.group.create" | "pealayer.controller_effect.group.create" => {
            Some(InteropCommand::CreateControllerEffectGroup {
                name: string(&["name"])?,
                icon: request
                    .params
                    .get("icon")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        }
        "controller_effect.delete" | "pealayer.controller_effect.delete" => {
            Some(InteropCommand::DeleteControllerEffect {
                reference: string(&["reference", "effect"])?,
            })
        }
        "controller_effect.save" | "pealayer.controller_effect.save" => {
            let effect = serde_json::from_value::<WebControllerEffectDraft>(request.params.clone())
                .map_err(|error| format!("invalid controller effect: {error}"))?;
            Some(InteropCommand::SaveControllerEffect { effect })
        }
        "controller_effect.record.start" | "pealayer.controller_effect.record.start" => {
            Some(InteropCommand::StartControllerEffectRecording {
                effect: request.params.get("effect").cloned().map(serde_json::from_value).transpose().map_err(|error|format!("invalid capture effect: {error}"))?,
                name: string(&["name"])?,
                category: request
                    .params
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("Recorded")
                    .to_string(),
                color: request
                    .params
                    .get("color")
                    .and_then(Value::as_str)
                    .unwrap_or("violet")
                    .to_string(),
                mode: request
                    .params
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or("automatic")
                    .to_string(),
            })
        }
        "controller_effect.record.status" | "pealayer.controller_effect.record.status" => {
            Some(InteropCommand::RefreshControllerEffectRecording)
        }
        "controller_effect.record.save" | "pealayer.controller_effect.record.save" => {
            Some(InteropCommand::SaveControllerEffectRecording)
        }
        "controller_effect.record.discard" | "pealayer.controller_effect.record.discard" => {
            Some(InteropCommand::DiscardControllerEffectRecording)
        }
        "recording" | "recording.set" | "pealayer.recording.set" => {
            let enabled = request
                .params
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| "missing boolean parameter: enabled".to_string())?;
            Some(InteropCommand::SetRecording { enabled })
        }
        "estop" | "emergency_stop" | "pealayer.estop.set" | "pealayer.emergency_stop.set" => {
            let active = request
                .params
                .get("active")
                .or_else(|| request.params.get("enabled"))
                .and_then(Value::as_bool)
                .ok_or_else(|| "missing boolean parameter: active".to_string())?;
            Some(InteropCommand::SetEmergencyStop { active })
        }
        "hardware.action.invoke" | "pealayer.hardware.action.invoke" => {
            Some(InteropCommand::InvokeHardwareAction {
                action_id: string(&["action_id", "action"])?,
            })
        }
        "hardware.pwm.set" | "pealayer.hardware.pwm.set" => {
            let channel = request
                .params
                .get("channel")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| "missing valid PWM channel".to_string())?;
            Some(InteropCommand::SetHardwarePwm {
                channel,
                percent: number(&["percent", "value"])?,
            })
        }
        "hardware.presentation.update" | "pealayer.hardware.presentation.update" => {
            Some(InteropCommand::UpdateHardwarePresentation {
                key: string(&["key", "channel"])?,
                fields: request
                    .params
                    .get("fields")
                    .cloned()
                    .ok_or_else(|| "missing hardware presentation fields".to_string())?,
            })
        }
        "hardware.strip.configure" | "pealayer.hardware.strip.configure" => {
            let pixels = request
                .params
                .get("pixels")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .ok_or_else(|| "missing valid addressable strip pixel count".to_string())?;
            Some(InteropCommand::ConfigureAddressableStrip { pixels })
        }
        "hardware.strip.fill" | "pealayer.hardware.strip.fill" => {
            let byte = |name: &str| {
                request
                    .params
                    .get(name)
                    .and_then(Value::as_u64)
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or_else(|| format!("missing valid strip {name}"))
            };
            Some(InteropCommand::FillAddressableStrip {
                red: byte("red")?,
                green: byte("green")?,
                blue: byte("blue")?,
                brightness: byte("brightness")?,
            })
        }
        "hardware.strip.clear" | "pealayer.hardware.strip.clear" => {
            Some(InteropCommand::ClearAddressableStrip)
        }
        "hardware.front_panel.press" | "pealayer.hardware.front_panel.press" => {
            Some(InteropCommand::PressFrontPanelKey {
                key: string(&["key"])?,
            })
        }
        "config.update" | "pealayer.config.update" => {
            crate::config::AppConfig::validate_patch_shape(&request.params)?;
            Some(InteropCommand::UpdateConfig {
                values: request.params.clone(),
            })
        }
        "config.reload" | "pealayer.config.reload" => Some(InteropCommand::ReloadConfig),
        "get_status" | "player.status" | "pealayer.status" | "pealayer.player.status" | "pealayer.messages.state" => None,
        method => return Err(format!("unknown Pealayer JSON-RPC method: {method}")),
    };
    if let Some(command) = &command {
        command.validate()?;
    }
    Ok(command)
}

pub fn json_rpc_result(id: &Value, result: Value) -> String {
    serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}).to_string()
}

pub fn json_rpc_error(id: &Value, code: i32, message: &str) -> String {
    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}).to_string()
}

static LIVE_STATUS: std::sync::RwLock<Option<PlayerStatusResponse>> = std::sync::RwLock::new(None);
static LIVE_CONFIG: std::sync::RwLock<Option<crate::config::AppConfig>> =
    std::sync::RwLock::new(None);

pub fn set_live_status(status: PlayerStatusResponse) {
    if let Ok(mut lock) = LIVE_STATUS.write() {
        *lock = Some(status);
    }
}
pub fn get_live_message_snapshot() -> crate::messaging::MessageSnapshot {
    LIVE_STATUS.read().ok().and_then(|status|status.as_ref().map(|s|s.messages.clone())).unwrap_or_default()
}
pub fn get_live_remote_revision() -> u64 {
    LIVE_STATUS.read().ok().and_then(|status|status.as_ref().map(|s|s.remote_browser.revision)).unwrap_or_default()
}

/// Avoid cloning the timeline and hardware snapshot on every video frame just
/// to decide whether appearance needs an immediate broadcast.
pub fn get_live_appearance() -> Option<AppearanceState> {
    LIVE_STATUS
        .read()
        .ok()
        .and_then(|status| status.as_ref().and_then(|status| status.appearance.clone()))
}

pub fn get_live_status() -> PlayerStatusResponse {
    if let Some(client)=crate::peer::client() && let Some(snapshot)=client.snapshot() && let Ok(status)=serde_json::from_value(snapshot.session.status){return status;}
    if let Ok(lock) = LIVE_STATUS.read() {
        if let Some(ref st) = *lock {
            return st.clone();
        }
    }
    PlayerStatusResponse {
        status: "initializing".to_string(),
        ..PlayerStatusResponse::default()
    }
}

pub fn set_live_config(config: crate::config::AppConfig) {
    if let Ok(mut lock) = LIVE_CONFIG.write() {
        *lock = Some(config);
    }
}

pub fn get_live_config() -> crate::config::AppConfig {
    LIVE_CONFIG
        .read()
        .ok()
        .and_then(|config| config.clone())
        .unwrap_or_else(crate::config::AppConfig::load)
}

pub fn get_socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("PEALAYER_SOCKET_PATH") {
        return PathBuf::from(path);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("pealayer.sock")
    } else {
        PathBuf::from("/tmp").join("pealayer.sock")
    }
}

fn local_endpoint_hash(identity: &str) -> u64 {
    identity
        .trim()
        .to_lowercase()
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

#[cfg(windows)]
pub fn windows_pipe_name(application_identity: &str) -> String {
    let session = crate::platform::windows::current_session_id().unwrap_or_default();
    format!(
        r"\\.\pipe\Pealayer.Control.{:016x}.{session}",
        local_endpoint_hash(application_identity)
    )
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn send_windows_native_request(
    payload: &str,
    application_identity: &str,
    timeout: std::time::Duration,
) -> Result<String, String> {
    use windows::Win32::Foundation::GetLastError;
    use windows::Win32::System::Pipes::CallNamedPipeW;
    use windows::core::PCWSTR;

    let pipe_name = windows_pipe_name(application_identity);
    let pipe_name_wide = wide_null(&pipe_name);
    let request = format!("{}\n", payload.trim());
    let mut response = vec![0u8; 65_536];
    let mut bytes_read = 0u32;
    let timeout_ms = timeout.as_millis().clamp(1, u128::from(u32::MAX)) as u32;
    let ok = unsafe {
        CallNamedPipeW(
            PCWSTR(pipe_name_wide.as_ptr()),
            Some(request.as_ptr().cast()),
            request.len() as u32,
            Some(response.as_mut_ptr().cast()),
            response.len() as u32,
            &mut bytes_read,
            timeout_ms,
        )
    };
    if !ok.as_bool() {
        return Err(format!(
            "native Pealayer pipe {pipe_name} is unavailable: {:?}",
            unsafe { GetLastError() }
        ));
    }
    response.truncate(bytes_read as usize);
    String::from_utf8(response)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("native Pealayer pipe returned invalid UTF-8: {error}"))
}

#[cfg(windows)]
pub fn send_native_request(
    payload: &str,
    application_identity: &str,
    timeout: std::time::Duration,
) -> Result<String, String> {
    send_windows_native_request(payload, application_identity, timeout)
}

#[cfg(unix)]
pub fn send_native_request(
    payload: &str,
    _application_identity: &str,
    timeout: std::time::Duration,
) -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(get_socket_path())
        .map_err(|error| format!("native Pealayer socket is unavailable: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(format!("{}\n", payload.trim()).as_bytes())
        .and_then(|_| stream.flush())
        .map_err(|error| error.to_string())?;
    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .map_err(|error| error.to_string())?;
    Ok(response.trim().to_string())
}

#[cfg(not(any(unix, windows)))]
pub fn send_native_request(
    _payload: &str,
    _application_identity: &str,
    _timeout: std::time::Duration,
) -> Result<String, String> {
    Err("native Pealayer IPC is not supported on this operating system".to_string())
}

#[cfg(windows)]
fn spawn_windows_pipe_listener(
    tx: std::sync::mpsc::Sender<InteropCommand>,
    egui_ctx: eframe::egui::Context,
    application_identity: String,
) {
    use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, GetLastError};
    use windows::Win32::Storage::FileSystem::{
        FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows::core::PCWSTR;

    let pipe_name = windows_pipe_name(&application_identity);
    let pipe_name_wide = wide_null(&pipe_name);
    let expected_session_id = crate::platform::windows::current_session_id().ok();
    let receipts = Arc::new(Mutex::new(LaunchReceiptCache::default()));
    thread::spawn(move || {
        loop {
            let handle = unsafe {
                CreateNamedPipeW(
                    PCWSTR(pipe_name_wide.as_ptr()),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_MESSAGE
                        | PIPE_READMODE_MESSAGE
                        | PIPE_WAIT
                        | PIPE_REJECT_REMOTE_CLIENTS,
                    PIPE_UNLIMITED_INSTANCES,
                    65_536,
                    65_536,
                    1_000,
                    None,
                )
            };
            if handle.is_invalid() {
                log::error!("Could not create native Pealayer pipe {pipe_name}");
                thread::sleep(std::time::Duration::from_millis(250));
                continue;
            }
            let connected = unsafe { ConnectNamedPipe(handle, None) }.is_ok()
                || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
            if connected {
                let mut request = vec![0u8; 65_536];
                let mut bytes_read = 0u32;
                if unsafe { ReadFile(handle, Some(&mut request), Some(&mut bytes_read), None) }
                    .is_ok()
                {
                    request.truncate(bytes_read as usize);
                    let payload = String::from_utf8_lossy(&request);
                    let response = dispatch_local_payload(
                        payload.trim(),
                        &tx,
                        &egui_ctx,
                        &receipts,
                        &application_identity,
                        expected_session_id,
                    );
                    let mut bytes_written = 0u32;
                    let _ = unsafe {
                        WriteFile(
                            handle,
                            Some(response.as_bytes()),
                            Some(&mut bytes_written),
                            None,
                        )
                    };
                    let _ = unsafe { FlushFileBuffers(handle) };
                }
                let _ = unsafe { DisconnectNamedPipe(handle) };
            }
            let _ = unsafe { CloseHandle(handle) };
        }
    });
}

pub fn parse_interop_request(
    line: &str,
) -> Result<(Option<serde_json::Value>, InteropCommand), String> {
    let val: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;

    // Check if JSON-RPC 2.0 format
    if val.get("jsonrpc").and_then(|v| v.as_str()) == Some("2.0") || val.get("method").is_some() {
        let request: JsonRpcRequest = serde_json::from_value(val).map_err(|e| e.to_string())?;
        let id = Some(request.id.clone());
        let command = command_from_json_rpc(&request)?.unwrap_or(InteropCommand::GetStatus);
        return Ok((id, command));
    }

    // Fall back to standard InteropCommand deserialization
    let cmd: InteropCommand = serde_json::from_value(val).map_err(|e| e.to_string())?;
    cmd.validate()?;
    Ok((None, cmd))
}

pub fn format_interop_response(
    id: Option<serde_json::Value>,
    result: &serde_json::Value,
) -> String {
    if let Some(id_val) = id {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": result
        })
        .to_string()
            + "\n"
    } else {
        result.to_string() + "\n"
    }
}

pub fn format_interop_error(id: Option<serde_json::Value>, code: i32, message: &str) -> String {
    if let Some(id_val) = id {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "error": {
                "code": code,
                "message": message
            }
        })
        .to_string()
            + "\n"
    } else {
        serde_json::json!({
            "status": "error",
            "message": message
        })
        .to_string()
            + "\n"
    }
}

#[cfg(any(unix, windows, test))]
fn dispatch_local_payload(
    payload: &str,
    tx: &std::sync::mpsc::Sender<InteropCommand>,
    egui_ctx: &eframe::egui::Context,
    launch_receipts: &Arc<Mutex<LaunchReceiptCache>>,
    expected_identity: &str,
    expected_session_id: Option<u32>,
) -> String {
    let (id, command) = match parse_interop_request(payload) {
        Ok(parsed) => parsed,
        Err(error) => return format_interop_error(None, -32600, &error),
    };
    if matches!(command, InteropCommand::GetStatus) {
        let value = serde_json::to_value(get_live_status())
            .unwrap_or_else(|_| serde_json::json!({"status":"initializing"}));
        return format_interop_response(id, &value);
    }
    let operation_id = match &command {
        InteropCommand::Launch { request } => {
            if let Err(error) =
                validate_launch_destination(request, expected_identity, expected_session_id)
            {
                return format_interop_error(id, -32600, &error);
            }
            Some(request.operation_id.clone())
        }
        _ => None,
    };
    if let Some(operation_id) = operation_id {
        let mut receipts = match launch_receipts.lock() {
            Ok(receipts) => receipts,
            Err(_) => {
                return format_interop_error(id, -32000, "launch receipt cache is unavailable");
            }
        };
        if !receipts.claim(&operation_id) {
            return format_interop_response(
                id,
                &serde_json::json!({"status":"accepted","duplicate":true}),
            );
        }
        if tx.send(command).is_err() {
            receipts.release(&operation_id);
            return format_interop_error(id, -32000, "application dispatcher is unavailable");
        }
    } else if tx.send(command).is_err() {
        return format_interop_error(id, -32000, "application dispatcher is unavailable");
    }
    egui_ctx.request_repaint();
    format_interop_response(id, &serde_json::json!({"status":"accepted"}))
}

#[cfg(unix)]
fn handle_client_connection<R: std::io::Read, W: Write>(
    mut reader: BufReader<R>,
    mut writer: W,
    tx: std::sync::mpsc::Sender<InteropCommand>,
    egui_ctx: eframe::egui::Context,
    launch_receipts: Arc<Mutex<LaunchReceiptCache>>,
    expected_identity: Arc<str>,
    expected_session_id: Option<u32>,
) {
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let response = dispatch_local_payload(
                trimmed,
                &tx,
                &egui_ctx,
                &launch_receipts,
                &expected_identity,
                expected_session_id,
            );
            let _ = writer.write_all(response.as_bytes());
            let _ = writer.flush();
        }
        line.clear();
    }
}

pub fn spawn_interop_listener(
    tx: std::sync::mpsc::Sender<InteropCommand>,
    egui_ctx: eframe::egui::Context,
    application_identity: String,
) {
    // TCP automation now shares the unified HTTP/WebSocket listener at
    // `/api/ipc`. Keep the native Unix socket because it consumes no TCP port.
    #[cfg(unix)]
    {
        let launch_receipts = Arc::new(Mutex::new(LaunchReceiptCache::default()));
        let application_identity: Arc<str> = Arc::from(application_identity);
        let tx_unix = tx.clone();
        let ctx_unix = egui_ctx.clone();
        let unix_launch_receipts = launch_receipts.clone();
        let unix_identity = application_identity.clone();
        thread::spawn(move || {
            let socket_path = get_socket_path();
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
            }

            if let Ok(listener) = UnixListener::bind(&socket_path) {
                for stream in listener.incoming() {
                    if let Ok(stream) = stream {
                        let tx_conn = tx_unix.clone();
                        let ctx_conn = ctx_unix.clone();
                        let receipts_conn = unix_launch_receipts.clone();
                        let identity_conn = unix_identity.clone();
                        thread::spawn(move || {
                            if let Ok(read_clone) = stream.try_clone() {
                                let reader = BufReader::new(read_clone);
                                handle_client_connection(
                                    reader,
                                    stream,
                                    tx_conn,
                                    ctx_conn,
                                    receipts_conn,
                                    identity_conn,
                                    None,
                                );
                            }
                        });
                    }
                }
            }
        });
    }
    #[cfg(windows)]
    spawn_windows_pipe_listener(tx, egui_ctx, application_identity);
    #[cfg(not(any(unix, windows)))]
    let _ = (tx, egui_ctx, application_identity);
}

pub fn spawn_interop_server(egui_ctx: eframe::egui::Context) -> Receiver<InteropCommand> {
    let (tx, rx) = channel::<InteropCommand>();
    let application_identity = crate::cli::resolved_instance_identity();
    spawn_interop_listener(tx.clone(), egui_ctx.clone(), application_identity.clone());
    let state_tx = crate::server::spawn_control_server_configured(
        crate::config::control_port(),
        egui_ctx,
        crate::server::WebRuntimeConfig::production(
            application_identity.clone(),
            "en".to_string(),
            "ltr".to_string(),
            "system".to_string(),
            [0, 120, 212],
        ),
        tx,
        application_identity,
    );
    // This compatibility helper has no owner for the state sender; keeping it
    // alive preserves the same process-lifetime server semantics as before.
    std::mem::forget(state_tx);
    rx
}

const PCCONTROLLER_ACTIONS: &str = "pealayer.play,pealayer.pause,pealayer.toggle,pealayer.stop,pealayer.next,pealayer.previous,pealayer.chapter.next,pealayer.chapter.previous,pealayer.chapter.set,pealayer.seek,pealayer.seek_to,pealayer.seek_absolute,pealayer.volume.set,pealayer.mute.set,pealayer.mute.toggle,pealayer.rate.set,pealayer.open,pealayer.fullscreen.set,pealayer.fullscreen.toggle,pealayer.workspace.set,pealayer.window.activate,pealayer.window.minimize,pealayer.window.maximize,pealayer.window.restore,pealayer.quit,pealayer.command";

struct ControllerAction {
    command: Option<InteropCommand>,
    acknowledgement: Value,
    receipt_key: String,
}

/// A targeted PCController action. The UI acknowledges it only after applying
/// the command, so PCController never records queueing as successful execution.
pub struct ControllerDelivery {
    pub command: InteropCommand,
    acknowledgement: Value,
    acknowledgement_tx: std::sync::mpsc::Sender<Value>,
}

impl ControllerDelivery {
    pub fn acknowledge_applied(self) {
        let _ = self.acknowledgement_tx.send(self.acknowledgement);
    }
}

fn controller_action_from_event(event: &Value, instance_id: &str) -> Option<ControllerAction> {
    let metadata = event.get("metadata")?.as_object()?;
    if metadata.get("target_instance")?.as_str()? != instance_id {
        return None;
    }
    let operation_id = metadata.get("operation_id")?.as_str()?.trim();
    let delivery_id = metadata.get("operation_delivery_id")?.as_str()?.trim();
    let expires_at = metadata.get("operation_expires_at")?.as_str()?.trim();
    if operation_id.is_empty() || delivery_id.is_empty() || expires_at.is_empty() {
        return None;
    }

    let kind = event.get("kind")?.as_str()?.trim().to_ascii_lowercase();
    let value = metadata
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let expired = expires_at
        .parse::<jiff::Timestamp>()
        .map(|deadline| deadline <= jiff::Timestamp::now())
        .unwrap_or(true);
    let command = if expired {
        None
    } else {
        match kind.as_str() {
            "pealayer.command" => serde_json::from_str::<InteropCommand>(value).ok().filter(|command| command.validate().is_ok()),
            "pealayer.play" => Some(InteropCommand::Play),
            "pealayer.pause" => Some(InteropCommand::Pause),
            "pealayer.toggle" => Some(InteropCommand::TogglePause),
            "pealayer.stop" => Some(InteropCommand::Stop),
            "pealayer.next" => Some(InteropCommand::Next),
            "pealayer.previous" => Some(InteropCommand::Previous),
            "pealayer.chapter.next" => Some(InteropCommand::NextChapter),
            "pealayer.chapter.previous" => Some(InteropCommand::PreviousChapter),
            "pealayer.chapter.set" => value
                .parse::<i64>()
                .ok()
                .filter(|index| *index >= 0)
                .map(|index| InteropCommand::SetChapter { index }),
            "pealayer.seek" => value
                .parse::<f64>()
                .ok()
                .filter(|seconds| seconds.is_finite())
                .map(|seconds| InteropCommand::Seek { seconds }),
            "pealayer.seek_absolute" => value
                .parse::<f64>()
                .ok()
                .filter(|percentage| percentage.is_finite() && (0.0..=100.0).contains(percentage))
                .map(|percentage| InteropCommand::SeekAbs { percentage }),
            "pealayer.seek_to" => value
                .parse::<f64>()
                .ok()
                .filter(|seconds| seconds.is_finite())
                .map(|seconds| InteropCommand::SeekTo { seconds }),
            "pealayer.volume.set" => value
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite() && (0.0..=130.0).contains(value))
                .map(|value| InteropCommand::SetVolume { value }),
            "pealayer.mute.set" => match value.to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => Some(InteropCommand::SetMute { muted: true }),
                "false" | "0" | "off" | "no" => Some(InteropCommand::SetMute { muted: false }),
                _ => None,
            },
            "pealayer.mute.toggle" if value.is_empty() => Some(InteropCommand::ToggleMute),
            "pealayer.rate.set" => value
                .parse::<f64>()
                .ok()
                .filter(|rate| rate.is_finite() && (0.05..=16.0).contains(rate))
                .map(|rate| InteropCommand::SetRate { rate }),
            "pealayer.open" if !value.is_empty() => Some(InteropCommand::Open {
                target: value.to_string(),
            }),
            "pealayer.fullscreen.set" => match value.to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => {
                    Some(InteropCommand::SetFullscreen { enabled: true })
                }
                "false" | "0" | "off" | "no" => {
                    Some(InteropCommand::SetFullscreen { enabled: false })
                }
                _ => None,
            },
            "pealayer.fullscreen.toggle" if value.is_empty() => {
                Some(InteropCommand::ToggleFullscreen)
            }
            "pealayer.workspace.set" if valid_workspace_profile_id(value) => {
                Some(InteropCommand::SetWorkspace {
                    profile: value.to_string(),
                })
            }
            "pealayer.window.activate" if value.is_empty() => Some(InteropCommand::Activate),
            "pealayer.window.minimize" if value.is_empty() => Some(InteropCommand::Minimize),
            "pealayer.window.maximize" if value.is_empty() => Some(InteropCommand::Maximize),
            "pealayer.window.restore" if value.is_empty() => Some(InteropCommand::Restore),
            "pealayer.quit" if value.is_empty() => Some(InteropCommand::Quit),
            _ => None,
        }
    };
    let reason = if expired {
        Some("expired")
    } else if command.is_none() {
        Some("unsupported_or_invalid_pealayer_action")
    } else {
        None
    };
    let mut acknowledgement = serde_json::json!({
        "operation_id": operation_id,
        "delivery_id": delivery_id,
        "instance_id": instance_id,
        "state": if command.is_some() { "applied" } else { "rejected" },
    });
    if let Some(reason) = reason {
        acknowledgement["reason"] = Value::String(reason.to_string());
    }
    Some(ControllerAction {
        command,
        acknowledgement,
        receipt_key: format!("{operation_id}\0{delivery_id}"),
    })
}

fn controller_rpc(id: u64, method: &str, params: Value) -> tungstenite::Message {
    tungstenite::Message::Text(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        })
        .to_string()
        .into(),
    )
}

pub(crate) fn controller_instance_id() -> String {
    format!("pealayer:desktop-{}", std::process::id())
}

pub(crate) fn controller_instance_identity(instance_id: &str, name: &str) -> Value {
    serde_json::json!({
        "id": instance_id, "surface":"pealayer", "page":"player", "state":"active", "lease_seconds":45,
        "self":{"kind":"native","pid":std::process::id(),"vars":{
            "rpc":format!("http://127.0.0.1:{}/api/rpc",crate::config::control_port()),
            "websocket":format!("ws://127.0.0.1:{}/ws",crate::config::control_port()),
            "ipc":format!("http://127.0.0.1:{}/api/ipc",crate::config::control_port()),
            "web_ui":format!("http://127.0.0.1:{}/",crate::config::control_port())}},
        "values":{"application":name,"version":env!("CARGO_PKG_VERSION"),"commit":env!("PEALAYER_GIT_COMMIT"),
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,
            "app_actions":PCCONTROLLER_ACTIONS,"control_contract":"pealayer.control",
            "control_transports":"native,http,websocket,json-rpc","coordinator":"pccontroller","serial_owner":"pccontroller"}
    })
}

fn report_controller_instance(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    id: u64,
    instance_id: &str,
) -> Result<(), String> {
    socket
        .send(controller_rpc(
            id,
            "controller.app.instance.report",
            controller_instance_identity(instance_id, &crate::config::resolved_app_name(&get_live_config())),
        ))
        .map_err(|error| format!("report Pealayer instance to PCController: {error}"))
}

fn send_action_ack(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    next_id: &mut u64,
    acknowledgement: Value,
) -> Result<(), String> {
    socket
        .send(controller_rpc(
            *next_id,
            "controller.app.action.ack",
            acknowledgement,
        ))
        .map_err(|error| format!("acknowledge PCController action: {error}"))?;
    *next_id = next_id.wrapping_add(1).max(1);
    Ok(())
}

fn gate_controller_subscription_message(
    message: Value,
    subscription_ready: &mut bool,
    pending: &mut std::collections::VecDeque<Value>,
    push_target: &crate::four_d::engine::ControllerPushTarget,
) -> Result<Vec<Value>, String> {
    if *subscription_ready {
        return Ok(vec![message]);
    }
    if message.get("id").and_then(Value::as_u64) == Some(1) {
        if let Some(error) = message.get("error").filter(|value| !value.is_null()) {
            return Err(format!("PCController subscription failed: {error}"));
        }
        let result = message
            .get("result")
            .filter(|value| value.is_object())
            .ok_or_else(|| "PCController subscription returned no result".to_string())?;
        if result.get("subscribed").and_then(Value::as_bool) != Some(true) {
            return Err("PCController did not confirm the subscription".to_string());
        }
        let instance_id = result
            .get("instance_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "PCController subscription omitted its host identity".to_string())?;
        if !push_target.matches_source_instance(instance_id) {
            return Err("PCController push identity differs from the selected RPC coordinator; waiting for its catalog".into());
        }
        push_target.observe_source_instance(instance_id);
        *subscription_ready = true;
        return Ok(pending.drain(..).collect());
    }
    if message.get("method").and_then(Value::as_str).is_some() {
        if pending.len() >= 256 {
            pending.pop_front();
        }
        pending.push_back(message);
    }
    Ok(Vec::new())
}

fn run_pccontroller_action_bridge(
    tx: &std::sync::mpsc::Sender<ControllerDelivery>,
    egui_ctx: &eframe::egui::Context,
    push_target: &crate::four_d::engine::ControllerPushTarget,
    endpoint: &str,
    connected_once: &mut bool,
) -> Result<(), String> {
    use std::collections::{HashSet, VecDeque};
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};
    use tungstenite::Message;

    let (mut socket, _) = tungstenite::connect(endpoint)
        .map_err(|error| format!("connect to PCController action WebSocket: {error}"))?;
    if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .map_err(|error| format!("configure PCController action read timeout: {error}"))?;
    }

    let instance_id = controller_instance_id();
    let mut next_id = 1_u64;
    socket
        .send(controller_rpc(
            next_id,
            "controller.subscribe",
            serde_json::json!({
                "topics":["state","events","status","opcodes"],
                "interval_ms":100,
                "after_id":0
            }),
        ))
        .map_err(|error| format!("subscribe to PCController actions: {error}"))?;
    next_id += 1;
    report_controller_instance(&mut socket, next_id, &instance_id)?;
    next_id += 1;

    let (acknowledgement_tx, acknowledgement_rx) = std::sync::mpsc::channel();
    let mut last_report = Instant::now();
    let mut receipts = HashSet::new();
    let mut receipt_order = VecDeque::new();
    let mut subscription_ready = false;
    let mut pending_pre_ack = VecDeque::new();
    let subscription_started = Instant::now();
    let mut subscribed_host = String::new();
    let mut last_ping = Instant::now();
    let mut pending_ping: Option<(u64, Instant)> = None;
    loop {
        if !push_target.is_alive() || push_target.websocket_endpoint().as_deref() != Some(endpoint)
        {
            let _ = socket.close(None);
            return Ok(());
        }
        if !subscription_ready && subscription_started.elapsed() > Duration::from_secs(5) {
            return Err("PCController subscription acknowledgement timed out".into());
        }
        if subscription_ready && !push_target.matches_source_instance(&subscribed_host) {
            return Err("PCController RPC coordinator changed; reconnecting push subscription".into());
        }
        if pending_ping.is_some_and(|(_,sent)|sent.elapsed() > Duration::from_secs(5)) {
            return Err("PCController action transport heartbeat timed out".into());
        }
        if subscription_ready && pending_ping.is_none() && last_ping.elapsed() >= Duration::from_secs(2) {
            socket.send(controller_rpc(next_id,"controller.ping",serde_json::json!({})))
                .map_err(|error|format!("ping PCController action transport: {error}"))?;
            pending_ping=Some((next_id,Instant::now()));
            next_id=next_id.wrapping_add(1).max(1);
            last_ping=Instant::now();
        }
        while let Ok(acknowledgement) = acknowledgement_rx.try_recv() {
            send_action_ack(&mut socket, &mut next_id, acknowledgement)?;
        }
        if last_report.elapsed() >= Duration::from_secs(30) {
            report_controller_instance(&mut socket, next_id, &instance_id)?;
            next_id = next_id.wrapping_add(1).max(1);
            last_report = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                let Ok(message) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                if pending_ping.is_some_and(|(id,_)|message.get("id").and_then(Value::as_u64)==Some(id)) {
                    if let Some(error)=message.get("error").filter(|value|!value.is_null()) { return Err(format!("PCController heartbeat rejected: {error}")); }
                    pending_ping=None;
                }
                if !subscription_ready && message.get("id").and_then(Value::as_u64)==Some(1) {
                    subscribed_host=message["result"]["instance_id"].as_str().unwrap_or_default().to_owned();
                }
                let was_subscription_ready = subscription_ready;
                let ready_messages = gate_controller_subscription_message(
                    message,
                    &mut subscription_ready,
                    &mut pending_pre_ack,
                    push_target,
                )?;
                if subscription_ready && !was_subscription_ready {
                    *connected_once = true;
                    egui_ctx.request_repaint();
                }
                for message in ready_messages {
                    let Some(method) = message.get("method").and_then(Value::as_str) else {
                        continue;
                    };
                    let params = message.get("params").unwrap_or(&Value::Null);
                    if push_target.apply_notification(method, params) {
                        egui_ctx.request_repaint();
                    }
                    if !matches!(method, "controller.state" | "controller.event") {
                        continue;
                    }
                    let Some(action) = message
                        .get("params")
                        .and_then(|event| controller_action_from_event(event, &instance_id))
                    else {
                        continue;
                    };
                    let first_delivery = receipts.insert(action.receipt_key.clone());
                    if first_delivery {
                        receipt_order.push_back(action.receipt_key.clone());
                        if receipt_order.len() > 256 {
                            if let Some(oldest) = receipt_order.pop_front() {
                                receipts.remove(&oldest);
                            }
                        }
                        if let Some(command) = action.command {
                            tx.send(ControllerDelivery {
                                command,
                                acknowledgement: action.acknowledgement,
                                acknowledgement_tx: acknowledgement_tx.clone(),
                            })
                            .map_err(|error| {
                                format!("deliver PCController action to player: {error}")
                            })?;
                            egui_ctx.request_repaint();
                        } else {
                            send_action_ack(&mut socket, &mut next_id, action.acknowledgement)?;
                        }
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                socket
                    .send(Message::Pong(payload))
                    .map_err(|error| format!("answer PCController ping: {error}"))?;
            }
            Ok(Message::Close(_)) => return Err("PCController action WebSocket closed".to_string()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(error) => return Err(format!("read PCController action WebSocket: {error}")),
        }
    }
}

pub fn spawn_pccontroller_action_bridge(
    egui_ctx: eframe::egui::Context,
    push_target: crate::four_d::engine::ControllerPushTarget,
) -> Receiver<ControllerDelivery> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let mut retry_delay = std::time::Duration::from_millis(250);
        while push_target.is_alive() {
            let Some(endpoint) = push_target.websocket_endpoint() else {
                thread::sleep(std::time::Duration::from_millis(100));
                retry_delay = std::time::Duration::from_millis(250);
                continue;
            };
            let mut connected_once = false;
            if let Err(error) = run_pccontroller_action_bridge(
                &tx,
                &egui_ctx,
                &push_target,
                &endpoint,
                &mut connected_once,
            ) {
                log::warn!(
                    "[PCController] {error}; retrying push transport in {} ms",
                    retry_delay.as_millis()
                );
            }
            if connected_once {
                retry_delay = std::time::Duration::from_millis(250);
            }
            let slices = (retry_delay.as_millis() / 50).max(1) as usize;
            for _ in 0..slices {
                if !push_target.is_alive()
                    || push_target.websocket_endpoint().as_deref() != Some(endpoint.as_str())
                {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(50));
            }
            if !connected_once {
                retry_delay = (retry_delay * 2).min(std::time::Duration::from_secs(5));
            }
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_folder_json_rpc_ipc_parity() {
        let target = "https://files.invalid/folder/";
        let expected = InteropCommand::BrowseRemote { target: target.into(), use_proxy: Some(false) };
        let json = serde_json::to_string(&expected).unwrap();
        assert_eq!(parse_interop_request(&json).unwrap().1, expected);
        let rpc = JsonRpcRequest { jsonrpc: Some("2.0".into()), id: Value::Null, method: "pealayer.remote.browse".into(), params: serde_json::json!({"target":target,"use_proxy":false}) };
        assert_eq!(command_from_json_rpc(&rpc).unwrap(), Some(expected));
        assert!(parse_text_command("browse_remote javascript:bad").is_err());
        assert!(matches!(parse_text_command("browse_remote").unwrap(), InteropCommand::BrowseRemote { .. }));
    }

    #[test]
    fn test_parse_interop_commands() {
        let play_json = r#"{"command":"play"}"#;
        let cmd: InteropCommand = serde_json::from_str(play_json).unwrap();
        assert!(matches!(cmd, InteropCommand::Play));

        let seek_json = r#"{"command":"seek","seconds":10.5}"#;
        let cmd: InteropCommand = serde_json::from_str(seek_json).unwrap();
        if let InteropCommand::Seek { seconds } = cmd {
            assert_eq!(seconds, 10.5);
        } else {
            panic!("Expected Seek command");
        }

        let open_json = r#"{"command":"open","target":"/video.mp4"}"#;
        let cmd: InteropCommand = serde_json::from_str(open_json).unwrap();
        if let InteropCommand::Open { target } = cmd {
            assert_eq!(target, "/video.mp4");
        } else {
            panic!("Expected Open command");
        }

        let open_alias_json = r#"{"command":"open_video","path":"/video2.mp4"}"#;
        let cmd: InteropCommand = serde_json::from_str(open_alias_json).unwrap();
        if let InteropCommand::Open { target } = cmd {
            assert_eq!(target, "/video2.mp4");
        } else {
            panic!("Expected Open command with aliases");
        }

        let live_json = r#"{"command":"open","target":"rtsp://camera.invalid/live"}"#;
        let cmd: InteropCommand = serde_json::from_str(live_json).unwrap();
        assert!(matches!(
            cmd,
            InteropCommand::Open { target } if target == "rtsp://camera.invalid/live"
        ));

        let vol_alias_json = r#"{"command":"volume","level":45.0}"#;
        let cmd: InteropCommand = serde_json::from_str(vol_alias_json).unwrap();
        if let InteropCommand::SetVolume { value } = cmd {
            assert_eq!(value, 45.0);
        } else {
            panic!("Expected SetVolume command with aliases");
        }

        assert_eq!(
            parse_text_command("preferences").unwrap(),
            InteropCommand::OpenPreferences
        );
        assert_eq!(
            parse_text_command("message Render complete").unwrap(),
            InteropCommand::ShowMessage {
                message: "Render complete".to_string()
            }
        );
        assert_eq!(
            parse_text_command("message").unwrap(),
            InteropCommand::ShowMessage {
                message: String::new()
            }
        );
        assert_eq!(
            parse_text_command("hide-osd").unwrap(),
            InteropCommand::HideOsd
        );
        assert_eq!(
            parse_text_command("estop on").unwrap(),
            InteropCommand::SetEmergencyStop { active: true }
        );
        assert_eq!(
            parse_text_command("emergency-stop off").unwrap(),
            InteropCommand::SetEmergencyStop { active: false }
        );
    }

    #[test]
    fn json_rpc_exposes_the_same_emergency_stop_control() {
        let request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(1),
            method: "pealayer.estop.set".to_string(),
            params: serde_json::json!({"active": true}),
        };
        assert_eq!(
            command_from_json_rpc(&request).unwrap(),
            Some(InteropCommand::SetEmergencyStop { active: true })
        );
    }

    #[test]
    fn launch_request_is_bounded_and_additive() {
        let json = r#"{
            "command":"launch",
            "request":{
                "operation_id":"launch-test-1",
                "application_identity":"Pealayer",
                "sender_session_id":null,
                "sender_working_directory":"/sender/work",
                "target":"media/clip.mkv",
                "fullscreen":true,
                "volume":42.0,
                "activate":true,
                "future_optional_field":"ignored"
            },
            "future_envelope_field":true
        }"#;
        let (_, command) = parse_interop_request(json).unwrap();
        let InteropCommand::Launch { request } = command else {
            panic!("expected launch command");
        };
        assert_eq!(request.operation_id, "launch-test-1");
        assert_eq!(request.target.as_deref(), Some("media/clip.mkv"));
        assert_eq!(request.volume, Some(42.0));

        let invalid = r#"{
            "command":"launch",
            "request":{
                "operation_id":"",
                "application_identity":"Pealayer",
                "sender_session_id":null,
                "sender_working_directory":null,
                "target":null,
                "fullscreen":false,
                "volume":131.0,
                "activate":true
            }
        }"#;
        assert!(parse_interop_request(invalid).is_err());
    }

    #[test]
    fn text_and_json_rpc_commands_share_the_same_model() {
        let text = parse_text_command("seek-to 12.5").unwrap();
        let request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(1),
            method: "pealayer.seek_to".to_string(),
            params: serde_json::json!({"seconds": 12.5}),
        };
        assert_eq!(command_from_json_rpc(&request).unwrap(), Some(text));
        let message_request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(2),
            method: "pealayer.message.show".to_string(),
            params: serde_json::json!({"message": "Hardware ready"}),
        };
        assert_eq!(
            command_from_json_rpc(&message_request).unwrap(),
            Some(InteropCommand::ShowMessage {
                message: "Hardware ready".to_string()
            })
        );
        let styled_osd_request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(3),
            method: "pealayer.osd.show".to_string(),
            params: serde_json::json!({
                "message": "Centered",
                "position": "center",
                "x_percent": 42.5,
                "y_percent": 60.0,
                "font_size": 30.0,
                "icon": "play",
                "text_color": "#ffffff",
                "background_color": "#112233cc"
            }),
        };
        assert!(matches!(
            command_from_json_rpc(&styled_osd_request).unwrap(),
            Some(InteropCommand::ShowOsd { message, options })
                if message == "Centered"
                    && options.position == Some(OsdAnchor::Center)
                    && options.x_percent == Some(42.5)
                    && options.icon.as_deref() == Some("play")
        ));
        let empty_message_request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(4),
            method: "pealayer.message.show".to_string(),
            params: serde_json::json!({"message": ""}),
        };
        assert_eq!(
            command_from_json_rpc(&empty_message_request).unwrap(),
            Some(InteropCommand::ShowMessage {
                message: String::new()
            })
        );
        let hide_request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(5),
            method: "pealayer.osd.hide".to_string(),
            params: serde_json::json!({}),
        };
        assert_eq!(
            command_from_json_rpc(&hide_request).unwrap(),
            Some(InteropCommand::HideOsd)
        );
        assert!(parse_text_command("volume 131").is_err());
        assert!(parse_text_command("rate 0").is_err());
        assert!(parse_text_command("seek-to -1").is_err());
        assert!(
            InteropCommand::UpdateConfig {
                values: serde_json::json!({"not_a_setting": true}),
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn toast_transports_share_validation_and_contract() {
        let command = parse_text_command("toast Hardware ready").unwrap();
        let direct = serde_json::from_str::<InteropCommand>(r#"{"command":"publish_toast","message":"Hardware ready"}"#).unwrap();
        assert_eq!(command, direct);
        let request = JsonRpcRequest { jsonrpc: Some("2.0".into()), id: serde_json::json!(1), method: "pealayer.toast.show".into(), params: serde_json::json!({"message":"Hardware ready"}) };
        assert_eq!(command_from_json_rpc(&request).unwrap(), Some(command));
        let invalid = JsonRpcRequest { params: serde_json::json!({"message":"","timeout_ms":1}), ..request };
        assert!(command_from_json_rpc(&invalid).is_err());
        let dismiss = JsonRpcRequest { method: "pealayer.toast.dismiss".into(), params: serde_json::json!({"id":"work.1"}), ..invalid };
        assert_eq!(command_from_json_rpc(&dismiss).unwrap(), Some(InteropCommand::DismissToast { id: "work.1".into() }));
        assert!(command_catalog()["messaging"]["surfaces"].as_array().unwrap().contains(&serde_json::json!("terminal")));
    }

    #[test]
    fn chapter_navigation_is_shared_by_text_and_json_rpc_controls() {
        assert_eq!(
            parse_text_command("chapter-next").unwrap(),
            InteropCommand::NextChapter
        );
        assert_eq!(
            parse_text_command("chapter-previous").unwrap(),
            InteropCommand::PreviousChapter
        );
        assert_eq!(
            parse_text_command("chapter 3").unwrap(),
            InteropCommand::SetChapter { index: 3 }
        );
        assert!(parse_text_command("chapter -1").is_err());

        let request = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(1),
            method: "pealayer.chapter.set".to_string(),
            params: serde_json::json!({"index": 2}),
        };
        assert_eq!(
            command_from_json_rpc(&request).unwrap(),
            Some(InteropCommand::SetChapter { index: 2 })
        );
    }

    #[test]
    fn web_controller_effect_commands_are_typed_and_validated() {
        let group: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"method":"controller_effect.group.create","params":{"name":"Cinema lighting","icon":"lamp"}}"#,
        ).unwrap();
        assert!(matches!(command_from_json_rpc(&group).unwrap(),
            Some(InteropCommand::CreateControllerEffectGroup { name, icon })
                if name == "Cinema lighting" && icon == "lamp"));
        assert!(
            InteropCommand::CreateControllerEffectGroup {
                name: " ".to_owned(),
                icon: String::new(),
            }
            .validate()
            .is_err()
        );
        let cue: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"method":"pealayer.controller_effect_cue.add","params":{"reference":"effect:lighting-primary","start_time_ms":1250}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&cue).unwrap(),
            Some(InteropCommand::AddControllerEffectCue { reference, start_time_ms: 1250 })
                if reference == "effect:lighting-primary"
        ));

        let cue_id = uuid::Uuid::new_v4().to_string();
        let update = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(2),
            method: "pealayer.timeline.effect.update".to_string(),
            params: serde_json::json!({
                "instance_id": cue_id,
                "start_time_ms": 10_000,
                "duration_ms": 1_000,
            }),
        };
        assert!(matches!(
            command_from_json_rpc(&update).unwrap(),
            Some(InteropCommand::UpdateEffectCue {
                start_time_ms: 10_000,
                duration_ms: 1_000,
                ..
            })
        ));

        let save: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":3,"method":"controller_effect.save","params":{"id":"lighting-primary","name":"Lighting primary","icon":"lightning","category":"Lighting","kind":"strip-stream","program":{"primitive":"police"},"default_fps":30,"duration_ms":5000,"default_pixels":100,"is_new":true}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&save).unwrap(),
            Some(InteropCommand::SaveControllerEffect { effect })
                if effect.id == "lighting-primary"
                    && effect.icon == "lightning"
                    && effect.program["primitive"] == "police"
        ));

        let invalid: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":4,"method":"controller_effect.play","params":{"reference":"effect; delete all"}}"#,
        )
        .unwrap();
        assert!(command_from_json_rpc(&invalid).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_named_pipe_delivers_typed_commands() {
        let identity = format!("Pealayer native IPC test {}", std::process::id());
        let (tx, rx) = std::sync::mpsc::channel();
        spawn_interop_listener(tx, eframe::egui::Context::default(), identity.clone());
        let payload = serde_json::to_string(&InteropCommand::SetRate { rate: 1.25 }).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let response = loop {
            match send_native_request(&payload, &identity, std::time::Duration::from_millis(250)) {
                Ok(response) => break response,
                Err(error) if std::time::Instant::now() < deadline => {
                    let _ = error;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(error) => panic!("native pipe did not become ready: {error}"),
            }
        };
        assert!(response.contains("\"status\":\"accepted\""));
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
            InteropCommand::SetRate { rate: 1.25 }
        );
    }

    #[test]
    fn launch_destination_rejects_other_identity_and_session() {
        let options = crate::cli::CliOptions {
            target: None,
            fullscreen: false,
            volume: None,
            commands: vec![],
            web_only: false,
        };
        let request = crate::cli::launch_request(&options);
        let identity = request.application_identity.clone();
        assert!(
            validate_launch_destination(&request, &identity, request.sender_session_id,).is_ok()
        );

        let mut other_identity = request.clone();
        other_identity.application_identity = "Different Player".to_string();
        assert!(
            validate_launch_destination(&other_identity, &identity, request.sender_session_id,)
                .is_err()
        );

        #[cfg(target_os = "windows")]
        {
            let mut other_session = request.clone();
            other_session.sender_session_id = Some(
                crate::platform::windows::current_session_id()
                    .expect("current Windows session")
                    .wrapping_add(1),
            );
            assert!(
                validate_launch_destination(&other_session, &identity, request.sender_session_id,)
                    .is_err()
            );
        }
    }

    #[test]
    fn launch_receipts_claim_once_and_recover_after_release() {
        let mut receipts = LaunchReceiptCache::default();
        assert!(receipts.claim("launch-one"));
        assert!(!receipts.claim("launch-one"));
        receipts.release("launch-one");
        assert!(receipts.claim("launch-one"));
    }

    #[test]
    fn test_status_response_serialization() {
        let resp = PlayerStatusResponse {
            status: "ok".to_string(),
            playing: true,
            estop_active: true,
            volume: 80.0,
            playback_time: 15.0,
            duration: 120.0,
            current_video: Some("/path/file.mp4".to_string()),
            fullscreen: true,
            workspace: "simple".to_string(),
            ..PlayerStatusResponse::default()
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"playing\":true"));
        assert!(json.contains("\"volume\":80.0"));
        assert!(json.contains("\"fullscreen\":true"));
        assert!(json.contains("\"estop_active\":true"));
    }

    #[test]
    fn test_parse_dual_protocol_requests() {
        // Standard NDJSON format
        let raw = r#"{"command":"play"}"#;
        let (id, cmd) = parse_interop_request(raw).unwrap();
        assert!(id.is_none());
        assert!(matches!(cmd, InteropCommand::Play));

        // JSON-RPC 2.0 format with id and method
        let rpc = r#"{"jsonrpc":"2.0","id":42,"method":"seek","params":{"seconds":15.0}}"#;
        let (id, cmd) = parse_interop_request(rpc).unwrap();
        assert_eq!(id, Some(serde_json::json!(42)));
        if let InteropCommand::Seek { seconds } = cmd {
            assert_eq!(seconds, 15.0);
        } else {
            panic!("Expected Seek command");
        }

        // JSON-RPC 2.0 set_volume
        let vol_rpc =
            r#"{"jsonrpc":"2.0","id":"vol-1","method":"set_volume","params":{"value":75.0}}"#;
        let (id, cmd) = parse_interop_request(vol_rpc).unwrap();
        assert_eq!(id, Some(serde_json::json!("vol-1")));
        if let InteropCommand::SetVolume { value } = cmd {
            assert_eq!(value, 75.0);
        } else {
            panic!("Expected SetVolume command");
        }
    }

    #[test]
    fn parses_namespaced_json_rpc_commands() {
        let request: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":7,"method":"pealayer.seek","params":{"seconds":12.5}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&request).unwrap(),
            Some(InteropCommand::Seek { seconds: 12.5 })
        ));
        assert!(json_rpc_result(&request.id, serde_json::json!({"ok":true})).contains("\"id\":7"));

        let preferences: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":8,"method":"pealayer.window.preferences"}"#,
        )
        .unwrap();
        assert_eq!(
            command_from_json_rpc(&preferences).unwrap(),
            Some(InteropCommand::OpenPreferences)
        );

        let front_panel: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":9,"method":"pealayer.board.info.open","params":{"tab":"front-panel"}}"#,
        )
        .unwrap();
        assert_eq!(
            command_from_json_rpc(&front_panel).unwrap(),
            Some(InteropCommand::OpenBoardInformation { tab: 2 })
        );
    }

    #[test]
    fn workspace_profile_rpc_uses_stable_ids_and_shared_crud_commands() {
        let restore = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(1),
            method: "pealayer.workspace.restore".to_string(),
            params: serde_json::json!({"profile":"workspace-cinema"}),
        };
        assert_eq!(
            command_from_json_rpc(&restore).unwrap(),
            Some(InteropCommand::SetWorkspace {
                profile: "workspace-cinema".to_string()
            })
        );

        let update = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: serde_json::json!(2),
            method: "pealayer.workspace.update".to_string(),
            params: serde_json::json!({
                "id":"workspace-cinema",
                "name":"Cinema authoring",
                "icon":"timeline",
                "capture":true
            }),
        };
        assert_eq!(
            command_from_json_rpc(&update).unwrap(),
            Some(InteropCommand::UpdateWorkspaceProfile {
                id: "workspace-cinema".to_string(),
                name: "Cinema authoring".to_string(),
                icon: "timeline".to_string(),
                capture: true,
            })
        );
    }

    #[test]
    fn parses_validated_config_json_rpc_commands() {
        let update: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":"preferences","method":"pealayer.config.update","params":{"theme":"dark","show_subseconds":false}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&update).unwrap(),
            Some(InteropCommand::UpdateConfig { values })
                if values["theme"] == "dark" && values["show_subseconds"] == false
        ));

        let reload: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":"preferences","method":"pealayer.config.reload"}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&reload).unwrap(),
            Some(InteropCommand::ReloadConfig)
        ));

        let unknown: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":"preferences","method":"pealayer.config.update","params":{"typo_setting":true}}"#,
        )
        .unwrap();
        assert!(
            command_from_json_rpc(&unknown)
                .unwrap_err()
                .contains("unknown configuration setting")
        );

        let live_request: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":8,"method":"pealayer.open","params":{"target":"rtsp://camera.invalid/live"}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&live_request).unwrap(),
            Some(InteropCommand::Open { target }) if target == "rtsp://camera.invalid/live"
        ));
    }

    #[test]
    fn parses_hardware_presentation_updates_for_web_channel_management() {
        let request: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":"channel","method":"hardware.presentation.update","params":{"key":"relay.5","fields":{"name":"Seat fan","order":2}}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&request).unwrap(),
            Some(InteropCommand::UpdateHardwarePresentation { key, fields })
                if key == "relay.5" && fields["name"] == "Seat fan" && fields["order"] == 2
        ));
    }

    #[test]
    fn test_live_status_snapshot() {
        let status = PlayerStatusResponse {
            status: "ok".to_string(),
            playing: true,
            volume: 92.0,
            playback_time: 45.5,
            duration: 120.0,
            current_video: Some("/path/sample.mkv".to_string()),
            ..PlayerStatusResponse::default()
        };
        set_live_status(status.clone());
        let retrieved = get_live_status();
        assert_eq!(retrieved.playing, true);
        assert_eq!(retrieved.volume, 92.0);
        assert_eq!(retrieved.playback_time, 45.5);
        assert_eq!(
            retrieved.current_video,
            Some("/path/sample.mkv".to_string())
        );
    }

    #[test]
    fn maps_exact_target_pccontroller_action() {
        let event = serde_json::json!({
            "kind": "pealayer.seek",
            "metadata": {
                "target_instance": "pealayer:test",
                "operation_id": "operation-1",
                "operation_delivery_id": "delivery-1",
                "operation_expires_at": "2099-01-01T00:00:00Z",
                "value": "-12.5",
            },
        });
        let action = controller_action_from_event(&event, "pealayer:test").unwrap();
        assert!(matches!(
            action.command,
            Some(InteropCommand::Seek { seconds: -12.5 })
        ));
        assert_eq!(action.acknowledgement["state"], "applied");
        assert_eq!(action.acknowledgement["delivery_id"], "delivery-1");
        assert!(controller_action_from_event(&event, "pealayer:other").is_none());
    }

    #[test]
    fn advertised_pccontroller_workspace_actions_are_executable() {
        assert_eq!(
            PCCONTROLLER_ACTIONS.split(',').collect::<Vec<_>>(),
            vec![
                "pealayer.play",
                "pealayer.pause",
                "pealayer.toggle",
                "pealayer.stop",
                "pealayer.next",
                "pealayer.previous",
                "pealayer.chapter.next",
                "pealayer.chapter.previous",
                "pealayer.chapter.set",
                "pealayer.seek",
                "pealayer.seek_to",
                "pealayer.seek_absolute",
                "pealayer.volume.set",
                "pealayer.mute.set",
                "pealayer.mute.toggle",
                "pealayer.rate.set",
                "pealayer.open",
                "pealayer.fullscreen.set",
                "pealayer.fullscreen.toggle",
                "pealayer.workspace.set",
                "pealayer.window.activate",
                "pealayer.window.minimize",
                "pealayer.window.maximize",
                "pealayer.window.restore",
                "pealayer.quit",
                "pealayer.command",
            ]
        );

        let action = |kind: &str, value: &str| {
            controller_action_from_event(
                &serde_json::json!({
                    "kind": kind,
                    "metadata": {
                        "target_instance": "pealayer:test",
                        "operation_id": format!("operation-{kind}-{value}"),
                        "operation_delivery_id": format!("delivery-{kind}-{value}"),
                        "operation_expires_at": "2099-01-01T00:00:00Z",
                        "value": value,
                    },
                }),
                "pealayer:test",
            )
            .unwrap()
        };

        assert!(matches!(
            action("pealayer.fullscreen.set", "true").command,
            Some(InteropCommand::SetFullscreen { enabled: true })
        ));
        assert!(matches!(
            action("pealayer.fullscreen.set", "false").command,
            Some(InteropCommand::SetFullscreen { enabled: false })
        ));
        assert!(matches!(
            action("pealayer.fullscreen.toggle", "").command,
            Some(InteropCommand::ToggleFullscreen)
        ));
        assert!(
            action("pealayer.fullscreen.toggle", "unexpected")
                .command
                .is_none()
        );
        assert!(matches!(
            action("pealayer.workspace.set", "nle").command,
            Some(InteropCommand::SetWorkspace { profile }) if profile == "nle"
        ));
        assert!(matches!(
            action("pealayer.workspace.set", "simple").command,
            Some(InteropCommand::SetWorkspace { profile }) if profile == "simple"
        ));
        let invalid = action("pealayer.fullscreen.set", "sometimes");
        assert!(invalid.command.is_none());
        assert_eq!(invalid.acknowledgement["state"], "rejected");
        assert_eq!(
            invalid.acknowledgement["reason"],
            "unsupported_or_invalid_pealayer_action"
        );

        for (kind, value) in [
            ("pealayer.seek", "NaN"),
            ("pealayer.seek", "inf"),
            ("pealayer.seek_absolute", "-0.01"),
            ("pealayer.seek_absolute", "100.01"),
            ("pealayer.seek_absolute", "NaN"),
            ("pealayer.volume.set", "-0.01"),
            ("pealayer.volume.set", "130.01"),
            ("pealayer.volume.set", "inf"),
        ] {
            let invalid = action(kind, value);
            assert!(invalid.command.is_none(), "{kind} accepted {value}");
            assert_eq!(invalid.acknowledgement["state"], "rejected");
        }
        assert!(matches!(
            action("pealayer.seek_absolute", "100").command,
            Some(InteropCommand::SeekAbs { percentage: 100.0 })
        ));
        assert!(matches!(
            action("pealayer.volume.set", "130").command,
            Some(InteropCommand::SetVolume { value: 130.0 })
        ));

        let app_page = action("app.page", "play");
        assert!(app_page.command.is_none());
        assert_eq!(app_page.acknowledgement["state"], "rejected");
    }

    #[test]
    fn expired_targeted_action_is_rejected() {
        let event = serde_json::json!({
            "kind": "pealayer.play",
            "metadata": {
                "target_instance": "pealayer:test",
                "operation_id": "operation-expired",
                "operation_delivery_id": "delivery-expired",
                "operation_expires_at": "2000-01-01T00:00:00Z",
            },
        });
        let action = controller_action_from_event(&event, "pealayer:test").unwrap();
        assert!(action.command.is_none());
        assert_eq!(action.acknowledgement["state"], "rejected");
        assert_eq!(action.acknowledgement["reason"], "expired");
    }

    #[test]
    fn controller_state_waits_for_subscription_identity_before_applying() {
        let handle = crate::four_d::engine::spawn_engine();
        *handle.hardware_capabilities.lock().unwrap() =
            Some(crate::four_d::controller::HardwareCapabilities {
                board_connected: true,
                host_instance_id: "old-host".to_string(),
                status_led_revision: 100,
                status_led: Some(crate::four_d::controller::HardwareStatusLed::default()),
                ..Default::default()
            });
        let target = handle.controller_push_target();
        let mut ready = false;
        let mut pending = std::collections::VecDeque::new();
        let state = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "controller.state",
            "params": {
                "kind": "status_led.changed",
                "metadata": {
                    "red": "1", "green": "2", "blue": "3",
                    "brightness": "255", "effect": "0", "condition": "0",
                    "revision": "1"
                }
            }
        });

        assert!(
            gate_controller_subscription_message(state, &mut ready, &mut pending, &target,)
                .unwrap()
                .is_empty()
        );
        assert!(!ready);
        assert_eq!(pending.len(), 1);
        assert_eq!(
            handle
                .hardware_capabilities
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .status_led_revision,
            100
        );

        let acknowledgement = serde_json::json!({"jsonrpc":"2.0","id":1,"result":{"subscribed":true,"instance_id":"new-host"}});
        assert!(gate_controller_subscription_message(acknowledgement.clone(), &mut ready, &mut pending, &target).is_err());
        assert!(!ready);
        // A WebSocket on another coordinator must not overwrite RPC identity.
        assert_eq!(handle.hardware_capabilities.lock().unwrap().as_ref().unwrap().host_instance_id,"old-host");
        // Model the independently refreshed authoritative RPC catalog.
        {
            let mut slot=handle.hardware_capabilities.lock().unwrap();
            let capabilities=slot.as_mut().unwrap();
            capabilities.host_instance_id="new-host".into();
            capabilities.status_led=None;
            capabilities.status_led_revision=0;
        }
        let buffered = gate_controller_subscription_message(
            acknowledgement,
            &mut ready,
            &mut pending,
            &target,
        )
        .unwrap();
        assert!(ready);
        assert_eq!(buffered.len(), 1);
        let params = buffered[0].get("params").unwrap();
        assert!(target.apply_notification("controller.state", params));

        let capabilities = handle.hardware_capabilities.lock().unwrap();
        let capabilities = capabilities.as_ref().unwrap();
        assert_eq!(capabilities.host_instance_id, "new-host");
        assert_eq!(capabilities.status_led_revision, 1);
        assert_eq!(capabilities.status_led.as_ref().unwrap().red, 1);
    }
}
