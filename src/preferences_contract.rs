use serde::Serialize;

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceControlKind {
    Accent,
    Boolean,
    Number,
    ReplacementList,
    Select,
    Text,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceOption {
    pub value: serde_json::Value,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_key: Option<&'static str>,
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
            custom_key: None,
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
                color: None,
            })
            .collect();
        control
    }

    fn accent(config: &crate::config::AppConfig) -> Self {
        let mut control = Self::boolean("accent_color", "appearance", "Interface", "Accent color");
        control.kind = PreferenceControlKind::Accent;
        control.custom_key = Some("custom_accent_color");
        let [red, green, blue] =
            crate::platform::windows::system_accent_color().unwrap_or_else(|| {
                if cfg!(target_os = "macos") {
                    [10, 132, 255]
                } else {
                    [0, 120, 212]
                }
            });
        let system_color = format!("#{red:02x}{green:02x}{blue:02x}");
        let custom_color = config
            .custom_accent_color
            .as_deref()
            .and_then(crate::config::parse_rgb_hex)
            .map(|[red, green, blue]| format!("#{red:02x}{green:02x}{blue:02x}"))
            .unwrap_or_else(|| "#0078d4".to_string());
        control.options = vec![
            PreferenceOption {
                value: serde_json::json!("system"),
                label: "System accent",
                color: Some(system_color),
            },
            PreferenceOption {
                value: serde_json::json!("pealayer_green"),
                label: "Pealayer green",
                color: Some("#38d27a".to_string()),
            },
            PreferenceOption {
                value: serde_json::json!("macos_blue"),
                label: "macOS blue",
                color: Some("#0a84ff".to_string()),
            },
            PreferenceOption {
                value: serde_json::json!("custom"),
                label: "Custom",
                color: Some(custom_color),
            },
        ];
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

    fn replacement_list(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
    ) -> Self {
        let mut control = Self::boolean(key, section, group, label);
        control.kind = PreferenceControlKind::ReplacementList;
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

pub fn preference_controls(config: &crate::config::AppConfig) -> Vec<PreferenceControl> {
    let mut controls = vec![
        PreferenceControl::select(
            "theme",
            "appearance",
            "Interface",
            "Theme",
            &[("system", "System"), ("light", "Light"), ("dark", "Dark")],
        ),
        PreferenceControl::accent(config),
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
        {
            let mut control = PreferenceControl::number(
                "playback_speed",
                "playback",
                "Player controls",
                "Playback speed",
                0.25,
                4.0,
                0.05,
            );
            control.description =
                Some("Normal speed used for playback and restored after a temporary fast-forward");
            control
        },
        {
            let mut control = PreferenceControl::number(
                "temporary_fast_forward_speed",
                "playback",
                "Player controls",
                "Hold-to-fast-forward speed",
                1.0,
                16.0,
                0.25,
            );
            control.description =
                Some("Speed used while holding the configured temporary fast-forward gesture");
            control
        },
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
        PreferenceControl::select(
            "subtitle_direction",
            "playback",
            "Subtitles",
            "Subtitle layout",
            &[
                ("auto", "Automatic"),
                ("ltr", "Left to right"),
                ("rtl", "Right to left"),
            ],
        ),
        {
            let mut control = PreferenceControl::replacement_list(
                "subtitle_text_replacements",
                "playback",
                "Subtitles",
                "Global text replacements",
            );
            control.description = Some("Applied in order to every text subtitle");
            control
        },
        PreferenceControl::boolean(
            "restore_last_media_on_startup",
            "playback",
            "Playback history",
            "Reopen the last media when Pealayer starts",
        ),
        PreferenceControl::boolean(
            "remember_playback_position",
            "playback",
            "Playback history",
            "Remember the last position of local and remote media",
        ),
        PreferenceControl::number(
            "playback_position_history_limit",
            "playback",
            "Playback history",
            "Maximum remembered videos",
            1.0,
            crate::config::MAX_PLAYBACK_POSITION_HISTORY_LIMIT as f64,
            1.0,
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
            "open_url_fetch_remote_info",
            "playback",
            "Open Location / URL",
            "Fetch remote media information automatically",
        ),
        PreferenceControl::boolean(
            "open_url_fetch_remote_thumbnail",
            "playback",
            "Open Location / URL",
            "Fetch a remote media thumbnail automatically",
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
            &[("hold", "Push"), ("toggle", "Toggle")],
        ),
        PreferenceControl::boolean(
            "compact_hardware_controls",
            "hardware",
            "Motion controls",
            "Use one-row compact hardware controls",
        ),
        PreferenceControl::boolean(
            "compact_timeline_tracks",
            "hardware",
            "Timeline",
            "Use compact timeline track rows",
        ),
        PreferenceControl::select(
            "non_user_control_visibility",
            "hardware",
            "Hardware channels",
            "Non-user and diagnostic controls",
            &[
                ("hidden", "Hide unless explicitly requested"),
                ("dimmed", "Show with reduced emphasis"),
                ("shown", "Show like user controls"),
            ],
        ),
        PreferenceControl::boolean(
            "prefix_relay_identifiers",
            "hardware",
            "Motion controls",
            "Prefix relay captions with channel identifiers",
        ),
        PreferenceControl::boolean(
            "live_pwm_updates",
            "hardware",
            "Output controls",
            "Update PWM outputs while dragging",
        ),
        PreferenceControl::boolean(
            "hardware_actions_on_press",
            "hardware",
            "Output controls",
            "Activate output buttons when pressed",
        ),
        PreferenceControl::boolean(
            "show_estop_control",
            "hardware",
            "Emergency stop",
            "Show E-STOP in the application header",
        ),
        PreferenceControl::boolean(
            "confirm_estop_release",
            "hardware",
            "Emergency stop",
            "Confirm before releasing E-STOP",
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
            "auto_reload_config",
            "advanced",
            "Config file",
            "Automatically reload configuration changes",
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
    for key in ["quick_seek_seconds", "mouse_seek_seconds_per_notch"] {
        if let Some(control) = controls.iter_mut().find(|control| control.key == key) {
            control.logarithmic = true;
        }
    }
    let mut recent_click = PreferenceControl::boolean(
        "open_url_recent_click_edits",
        "playback",
        "Open Location / URL",
        "Play when a recent location is clicked",
    );
    recent_click.inverted = true;
    controls.insert(14, recent_click);
    controls
}

pub fn preferences_contract(config: &crate::config::AppConfig) -> PreferencesContract {
    PreferencesContract {
        format: "pealayer-preferences",
        sections: preference_sections(),
        controls: preference_controls(config),
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
        let values = serde_json::to_value(&config).unwrap();
        for control in preference_controls(&config) {
            assert!(
                value_at_path(&values, control.key).is_some(),
                "{}",
                control.key
            );
        }
    }

    #[test]
    fn subtitle_controls_are_shared_and_seed_persian_normalization() {
        let config = crate::config::AppConfig::default();
        let controls = preference_controls(&config);
        let direction = controls
            .iter()
            .find(|control| control.key == "subtitle_direction")
            .expect("subtitle direction control");
        assert!(matches!(direction.kind, PreferenceControlKind::Select));
        assert_eq!(direction.section, "playback");

        let replacements = controls
            .iter()
            .find(|control| control.key == "subtitle_text_replacements")
            .expect("subtitle replacement control");
        assert!(matches!(
            replacements.kind,
            PreferenceControlKind::ReplacementList
        ));
        assert_eq!(
            crate::subtitle::apply_text_replacements("جيك", &config.subtitle_text_replacements),
            "جیک"
        );
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

    #[test]
    fn accent_is_one_compound_control_with_swatch_metadata() {
        let config = crate::config::AppConfig {
            custom_accent_color: Some("#A142F4".to_string()),
            ..crate::config::AppConfig::default()
        };
        let controls = preference_controls(&config);
        let accent = controls
            .iter()
            .find(|control| control.key == "accent_color")
            .expect("accent control");
        assert!(matches!(accent.kind, PreferenceControlKind::Accent));
        assert_eq!(accent.custom_key, Some("custom_accent_color"));
        assert!(
            accent
                .options
                .iter()
                .all(|option| option.value != serde_json::json!("windows_blue")),
            "Windows blue must not be offered as an accent preset"
        );
        assert_eq!(
            accent
                .options
                .iter()
                .find(|option| option.value == serde_json::json!("custom"))
                .and_then(|option| option.color.as_deref()),
            Some("#a142f4")
        );
        assert!(
            controls
                .iter()
                .all(|control| control.key != "custom_accent_color"),
            "the custom hex value belongs inline with the accent picker"
        );
    }

    #[test]
    fn non_user_controls_have_one_explicit_three_state_policy() {
        let controls = preference_controls(&crate::config::AppConfig::default());
        let visibility = controls
            .iter()
            .find(|control| control.key == "non_user_control_visibility")
            .expect("non-user control visibility preference");
        assert!(matches!(visibility.kind, PreferenceControlKind::Select));
        assert_eq!(
            visibility
                .options
                .iter()
                .map(|option| option.value.as_str().unwrap())
                .collect::<Vec<_>>(),
            ["hidden", "dimmed", "shown"]
        );
    }

    #[test]
    fn emergency_stop_visibility_and_release_guard_are_shared_preferences() {
        let controls = preference_controls(&crate::config::AppConfig::default());
        for key in ["show_estop_control", "confirm_estop_release"] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .unwrap_or_else(|| panic!("missing {key} preference"));
            assert!(matches!(control.kind, PreferenceControlKind::Boolean));
            assert_eq!(control.section, "hardware");
        }
    }

    #[test]
    fn timeline_density_is_a_shared_persistent_preference() {
        let config = crate::config::AppConfig::default();
        assert!(config.compact_timeline_tracks);
        let controls = preference_controls(&config);
        let control = controls
            .iter()
            .find(|control| control.key == "compact_timeline_tracks")
            .expect("timeline density preference");
        assert!(matches!(control.kind, PreferenceControlKind::Boolean));
        assert_eq!(control.section, "hardware");
        assert_eq!(control.group, "Timeline");
    }

    #[test]
    fn playback_speeds_are_shared_numeric_preferences() {
        let controls = preference_controls(&crate::config::AppConfig::default());
        for (key, min, max) in [
            ("playback_speed", 0.25, 4.0),
            ("temporary_fast_forward_speed", 1.0, 16.0),
        ] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .unwrap_or_else(|| panic!("missing {key} preference"));
            assert!(matches!(control.kind, PreferenceControlKind::Number));
            assert_eq!(control.section, "playback");
            assert_eq!(control.minimum, Some(min));
            assert_eq!(control.maximum, Some(max));
        }
    }
}
