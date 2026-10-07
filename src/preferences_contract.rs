use serde::Serialize;

fn timeline_wheel_control(key: &'static str, label: &'static str) -> PreferenceControl {
    let mut control = PreferenceControl::select(key, "input", "Timeline navigation", label, &[
        ("zoom", "Zoom"),
        ("vertical_scroll", "Scroll vertically"),
        ("horizontal_scroll", "Scroll horizontally"),
        ("none", "No action"),
    ]);
    control.description = Some("Vertical wheel action. Horizontal wheel always pans horizontally. Combined modifiers use Shift, then Ctrl / Command, then Alt.");
    control
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceControlKind {
    Accent,
    Boolean,
    Number,
    MultiSelect,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<&'static str>,
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
    pub integer: bool,
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
    pub defaults: serde_json::Value,
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
            integer: false,
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
                description: None,
                icon: None,
            })
            .collect();
        control
    }

    fn multi_select(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
        options: Vec<PreferenceOption>,
    ) -> Self {
        let mut control = Self::boolean(key, section, group, label);
        control.kind = PreferenceControlKind::MultiSelect;
        control.options = options;
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
                description: None,
                icon: None,
            },
            PreferenceOption {
                value: serde_json::json!("pealayer_green"),
                label: "Pealayer green",
                color: Some("#38d27a".to_string()),
                description: None,
                icon: None,
            },
            PreferenceOption {
                value: serde_json::json!("macos_blue"),
                label: "macOS blue",
                color: Some("#0a84ff".to_string()),
                description: None,
                icon: None,
            },
            PreferenceOption {
                value: serde_json::json!("custom"),
                label: "Custom",
                color: Some(custom_color),
                description: None,
                icon: None,
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
            id: "web",
            label: "Web UI",
            icon: "globe",
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
            "color_palette",
            "appearance",
            "Interface",
            "Color palette",
            &[("native", "Neutral (default)"), ("studio", "Studio")],
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
        {
            let mut control = PreferenceControl::select(
                "always_on_top", "appearance", "Window", "Always on top",
                &[
                    ("never", "Never"),
                    ("always", "Always"),
                    ("while_playing_video", "While playing video"),
                ],
            );
            control.description = Some("Keep the player window above other windows. While playing video returns to normal on pause or end; audio-only playback does not pin the window.");
            control
        },
        {
            let mut control = PreferenceControl::boolean(
                "consistent_video_aspect_ratio",
                "appearance",
                "Window",
                "Match window to video aspect ratio",
            );
            control.description = Some(
                "In the Simple workspace, resizes the window when MPV loads a video or changes the active video stream",
            );
            control
        },
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
        {
            let mut control = PreferenceControl::boolean(
                "seekbar_hover_thumbnails",
                "playback",
                "Seek preview",
                "Show preview card in Simple workspace",
            );
            control.description =
                Some("Shows a compact decoded-frame card above the Simple player seekbar");
            control
        },
        {
            let mut control = PreferenceControl::boolean(
                "nle_seekbar_hover_thumbnails",
                "playback",
                "Seek preview",
                "Show preview card in NLE workspace",
            );
            control.description =
                Some("Disabled by default so the NLE transport and timeline remain unobstructed");
            control
        },
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
            "Text direction",
            &[
                ("auto", "Automatic"),
                ("ltr", "Left to right"),
                ("rtl", "Right to left"),
            ],
        ),
        {
            let mut control = PreferenceControl::select(
                "subtitle_alignment",
                "playback",
                "Subtitles",
                "Text alignment",
                &[("left", "Left"), ("center", "Center"), ("right", "Right"), ("subtitle_style", "Subtitle style")],
            );
            control.description = Some("Horizontal placement is independent of text direction; subtitle style preserves native styling without text processing");
            control
        },
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
        PreferenceControl::boolean("remote_folder_auto_next", "playback", "Remote folders", "Automatically play the next file in the folder"),
        PreferenceControl::boolean("remote_folder_thumbnails", "playback", "Remote folders", "Generate thumbnails for listed media files"),
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
        PreferenceControl::boolean(
            "timeline_header_wheel_vertical_scroll",
            "input",
            "Timeline navigation",
            "Scroll track headers vertically with the mouse wheel",
        ),
        timeline_wheel_control("timeline_plain_wheel_action", "Mouse wheel"),
        timeline_wheel_control("timeline_ctrl_wheel_action", "Ctrl / Command + mouse wheel"),
        timeline_wheel_control("timeline_shift_wheel_action", "Shift + mouse wheel"),
        timeline_wheel_control("timeline_alt_wheel_action", "Alt + mouse wheel"),
        PreferenceControl::boolean(
            "timeline_middle_button_pan",
            "input",
            "Timeline navigation",
            "Pan the timeline with the middle mouse button",
        ),
        PreferenceControl::boolean(
            "timeline_middle_axis_lock_modifiers",
            "input",
            "Timeline navigation",
            "Constrain middle-button pan with Shift or Ctrl",
        ),
        PreferenceControl::boolean(
            "timeline_animated_navigation",
            "input",
            "Timeline navigation",
            "Animate timeline navigation",
        ),
        PreferenceControl::number(
            "timeline_navigation_transition_ms",
            "input",
            "Timeline navigation",
            "Navigation transition duration (milliseconds)",
            50.0,
            2_000.0,
            10.0,
        ),
        {
            let mut control = PreferenceControl::boolean(
                "human_readable_time_units",
                "input",
                "Numeric input",
                "Human-readable time units",
            );
            control.description = Some(
                "Automatically normalize time fields to units such as s, min, and h. Disable to always display milliseconds.",
            );
            control
        },
        PreferenceControl::boolean(
            "keyboard_shortcuts_enabled",
            "input",
            "Keyboard shortcuts",
            "Enable in-app keyboard shortcuts and hardware bindings",
        ),
        PreferenceControl::boolean(
            "global_hardware_hotkeys_enabled",
            "input",
            "Keyboard shortcuts",
            "Allow hardware hotkeys while Pealayer is in the background",
        ),
        {
            let mut control = PreferenceControl::boolean(
                "media_keys_enabled", "input", "Keyboard shortcuts",
                "Respond to keyboard media controls",
            );
            control.description = Some("Play, pause, stop, next/previous and seek through the OS media session, even in the background. Independent of in-app shortcuts and hardware hotkeys.");
            control
        },
        PreferenceControl::text("application_shortcuts.fullscreen", "input", "Application shortcuts", "Toggle fullscreen", "F11 (empty disables)"),
        PreferenceControl::text("application_shortcuts.media_information", "input", "Application shortcuts", "Media information", "Shift+F10"),
        PreferenceControl::text("application_shortcuts.media_folder", "input", "Application shortcuts", "Open media containing folder", "Ctrl+Shift+F10"),
        PreferenceControl::text("application_shortcuts.preferences", "input", "Application shortcuts", "Open Preferences", "Ctrl+, / Cmd+,"),
        PreferenceControl::text("application_shortcuts.edit_config", "input", "Application shortcuts", "Edit configuration file", "Ctrl+Shift+, / Cmd+Shift+,"),
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
            "Left-button hold / drag while paused",
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
            "Left-button hold / drag while playing",
            &[
                ("move_window", "Move application window"),
                ("seek", "Seek video"),
                ("temporary_fast_forward", "Temporarily fast-forward"),
                ("none", "Do nothing"),
            ],
        ),
        PreferenceControl::select(
            "middle_click_action",
            "input",
            "Video surface",
            "Middle-button click",
            &[
                ("play_pause", "Play / pause"),
                ("toggle_mute", "Mute / unmute"),
                ("toggle_fullscreen", "Toggle fullscreen"),
                ("context_menu", "Open context menu"),
                ("none", "Do nothing"),
            ],
        ),
        PreferenceControl::select(
            "middle_hold_action",
            "input",
            "Video surface",
            "Middle-button hold / drag",
            &[
                ("move_window", "Move application window"),
                ("seek", "Seek video"),
                ("temporary_fast_forward", "Temporarily fast-forward"),
                ("none", "Do nothing"),
            ],
        ),
        PreferenceControl::select(
            "right_click_action",
            "input",
            "Video surface",
            "Right-button click",
            &[
                ("play_pause", "Play / pause"),
                ("toggle_mute", "Mute / unmute"),
                ("toggle_fullscreen", "Toggle fullscreen"),
                ("context_menu", "Open context menu"),
                ("none", "Do nothing"),
            ],
        ),
        PreferenceControl::select(
            "right_hold_action",
            "input",
            "Video surface",
            "Right-button hold / drag",
            &[
                ("move_window", "Move application window"),
                ("seek", "Seek video"),
                ("temporary_fast_forward", "Temporarily fast-forward"),
                ("none", "Do nothing"),
            ],
        ),
        {
            let mut control = PreferenceControl::boolean(
                "web_enabled",
                "web",
                "Web server",
                "Enable Web UI and network APIs",
            );
            control.description = Some(
                "Hosts the PWA, REST, WebSocket, JSON-RPC, and HTTP IPC surfaces after restart",
            );
            control
        },
        {
            let options = crate::network::discover_bind_targets()
                .into_iter()
                .map(|target| PreferenceOption {
                    value: serde_json::Value::String(target.value),
                    label: target.label,
                    color: None,
                    description: Some(target.detail),
                    icon: Some(target.icon),
                })
                .collect();
            let mut control = PreferenceControl::multi_select(
                "web_listen_addresses",
                "web",
                "Web server",
                "Listening interfaces and addresses",
                options,
            );
            control.description = Some(
                "Select one or more discovered host addresses; wildcard entries include future adapters and apply after restart",
            );
            control
        },
        {
            let mut control = PreferenceControl::number(
                "web_port",
                "web",
                "Web server",
                "Port",
                1.0,
                65_535.0,
                1.0,
            );
            control.description = Some(
                "Shared by the Web UI, REST, WebSocket, JSON-RPC, and HTTP IPC surfaces; applies after restart",
            );
            control
        },
        PreferenceControl::boolean(
            "web_allow_control",
            "web",
            "Permissions",
            "Allow player, window, OSD, and hardware control",
        ),
        PreferenceControl::boolean(
            "web_allow_configuration",
            "web",
            "Permissions",
            "Allow configuration access",
        ),
        PreferenceControl::boolean(
            "web_allow_file_access",
            "web",
            "Permissions",
            "Allow host file browsing, retrieval and downloads",
        ),
        PreferenceControl::boolean(
            "web_allow_updates",
            "web",
            "Permissions",
            "Allow application update operations",
        ),
        PreferenceControl::boolean(
            "web_sync_state",
            "web",
            "Live synchronization",
            "Synchronize live player and hardware state",
        ),
        {
            let mut control = PreferenceControl::number(
                "web_sync_interval_ms",
                "web",
                "Live synchronization",
                "State refresh interval (milliseconds)",
                16.0,
                5_000.0,
                1.0,
            );
            control.description =
                Some("Limits WebSocket snapshot frequency; 100 ms equals 10 updates per second");
            control
        },
        {
            let mut control = PreferenceControl::boolean(
                "window_magnetic_snap",
                "input",
                "Window movement",
                "Magnetic window snap",
            );
            control.description = Some(
                "Snap near monitor work-area edges. Hold Ctrl while dragging to bypass it temporarily.",
            );
            control
        },
        {
            let mut control = PreferenceControl::number(
                "window_magnetic_snap_distance",
                "input",
                "Window movement",
                "Snap distance",
                1.0,
                128.0,
                1.0,
            );
            control.description = Some("Distance in device-independent pixels.");
            control
        },
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
            "windows_video_taskbar_thumbnail",
            "advanced",
            "Windows shell",
            "Show the video in the taskbar thumbnail",
        ),
        PreferenceControl::boolean(
            "windows_thumbnail_toolbar",
            "advanced",
            "Windows shell",
            "Show playback actions below the taskbar thumbnail",
        ),
        PreferenceControl::boolean(
            "windows_jump_list_quick_actions",
            "advanced",
            "Windows shell",
            "Show quick actions in the taskbar and Start menu",
        ),
        {
            let mut control = PreferenceControl::select(
                "windows_video_renderer",
                "advanced",
                "Windows graphics and composition",
                "Video renderer",
                &[("open_gl", "OpenGL"), ("d3d11", "D3D11 / DirectComposition")],
            );
            control.description = Some(
                "Selects libmpv's Windows video presentation path. Restart Pealayer after changing this setting.",
            );
            control
        },
        PreferenceControl::boolean(
            "opengl_vsync",
            "advanced",
            "Windows graphics and composition",
            "Use OpenGL vertical sync",
        ),
        {
            let mut control = PreferenceControl::boolean(
                "live_video_during_window_move",
                "advanced",
                "Windows graphics and composition",
                "Keep video playing while moving the window",
            );
            control.description = Some(
                "Render live libmpv frames during a native window drag. Turn this off only as a compatibility fallback for a problematic graphics driver.",
            );
            control
        },
        {
            let mut control = PreferenceControl::boolean(
                "compositor_paced_window_move",
                "advanced",
                "Windows graphics and composition",
                "Use compositor-paced window movement",
            );
            control.description = Some(
                "Synchronize live move/resize repaints to DWM only while the window is being dragged, independent of the media frame rate.",
            );
            control
        },
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
    let values = serde_json::to_value(config).unwrap_or_else(|_| serde_json::json!({}));
    let mut controls = preference_controls(config);
    for control in &mut controls {
        if matches!(control.kind, PreferenceControlKind::Number) {
            control.integer = value_at_path(&values, control.key).is_some_and(|v| v.is_i64() || v.is_u64());
        }
    }
    PreferencesContract {
        format: "pealayer-preferences",
        sections: preference_sections(),
        controls,
        values,
        defaults: serde_json::to_value(crate::config::AppConfig::default()).unwrap_or_default(),
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
    fn application_shortcuts_are_shared_editable_controls() {
        let config = crate::config::AppConfig::default();
        let contract = preferences_contract(&config);
        for key in ["fullscreen", "media_information", "media_folder", "preferences", "edit_config"] {
            let path = format!("application_shortcuts.{key}");
            let control = contract.controls.iter().find(|control| control.key == path).unwrap();
            assert!(matches!(control.kind, PreferenceControlKind::Text));
            assert_eq!(control.section, "input");
            assert!(value_at_path(&contract.values, &path).unwrap().is_string());
        }
        let mut values = contract.values;
        set_value_at_path(&mut values, "application_shortcuts.fullscreen", serde_json::json!("Alt+Enter")).unwrap();
        let restored: crate::config::AppConfig = serde_json::from_value(values).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.application_shortcuts.fullscreen, "Alt+Enter");
    }

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

        let alignment = controls.iter().find(|control| control.key == "subtitle_alignment")
            .expect("subtitle alignment control");
        assert!(matches!(alignment.kind, PreferenceControlKind::Select));
        assert_eq!(alignment.options.iter().map(|option| option.value.as_str().unwrap()).collect::<Vec<_>>(),
            vec!["left", "center", "right", "subtitle_style"]);
        let contract = preferences_contract(&config);
        assert_eq!(contract.values["subtitle_alignment"], "center");
        assert_eq!(contract.defaults["subtitle_alignment"], "center");
        assert!(crate::config::AppConfig::validate_patch_shape(&serde_json::json!({"subtitle_alignment":"right"})).is_ok());

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
    fn web_preferences_include_real_interface_metadata_and_permissions() {
        let controls = preference_controls(&crate::config::AppConfig::default());
        let listeners = controls
            .iter()
            .find(|control| control.key == "web_listen_addresses")
            .expect("Web listener selector");
        assert!(matches!(listeners.kind, PreferenceControlKind::MultiSelect));
        assert_eq!(listeners.section, "web");
        assert!(listeners.options.iter().any(|option| {
            option.value == serde_json::json!("0.0.0.0")
                && option.icon == Some("globe")
                && option.description.is_some()
        }));
        for key in [
            "web_allow_control",
            "web_allow_configuration",
            "web_allow_file_access",
            "web_allow_updates",
            "web_sync_state",
            "web_sync_interval_ms",
        ] {
            assert!(
                controls.iter().any(|control| control.key == key),
                "missing {key}"
            );
        }
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
    fn seekbar_thumbnail_preview_is_configurable_per_workspace() {
        let config = crate::config::AppConfig::default();
        assert!(!config.seekbar_hover_thumbnails);
        assert!(!config.nle_seekbar_hover_thumbnails);
        let controls = preference_controls(&config);
        for key in ["seekbar_hover_thumbnails", "nle_seekbar_hover_thumbnails"] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .expect("workspace seekbar hover thumbnail preference");
            assert!(matches!(control.kind, PreferenceControlKind::Boolean));
            assert_eq!(control.section, "playback");
            assert_eq!(control.group, "Seek preview");
        }
    }

    #[test]
    fn consistent_video_aspect_ratio_is_enabled_in_shared_window_preferences() {
        let config = crate::config::AppConfig::default();
        assert!(config.consistent_video_aspect_ratio);
        let controls = preference_controls(&config);
        let control = controls
            .iter()
            .find(|control| control.key == "consistent_video_aspect_ratio")
            .expect("consistent video aspect ratio preference");
        assert!(matches!(control.kind, PreferenceControlKind::Boolean));
        assert_eq!(control.section, "appearance");
        assert_eq!(control.group, "Window");
    }

    #[test]
    fn always_on_top_is_shared_and_defaults_to_never() {
        use crate::config::AlwaysOnTopMode;
        let config: crate::config::AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.always_on_top, AlwaysOnTopMode::Never);
        let controls = preference_controls(&config);
        let control = controls.iter().find(|control| control.key == "always_on_top").unwrap();
        assert_eq!(control.section, "appearance");
        assert_eq!(control.group, "Window");
        assert!(matches!(control.kind, PreferenceControlKind::Select));
        assert_eq!(control.options.len(), 3);
        for (mode, value) in [
            (AlwaysOnTopMode::Never, "never"),
            (AlwaysOnTopMode::Always, "always"),
            (AlwaysOnTopMode::WhilePlayingVideo, "while_playing_video"),
        ] {
            assert_eq!(serde_json::to_value(mode).unwrap(), value);
            assert!(control.options.iter().any(|option| option.value == value));
            let patched = config.apply_patch(&serde_json::json!({"always_on_top": value})).unwrap();
            assert_eq!(patched.always_on_top, mode);
            for loaded in [false, true] {
                for paused in [false, true] {
                    for ended in [false, true] {
                        let expected = mode == AlwaysOnTopMode::Always
                            || (mode == AlwaysOnTopMode::WhilePlayingVideo && loaded && !paused && !ended);
                        assert_eq!(mode.is_active(loaded, paused, ended), expected);
                    }
                }
            }
        }
        assert!(config.apply_patch(&serde_json::json!({"always_on_top": "sometimes"})).is_err());
    }

    #[test]
    fn magnetic_window_snap_matches_the_shared_rayanlamp_contract() {
        let config = crate::config::AppConfig::default();
        assert!(!config.window_magnetic_snap);
        assert_eq!(config.window_magnetic_snap_distance, 16);
        let controls = preference_controls(&config);
        let enabled = controls
            .iter()
            .find(|control| control.key == "window_magnetic_snap")
            .expect("magnetic snap toggle");
        assert!(matches!(enabled.kind, PreferenceControlKind::Boolean));
        assert_eq!(enabled.section, "input");
        assert_eq!(enabled.group, "Window movement");
        let distance = controls
            .iter()
            .find(|control| control.key == "window_magnetic_snap_distance")
            .expect("magnetic snap distance");
        assert!(matches!(distance.kind, PreferenceControlKind::Number));
        assert_eq!(
            (distance.minimum, distance.maximum),
            (Some(1.0), Some(128.0))
        );
    }

    #[test]
    fn live_video_during_native_window_movement_is_enabled_and_configurable() {
        let config = crate::config::AppConfig::default();
        assert!(config.live_video_during_window_move);
        assert!(config.compositor_paced_window_move);
        let controls = preference_controls(&config);
        let control = controls
            .iter()
            .find(|control| control.key == "live_video_during_window_move")
            .expect("live window-movement video preference");
        assert!(matches!(control.kind, PreferenceControlKind::Boolean));
        assert_eq!(control.section, "advanced");
        assert_eq!(control.group, "Windows graphics and composition");
        assert!(
            control
                .description
                .is_some_and(|description| description.contains("compatibility fallback"))
        );
        let pacing = controls
            .iter()
            .find(|control| control.key == "compositor_paced_window_move")
            .expect("compositor-paced movement preference");
        assert!(matches!(pacing.kind, PreferenceControlKind::Boolean));
        assert_eq!(pacing.group, "Windows graphics and composition");
    }

    #[test]
    fn windows_taskbar_media_surfaces_are_enabled_and_configurable() {
        let config = crate::config::AppConfig::default();
        assert!(config.windows_video_taskbar_thumbnail);
        assert!(config.windows_thumbnail_toolbar);
        assert!(config.windows_jump_list_quick_actions);
        let controls = preference_controls(&config);
        for key in [
            "windows_video_taskbar_thumbnail",
            "windows_thumbnail_toolbar",
            "windows_jump_list_quick_actions",
        ] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .unwrap_or_else(|| panic!("missing {key} preference"));
            assert!(matches!(control.kind, PreferenceControlKind::Boolean));
            assert_eq!(control.section, "advanced");
            assert_eq!(control.group, "Windows shell");
        }
    }

    #[test]
    fn timeline_navigation_gestures_are_shared_persistent_preferences() {
        let config = crate::config::AppConfig::default();
        let previous: crate::config::AppConfig = serde_json::from_str(r#"{"timeline_ctrl_wheel_zoom":true}"#).unwrap();
        use crate::config::TimelineWheelBehavior as Wheel;
        assert_eq!(previous.timeline_ctrl_wheel_action, Wheel::VerticalScroll);
        let changed = crate::config::AppConfig {
            timeline_plain_wheel_action: Wheel::VerticalScroll,
            timeline_ctrl_wheel_action: Wheel::Zoom,
            timeline_shift_wheel_action: Wheel::None,
            timeline_alt_wheel_action: Wheel::HorizontalScroll,
            ..config.clone()
        };
        let serialized = serde_json::to_string(&changed).unwrap();
        let reloaded: crate::config::AppConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(reloaded, changed);
        let status = crate::platform::interop::PlayerStatusResponse {
            timeline_wheel_preferences: Some(crate::config::TimelineWheelPreferences {
                plain: changed.timeline_plain_wheel_action,
                ctrl: changed.timeline_ctrl_wheel_action,
                shift: changed.timeline_shift_wheel_action,
                alt: changed.timeline_alt_wheel_action,
            }),
            ..Default::default()
        };
        let wire = serde_json::to_value(&status).unwrap();
        assert_eq!(wire["timeline_wheel_preferences"], serde_json::json!({
            "plain": "vertical_scroll", "ctrl": "zoom", "shift": "none", "alt": "horizontal_scroll"
        }));
        assert_eq!(serde_json::from_value::<crate::platform::interop::PlayerStatusResponse>(wire)
            .unwrap().timeline_wheel_preferences, status.timeline_wheel_preferences);
        assert!(crate::config::AppConfig::validate_patch_shape(&serde_json::json!({
            "timeline_ctrl_wheel_action": "zoom", "timeline_alt_wheel_action": "horizontal_scroll"
        })).is_ok());
        assert!(serde_json::from_str::<crate::config::AppConfig>(r#"{"timeline_alt_wheel_action":"invalid"}"#).is_err());
        assert!(config.timeline_header_wheel_vertical_scroll);
        assert_eq!(config.timeline_plain_wheel_action, Wheel::Zoom);
        assert_eq!(config.timeline_ctrl_wheel_action, Wheel::VerticalScroll);
        assert_eq!(config.timeline_shift_wheel_action, Wheel::HorizontalScroll);
        assert_eq!(config.timeline_alt_wheel_action, Wheel::Zoom);
        assert!(config.timeline_middle_button_pan);
        assert!(config.timeline_middle_axis_lock_modifiers);
        assert!(config.timeline_animated_navigation);
        assert_eq!(config.timeline_navigation_transition_ms, 220);

        let controls = preference_controls(&config);
        for key in [
            "timeline_header_wheel_vertical_scroll",
            "timeline_middle_button_pan",
            "timeline_middle_axis_lock_modifiers",
            "timeline_animated_navigation",
        ] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .unwrap_or_else(|| panic!("missing {key} preference"));
            assert!(matches!(control.kind, PreferenceControlKind::Boolean));
            assert_eq!(control.section, "input");
            assert_eq!(control.group, "Timeline navigation");
        }

        for key in ["timeline_plain_wheel_action", "timeline_ctrl_wheel_action", "timeline_shift_wheel_action", "timeline_alt_wheel_action"] {
            let control = controls.iter().find(|control| control.key == key).unwrap();
            assert!(matches!(control.kind, PreferenceControlKind::Select));
            assert_eq!(control.options.len(), 4);
            assert_eq!(control.group, "Timeline navigation");
            assert_eq!(control.section, "input");
        }

        let transition = controls
            .iter()
            .find(|control| control.key == "timeline_navigation_transition_ms")
            .expect("missing timeline navigation transition preference");
        assert!(matches!(transition.kind, PreferenceControlKind::Number));
        assert_eq!(transition.section, "input");
        assert_eq!(transition.group, "Timeline navigation");
        assert_eq!(transition.minimum, Some(50.0));
        assert_eq!(transition.maximum, Some(2_000.0));
        assert_eq!(transition.step, Some(10.0));
    }

    #[test]
    fn keyboard_and_global_hotkey_policies_are_shared_preferences() {
        let config = crate::config::AppConfig::default();
        assert!(config.keyboard_shortcuts_enabled);
        assert!(config.media_keys_enabled);
        assert!(config.global_hardware_hotkeys_enabled);

        let controls = preference_controls(&config);
        for key in [
            "keyboard_shortcuts_enabled",
            "media_keys_enabled",
            "global_hardware_hotkeys_enabled",
        ] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .unwrap_or_else(|| panic!("missing {key} preference"));
            assert!(matches!(control.kind, PreferenceControlKind::Boolean));
            assert_eq!(control.section, "input");
            assert_eq!(control.group, "Keyboard shortcuts");
        }
    }

    #[test]
    fn media_key_policy_defaults_to_enabled_and_is_persistent_and_api_configurable() {
        let old: crate::config::AppConfig = serde_json::from_str("{}").unwrap();
        assert!(old.media_keys_enabled);
        let disabled = old.apply_patch(&serde_json::json!({"media_keys_enabled": false})).unwrap();
        assert!(!disabled.media_keys_enabled);
        // OS media controls do not depend on local shortcuts or hardware hotkeys.
        assert!(disabled.keyboard_shortcuts_enabled);
        assert!(disabled.global_hardware_hotkeys_enabled);
        let restored: crate::config::AppConfig = serde_json::from_str(
            &serde_json::to_string(&disabled).unwrap(),
        ).unwrap();
        assert!(!restored.media_keys_enabled);
        let contract = preferences_contract(&restored);
        assert_eq!(contract.defaults["media_keys_enabled"], true);
        assert_eq!(contract.values["media_keys_enabled"], false);
        assert!(old.apply_patch(&serde_json::json!({"media_keys_enabled": "off"})).is_err());
        assert!(disabled.apply_patch(&serde_json::json!({"media_keys_enabled": true})).unwrap().media_keys_enabled);
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

    #[test]
    fn human_readable_time_units_are_enabled_by_default_and_shared() {
        let config = crate::config::AppConfig::default();
        assert!(config.human_readable_time_units);
        assert!(
            serde_json::from_str::<crate::config::AppConfig>("{}")
                .unwrap()
                .human_readable_time_units
        );

        let controls = preference_controls(&config);
        let control = controls
            .iter()
            .find(|control| control.key == "human_readable_time_units")
            .expect("missing time-unit preference");
        assert!(matches!(control.kind, PreferenceControlKind::Boolean));
        assert_eq!(control.section, "input");
        assert_eq!(control.group, "Numeric input");

        let patched = config
            .apply_patch(&serde_json::json!({"human_readable_time_units": false}))
            .unwrap();
        assert!(!patched.human_readable_time_units);
    }

    #[test]
    fn every_video_surface_mouse_button_action_is_shared_and_persistent() {
        let config = crate::config::AppConfig::default();
        assert_eq!(
            config.middle_click_action,
            crate::config::PlayerClickAction::None
        );
        assert_eq!(
            config.middle_hold_action,
            crate::config::PlayerDragAction::None
        );
        assert_eq!(
            config.right_click_action,
            crate::config::PlayerClickAction::ContextMenu
        );
        assert_eq!(
            config.right_hold_action,
            crate::config::PlayerDragAction::None
        );

        let controls = preference_controls(&config);
        for key in [
            "paused_drag_action",
            "playing_drag_action",
            "middle_click_action",
            "middle_hold_action",
            "right_click_action",
            "right_hold_action",
        ] {
            let control = controls
                .iter()
                .find(|control| control.key == key)
                .unwrap_or_else(|| panic!("missing {key} preference"));
            assert!(matches!(control.kind, PreferenceControlKind::Select));
            assert_eq!(control.section, "input");
            assert_eq!(control.group, "Video surface");
        }

        let right_click = controls
            .iter()
            .find(|control| control.key == "right_click_action")
            .expect("right-button click preference");
        assert!(right_click.options.iter().any(|option| {
            option.value == serde_json::Value::String("context_menu".to_string())
        }));
        let middle_click = controls
            .iter()
            .find(|control| control.key == "middle_click_action")
            .expect("middle-button click preference");
        assert!(middle_click.options.iter().any(|option| {
            option.value == serde_json::Value::String("context_menu".to_string())
        }));
    }
}
