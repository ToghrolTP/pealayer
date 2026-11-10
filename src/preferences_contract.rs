use serde::Serialize;

/// Process-local navigation only; never saves or changes a preference value.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreferencesNavigation {
    #[serde(default)]
    pub section: Option<String>,
    pub scroll_y: f32,
}

impl PreferencesNavigation {
    pub(crate) fn timeline_toolbar() -> Self {
        Self { section: Some("timeline_toolbar".into()), scroll_y: 0.0 }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.scroll_y.is_finite() || !(0.0..=1_000_000.0).contains(&self.scroll_y) {
            return Err("scroll_y must be a finite offset between 0 and 1000000 logical pixels".into());
        }
        if self.section.as_ref().is_some_and(|id| !preference_sections().iter().any(|s| s.id == id)) {
            return Err("Unknown Preferences section; use an ID from /api/preferences".into());
        }
        Ok(())
    }
}

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
    ActionList,
    Accent,
    Color,
    Boolean,
    File,
    Number,
    MultiSelect,
    ReplacementList,
    Select,
    ChoiceSlider,
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
    /// Semantic Phosphor name shared by native and Web renderers.
    pub icon: &'static str,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub file_extensions: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceGroup {
    pub section: &'static str,
    pub name: &'static str,
    pub collapsible: bool,
    pub default_open: bool,
    pub configured_count: usize,
}

pub fn preference_groups(controls: &[PreferenceControl], values: &serde_json::Value) -> Vec<PreferenceGroup> {
    let mut groups: Vec<PreferenceGroup> = Vec::new();
    for control in controls {
        if groups.iter().any(|group| group.section == control.section && group.name == control.group) {
            continue;
        }
        let members = controls.iter().filter(|candidate| candidate.section == control.section && candidate.group == control.group);
        let collapsible = members.clone().all(|control| matches!(control.kind, PreferenceControlKind::File));
        let configured_count = members.filter(|control| value_at_path(values, control.key)
            .and_then(serde_json::Value::as_str).is_some_and(|value| !value.trim().is_empty())).count();
        groups.push(PreferenceGroup { section: control.section, name: control.group,
            collapsible, default_open: !collapsible || configured_count > 0, configured_count });
    }
    // Optional appearance overrides follow the common appearance settings;
    // configuration-file operations close Advanced on every surface.
    groups.sort_by_key(|group| matches!((group.section, group.name),
        ("appearance", "Application icons") | ("advanced", "Config file")));
    groups
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceSection {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub navigation_group: &'static str,
}

#[cfg(test)]
#[test]
fn timeline_toolbar_page_has_one_canonical_visibility_control() {
    let controls = preference_controls(&crate::config::AppConfig::default());
    let matches: Vec<_> = controls.iter().filter(|control| control.key == "timeline_toolbar_visible").collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].section, "timeline_toolbar");
    assert_eq!(matches[0].group, "Timeline toolbar");
    // Workspace snapshots store these selections by index.
    assert_eq!(preference_sections().iter().take(6).map(|section| section.id).collect::<Vec<_>>(),
        ["appearance", "playback", "hardware", "input", "web", "advanced"]);
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferencesContract {
    pub format: &'static str,
    pub sections: Vec<PreferenceSection>,
    pub controls: Vec<PreferenceControl>,
    pub groups: Vec<PreferenceGroup>,
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
            icon: "check-square",
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
            file_extensions: Vec::new(),
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

    fn choice_slider(
        key: &'static str,
        section: &'static str,
        group: &'static str,
        label: &'static str,
        options: &[(&'static str, &'static str)],
    ) -> Self {
        let mut control = Self::select(key, section, group, label, options);
        control.kind = PreferenceControlKind::ChoiceSlider;
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
            navigation_group: "Personalization",
            label: "Appearance",
            icon: "sparkle",
        },
        PreferenceSection {
            id: "playback",
            navigation_group: "Experience",
            label: "Playback",
            icon: "play",
        },
        PreferenceSection {
            id: "hardware",
            navigation_group: "Connections",
            label: "Hardware",
            icon: "plug",
        },
        PreferenceSection {
            id: "input",
            navigation_group: "Connections",
            label: "Input",
            icon: "sliders",
        },
        PreferenceSection {
            id: "web",
            navigation_group: "Connections",
            label: "Web UI",
            icon: "globe",
        },
        PreferenceSection {
            id: "advanced",
            navigation_group: "Application",
            label: "Advanced",
            icon: "gear",
        },
        PreferenceSection {
            id: "timeline_toolbar",
            navigation_group: "Application",
            label: "Timeline toolbar",
            icon: "sliders",
        },
        PreferenceSection { id: "sidebar_rail", navigation_group: "Application", label: "Sidebar rail", icon: "sliders" },
        #[cfg(any(feature = "workspace-toolbar", feature = "workspace-terminal"))]
        PreferenceSection { id: "workspace_tools", navigation_group: "Application", label: "Workspace tools", icon: "terminal-window" },
        #[cfg(feature = "workspace-terminal")]
        PreferenceSection { id: "terminal", navigation_group: "Application", label: "Terminal", icon: "terminal-window" },
    ]
}

fn application_icon_control(key: &'static str, label: &'static str, placeholder: &'static str) -> PreferenceControl {
    let mut control = PreferenceControl::text(key, "appearance", "Application icons", label, placeholder);
    control.kind = PreferenceControlKind::File;
    control.file_extensions = vec!["png", "jpg", "jpeg", "webp", "ico"];
    control
}

fn seekbar_color_control(key: &'static str, label: &'static str) -> PreferenceControl {
    let mut control = PreferenceControl::text(key, "playback", "Seekbar markers", label, "#RRGGBB");
    control.kind = PreferenceControlKind::Color;
    control
}

fn designer_color_control(key: &'static str, label: &'static str) -> PreferenceControl {
    let mut control = PreferenceControl::text(key, "hardware", "Effect designer colors", label, "#RRGGBB");
    control.kind = PreferenceControlKind::Color;
    control
}

