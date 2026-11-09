//! Optional application command surface. Visibility never owns workspace panels.
use eframe::egui;
use serde::{Deserialize, Serialize};

use super::{icons, sidebar_shortcuts::RailAction};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolbarPreferences {
    pub enabled: bool,
    pub order: Vec<String>,
    pub hidden: Vec<String>,
}

impl Default for ToolbarPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            order: Vec::new(),
            hidden: commands()
                .into_iter()
                .filter(|command| !command.visible_by_default())
                .map(|command| command.id().to_owned())
                .collect(),
        }
    }
}

impl ToolbarPreferences {
    /// Reconcile known commands for display without deleting stored extension IDs.
    pub fn ordered_ids(&self) -> Vec<String> {
        let catalog = commands();
        let mut result = Vec::new();
        for id in self
            .order
            .iter()
            .map(String::as_str)
            .chain(catalog.iter().map(|c| c.id()))
        {
            if catalog.iter().any(|c| c.id() == id) && !result.iter().any(|item| item == id) {
                result.push(id.to_owned());
            }
        }
        result
    }

    pub fn set_visible(&mut self, id: &str, visible: bool) {
        self.hidden.retain(|item| item != id);
        if !visible {
            self.hidden.push(id.to_owned());
        }
    }

    fn move_command(&mut self, id: &str, delta: isize) {
        let mut order = self.ordered_ids();
        if let Some(index) = order.iter().position(|item| item == id) {
            let target = index
                .saturating_add_signed(delta)
                .min(order.len().saturating_sub(1));
            order.swap(index, target);
            // Keep configured IDs from optional/unavailable modules, not only visible commands.
            order.extend(
                self.order
                    .iter()
                    .filter(|item| !commands().iter().any(|c| c.id() == item.as_str()))
                    .cloned(),
            );
            self.order = order;
        }
    }
}

#[derive(Default)]
pub struct ToolbarState {
    pub preferences: ToolbarPreferences,
    pub palette_open: bool,
    query: String,
    selected: usize,
    palette_was_open: bool,
    previous_focus: Option<egui::Id>,
}

