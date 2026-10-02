use serde::Serialize;

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceControlKind {
    Boolean,
    Number,
    Select,
    Text,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceOption {
    pub value: serde_json::Value,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceControl {
    pub key: &'static str,
    pub section: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub kind: PreferenceControlKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<PreferenceOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub logarithmic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inverted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceSection {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferencesContract {
    pub format: &'static str,
    pub sections: Vec<PreferenceSection>,
    pub controls: Vec<PreferenceControl>,
    pub values: serde_json::Value,
}

impl PreferenceControl {
    fn boolean(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
    ) -> Self {
        Self {
            key,
            section,
            group,
            label,
            kind: PreferenceControlKind::Boolean,
            description: None,
            options: Vec::new(),
            minimum: None,
            maximum: None,
            step: None,
            logarithmic: false,
            inverted: false,
            placeholder: None,
        }
    }

    fn select(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
        options: &[(&'static str, &'static str)],
    ) -> Self {
        let mut control = Self::boolean(key, section, group, label);
        control.kind = PreferenceControlKind::Select;
        control.options = options
            .iter()
            .map(|(value, label)| PreferenceOption {
                value: serde_json::Value::String((*value).to_string()),
                label,
            })
            .collect();
        control
    }

    fn number(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
        minimum: f64,
        maximum: f64,
        step: f64,
    ) -> Self {
        let mut control = Self::boolean(key, section, group, label);
        control.kind = PreferenceControlKind::Number;
        control.minimum = Some(minimum);
        control.maximum = Some(maximum);
        control.step = Some(step);
        control
    }

    fn text(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
        placeholder: &'static str,
    ) -> Self {
        let mut control = Self::boolean(key, section, group, label);
        control.kind = PreferenceControlKind::Text;
        control.placeholder = Some(placeholder);
        control
    }
}

pub fn preference_sections() -> Vec<PreferenceSection> {
    vec![
        PreferenceSection {
            id: "appearance",
            label: "Appearance",
            icon: "sparkle",
        },
        PreferenceSection {
            id: "playback",
            label: "Playback",
            icon: "play",
        },
        PreferenceSection {
            id: "hardware",
            label: "Hardware",
            icon: "plug",
        },
        PreferenceSection {
            id: "input",
            label: "Input",
            icon: "sliders",
        },
        PreferenceSection {
            id: "advanced",
            label: "Advanced",
            icon: "gear",
        },
    ]
}

pub fn preference_controls() -> Vec<PreferenceControl> {
    let mut controls = vec![
        PreferenceControl::select(
            "theme",
            "appearance",
            "Interface",
            "Theme",
            &[("system", "System"), ("light", "Light"), ("dark", "Dark")],
        ),
        PreferenceControl::select(
            "language",
            "appearance",
            "Interface",
            "Language",
            &[
                ("system", "System language"),
                ("en", "English"),
                ("fa", "Persian"),
            ],
        ),
        PreferenceControl::select(
            "fullscreen_video_background",
            "appearance",
            "Interface",
            "Fullscreen background",
            &[
                ("black", "Black"),
                ("dark_gray", "Dark gray"),
                ("theme", "Use app theme"),
            ],
        ),
        PreferenceControl::select(
            "osd_position",
            "appearance",
            "On-screen display",
            "Position",
            &[("top_left", "Top left"), ("center", "Center")],
        ),
        PreferenceControl::number(
            "osd_timeout_seconds",
            "appearance",
            "On-screen display",
            "OSD timeout (seconds)",
            1.0,
            12.0,
            0.5,
        ),
        PreferenceControl::boolean(
            "click_player_to_toggle",
            "playback",
            "Player controls",
            "Single-click the picture to play or pause",
        ),
        PreferenceControl::boolean(
            "show_subseconds",
            "playback",
            "Player controls",
            "Show milliseconds in time displays",
        ),
        PreferenceControl::number(
            "quick_seek_seconds",
            "playback",
            "Player controls",
            "Skip button and arrow-key step (seconds)",
            0.1,
            600.0,
            0.1,
        ),
        PreferenceControl::number(
            "frame_step_count",
            "playback",
            "Player controls",
            "Frames per frame-step action",
            1.0,
            120.0,
            1.0,
        ),
        PreferenceControl::number(
            "wheel_seek_seconds",
            "playback",
            "Player controls",
            "Mouse-wheel seek step (seconds)",
            0.1,
            60.0,
            0.1,
        ),
        PreferenceControl::boolean(
            "open_url_multiline",
            "playback",
            "Open Location / URL",
            "Wrap long URLs in a text area",
        ),
        PreferenceControl::boolean(
            "open_url_history_expanded",
            "playback",
            "Open Location / URL",
            "Expand recent URL history by default",
        ),
        PreferenceControl::boolean(
            "open_url_use_proxy",
            "playback",
            "Open Location / URL",
            "Use a proxy for remote inspection and playback",
        ),
        PreferenceControl::text(
            "open_url_proxy_url",
            "playback",
            "Open Location / URL",
            "Custom proxy URL",
            "http://proxy.example:8080",
        ),
        PreferenceControl::boolean(
            "auto_connect_hardware",
            "hardware",
            "Connection",
            "Discover and connect to PCController on startup",
        ),
        PreferenceControl::boolean(
            "pause_on_hardware_disconnect",
            "hardware",
            "Connection",
            "Pause playback when hardware disconnects unexpectedly",
        ),
        PreferenceControl::text(
            "hardware_endpoint",
            "hardware",
            "Connection",
            "Preferred endpoint",
            "pccontroller://host:port, tcp://host:port, or direct:<device>",
        ),
        PreferenceControl::select(
            "motion_control_mode",
            "hardware",
            "Motion controls",
            "Button behavior",
            &[
                ("toggle", "Toggle on press"),
                ("hold", "Run only while held"),
            ],
        ),
        PreferenceControl::boolean(
            "compact_hardware_controls",
            "hardware",
            "Motion controls",
            "Use one-row compact hardware controls",
        ),
        PreferenceControl::select(
            "paused_drag_action",
            "input",
            "Video surface",
            "Drag while paused",
            &[
                ("move_window", "Move application window"),
                ("seek", "Seek video"),
                ("temporary_fast_forward", "Temporarily fast-forward"),
                ("none", "Do nothing"),
            ],
        ),
        PreferenceControl::select(
            "playing_drag_action",
            "input",
            "Video surface",
            "Drag while playing",
            &[
                ("move_window", "Move application window"),
                ("seek", "Seek video"),
                ("temporary_fast_forward", "Temporarily fast-forward"),
                ("none", "Do nothing"),
            ],
        ),
        PreferenceControl::boolean(
            "single_instance",
            "advanced",
            "Application instance",
            "Use a single application instance",
        ),
        PreferenceControl::boolean(
            "windows_dwm_theming",
            "advanced",
            "Windows graphics and composition",
            "Use DWM title-bar theming",
        ),
        PreferenceControl::boolean(
            "windows_mica_backdrop",
            "advanced",
            "Windows graphics and composition",
            "Use Mica backdrop",
        ),
        PreferenceControl::boolean(
            "opengl_vsync",
            "advanced",
            "Windows graphics and composition",
            "Use OpenGL vertical sync",
        ),
        PreferenceControl::boolean(
            "native_dialog_windows",
            "advanced",
            "Dialog windows",
            "Open Preferences in a separate native window",
        ),
        PreferenceControl::boolean(
            "status_bar.hardware",
            "advanced",
            "Status bar",
            "Hardware connection",
        ),
        PreferenceControl::boolean(
            "status_bar.status_rgb",
            "advanced",
            "Status bar",
            "Physical status RGB",
        ),
        PreferenceControl::boolean(
            "status_bar.warnings",
            "advanced",
            "Status bar",
            "Hardware warnings",
        ),
        PreferenceControl::boolean(
            "status_bar.media_rate",
            "advanced",
            "Status bar",
            "Media rate",
        ),
        PreferenceControl::boolean(
            "status_bar.telemetry",
            "advanced",
            "Status bar",
            "Telemetry",
        ),
        PreferenceControl::boolean(
            "status_bar.workspace",
            "advanced",
            "Status bar",
            "Workspace mode",
        ),
    ];
    controls[7].logarithmic = true;
    controls[9].logarithmic = true;
    let mut recent_click = PreferenceControl::boolean(
        "open_url_recent_click_edits",
        "playback",
        "Open Location / URL",
        "Play when a recent location is clicked",
    );
    recent_click.inverted = true;
    controls.insert(12, recent_click);
    controls
}

pub fn preferences_contract(config: &crate::config::AppConfig) -> PreferencesContract {
    PreferencesContract {
        format: "pealayer-preferences",
        sections: preference_sections(),
        controls: preference_controls(),
        values: serde_json::to_value(config).unwrap_or_else(|_| serde_json::json!({})),
    }
}

pub fn value_at_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.').try_fold(root, |value, key| value.get(key))
}

pub fn set_value_at_path(
    root: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut parts = path.split('.').peekable();
    let mut cursor = root;
    while let Some(key) = parts.next() {
        if parts.peek().is_none() {
            let object = cursor
                .as_object_mut()
                .ok_or_else(|| format!("preference path is not an object: {path}"))?;
            if !object.contains_key(key) {
                return Err(format!("unknown preference setting: {path}"));
            }
            object.insert(key.to_string(), value);
            return Ok(());
        }
        cursor = cursor
            .get_mut(key)
            .ok_or_else(|| format!("unknown preference setting: {path}"))?;
    }
    Err("preference path must not be empty".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_keys_resolve_against_the_real_config() {
        let config = crate::config::AppConfig::default();
        let values = serde_json::to_value(config).unwrap();
        for control in preference_controls() {
            assert!(
                value_at_path(&values, control.key).is_some(),
                "{}",
                control.key
            );
        }
    }

    #[test]
    fn nested_values_are_updated_without_flattening_the_config() {
        let mut values = serde_json::to_value(crate::config::AppConfig::default()).unwrap();
        set_value_at_path(&mut values, "status_bar.hardware", serde_json::json!(false)).unwrap();
        assert_eq!(
            value_at_path(&values, "status_bar.hardware"),
            Some(&serde_json::json!(false))
        );
        let config: crate::config::AppConfig = serde_json::from_value(values).unwrap();
        assert!(!config.status_bar.hardware);
    }
}