/// Optional surfaces advertise only controls their compiled host can use.
/// Their serialized values remain in the config even when the feature is omitted.
fn workspace_tool_controls(config: &crate::config::AppConfig) -> Vec<PreferenceControl> {
    let mut controls = Vec::new();
    #[cfg(feature = "workspace-toolbar")]
    {
        let mut enabled = PreferenceControl::boolean("ui.toolbar.enabled", "workspace_tools", "Command toolbar", "Show command toolbar");
        enabled.description = Some("Open the command palette with Ctrl+Shift+P even when the toolbar is hidden.");
        controls.push(enabled);
        let mut actions = PreferenceControl::boolean("ui.toolbar.order", "workspace_tools", "Command toolbar", "Toolbar actions");
        actions.kind = PreferenceControlKind::ActionList;
        actions.custom_key = Some("ui.toolbar.hidden");
        actions.options = crate::ui::workspace_toolbar::command_options().into_iter().map(|(id, label, icon)| PreferenceOption {
            value: serde_json::json!(id), label, icon: Some(icon), color: None, description: None,
        }).collect();
        controls.push(actions);
    }
    #[cfg(feature = "workspace-terminal")]
    {
        let start = controls.len();
        let mut enabled = PreferenceControl::boolean("ui.terminal.enabled", "workspace_tools", "Terminal", "Show terminal panel");
        enabled.description = Some("Showing the panel does not start a shell or connect to a host. Choose Connect in the terminal when ready.");
        controls.push(enabled);
        let mut target = PreferenceControl::select("ui.terminal.target", "workspace_tools", "Terminal", "Default session", &[
            ("pc_controller", "PCController"), ("system_shell", "System shell"), ("portable_shell", "Pealayer Shell"),
        ]);
        for (option, (icon, description)) in target.options.iter_mut().zip([
            ("plug", "Connect to the selected PCController host's interactive terminal."),
            ("terminal-window", "Run a shell on this Pealayer host, not on a remote playback authority."),
            ("terminal-window", "Embedded Pealayer command shell. No external executable is needed."),
        ]) {
            option.icon = Some(icon);
            option.description = Some(description.into());
        }
        controls.push(target);
        match config.ui.terminal.target {
            crate::ui::terminal::TerminalTarget::PcController => {
                let mut endpoint = PreferenceControl::text("ui.terminal.controller_endpoint", "workspace_tools", "Terminal connection", "PCController endpoint", "Use the selected PCController host");
                endpoint.description = Some("Optional pccontroller://host:port or tcp://host:port endpoint. Leave blank to use the selected controller.");
                controls.push(endpoint);
            }
            crate::ui::terminal::TerminalTarget::SystemShell => {
                let mut program = PreferenceControl::text("ui.terminal.shell_program", "workspace_tools", "Terminal connection", "Shell executable", "System default shell");
                program.kind = PreferenceControlKind::File;
                #[cfg(windows)] { program.file_extensions = vec!["exe"]; }
                controls.push(program);
                let mut arguments = PreferenceControl::text("ui.terminal.shell_arguments", "workspace_tools", "Terminal connection", "Shell arguments", "Optional arguments");
                arguments.description = Some("Separate arguments with spaces; quote an argument containing spaces. Applies to newly connected sessions.");
                controls.push(arguments);
            }
            crate::ui::terminal::TerminalTarget::PortableShell => {}
        }
        controls.extend([
            PreferenceControl::number("ui.terminal.font_size", "workspace_tools", "Terminal appearance", "Font size", 9.0, 28.0, 0.5),
            PreferenceControl::number("ui.terminal.scrollback_lines", "workspace_tools", "Terminal appearance", "Scrollback lines", 100.0, 10000.0, 100.0),
        ]);
        controls.extend([
            PreferenceControl::boolean("ui.terminal.cursor_blink", "terminal", "Terminal appearance", "Blink cursor"),
            PreferenceControl::select("ui.terminal.cursor_style", "terminal", "Terminal appearance", "Cursor style", &[("bar", "Bar"), ("block", "Block"), ("underline", "Underline")]),
            PreferenceControl::boolean("ui.terminal.accent_commands", "terminal", "Terminal appearance", "Use app accent for commands"),
            PreferenceControl::boolean("ui.terminal.allow_system_shell", "terminal", "Security", "Allow local system shells"),
            PreferenceControl::boolean("ui.terminal.allow_remote", "terminal", "Security", "Allow paired remote terminal access"),
            PreferenceControl::boolean("ui.terminal.allow_remote_system_shell", "terminal", "Security", "Allow paired remote access to system shells"),
        ]);
        for control in &mut controls[start..] { control.section = "terminal"; }
        for control in controls[start..].iter_mut().filter(|control|control.group=="Security") {
            control.description=Some("Only the native host can change terminal security. Remote access also requires a native-issued, expiring access key. Use a trusted encrypted connection outside localhost.");
        }
    }
    // Avoid feature-off warnings without changing or discarding persisted settings.
    let _ = config;
    controls
}