impl ToolbarState {
    pub fn with_preferences(preferences: ToolbarPreferences) -> Self {
        Self {
            preferences,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    OpenMedia,
    SaveTimeline,
    PlayPause,
    Stop,
    PreviousChapter,
    NextChapter,
    Mute,
    Fullscreen,
    Terminal,
    Rail(RailAction),
}

impl Command {
    fn id(self) -> &'static str {
        match self {
            Self::OpenMedia => "open_media",
            Self::SaveTimeline => "save_timeline",
            Self::PlayPause => "play_pause",
            Self::Stop => "stop",
            Self::PreviousChapter => "previous_chapter",
            Self::NextChapter => "next_chapter",
            Self::Mute => "mute",
            Self::Fullscreen => "fullscreen",
            Self::Terminal => "terminal",
            Self::Rail(action) => match action {
                RailAction::HardwareMonitor => "hardware_monitor",
                RailAction::MediaInspector => "media_inspector",
                RailAction::EffectControls => "effect_controls",
                RailAction::EffectsLibrary => "effects_library",
                RailAction::ProgramMonitor => "program_monitor",
                RailAction::Timeline => "timeline",
                RailAction::RfManager => "rf_manager",
                RailAction::EffectsManager => "effects_manager",
                RailAction::BoardInformation => "board_information",
                RailAction::Downloads => "downloads",
                RailAction::OpenLocation => "open_location",
                RailAction::Preferences => "preferences",
                RailAction::KeyboardShortcuts => "keyboard_shortcuts",
                RailAction::WorkspaceProfiles => "workspace_profiles",
                RailAction::AudioSettings => "audio_settings",
                RailAction::SubtitleSettings => "subtitle_settings",
                RailAction::About => "about",
            },
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::OpenMedia => "Open media",
            Self::SaveTimeline => "Save Timeline",
            Self::PlayPause => "Play / Pause",
            Self::Stop => "Stop",
            Self::PreviousChapter => "Previous chapter",
            Self::NextChapter => "Next chapter",
            Self::Mute => "Mute / Unmute",
            Self::Fullscreen => "Toggle fullscreen",
            Self::Terminal => "Terminal",
            Self::Rail(action) => action.label(),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::OpenMedia => icons::FOLDER_OPEN,
            Self::SaveTimeline => icons::FLOPPY_DISK,
            Self::PlayPause => icons::PLAY,
            Self::Stop => icons::STOP,
            Self::PreviousChapter => icons::SKIP_BACK,
            Self::NextChapter => icons::SKIP_FORWARD,
            Self::Mute => icons::SPEAKER_HIGH,
            Self::Fullscreen => icons::FRAME_CORNERS,
            Self::Terminal => egui_phosphor::regular::TERMINAL_WINDOW,
            Self::Rail(action) => action.icon(),
        }
    }

    fn visible_by_default(self) -> bool {
        matches!(
            self,
            Self::OpenMedia
                | Self::SaveTimeline
                | Self::PlayPause
                | Self::Stop
                | Self::Mute
                | Self::Fullscreen
                | Self::Terminal
                | Self::Rail(RailAction::Preferences | RailAction::WorkspaceProfiles)
        )
    }
}

fn commands() -> Vec<Command> {
    let mut commands = vec![
        Command::OpenMedia,
        Command::SaveTimeline,
        Command::PlayPause,
        Command::Stop,
        Command::PreviousChapter,
        Command::NextChapter,
        Command::Mute,
        Command::Fullscreen,
    ];
    if cfg!(feature = "workspace-terminal") {
        commands.push(Command::Terminal);
    }
    commands.extend(RailAction::ALL.into_iter().map(Command::Rail));
    commands
}

/// Stable ID, human label, icon: shared by native Preferences and this toolbar.
pub fn command_options() -> Vec<(&'static str, &'static str, &'static str)> {
    commands()
        .into_iter()
        .map(|command| (command.id(), command.label(), command.icon()))
        .collect()
}

fn keyboard_claim_id() -> egui::Id {
    egui::Id::unique("workspace-command-palette-keyboard")
}

/// The opening/closing frame is claimed too, preventing Enter/Space falling through.
pub fn owns_keyboard(ctx: &egui::Context) -> bool {
    let frame = ctx.cumulative_frame_nr();
    ctx.data(|data| data.get_temp::<u64>(keyboard_claim_id()) == Some(frame))
}

#[cfg(feature = "workspace-toolbar")]
mod native {
    use super::*;
    use crate::{app::PealayerApp, platform::interop::InteropCommand};

    impl Command {
        fn available(self, app: &mut PealayerApp) -> bool {
            match self {
                Self::SaveTimeline => {
                    crate::peer::active()
                        || app.current_video_path.as_ref().is_some_and(|path| {
                            !crate::media::is_remote_media_target(&path.to_string_lossy())
                        })
                }
                Self::PlayPause | Self::Stop => app.current_video_path.is_some(),
                Self::PreviousChapter | Self::NextChapter => !app.media_chapters().is_empty(),
                Self::Rail(action) => {
                    action.available(app) && !(action.tab().is_some() && app.dock_state.locked)
                }
                _ => true,
            }
        }

        fn current_icon(self, app: &PealayerApp) -> &'static str {
            match self {
                Self::PlayPause if !app.is_paused => icons::PAUSE,
                Self::Mute if app.is_muted => icons::SPEAKER_SLASH,
                _ => self.icon(),
            }
        }

        fn activate(self, app: &mut PealayerApp, ctx: &egui::Context) {
            if !self.available(app) {
                return;
            }
            match self {
                Self::OpenMedia => app.open_media_file_dialog(ctx),
                Self::SaveTimeline => {
                    if crate::peer::active() {
                        super::super::peer_browser::open(
                            ctx,
                            super::super::peer_browser::Purpose::TimelineSave,
                            None,
                        );
                    } else if let Err(error) = app.save_current_timeline_sidecar() {
                        app.show_error = Some(error);
                    }
                }
                Self::Rail(action) => action.activate(app, ctx),
                Self::Terminal => {
                    #[cfg(feature = "workspace-terminal")]
                    crate::ui::terminal::toggle(app);
                }
                other => {
                    let command = match other {
                        Self::PlayPause => InteropCommand::TogglePause,
                        Self::Stop => InteropCommand::Stop,
                        Self::PreviousChapter => InteropCommand::PreviousChapter,
                        Self::NextChapter => InteropCommand::NextChapter,
                        Self::Mute => InteropCommand::ToggleMute,
                        Self::Fullscreen => InteropCommand::ToggleFullscreen,
                        _ => unreachable!(),
                    };
                    app.apply_interop_command(ctx, command, "Workspace toolbar");
                }
            }
        }
    }

    fn configure(app: &mut PealayerApp, ui: &mut egui::Ui) {
        ui.menu_button(
            format!("{}  {}", icons::LIST, app.tr("Show / hide actions")),
            |ui| {
                for command in commands() {
                    let mut visible = !app
                        .toolbar_state
                        .preferences
                        .hidden
                        .iter()
                        .any(|id| id == command.id());
                    if ui
                        .checkbox(
                            &mut visible,
                            format!("{}  {}", command.icon(), app.tr(command.label())),
                        )
                        .changed()
                    {
                        app.toolbar_state
                            .preferences
                            .set_visible(command.id(), visible);
                        app.deferred_config_save = true;
                    }
                }
            },
        );
        if ui
            .button(format!(
                "{}  {}",
                icons::ARROW_COUNTER_CLOCKWISE,
                app.tr("Reset shortcuts")
            ))
            .clicked()
        {
            app.toolbar_state.preferences = ToolbarPreferences {
                enabled: true,
                ..Default::default()
            };
            app.deferred_config_save = true;
            ui.close();
        }
        if ui
            .button(format!("{}  {}", icons::EYE_SLASH, app.tr("Hide toolbar")))
            .clicked()
        {
            app.toolbar_state.preferences.enabled = false;
            app.deferred_config_save = true;
            ui.close();
        }
    }

    fn action_menu(app: &mut PealayerApp, ui: &mut egui::Ui, command: Command) {
        for (label, delta, icon) in [
            ("Move earlier", -1, icons::CARET_LEFT),
            ("Move later", 1, icons::CARET_RIGHT),
        ] {
            if ui.button(format!("{}  {}", icon, app.tr(label))).clicked() {
                app.toolbar_state
                    .preferences
                    .move_command(command.id(), delta);
                app.deferred_config_save = true;
                ui.close();
            }
        }
        if ui
            .button(format!("{}  {}", icons::EYE_SLASH, app.tr("Hide action")))
            .clicked()
        {
            app.toolbar_state
                .preferences
                .set_visible(command.id(), false);
            app.deferred_config_save = true;
            ui.close();
        }
        ui.separator();
        configure(app, ui);
    }

    pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
        if !app.toolbar_state.preferences.enabled {
            return;
        }
        let ctx = ui.ctx().clone();
        let interactive =
            ui.is_enabled() && ctx.memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
        let visible: Vec<_> = app
            .toolbar_state
            .preferences
            .ordered_ids()
            .iter()
            .filter(|id| !app.toolbar_state.preferences.hidden.contains(id))
            .filter_map(|id| commands().into_iter().find(|command| command.id() == id))
            .collect();
        let height = super::super::control_density::height(ui);
        let spacing = ui.spacing().item_spacing.x;
        // Always reserve the search and overflow/configuration controls first.
        let slots = ((ui.available_width() + spacing) / (height + spacing))
            .floor()
            .max(0.0) as usize;
        let count = slots.saturating_sub(2).min(visible.len());
        let mut chosen = None;
        ui.add_enabled_ui(interactive, |ui| {
            ui.with_layout(
                super::super::i18n::layout(app.rtl, egui::Align::Center),
                |ui| {
                    for command in visible.iter().copied().take(count) {
                        let available = command.available(app);
                        let icon = command.current_icon(app);
                        let response = ui
                            .add_enabled_ui(available, |ui| {
                                ui.add_sized(egui::Vec2::splat(height), egui::Button::new(icon))
                            })
                            .inner
                            .on_hover_text(app.tr(command.label()));
                        if response.clicked() {
                            chosen = Some(command);
                        }
                        response.context_menu(|ui| action_menu(app, ui, command));
                    }
                    if slots >= 2
                        && ui
                            .add_sized(
                                egui::Vec2::splat(height),
                                egui::Button::new(icons::MAGNIFYING_GLASS),
                            )
                            .on_hover_text(format!("{} (Ctrl+Shift+P)", app.tr("Command palette")))
                            .clicked()
                    {
                        app.toolbar_state.palette_open = true;
                    }
                    ui.menu_button(icons::DOTS_THREE, |ui| {
                        if slots < 2
                            && ui
                                .button(format!(
                                    "{}  {}",
                                    icons::MAGNIFYING_GLASS,
                                    app.tr("Command palette")
                                ))
                                .clicked()
                        {
                            app.toolbar_state.palette_open = true;
                            ui.close();
                        }
                        for command in visible.iter().copied().skip(count) {
                            let enabled = command.available(app);
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(format!(
                                        "{}  {}",
                                        command.current_icon(app),
                                        app.tr(command.label())
                                    )),
                                )
                                .clicked()
                            {
                                chosen = Some(command);
                                ui.close();
                            }
                        }
                        if count < visible.len() {
                            ui.separator();
                        }
                        configure(app, ui);
                    });
                },
            );
        });
        if let Some(command) = chosen {
            command.activate(app, &ctx);
        }
    }

    fn matching_commands(app: &PealayerApp) -> Vec<Command> {
        let query = app.toolbar_state.query.to_lowercase();
        commands()
            .into_iter()
            .filter(|command| {
                let searchable = format!(
                    "{} {} {}",
                    command.id(),
                    command.label(),
                    app.tr(command.label())
                )
                .to_lowercase();
                query
                    .split_whitespace()
                    .all(|word| searchable.contains(word))
            })
            .collect()
    }

    fn palette_blocked(ctx: &egui::Context) -> bool {
        let own_layer = egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("workspace-command-palette"),
        );
        ctx.memory(|memory| {
            if memory
                .top_modal_layer()
                .is_some_and(|layer| layer != own_layer)
            {
                return true;
            }
            // top_modal_layer is deliberately a PREVIOUS-frame snapshot in
            // egui. A newly shown dialog must win this frame too. Areas include
            // both current and previous visible roots; tooltip/drag overlays
            // and panel-owned sublayers must not claim application shortcuts.
            let areas = memory.areas();
            areas.visible_layer_ids().into_iter().any(|layer| {
                layer != own_layer
                    && layer.order == egui::Order::Foreground
                    && areas.parent_layer(layer).is_none()
            })
        })
    }

    pub fn draw_palette(app: &mut PealayerApp, ctx: &egui::Context) {
        let terminal_focused = {
            #[cfg(feature = "workspace-terminal")]
            {
                crate::ui::terminal::owns_keyboard(ctx)
            }
            #[cfg(not(feature = "workspace-terminal"))]
            {
                false
            }
        };
        let blocked = palette_blocked(ctx);
        if !app.toolbar_state.palette_open
            && app.keyboard_shortcuts_enabled
            && !terminal_focused
            && !egui::Popup::is_any_open(ctx)
            && !blocked
            && ctx.input_mut(|input| {
                input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::P)
            })
        {
            app.toolbar_state.palette_open = true;
        }
        // A request from a menu or local API is retained until the active
        // dialog closes, rather than stacking over it or stealing its input.
        if blocked {
            return;
        }
        if !app.toolbar_state.palette_open {
            app.toolbar_state.palette_was_open = false;
            return;
        }
        let first_frame = !app.toolbar_state.palette_was_open;
        if first_frame {
            app.toolbar_state.previous_focus = ctx.memory(|memory| memory.focused());
            app.toolbar_state.query.clear();
            app.toolbar_state.selected = 0;
        }
        app.toolbar_state.palette_was_open = true;
        let frame_number = ctx.cumulative_frame_nr();
        ctx.data_mut(|data| data.insert_temp(keyboard_claim_id(), frame_number));
        let tokens = super::super::craft_theme::current(ctx);
        let width = (ctx.content_rect().width() - 48.0).clamp(120.0, 560.0);
        let height = (ctx.content_rect().height() - 160.0).clamp(80.0, 360.0);
        let mut chosen = None;
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::unique("workspace-command-palette"))
            .frame(super::super::dialog::confirmation_modal_frame(&tokens))
            .backdrop_color(tokens.modal_backdrop)
            .show(ctx, |ui| {
                ui.set_width(width);
                ui.with_layout(
                    if app.rtl {
                        egui::Layout::left_to_right(egui::Align::Center)
                    } else {
                        egui::Layout::right_to_left(egui::Align::Center)
                    },
                    |ui| {
                        close |= super::super::dialog::close_button(ui, &app.tr("Close")).clicked();
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 32.0),
                            super::super::i18n::layout(app.rtl, egui::Align::Center),
                            |ui| {
                                ui.label(icons::MAGNIFYING_GLASS);
                                ui.strong(app.tr("Command palette"));
                            },
                        );
                    },
                );
                ui.separator();
                // Claim navigation before TextEdit can consume arrows/Enter or
                // surrender its focus on Escape. No Context access inside the transaction.
                let owns_input = ui.is_enabled()
                    && ctx.memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
                let (previous, next, enter, escape) = if owns_input {
                    ctx.input_mut(|input| {
                        (
                            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                        )
                    })
                } else {
                    (false, false, false, false)
                };
                close |= escape;
                let hint = app.tr("Search commands…");
                let response = ui.add_sized(
                    [width, super::super::control_density::height(ui)],
                    super::super::dialog::singleline_text_edit(ui, &mut app.toolbar_state.query)
                        .id(egui::Id::unique("workspace-command-search"))
                        .hint_text(hint),
                );
                if first_frame {
                    response.request_focus();
                }
                if response.changed() {
                    app.toolbar_state.selected = 0;
                }
                let matches = matching_commands(app);
                app.toolbar_state.selected = app
                    .toolbar_state
                    .selected
                    .min(matches.len().saturating_sub(1));
                if !matches.is_empty() {
                    if previous {
                        app.toolbar_state.selected =
                            (app.toolbar_state.selected + matches.len() - 1) % matches.len();
                    }
                    if next {
                        app.toolbar_state.selected =
                            (app.toolbar_state.selected + 1) % matches.len();
                    }
                    if enter {
                        chosen = matches
                            .get(app.toolbar_state.selected)
                            .copied()
                            .filter(|command| command.available(app));
                    }
                }
                egui::ScrollArea::vertical()
                    .id_salt("workspace-command-results")
                    .max_height(height)
                    .show(ui, |ui| {
                        if matches.is_empty() {
                            ui.weak(app.tr("No matching commands"));
                        }
                        for (index, command) in matches.into_iter().enumerate() {
                            let enabled = command.available(app);
                            let selected = app.toolbar_state.selected == index;
                            let caption = format!(
                                "{}  {}",
                                command.current_icon(app),
                                app.tr(command.label())
                            );
                            let button = egui::Button::new("")
                                .selected(selected)
                                .truncate()
                                .min_size(egui::vec2(
                                    width,
                                    super::super::control_density::height(ui),
                                ));
                            let button = if app.rtl {
                                button.right_text(caption)
                            } else {
                                button.left_text(caption)
                            };
                            let response = ui.add_enabled(enabled, button);
                            if response.clicked() {
                                chosen = Some(command);
                            }
                            if selected && (previous || next || response.changed() || first_frame) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                        }
                    });
                ui.separator();
                ui.weak(app.tr("↑ ↓ Navigate · Enter Run · Esc Close"));
            });
        close |= modal.should_close();
        if close || chosen.is_some() {
            app.toolbar_state.palette_open = false;
            app.toolbar_state.palette_was_open = false;
            let previous = app.toolbar_state.previous_focus.take();
            ctx.memory_mut(|memory| {
                memory.surrender_focus(egui::Id::unique("workspace-command-search"));
                if let Some(previous) = previous {
                    memory.request_focus(previous);
                }
            });
        }
        // Never run a command from the same input as a close/cancel request.
        if !close {
            if let Some(command) = chosen {
                command.activate(app, ctx);
            }
        }
    }
}