pub fn preference_controls(config: &crate::config::AppConfig) -> Vec<PreferenceControl> {
    let audio_devices = crate::peer::client().and_then(|client| client.snapshot()).and_then(|snapshot| serde_json::from_value::<Vec<crate::mpv::audio_output::AudioDevice>>(snapshot.session.status.get("audio_devices")?.clone()).ok()).unwrap_or_else(crate::mpv::audio_output::available_audio_devices);
    let audio_output_control = |key, label, selected: &str, follow_media: bool| {
        let mut control = PreferenceControl::select(key, "playback", "Audio output", label, &[]);
        control.description = Some(if follow_media {
            "Select a separate output for sound effects, or follow the media output"
        } else {
            "Select a system audio device and backend; System default is recommended"
        });
        if follow_media {
            control.options.push(PreferenceOption {
                value: serde_json::json!(""),
                label: "Same as media output",
                color: None,
                description: None,
                icon: None,
            });
        }
        control.options.extend(audio_devices.iter().map(|device| PreferenceOption {
            value: serde_json::Value::String(device.name.clone()),
            label: "Audio device",
            color: None,
            description: Some(device.description.clone()),
            icon: Some("speaker-high"),
        }));
        if !selected.is_empty() && !control.options.iter().any(|option| option.value.as_str() == Some(selected)) {
            control.options.push(PreferenceOption {
                value: serde_json::Value::String(selected.to_string()),
                label: "Unavailable audio device",
                color: None,
                description: Some(format!("Unavailable: {selected}")),
                icon: None,
            });
        }
        control
    };
    let mut controls = vec![
        PreferenceControl::boolean("simple_workspace.show_menu_bar", "playback", "Simple workspace", "Show menu bar in Simple workspace"),
        PreferenceControl::boolean("simple_workspace.show_toolbar", "playback", "Simple workspace", "Show command toolbar in Simple workspace"),
        PreferenceControl::boolean("simple_workspace.show_status_bar", "playback", "Simple workspace", "Show status bar in Simple workspace"),
        PreferenceControl::select("simple_workspace.backdrop", "playback", "Simple workspace", "Control bar background", &[("auto","Automatic GPU blur"),("blur","GPU blur"),("solid","Solid background")]),
        PreferenceControl::number("simple_workspace.controls_x", "playback", "Simple workspace", "Control bar horizontal position", 0.0, 1.0, 0.01),
        PreferenceControl::number("simple_workspace.controls_y", "playback", "Simple workspace", "Control bar vertical position", 0.0, 1.0, 0.01),
        PreferenceControl::boolean("ui.sidebar_rail.enabled", "sidebar_rail", "Sidebar rail", "Show sidebar rail"),
        PreferenceControl::number("ui.sidebar_rail.width", "sidebar_rail", "Sidebar rail", "Sidebar pane width", 240.0, 600.0, 10.0),
        {
            let mut control = PreferenceControl::boolean("ui.sidebar_shortcuts.order", "sidebar_rail", "Shortcuts", "Shortcuts");
            control.kind = PreferenceControlKind::ActionList;
            control.custom_key = Some("ui.sidebar_shortcuts.hidden");
            control.options = crate::ui::sidebar_shortcuts::RailAction::ALL.into_iter().filter(|action| action.included()).map(|action| PreferenceOption {
                value: serde_json::to_value(action).unwrap(), label: action.label(), color: None,
                description: None, icon: Some(action.icon()),
            }).collect();
            control
        },
        {
            let mut c = PreferenceControl::select("external_mpv.mode", "playback", "External mpv", "Player mode", &[("internal", "Internal player"), ("external", "External mpv only"), ("dual", "Both players"), ("remote", "Control existing mpv")]);
            for (option, (icon, description)) in c.options.iter_mut().zip([
                ("monitor-play", "Play inside Pealayer. No external window."),
                ("arrow-square-out", "Play in a separate mpv window. Closing it stops playback."),
                ("squares-four", "External playback with a muted internal preview. Falls back to internal when mpv closes."),
                ("plugs-connected", "Attach to an existing IPC endpoint. Pealayer does not own or close that process."),
            ]) {
                option.icon = Some(icon); option.description = Some(description.into());
            }
            c
        },
        PreferenceControl::boolean("external_mpv.use_mpv_config", "playback", "External mpv", "Load mpv's own configuration and scripts when launching"),
        {
            let mut c = PreferenceControl::text("external_mpv.executable", "playback", "External mpv", "mpv executable", "mpv or full executable path");
            c.kind = PreferenceControlKind::File;
            #[cfg(windows)] {c.file_extensions = vec!["exe"];}
            c
        },
        {
            let mut c = PreferenceControl::text("external_mpv.endpoint", "playback", "External mpv", "IPC endpoint", "Blank to launch; named pipe / Unix socket to attach");
            c.description = Some("For an existing player, start mpv with --input-ipc-server at this endpoint. No Lua helper is required. Dual mode uses a muted internal preview.");
            c
        },
        PreferenceControl::select(
            "theme",
            "appearance",
            "Interface",
            "Theme",
            &[("system", "System"), ("light", "Light"), ("dark", "Dark")],
        ),
        PreferenceControl::select(
            "color_palette", "appearance", "Interface", "Color palette",
            &[("native", "Neutral (default)"), ("studio", "Studio")],
        ),
        PreferenceControl::accent(config),
        PreferenceControl::select("local_media_title_format", "appearance", "Window title", "Local media title",
            &[("file_name", "Filename"), ("file_stem", "Filename without extension"), ("location", "Full location")]),
        PreferenceControl::select("remote_media_title_format", "appearance", "Window title", "Remote media title",
            &[("file_name", "Filename"), ("file_stem", "Filename without extension"), ("location", "Full location")]),
        PreferenceControl::select("workspace_ui", "appearance", "Interface", "Workspace interface",
            &[("craft_ui", "CraftUI"), ("egui_dock", "egui Dock")]),
        PreferenceControl::select("native_monitor_style", "appearance", "Workspace panels", "Program Monitor controls",
            &[("compact", "Compact"), ("film_craft", "FilmCraft")]),
        PreferenceControl::select("native_transport_style", "appearance", "Workspace panels", "Simple transport controls",
            &[("compact", "Compact"), ("film_craft", "FilmCraft")]),
        PreferenceControl::select("monitor_preview.aspect", "appearance", "Program Monitor preview", "Preview aspect ratio",
            &[("source", "Fit source"), ("wide", "16:9"), ("classic", "4:3"), ("cinema", "2.35:1")]),
        PreferenceControl::select("monitor_preview.resolution", "appearance", "Program Monitor preview", "Preview render resolution",
            &[("full", "Full"), ("half", "1/2"), ("quarter", "1/4"), ("eighth", "1/8")]),
        PreferenceControl::select("native_timeline_style", "appearance", "Workspace panels", "Timeline ruler",
            &[("standard", "Standard"), ("film_craft", "FilmCraft")]),
        PreferenceControl::boolean("craft_show_close_buttons", "appearance", "Workspace panels", "Show CraftUI panel close buttons"),
        PreferenceControl::boolean("craft_show_collapse_buttons", "appearance", "Workspace panels", "Show CraftUI collapse buttons"),
        PreferenceControl::boolean("preferences_grouped_navigation", "appearance", "Interface", "Group Preferences navigation"),
        PreferenceControl::choice_slider("ui_control_density", "appearance", "Interface", "Control density",
            &[("extra_compact", "Extra compact"), ("compact", "Compact"), ("regular", "Regular"),
              ("comfortable", "Comfortable"), ("spacious", "Spacious")]),
        PreferenceControl::select("ui_font_family", "appearance", "Typography", "UI font",
            &[("system", "System font"), ("bundled", "Bundled font"), ("vazirmatn", "Vazirmatn"), ("custom", "Choose installed font")]),
        PreferenceControl::select("monospace_font_family", "appearance", "Typography", "Monospace font",
            &[("system", "System font"), ("bundled", "Bundled font"), ("custom", "Choose installed font")]),
        PreferenceControl::select("ui_animation_mode", "appearance", "Interface", "Animations and transitions",
            &[("system", "System default"), ("enabled", "Enabled"), ("disabled", "Disabled")]),
        application_icon_control("app_icon", "Default icon", "Bundled application icon"),
        PreferenceControl::select(
            "app_icon_preset", "appearance", "Interface", "Application icon",
            &[("current", "Current"), ("classic", "Classic (previous)")],
        ),
        application_icon_control("app_icon_playing", "Playing", "Use default icon"),
        application_icon_control("app_icon_paused", "Paused", "Use default icon"),
        application_icon_control("app_icon_stopped", "Stopped", "Use default icon"),
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
        audio_output_control("audio_device", "Media output device / backend", &config.audio_device, false),
        audio_output_control("sfx_audio_device", "SFX output device / backend", &config.sfx_audio_device, true),
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
                "Show thumbnails in Simple seek preview",
            );
            control.description =
                Some("Adds a decoded frame to the shared cursor-position and chapter hover card");
            control
        },
        {
            let mut control = PreferenceControl::boolean(
                "nle_seekbar_hover_thumbnails",
                "playback",
                "Seek preview",
                "Show thumbnails in NLE seek preview",
            );
            control.description =
                Some("Adds a decoded frame to the shared cursor-position and chapter hover card");
            control
        },
        seekbar_color_control("seekbar_markers.chapter_color", "Chapter markers"),
        seekbar_color_control("seekbar_markers.active_chapter_color", "Active chapter background"),
        seekbar_color_control("seekbar_markers.keyframe_color", "Keyframe markers"),
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
            "timeline_subtitle_segments",
            "playback",
            "Subtitles",
            "Show subtitle cue segments in the timeline",
        ),
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
        PreferenceControl::boolean("clipboard_url_detection", "playback", "Open Location / URL", "Inspect newly copied HTTP(S) links"),
        PreferenceControl::select("clipboard_link_behavior", "playback", "Open Location / URL", "Copied-link verification", &[
            ("verify_before_dialog", "Verify support before opening the dialog"),
            ("show_while_verifying", "Show the dialog while verifying support"),
            ("show_without_verification", "Show the dialog without verification"),
        ]),
        PreferenceControl::boolean("open_url_auto_proxy", "playback", "Open Location / URL", "Automatically choose a working proxy or direct connection"),
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
        PreferenceControl::select("remote_folder_view", "playback", "Remote folders", "Remote folder layout", &[("list", "List"), ("grid", "Grid")]),
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
        {
            let mut control = PreferenceControl::boolean(
                "allow_unattended_hardware_takeover", "hardware", "Publishing authority",
                "Allow unattended publishing handoffs",
            );
            control.description = Some("Automatically pause this publisher and accept an incoming handoff. Production lock always prevents takeover. Enable only with trusted controller access.");
            control
        },
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
        designer_color_control("effect_designer_cue_colors.motion", "Seat motion"),
        designer_color_control("effect_designer_cue_colors.relay", "Relay"),
        designer_color_control("effect_designer_cue_colors.pwm", "PWM"),
        designer_color_control("effect_designer_cue_colors.beep", "Buzzer"),
        designer_color_control("effect_designer_cue_colors.rgb", "Status RGB"),
        designer_color_control("effect_designer_cue_colors.addressable", "Strip pixel"),
        designer_color_control("effect_designer_cue_colors.rf", "RF"),
        designer_color_control("effect_designer_cue_colors.display", "Display"),
        designer_color_control("effect_designer_cue_colors.menu", "Front-panel action"),
        designer_color_control("effect_designer_cue_colors.opcode", "Raw opcode"),
        {
            let mut control = PreferenceControl::boolean(
                "timeline_hide_cue_text_overflow",
                "hardware",
                "Timeline",
                "Keep cue labels inside clips",
            );
            control.description = Some(
                "Truncate long cue labels with an ellipsis. Hover a cue to see its full details; disable this to allow labels to overflow.",
            );
            control
        },
        {
            let mut control = PreferenceControl::select(
                "timeline_cue_editor_presentation",
                "hardware",
                "Timeline",
                "Open cue properties in",
                &[
                    ("dialog", "Modal dialog"),
                    ("panel", "Effect Controls panel"),
                ],
            );
            control.description = Some(
                "Choose how newly inserted and double-clicked cues are edited. Discarding a new modal cue removes it; panel mode keeps it immediately.",
            );
            control
        },
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
        PreferenceControl::boolean(
            "timeline_follow_playhead",
            "input",
            "Timeline navigation",
            "Keep the playhead in view during playback",
        ),
        PreferenceControl::boolean(
            "timeline_toolbar_visible",
            "timeline_toolbar",
            "Timeline toolbar",
            "Show timeline toolbar",
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
            control.description = Some("Play, pause, stop, next/previous and seek through the OS media session. Independent of in-app shortcuts and hardware hotkeys.");
            control
        },
        {
            let mut control=PreferenceControl::boolean("media_keys_background_enabled","input","Keyboard shortcuts",
                "Allow media controls while Pealayer is in the background");
            control.description=Some("Keep keyboard media buttons and OS transport controls working when another window has focus, including when Pealayer is minimized. Requires Respond to keyboard media controls; does not override the OS choice of media player.");
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
        PreferenceControl::boolean("pwm_card.show_on", "hardware", "MOSFET cards", "Show On button"),
        PreferenceControl::boolean("hardware_number_shortcuts_enabled", "input", "Hardware shortcuts", "R / P then digit toggles channel (0 = 10)"),
        PreferenceControl::boolean("pwm_card.show_off", "hardware", "MOSFET cards", "Show Off button"),
        PreferenceControl::boolean("pwm_card.show_toggle", "hardware", "MOSFET cards", "Show Toggle button"),
        PreferenceControl::boolean("pwm_card.show_step_buttons", "hardware", "MOSFET cards", "Show slider step buttons"),
        PreferenceControl::boolean("pwm_card.show_decrease", "hardware", "MOSFET cards", "Show Decrease button"),
        PreferenceControl::boolean("pwm_card.show_increase", "hardware", "MOSFET cards", "Show Increase button"),
        PreferenceControl::boolean("pwm_card.show_other", "hardware", "MOSFET cards", "Show other action buttons"),
        PreferenceControl::select("pwm_card.indicator_action", "hardware", "MOSFET cards", "Indicator click action",
            &[("toggle", "Toggle"), ("on", "On"), ("off", "Off")]),
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
        PreferenceControl::boolean(
            "video_click_frame_only",
            "input",
            "Video surface",
            "Apply video gestures only inside the picture",
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
                ("boss_key", "Boss key: pause and minimize"),
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
                ("boss_key", "Boss key: pause and minimize"),
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
        PreferenceControl::boolean("binary_udp.enabled", "web", "Binary UDP", "Enable binary UDP"),
        PreferenceControl::text("binary_udp.listen", "web", "Binary UDP", "UDP listening address", "127.0.0.1:8080"),
        PreferenceControl::boolean("binary_udp.discovery", "web", "Binary UDP", "Discover and announce peers"),
        PreferenceControl::boolean("binary_udp.broadcast_events", "web", "Binary UDP", "Broadcast state events"),
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
            "Show frame rate",
        ),
        PreferenceControl::select(
            "status_bar.fps_mode", "advanced", "Status bar", "Frame rate source",
            &[("media", "Media frame rate"), ("ui", "Screen render rate"), ("auto", "Auto")],
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
        PreferenceControl::boolean("status_bar.chart", "advanced", "Status bar", "Telemetry log chart"),
        PreferenceControl::boolean("status_bar.progress", "advanced", "Status bar", "Operation progress"),
        PreferenceControl::boolean("status_bar.hot", "advanced", "Status bar", "HOT alerts"),
        PreferenceControl::boolean("status_bar.dividers", "advanced", "Status bar", "Show dividers"),
        PreferenceControl::boolean("status_bar.semantic_colors", "advanced", "Status bar", "Semantic colors"),
    ];
    controls.extend(workspace_tool_controls(config));
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
    for (key, label, choice, selected) in [
        ("ui_font_name", "Installed UI font", config.ui_font_family, &config.ui_font_name),
        ("monospace_font_name", "Installed monospace font", config.monospace_font_family, &config.monospace_font_name),
    ] {
        if choice != crate::config::UiFontFamily::Custom { continue; }
        let mut control = PreferenceControl::select(key, "appearance", "Typography", label, &[]);
        control.description = Some("Fonts installed on the application host. Unavailable families fall back to the system font.");
        let mut names = crate::ui::fonts::family_names();
        if !selected.is_empty() && !names.contains(selected) { names.insert(0, selected.clone()); }
        control.options = names.into_iter().map(|name| PreferenceOption {
            value: serde_json::json!(name), label: "Installed font", description: Some(name), color: None, icon: Some("text-t"),
        }).collect();
        let after = if key == "ui_font_name" { "ui_font_family" } else { "monospace_font_family" };
        let index = controls.iter().position(|control| control.key == after).unwrap() + 1;
        controls.insert(index, control);
    }
    controls.extend(mouse_binding_controls());
    let mut hints = std::collections::HashSet::new();
    for control in &mut controls {
        control.icon = semantic_preference_icon(control);
        if let Some(description) = control.description
            && !hints.insert((control.section, control.group, description)) {
            control.description = None;
        }
    }
    controls
}

fn mouse_binding_controls()->Vec<PreferenceControl> {
    const VIDEO:&[(&str,&str)]=&[("none","Do nothing"),("inherit","Use unmodified action"),
        ("seek_backward","Seek backward"),("seek_forward","Seek forward"),
        ("previous_chapter","Previous chapter"),("next_chapter","Next chapter"),
        ("previous_file","Previous file"),("next_file","Next file"),
        ("frame_backward","Previous frame"),("frame_forward","Next frame"),
        ("play_pause","Play / pause"),("toggle_mute","Mute / unmute"),
        ("toggle_fullscreen","Toggle fullscreen"),("boss_key","Boss key: pause and minimize")];
    const CANVAS:&[(&str,&str)]=&[("none","Do nothing"),("inherit","Use unmodified action"),
        ("pan_left","Pan left"),("pan_right","Pan right"),("zoom_out","Zoom out"),("zoom_in","Zoom in")];
    const BROWSER:&[(&str,&str)]=&[("none","Do nothing"),("inherit","Use unmodified action"),
        ("history_back","Back"),("history_forward","Forward"),("parent_folder","Parent folder")];
    let mut result=Vec::new();
    macro_rules! scope {($name:literal,$group:literal,$options:expr)=>{
        for (key,label,modified) in [
            (concat!("mouse_bindings.",$name,".back"),"Back button (4)",false),
            (concat!("mouse_bindings.",$name,".forward"),"Forward button (5)",false),
            (concat!("mouse_bindings.",$name,".ctrl_back"),"Ctrl / Command + Back",true),
            (concat!("mouse_bindings.",$name,".ctrl_forward"),"Ctrl / Command + Forward",true),
            (concat!("mouse_bindings.",$name,".alt_back"),"Alt + Back",true),
            (concat!("mouse_bindings.",$name,".alt_forward"),"Alt + Forward",true),
            (concat!("mouse_bindings.",$name,".shift_back"),"Shift + Back",true),
            (concat!("mouse_bindings.",$name,".shift_forward"),"Shift + Forward",true),
        ] {
            let mut control=PreferenceControl::select(key,"input",$group,label,$options);
            if !modified {control.options.retain(|o|o.value!="inherit");}
            control.description=Some("Only the surface where the press starts owns it. Combined modifiers use Ctrl / Command, then Alt, then Shift. Actions are captured on press; dragging or cancelling never invokes a click.");
            result.push(control);
        }
    }}
    scope!("video","Video: Back / Forward buttons",VIDEO);
    scope!("timeline","Timeline: Back / Forward buttons",CANVAS);
    scope!("effect_designer","Effect Designer: Back / Forward buttons",CANVAS);
    scope!("remote_browser","Remote browser: Back / Forward buttons",BROWSER);
    for (key,label) in [("mouse_bindings.ctrl_left_drag","Ctrl / Command + left drag"),("mouse_bindings.alt_middle_drag","Alt + middle drag")] {
        let mut control=PreferenceControl::select(key,"input","Video modifiers",label,&[("move_window","Move application window"),("seek","Seek video"),("temporary_fast_forward","Temporarily fast-forward"),("none","Do nothing")]);
        control.description=Some("Modifier-drag mode does not also run the button's click action. Video gestures honor the picture-only setting.");
        result.push(control);
    }
    result.push(PreferenceControl::boolean("mouse_bindings.shift_right_context_menu","input","Video modifiers","Shift + right button always opens the context menu"));
    result
}