#[cfg(feature = "workspace-toolbar")]
pub use native::{draw, draw_palette};

#[cfg(not(feature = "workspace-toolbar"))]
pub fn draw(_: &mut crate::app::PealayerApp, _: &mut egui::Ui) {}
#[cfg(not(feature = "workspace-toolbar"))]
pub fn draw_palette(_: &mut crate::app::PealayerApp, _: &egui::Context) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_is_hidden_by_default_and_serialization_retains_customization() {
        let mut prefs = ToolbarPreferences::default();
        assert!(!prefs.enabled);
        prefs.order = vec![
            "preferences".into(),
            "open_media".into(),
            "preferences".into(),
            "custom-extension".into(),
        ];
        prefs.set_visible("play_pause", false);
        let roundtrip: ToolbarPreferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(roundtrip, prefs);
        let ids = roundtrip.ordered_ids();
        assert_eq!(&ids[..2], &["preferences", "open_media"]);
        assert_eq!(
            ids.iter().filter(|id| id.as_str() == "preferences").count(),
            1
        );
        assert!(!ids.iter().any(|id| id == "custom-extension"));
        assert!(roundtrip.order.iter().any(|id| id == "custom-extension"));
    }

    #[test]
    fn command_catalog_is_unique_and_restorable_without_resetting_enabled_state() {
        let options = command_options();
        let unique: std::collections::HashSet<_> = options.iter().map(|entry| entry.0).collect();
        assert_eq!(unique.len(), options.len());
        let mut prefs = ToolbarPreferences {
            enabled: true,
            ..Default::default()
        };
        prefs.set_visible("open_media", false);
        prefs.set_visible("open_media", false);
        assert_eq!(
            prefs
                .hidden
                .iter()
                .filter(|id| id.as_str() == "open_media")
                .count(),
            1
        );
        prefs.set_visible("open_media", true);
        assert!(!prefs.hidden.iter().any(|id| id == "open_media"));
        assert!(prefs.enabled);
    }

    #[test]
    fn reordering_keeps_all_commands_and_optional_identifiers() {
        let mut prefs = ToolbarPreferences::default();
        prefs.order.push("extension-command".into());
        let total = prefs.ordered_ids().len();
        prefs.move_command("save_timeline", -1);
        assert_eq!(prefs.ordered_ids()[0], "save_timeline");
        assert_eq!(prefs.ordered_ids().len(), total);
        assert!(prefs.order.iter().any(|id| id == "extension-command"));
    }

    #[cfg(feature = "workspace-toolbar")]
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    fn palette_frame(
        ctx: &egui::Context,
        app: &mut crate::app::PealayerApp,
        events: Vec<egui::Event>,
        blocker: bool,
    ) -> bool {
        let mut claimed = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |_| {
                if blocker {
                    egui::Modal::new(egui::Id::unique("toolbar-test-other-modal")).show(
                        ctx,
                        |ui| {
                            ui.label("Other editor");
                        },
                    );
                }
                draw_palette(app, ctx);
                claimed = owns_keyboard(ctx);
            },
        );
        output.textures_delta.clear();
        claimed
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn palette_is_available_with_hidden_toolbar_but_respects_shortcut_policy_and_other_modals() {
        let _guard = crate::app::tests::lock_app_tests();
        for (enabled, blocker) in [(true, false), (false, false), (true, true)] {
            let ctx = egui::Context::default();
            let mut app = crate::app::PealayerApp::default();
            app.keyboard_shortcuts_enabled = enabled;
            let dock = app.dock_state.clone();
            let claimed = palette_frame(
                &ctx,
                &mut app,
                vec![key(
                    egui::Key::P,
                    egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                )],
                blocker,
            );
            assert_eq!(app.toolbar_state.palette_open, enabled && !blocker);
            assert_eq!(claimed, enabled && !blocker);
            assert!(!app.toolbar_state.preferences.enabled);
            assert_eq!(app.dock_state, dock);
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn explicit_palette_request_waits_for_other_modal_without_stealing_focus() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = crate::app::PealayerApp::default();
        app.toolbar_state.palette_open = true;
        assert!(!palette_frame(&ctx, &mut app, vec![], true));
        assert!(
            app.toolbar_state.palette_open,
            "preserve the explicit pending request"
        );
        assert_ne!(
            ctx.memory(|memory| memory.focused()),
            Some(egui::Id::unique("workspace-command-search"))
        );
        assert!(!palette_frame(&ctx, &mut app, vec![], true));
        // egui retains its previous-frame modal snapshot for one closing frame.
        assert!(!palette_frame(&ctx, &mut app, vec![], false));
        assert!(palette_frame(&ctx, &mut app, vec![], false));
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(egui::Id::unique("workspace-command-search"))
        );
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn palette_filters_typed_text_and_enter_dispatches_only_selected_command_while_escape_cancels()
    {
        let _guard = crate::app::tests::lock_app_tests();
        for cancel in [false, true] {
            let ctx = egui::Context::default();
            let mut app = crate::app::PealayerApp::default();
            app.toolbar_state.palette_open = true;
            palette_frame(&ctx, &mut app, vec![], false);
            palette_frame(
                &ctx,
                &mut app,
                vec![egui::Event::Text("about".into())],
                false,
            );
            assert_eq!(app.toolbar_state.query, "about");
            let mut events = vec![key(egui::Key::Enter, egui::Modifiers::NONE)];
            if cancel {
                events.push(key(egui::Key::Escape, egui::Modifiers::NONE));
            }
            assert!(
                palette_frame(&ctx, &mut app, events, false),
                "closing frame must keep keyboard ownership"
            );
            assert!(!app.toolbar_state.palette_open);
            assert_eq!(app.show_about_dialog, !cancel);
            assert!(!palette_frame(&ctx, &mut app, vec![], false));
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn palette_arrows_move_selection_without_losing_search_focus_or_invoking_disabled_commands() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = crate::app::PealayerApp::default();
        app.toolbar_state.palette_open = true;
        palette_frame(&ctx, &mut app, vec![], false);
        palette_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Text("chapter".into())],
            false,
        );
        palette_frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
            false,
        );
        assert_eq!(app.toolbar_state.selected, 1);
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(egui::Id::unique("workspace-command-search"))
        );
        palette_frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
            false,
        );
        assert!(
            app.toolbar_state.palette_open,
            "unavailable command must not dispatch or close palette"
        );
        palette_frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
            false,
        );
        assert_eq!(app.toolbar_state.selected, 0, "navigation wraps");
    }
}