fn semantic_preference_icon(control: &PreferenceControl) -> &'static str {
    match control.key {
        "ui.toolbar.enabled" => "sliders-horizontal",
        "ui.toolbar.order" => "list-checks",
        "ui.terminal.enabled" | "ui.terminal.target" => "terminal-window",
        "ui.terminal.controller_endpoint" => "plug",
        "ui.terminal.shell_program" => "terminal-window",
        "ui.terminal.shell_arguments" => "keyboard",
        "ui.terminal.font_size" => "text-t",
        "ui.terminal.scrollback_lines" => "text-align-left",
        "status_bar.fps_mode" => "gauge",
        "status_bar.media_rate" => "film-strip",
        "theme" => "circle-half",
        "accent_color" | "accent" => "palette",
        "color_palette" => "swatches",
        "ui_animation_mode" => "waveform",
        "ui_control_density" => "sliders-horizontal",
        "workspace_ui" => "tabs",
        "native_monitor_style" => "monitor-play",
        "native_transport_style" => "play",
        "monitor_preview.aspect" => "frame-corners",
        "monitor_preview.resolution" => "monitor",
        "native_timeline_style" => "ruler",
        "craft_show_close_buttons" => "x",
        "craft_show_collapse_buttons" => "caret-down",
        "preferences_grouped_navigation" => "list-checks",
        "ui_font_family" | "monospace_font_family" | "ui_font_name" | "monospace_font_name" => "text-t",
        "language" => "translate",
        "fullscreen_video_background" => "frame-corners",
        "always_on_top" => "push-pin",
        "consistent_video_aspect_ratio" => "arrows-out",
        "osd_position" => "target",
        "osd_timeout_seconds" => "clock",
        "app_icon" => "image",
        "app_icon_preset" => "image",
        "app_icon_playing" => "play",
        "app_icon_paused" => "pause",
        "app_icon_stopped" => "stop-circle",
        "audio_device" | "sfx_audio_device" => "speaker-high",
        key if key.starts_with("subtitle_") => "subtitles",
        key if key.starts_with("timeline_") => "waveform",
        key if key.starts_with("web_") => "globe",
        key if key.starts_with("mouse_bindings.") => "mouse",
        key if key.starts_with("keyboard_") || key.starts_with("shortcut_") || key.starts_with("media_keys_") => "keyboard",
        _ => match control.kind {
            PreferenceControlKind::ActionList => "list-checks",
            PreferenceControlKind::Accent | PreferenceControlKind::Color => "palette",
            PreferenceControlKind::Boolean => "check-square",
            PreferenceControlKind::File => "image",
            PreferenceControlKind::Number | PreferenceControlKind::ChoiceSlider => "sliders-horizontal",
            PreferenceControlKind::MultiSelect => "globe",
            PreferenceControlKind::ReplacementList => "text-align-left",
            PreferenceControlKind::Select => "list-checks",
            PreferenceControlKind::Text => "pencil-simple",
        },
    }
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
        groups: preference_groups(&controls, &values),
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
    #[test]
    fn mouse_bindings_are_complete_in_shared_preferences_and_reject_cross_surface_actions() {
        let cfg=crate::config::AppConfig::default();let contract=super::preferences_contract(&cfg);
        let controls=contract.controls.iter().filter(|c|c.key.starts_with("mouse_bindings.")).collect::<Vec<_>>();
        assert_eq!(controls.len(),35);
        for control in controls {
            assert_eq!(control.section,"input");assert_eq!(control.icon,"mouse");
            let value=super::value_at_path(&contract.values,control.key).unwrap();
            if !control.options.is_empty(){assert!(control.options.iter().any(|o|&o.value==value),"{}",control.key);}
        }
        let changed=cfg.apply_patch(&serde_json::json!({"mouse_bindings":{"video":{"back":"boss_key"}},"middle_click_action":"boss_key"})).unwrap();
        assert_eq!(changed.mouse_bindings.video.back,crate::pointer_bindings::Action::BossKey);
        assert_eq!(changed.mouse_bindings.timeline,cfg.mouse_bindings.timeline);
        assert!(cfg.apply_patch(&serde_json::json!({"mouse_bindings":{"timeline":{"back":"boss_key"}}})).is_err());
    }
    #[test]
    fn media_title_choices_have_independent_shared_settings_and_filename_defaults() {
        let contract = super::preferences_contract(&crate::config::AppConfig::default());
        for key in ["local_media_title_format", "remote_media_title_format"] {
            let control = contract.controls.iter().find(|c| c.key == key).unwrap();
            assert_eq!(control.section, "appearance");
            assert_eq!(control.group, "Window title");
            assert_eq!(control.options.iter().map(|o| o.value.as_str().unwrap()).collect::<Vec<_>>(), ["file_name", "file_stem", "location"]);
            assert_eq!(super::value_at_path(&contract.defaults, key), Some(&serde_json::json!("file_name")));
        }
    }

    #[test]
    fn native_panel_styles_have_independent_api_controls_and_preserve_saved_layouts() {
        use crate::config::{AppConfig, ControlBarAction, WorkspaceUi};
        for workspace_ui in [WorkspaceUi::EguiDock, WorkspaceUi::CraftUi] {
            let mut config = AppConfig { workspace_ui, ..Default::default() }
                .apply_patch(&serde_json::json!({})).unwrap();
            config.ui.sidebar_rail.width = 396.0;
            config.workspace_session.name = "Authoring layout".into();
            config.workspace_session.dialogs.about = true;
            config.control_bars.monitor.order.swap(0, 3);
            config.control_bars.monitor.hidden[ControlBarAction::VideoTrack as usize] = false;
            let original = serde_json::to_value(&config).unwrap();
            let contract = super::preferences_contract(&config);
            for (key, default, icon) in [
                ("native_monitor_style", "compact", "monitor-play"),
                ("native_timeline_style", "standard", "ruler"),
            ] {
                let controls: Vec<_> = contract.controls.iter().filter(|row| row.key == key).collect();
                assert_eq!(controls.len(), 1, "one shared preference owner for {key}");
                let control = controls[0];
                assert!(matches!(control.kind, super::PreferenceControlKind::Select));
                assert_eq!((control.section, control.group, control.icon), ("appearance", "Workspace panels", icon));
                assert_eq!(control.options.iter().map(|option| option.value.as_str().unwrap()).collect::<Vec<_>>(), [default, "film_craft"]);
                assert_eq!(contract.defaults[key], default);
            }
            for monitor in ["compact", "film_craft"] {
                for timeline in ["standard", "film_craft"] {
                    let patched = config.apply_patch(&serde_json::json!({"native_monitor_style": monitor})).unwrap()
                        .apply_patch(&serde_json::json!({"native_timeline_style": timeline})).unwrap();
                    let restored: AppConfig = serde_json::from_value(serde_json::to_value(&patched).unwrap()).unwrap();
                    let mut expected = original.clone();
                    expected["native_monitor_style"] = serde_json::json!(monitor);
                    expected["native_timeline_style"] = serde_json::json!(timeline);
                    let api = super::preferences_contract(&restored);
                    assert_eq!(api.values, expected, "style-only API patches must preserve all other settings");
                }
            }
            for patch in [serde_json::json!({"native_monitor_style": "old"}), serde_json::json!({"native_timeline_style": "new"}), serde_json::json!({"native_monitor_style": false})] {
                assert!(config.apply_patch(&patch).is_err());
                assert_eq!(serde_json::to_value(&config).unwrap(), original);
            }
        }
    }

    #[test]
    fn workspace_tools_contract_preserves_defaults_and_respects_compiled_features() {
        let config = crate::config::AppConfig::default();
        let contract = super::preferences_contract(&config);
        assert_eq!(super::value_at_path(&contract.values, "ui.toolbar.enabled"), Some(&serde_json::json!(false)));
        assert_eq!(super::value_at_path(&contract.values, "ui.terminal.enabled"), Some(&serde_json::json!(false)));
        assert_eq!(contract.sections.iter().any(|section| section.id == "workspace_tools"),
            cfg!(any(feature = "workspace-toolbar", feature = "workspace-terminal")));
        assert_eq!(contract.controls.iter().any(|control| control.key == "ui.toolbar.enabled"), cfg!(feature = "workspace-toolbar"));
        assert_eq!(contract.controls.iter().any(|control| control.key == "ui.terminal.enabled"), cfg!(feature = "workspace-terminal"));
        for control in contract.controls.iter().filter(|control| control.section == "workspace_tools") {
            assert!(super::value_at_path(&contract.values, control.key).is_some(), "{} has no config owner", control.key);
            assert!(!control.icon.is_empty());
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn workspace_toolbar_action_list_uses_the_live_catalog_and_independent_visibility_key() {
        let contract = super::preferences_contract(&crate::config::AppConfig::default());
        let list = contract.controls.iter().find(|control| control.key == "ui.toolbar.order").unwrap();
        assert!(matches!(list.kind, super::PreferenceControlKind::ActionList));
        assert_eq!(list.custom_key, Some("ui.toolbar.hidden"));
        let catalog = crate::ui::workspace_toolbar::command_options();
        assert_eq!(list.options.len(), catalog.len());
        for (option, (id, label, icon)) in list.options.iter().zip(catalog) {
            assert_eq!(option.value, serde_json::json!(id));
            assert_eq!(option.label, label);
            assert_eq!(option.icon, Some(icon));
        }
    }

    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn terminal_preferences_show_the_selected_target_without_erasing_other_targets() {
        use crate::ui::terminal::TerminalTarget;
        let mut config = crate::config::AppConfig::default();
        config.ui.terminal.controller_endpoint = "pccontroller://example.invalid:8787".into();
        config.ui.terminal.shell_program = "shell-fixture".into();
        for (target, expected, absent) in [
            (TerminalTarget::PcController, "ui.terminal.controller_endpoint", "ui.terminal.shell_program"),
            (TerminalTarget::SystemShell, "ui.terminal.shell_program", "ui.terminal.controller_endpoint"),
            (TerminalTarget::PortableShell, "ui.terminal.target", "ui.terminal.shell_program"),
        ] {
            config.ui.terminal.target = target;
            let contract = super::preferences_contract(&config);
            assert!(contract.controls.iter().any(|control| control.key == expected));
            assert!(!contract.controls.iter().any(|control| control.key == absent));
            assert!(super::value_at_path(&contract.values, absent).unwrap().as_str().is_some_and(|value| !value.is_empty()));
            for (key, minimum, maximum) in [
                ("ui.terminal.font_size", 9.0, 28.0),
                ("ui.terminal.scrollback_lines", 100.0, 10000.0),
            ] {
                let row = contract.controls.iter().find(|control| control.key == key).unwrap();
                assert_eq!(row.minimum, Some(minimum));
                assert_eq!(row.maximum, Some(maximum));
                assert_eq!(row.integer, key == "ui.terminal.scrollback_lines");
            }
        }
    }

    #[test]
    fn workspace_tool_value_edits_do_not_reset_rail_or_workspace_data() {
        let mut config = crate::config::AppConfig::default();
        config.ui.sidebar_shortcuts.order = vec![crate::ui::sidebar_shortcuts::RailAction::Timeline];
        let original = serde_json::to_value(&config).unwrap();
        let mut values = original.clone();
        super::set_value_at_path(&mut values, "ui.toolbar.enabled", serde_json::json!(true)).unwrap();
        super::set_value_at_path(&mut values, "ui.toolbar.order", serde_json::json!(["preferences", "open_media"])).unwrap();
        super::set_value_at_path(&mut values, "ui.toolbar.hidden", serde_json::json!(["stop"])).unwrap();
        super::set_value_at_path(&mut values, "ui.terminal.font_size", serde_json::json!(14.5)).unwrap();
        let restored: crate::config::AppConfig = serde_json::from_value(values).unwrap();
        let mut roundtrip = serde_json::to_value(restored).unwrap();
        roundtrip["ui"]["toolbar"] = original["ui"]["toolbar"].clone();
        roundtrip["ui"]["terminal"] = original["ui"]["terminal"].clone();
        assert_eq!(roundtrip, original);
    }

    #[test]
    fn craft_header_visibility_is_shared_persistent_and_live_previewed() {
        let mut config=crate::config::AppConfig::default();
        assert!(config.craft_show_close_buttons);
        assert!(!config.craft_show_collapse_buttons);
        let contract=super::preferences_contract(&config);
        for (key,value,icon) in [("craft_show_close_buttons",true,"x"),("craft_show_collapse_buttons",false,"caret-down")] {
            let row=contract.controls.iter().find(|row|row.key==key).unwrap();
            assert_eq!(row.group,"Workspace panels");
            assert_eq!(row.icon,icon);
            assert_eq!(contract.values[key],serde_json::json!(value));
        }
        config.craft_show_close_buttons=false;config.craft_show_collapse_buttons=true;
        let restored:crate::config::AppConfig=serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
        let mut preview=crate::config::AppConfig::default();preview.copy_appearance_from(&restored);
        assert!(!preview.craft_show_close_buttons);assert!(preview.craft_show_collapse_buttons);
    }
    #[test]
    fn density_slider_choices_match_serialized_shared_sizes() {
        use crate::config::UiControlDensity;
        let contract = super::preferences_contract(&crate::config::AppConfig::default());
        let control = contract.controls.iter().find(|control| control.key == "ui_control_density").unwrap();
        assert!(matches!(control.kind, super::PreferenceControlKind::ChoiceSlider));
        assert_eq!(control.options.len(), 5);
        assert_eq!(contract.values["ui_control_density"], serde_json::json!("compact"));
        for ((density, option), height) in UiControlDensity::ALL.into_iter().zip(&control.options).zip([18.0, 24.0, 28.0, 32.0, 36.0]) {
            assert_eq!(serde_json::to_value(density).unwrap(), option.value);
            assert_eq!(serde_json::from_value::<UiControlDensity>(option.value.clone()).unwrap(), density);
            assert_eq!(density.base_height(), height);
        }
    }

    #[test]
    fn appearance_rows_have_semantic_icons_and_language_is_last() {
        let controls = preference_controls(&crate::config::AppConfig::default());
        let interface: Vec<_> = controls.iter().filter(|control| control.section == "appearance" && control.group == "Interface").collect();
        assert_eq!(interface.iter().map(|control| control.key).collect::<Vec<_>>(),
            ["theme", "color_palette", "accent_color", "workspace_ui", "preferences_grouped_navigation", "ui_control_density", "ui_animation_mode", "app_icon_preset", "fullscreen_video_background", "language"]);
        assert_eq!(interface.iter().map(|control| control.icon).collect::<Vec<_>>(),
            ["circle-half", "swatches", "palette", "tabs", "list-checks", "sliders-horizontal", "waveform", "image", "frame-corners", "translate"]);
        for (key, icon) in [("always_on_top", "push-pin"), ("osd_position", "target"), ("osd_timeout_seconds", "clock"),
            ("app_icon_playing", "play"), ("app_icon_paused", "pause"), ("app_icon_stopped", "stop-circle")] {
            assert_eq!(controls.iter().find(|control| control.key == key).unwrap().icon, icon);
        }
    }
    use super::*;

    #[test]
    fn preference_cards_and_help_are_unique_and_ordered() {
        let contract = preferences_contract(&crate::config::AppConfig::default());
        let mut controls = std::collections::HashSet::new();
        let mut hints = std::collections::HashSet::new();
        let mut cards = std::collections::HashSet::new();
        for control in &contract.controls {
            assert!(controls.insert(control.key), "duplicate control: {}", control.key);
            if let Some(description) = control.description {
                assert!(hints.insert((control.section, control.group, description)), "duplicate help in {}", control.group);
            }
        }
        for group in &contract.groups {
            assert!(cards.insert((group.section, group.name)), "duplicate card: {}", group.name);
        }
        assert_eq!(contract.groups.iter().filter(|group| group.section == "advanced").last().unwrap().name, "Config file");
        assert_eq!(contract.groups.iter().filter(|group| group.section == "appearance").last().unwrap().name, "Application icons");
    }

    #[test]
    fn application_icons_share_file_controls_and_optional_disclosure() {
        let mut config = crate::config::AppConfig::default();
        let contract = preferences_contract(&config);
        let group = contract.groups.iter().find(|group| group.name == "Application icons").unwrap();
        assert!(group.collapsible);
        assert!(!group.default_open);
        assert_eq!(group.configured_count, 0);
        let controls = contract.controls.iter().filter(|control| control.group == group.name).collect::<Vec<_>>();
        assert_eq!(controls.iter().map(|control| control.key).collect::<Vec<_>>(),
            ["app_icon", "app_icon_playing", "app_icon_paused", "app_icon_stopped"]);
        for control in controls {
            assert!(matches!(control.kind, PreferenceControlKind::File));
            assert!(control.description.is_none());
            assert_eq!(control.file_extensions, ["png", "jpg", "jpeg", "webp", "ico"]);
            assert_eq!(serde_json::to_value(control).unwrap()["kind"], "file");
        }
        assert!(contract.groups.iter().filter(|group| !group.collapsible).all(|group| group.default_open));
        config.app_icon_paused = Some("custom-paused.png".into());
        let configured = preferences_contract(&config);
        let group = configured.groups.iter().find(|group| group.name == "Application icons").unwrap();
        assert!(group.default_open);
        assert_eq!(group.configured_count, 1);
        config.app_icon_paused = Some("  ".into());
        let empty = preferences_contract(&config);
        assert!(!empty.groups.iter().find(|group| group.name == "Application icons").unwrap().default_open);
    }

    #[test]
    fn bundled_icon_choices_are_shared_without_expanding_custom_overrides() {
        let mut config = crate::config::AppConfig::default();
        config.app_icon_preset = crate::config::AppIconPreset::Classic;
        let contract = preferences_contract(&config);
        let control = contract.controls.iter().find(|control| control.key == "app_icon_preset").unwrap();
        assert!(matches!(control.kind, PreferenceControlKind::Select));
        assert_eq!(control.section, "appearance");
        assert_eq!(control.group, "Interface");
        assert_eq!(control.options.iter().map(|option| option.value.as_str().unwrap()).collect::<Vec<_>>(),
            ["current", "classic"]);
        assert_eq!(contract.values["app_icon_preset"], "classic");
        assert!(!contract.groups.iter().find(|group| group.name == "Application icons").unwrap().default_open);
    }

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
        assert!(config.timeline_hide_cue_text_overflow);
        let controls = preference_controls(&config);
        let control = controls
            .iter()
            .find(|control| control.key == "compact_timeline_tracks")
            .expect("timeline density preference");
        assert!(matches!(control.kind, PreferenceControlKind::Boolean));
        assert_eq!(control.section, "hardware");
        assert_eq!(control.group, "Timeline");

        let overflow_control = controls
            .iter()
            .find(|control| control.key == "timeline_hide_cue_text_overflow")
            .expect("timeline cue text overflow preference");
        assert!(matches!(overflow_control.kind, PreferenceControlKind::Boolean));
        assert_eq!(overflow_control.section, "hardware");
        assert_eq!(overflow_control.group, "Timeline");

        let cue_editor = controls
            .iter()
            .find(|control| control.key == "timeline_cue_editor_presentation")
            .expect("timeline cue editor presentation preference");
        assert!(matches!(cue_editor.kind, PreferenceControlKind::Select));
        assert_eq!(cue_editor.section, "hardware");
        assert_eq!(cue_editor.group, "Timeline");
        assert_eq!(
            cue_editor.options.iter().map(|option| option.value.as_str().unwrap()).collect::<Vec<_>>(),
            ["dialog", "panel"]
        );
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
        assert_eq!(previous.timeline_ctrl_wheel_action, Wheel::Zoom);
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
        assert_eq!(config.timeline_plain_wheel_action, Wheel::VerticalScroll);
        assert_eq!(config.timeline_ctrl_wheel_action, Wheel::Zoom);
        assert_eq!(config.timeline_shift_wheel_action, Wheel::HorizontalScroll);
        assert_eq!(config.timeline_alt_wheel_action, Wheel::Zoom);
        assert!(config.timeline_middle_button_pan);
        assert!(config.timeline_middle_axis_lock_modifiers);
        assert!(config.timeline_animated_navigation);
        assert_eq!(config.timeline_navigation_transition_ms, 100);

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
        assert!(config.media_keys_background_enabled);
        assert!(config.global_hardware_hotkeys_enabled);

        let controls = preference_controls(&config);
        for key in [
            "keyboard_shortcuts_enabled",
            "media_keys_enabled",
            "media_keys_background_enabled",
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
    fn background_media_control_scope_is_persistent_shared_and_independent() {
        let original=crate::config::AppConfig::default();
        let edited=original.apply_patch(&serde_json::json!({"media_keys_background_enabled":false})).unwrap();
        assert!(edited.media_keys_enabled && edited.keyboard_shortcuts_enabled && edited.global_hardware_hotkeys_enabled);
        assert!(!edited.media_keys_background_enabled);
        let restored:crate::config::AppConfig=serde_json::from_value(serde_json::to_value(&edited).unwrap()).unwrap();
        assert_eq!(restored,edited);
        let contract=preferences_contract(&edited);
        assert_eq!(contract.defaults["media_keys_background_enabled"],true);
        assert_eq!(contract.values["media_keys_background_enabled"],false);
        assert!(original.apply_patch(&serde_json::json!({"media_keys_background_enabled":"yes"})).is_err());
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
            crate::config::PlayerClickAction::ToggleFullscreen
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
            crate::config::PlayerDragAction::Seek
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
