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

    fn move_to(&mut self, action: Command, target: Command, after: bool) -> bool {
        let catalog = commands();
        let mut order: Vec<_> = self
            .ordered_ids()
            .iter()
            .filter_map(|id| catalog.iter().copied().find(|item| item.id() == id))
            .collect();
        if !super::action_customization::move_action(&mut order, action, target, after) {
            return false;
        }
        let mut ids: Vec<String> = order.into_iter().map(|item| item.id().into()).collect();
        ids.extend(
            self.order
                .iter()
                .filter(|id| !catalog.iter().any(|item| item.id() == id.as_str()))
                .cloned(),
        );
        self.order = ids;
        true
    }
}

#[derive(Default)]
pub struct ToolbarState {
    pub preferences: ToolbarPreferences,
    pub palette_open: bool,
    pub editor_open: bool,
    query: String,
    selected: usize,
    palette_was_open: bool,
    previous_focus: Option<egui::Id>,
    editor_was_open: bool,
    editor_previous_focus: Option<egui::Id>,
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
                RailAction::Terminal => "terminal",
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

    fn group(self) -> u8 {
        match self {
            Self::OpenMedia | Self::SaveTimeline | Self::Rail(RailAction::OpenLocation) => 0,
            Self::PlayPause
            | Self::Stop
            | Self::PreviousChapter
            | Self::NextChapter
            | Self::Mute
            | Self::Fullscreen => 1,
            Self::Terminal
            | Self::Rail(
                RailAction::HardwareMonitor
                | RailAction::MediaInspector
                | RailAction::EffectControls
                | RailAction::EffectsLibrary
                | RailAction::ProgramMonitor
                | RailAction::Timeline
                | RailAction::Terminal,
            ) => 2,
            _ => 3,
        }
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
    commands.extend(
        RailAction::ALL
            .into_iter()
            .filter(|action| *action != RailAction::Terminal)
            .map(Command::Rail),
    );
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

    #[derive(Clone)]
    struct WorkspaceToolbarDrag {
        command: Command,
        editor: bool,
    }

    /// One fixed hit rectangle: no resting border or allocation change on hover.
    fn flat_control(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        id: egui::Id,
        icon: &str,
        label: &str,
        enabled: bool,
        active: bool,
        ink: egui::Color32,
    ) -> egui::Response {
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        let tokens = super::super::craft_theme::current(ui.ctx());
        let usable = enabled && ui.is_enabled();
        let fill = if usable && response.is_pointer_button_down_on() {
            tokens.pressed
        } else if usable && response.hovered() {
            tokens.hover
        } else if active && usable {
            ink.gamma_multiply(0.14)
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect.shrink(1.0), 3.0, fill);
        if active && usable {
            ui.painter().hline(
                (rect.left() + 5.0)..=(rect.right() - 5.0),
                rect.bottom() - 2.0,
                egui::Stroke::new(2.0, ink),
            );
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                3.0,
                egui::Stroke::new(1.0, tokens.focus),
                egui::StrokeKind::Inside,
            );
        }
        let color = if usable {
            ink
        } else {
            ui.visuals().weak_text_color()
        };
        let font = egui::TextStyle::Button.resolve(ui.style());
        let galley = ui.painter().layout_no_wrap(icon.into(), font, color);
        let bounds = if galley.mesh_bounds.is_positive() {
            galley.mesh_bounds
        } else {
            egui::Rect::from_min_size(egui::Pos2::ZERO, galley.size())
        };
        ui.painter().with_clip_rect(rect).galley(
            rect.center() - bounds.center().to_vec2(),
            galley,
            color,
        );
        response
            .widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, usable, active, label));
        response.on_hover_cursor(if usable {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        })
    }

    fn cancel_drag(ctx: &egui::Context, allowed: bool, editor: bool) {
        if egui::DragAndDrop::payload::<WorkspaceToolbarDrag>(ctx)
            .is_some_and(|drag| drag.editor == editor)
            && (!allowed
                || ctx.input(|input| input.key_pressed(egui::Key::Escape) || !input.focused))
        {
            egui::DragAndDrop::clear_payload(ctx);
        }
    }

    fn paint_drag_preview(ui: &egui::Ui, editor: bool) {
        let Some(drag) = egui::DragAndDrop::payload::<WorkspaceToolbarDrag>(ui.ctx()) else {
            return;
        };
        if drag.editor != editor {
            return;
        }
        let Some(pointer) = ui.input(|input| input.pointer.interact_pos()) else {
            return;
        };
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::unique("workspace-toolbar-drag-preview"),
        ));
        let tokens = super::super::craft_theme::current(ui.ctx());
        let galley = painter.layout_no_wrap(
            drag.command.icon().into(),
            egui::TextStyle::Button.resolve(ui.style()),
            tokens.icon,
        );
        let rect = egui::Rect::from_center_size(
            pointer + egui::vec2(10.0, 10.0),
            egui::Vec2::splat(super::super::control_density::height(ui)),
        );
        painter.rect_filled(rect, 3.0, tokens.hover.gamma_multiply(0.9));
        painter.galley(
            rect.center() - galley.mesh_bounds.center().to_vec2(),
            galley,
            tokens.icon,
        );
    }

    fn drop_target(
        app: &mut PealayerApp,
        ui: &mut egui::Ui,
        response: &egui::Response,
        command: Command,
        vertical: bool,
        allowed: bool,
    ) {
        if !allowed {
            return;
        }
        // A row contains interactive children (grip, checkbox and arrows).
        // Their hit responses intentionally win over the row's passive scope
        // response. Use the clipped, modal-layer-aware row geometry for editor
        // drops instead of requiring that passive response to own the pointer.
        let payload = if vertical && ui.rect_contains_pointer(response.rect) {
            egui::DragAndDrop::payload::<WorkspaceToolbarDrag>(ui.ctx())
        } else if !vertical {
            response.dnd_hover_payload::<WorkspaceToolbarDrag>()
        } else {
            None
        };
        if let Some(drag) = payload {
            if drag.editor != vertical {
                return;
            }
            let after = ui.input(|input| {
                input.pointer.interact_pos().is_some_and(|point| {
                    if vertical {
                        point.y > response.rect.center().y
                    } else {
                        (point.x > response.rect.center().x) != app.rtl
                    }
                })
            });
            let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
            if vertical {
                ui.painter().hline(
                    response.rect.x_range(),
                    if after {
                        response.rect.bottom()
                    } else {
                        response.rect.top()
                    },
                    stroke,
                );
            } else {
                ui.painter().vline(
                    if after != app.rtl {
                        response.rect.right()
                    } else {
                        response.rect.left()
                    },
                    response.rect.y_range(),
                    stroke,
                );
            }
            let released = if vertical {
                ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary))
                    && egui::DragAndDrop::take_payload::<WorkspaceToolbarDrag>(ui.ctx()).is_some()
            } else {
                super::super::action_customization::drop_payload::<WorkspaceToolbarDrag>(response)
                    .is_some()
            };
            if released
                && app
                    .toolbar_state
                    .preferences
                    .move_to(drag.command, command, after)
            {
                app.deferred_config_save = true;
            }
        }
    }

    impl Command {
        fn available(self, app: &mut PealayerApp) -> bool {
            match self {
                Self::Terminal => !app.dock_state.locked,
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
                Self::PlayPause if app.current_video_path.is_some() && !app.is_paused => {
                    icons::PAUSE
                }
                Self::Mute if app.is_muted => icons::SPEAKER_SLASH,
                _ => self.icon(),
            }
        }

        fn active_ink(self, app: &PealayerApp, ui: &egui::Ui) -> (bool, egui::Color32) {
            let tokens = super::super::craft_theme::current(ui.ctx());
            match self {
                Self::PlayPause => (
                    app.current_video_path.is_some() && !app.is_paused,
                    tokens.render_green,
                ),
                Self::Stop => (false, tokens.danger),
                Self::Mute if app.is_muted => (true, tokens.hot_text),
                Self::Fullscreen => (
                    ui.input(|input| input.viewport().fullscreen.unwrap_or(app.was_fullscreen)),
                    tokens.accent,
                ),
                Self::Terminal => (app.terminal_state.preferences.enabled, tokens.accent),
                Self::Rail(action) => (
                    action
                        .pane()
                        .is_some_and(|pane| app.sidebar_rail_state.open == Some(pane)),
                    tokens.icon,
                ),
                _ => (false, tokens.icon),
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
        ui.set_max_width(260.0);
        if ui
            .button(format!(
                "{}  {}",
                icons::SLIDERS_HORIZONTAL,
                app.tr("Customize toolbar…")
            ))
            .clicked()
        {
            app.toolbar_state.editor_open = true;
            app.toolbar_state.palette_open = false;
            ui.close();
        }
        ui.add_enabled_ui(!app.dock_state.locked, |ui| {
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
        });
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
        ui.add_enabled_ui(!app.dock_state.locked, |ui| {
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
        });
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
        let spacing = ui.spacing().item_spacing.x.min(6.0);
        let (bar, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::hover(),
        );
        let geometry = geometry(bar, height, spacing, &visible, app.rtl);
        let count = geometry.actions.len();
        let customize = interactive && !app.dock_state.locked;
        cancel_drag(&ctx, customize, false);
        let mut chosen = None;
        ui.add_enabled_ui(interactive, |ui| {
            for x in &geometry.separators {
                ui.painter().vline(
                    *x,
                    (bar.top() + 5.0)..=(bar.bottom() - 5.0),
                    egui::Stroke::new(1.0, super::super::craft_theme::current(&ctx).separator),
                );
            }
            for (command, rect) in visible.iter().copied().zip(&geometry.actions) {
                let available = command.available(app);
                let (active, ink) = command.active_ink(app, ui);
                let response = flat_control(
                    ui,
                    *rect,
                    egui::Id::unique(("workspace-toolbar-action", command.id())),
                    command.current_icon(app),
                    &app.tr(command.label()),
                    available,
                    active,
                    ink,
                )
                .on_hover_text(app.tr(command.label()));
                if customize {
                    super::super::action_customization::register_drag(
                        &response,
                        super::super::action_customization::moved_for_drag(&ctx),
                        |_| WorkspaceToolbarDrag {
                            command,
                            editor: false,
                        },
                    );
                }
                drop_target(app, ui, &response, command, false, customize);
                if response.clicked() && available && !egui::DragAndDrop::has_any_payload(&ctx) {
                    chosen = Some(command);
                }
                if interactive {
                    response.context_menu(|ui| action_menu(app, ui, command));
                }
            }
            if geometry.empty.width() > 0.0 {
                let empty = ui.interact(
                    geometry.empty,
                    egui::Id::unique("workspace-toolbar-empty"),
                    egui::Sense::click(),
                );
                if interactive {
                    empty.context_menu(|ui| configure(app, ui));
                    if empty.double_clicked() {
                        app.toolbar_state.editor_open = true;
                    }
                }
            }
            if let Some(rect) = geometry.search {
                let response = flat_control(
                    ui,
                    rect,
                    egui::Id::unique("workspace-toolbar-search"),
                    icons::MAGNIFYING_GLASS,
                    &app.tr("Command palette"),
                    true,
                    false,
                    ui.visuals().text_color(),
                )
                .on_hover_text(format!("{} (Ctrl+Shift+P)", app.tr("Command palette")));
                if response.clicked() {
                    app.toolbar_state.palette_open = true;
                }
            }
            let response = flat_control(
                ui,
                geometry.menu,
                egui::Id::unique("workspace-toolbar-menu"),
                icons::DOTS_THREE,
                &app.tr("Toolbar menu"),
                true,
                false,
                ui.visuals().text_color(),
            )
            .on_hover_text(app.tr("Toolbar menu"));
            if interactive {
                egui::Popup::menu(&response).show(|ui| {
                    ui.set_max_width(260.0);
                    if geometry.search.is_none()
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
            }
        });
        paint_drag_preview(ui, false);
        if let Some(command) = chosen {
            command.activate(app, &ctx);
        }
    }

    struct ToolbarGeometry {
        actions: Vec<egui::Rect>,
        separators: Vec<f32>,
        empty: egui::Rect,
        search: Option<egui::Rect>,
        menu: egui::Rect,
    }

    fn geometry(
        bar: egui::Rect,
        height: f32,
        spacing: f32,
        visible: &[Command],
        rtl: bool,
    ) -> ToolbarGeometry {
        let square = |x: f32, width: f32| {
            egui::Rect::from_min_size(egui::pos2(x, bar.top()), egui::vec2(width, height))
        };
        let menu = square(
            (bar.right() - height).max(bar.left()),
            height.min(bar.width()),
        );
        let search = (bar.width() >= height * 2.0 + spacing)
            .then(|| square(menu.left() - spacing - height, height));
        let limit = search.unwrap_or(menu).left() - spacing;
        let mut actions = Vec::new();
        let mut separators = Vec::new();
        let mut x = bar.left();
        let mut group = None;
        for command in visible {
            let separated = group.is_some_and(|last| last != command.group());
            let next = x + if separated { 8.0 } else { 0.0 };
            if next + height > limit {
                break;
            }
            if separated {
                separators.push(x + 4.0);
            }
            actions.push(square(next, height));
            x = next + height + spacing;
            group = Some(command.group());
        }
        let empty = egui::Rect::from_min_max(
            egui::pos2(x.min(limit.max(bar.left())), bar.top()),
            egui::pos2(limit.max(bar.left()), bar.bottom()),
        );
        let mirror = |rect: egui::Rect| {
            if rtl {
                egui::Rect::from_min_max(
                    egui::pos2(bar.left() + bar.right() - rect.right(), rect.top()),
                    egui::pos2(bar.left() + bar.right() - rect.left(), rect.bottom()),
                )
            } else {
                rect
            }
        };
        ToolbarGeometry {
            actions: actions.into_iter().map(mirror).collect(),
            separators: separators
                .into_iter()
                .map(|x| if rtl { bar.left() + bar.right() - x } else { x })
                .collect(),
            empty: mirror(empty),
            search: search.map(mirror),
            menu: mirror(menu),
        }
    }

    fn draw_editor(app: &mut PealayerApp, ctx: &egui::Context) {
        if modal_blocked(ctx, "workspace-toolbar-editor") || egui::Popup::is_any_open(ctx) {
            return;
        }
        let first = !app.toolbar_state.editor_was_open;
        if first {
            app.toolbar_state.editor_previous_focus = ctx.memory(|memory| memory.focused());
        }
        app.toolbar_state.editor_was_open = true;
        // The background toolbar may still draw while this modal owns input.
        // Its cancellation path must never erase this editor's independent drag.
        cancel_drag(ctx, false, false);
        let frame = ctx.cumulative_frame_nr();
        ctx.data_mut(|data| data.insert_temp(keyboard_claim_id(), frame));
        let tokens = super::super::craft_theme::current(ctx);
        let width = (ctx.content_rect().width() - 48.0).clamp(160.0, 480.0);
        let mut close = false;
        let mut moved = None;
        let before = app.toolbar_state.preferences.clone();
        let modal = egui::Modal::new(egui::Id::unique("workspace-toolbar-editor"))
            .frame(super::super::dialog::confirmation_modal_frame(&tokens))
            .backdrop_color(tokens.modal_backdrop)
            .show(ctx, |ui| {
                ui.set_width(width);
                ui.allocate_ui_with_layout(
                    egui::vec2(
                        width,
                        32.0_f32.max(super::super::control_density::height(ui)),
                    ),
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
                                ui.label(icons::SLIDERS_HORIZONTAL);
                                ui.strong(app.tr("Customize toolbar"));
                            },
                        );
                    },
                );
                ui.separator();
                ui.weak(app.tr(if app.dock_state.locked {
                    "Unlock the workspace to customize actions."
                } else {
                    "Drag to reorder. Choose which actions appear."
                }));
                let allowed = !app.dock_state.locked
                    && ui.is_enabled()
                    && ctx.memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
                cancel_drag(ctx, allowed, true);
                let options = commands();
                let ordered = app.toolbar_state.preferences.ordered_ids();
                let list_height = (ctx.content_rect().height() - 230.0).clamp(80.0, 440.0);
                egui::ScrollArea::vertical()
                    .id_salt("workspace-toolbar-editor-actions")
                    // The centered modal is measured from its previous frame.
                    // A max-only viewport feeds that old height back into the
                    // next measurement and shifts the rows while clicking.
                    .min_scrolled_height(list_height)
                    .max_height(list_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_enabled_ui(allowed, |ui| {
                            for (index, id) in ordered.iter().enumerate() {
                                let Some(command) =
                                    options.iter().copied().find(|command| command.id() == id)
                                else {
                                    continue;
                                };
                                let mut visible =
                                    !app.toolbar_state.preferences.hidden.contains(id);
                                // A horizontal scope inside a vertical scroll area
                                // otherwise inherits its unbounded remaining height.
                                // Give every row an explicit density-aware slot so
                                // centering cannot grow/reposition the whole modal.
                                let row_height = super::super::control_density::height(ui)
                                    .max(ui.text_style_height(&egui::TextStyle::Button));
                                let row = ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), row_height),
                                    super::super::i18n::layout(app.rtl, egui::Align::Center),
                                    |ui| {
                                        let side = super::super::control_density::height(ui);
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::Vec2::splat(side),
                                            egui::Sense::hover(),
                                        );
                                        let grip = flat_control(
                                            ui,
                                            rect,
                                            egui::Id::unique(("workspace-toolbar-editor-grip", id)),
                                            egui_phosphor::regular::DOTS_SIX_VERTICAL,
                                            &app.tr("Drag to reorder"),
                                            allowed,
                                            false,
                                            ui.visuals().weak_text_color(),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::Grab);
                                        if allowed {
                                            super::super::action_customization::register_drag(
                                                &grip,
                                                super::super::action_customization::moved_for_drag(
                                                    ctx,
                                                ),
                                                |_| WorkspaceToolbarDrag {
                                                    command,
                                                    editor: true,
                                                },
                                            );
                                        }
                                        if ui
                                            .checkbox(
                                                &mut visible,
                                                format!(
                                                    "{}  {}",
                                                    command.icon(),
                                                    app.tr(command.label())
                                                ),
                                            )
                                            .changed()
                                        {
                                            app.toolbar_state.preferences.set_visible(id, visible);
                                        }
                                        // Keyboard-accessible alternative to dragging, same stable ordering.
                                        ui.with_layout(
                                            if app.rtl {
                                                egui::Layout::left_to_right(egui::Align::Center)
                                            } else {
                                                egui::Layout::right_to_left(egui::Align::Center)
                                            },
                                            |ui| {
                                                if ui
                                                    .add_enabled(
                                                        index + 1 < ordered.len(),
                                                        egui::Button::new(icons::CARET_DOWN)
                                                            .frame(false),
                                                    )
                                                    .on_hover_text(app.tr("Move later"))
                                                    .clicked()
                                                {
                                                    moved = Some((id.clone(), 1));
                                                }
                                                if ui
                                                    .add_enabled(
                                                        index > 0,
                                                        egui::Button::new(
                                                            egui_phosphor::regular::CARET_UP,
                                                        )
                                                        .frame(false),
                                                    )
                                                    .on_hover_text(app.tr("Move earlier"))
                                                    .clicked()
                                                {
                                                    moved = Some((id.clone(), -1));
                                                }
                                            },
                                        );
                                    },
                                );
                                drop_target(app, ui, &row.response, command, true, allowed);
                            }
                        });
                    });
                ui.separator();
                paint_drag_preview(ui, true);
                ui.allocate_ui_with_layout(
                    egui::vec2(width, super::super::control_density::height(ui)),
                    if app.rtl {
                        egui::Layout::left_to_right(egui::Align::Center)
                    } else {
                        egui::Layout::right_to_left(egui::Align::Center)
                    },
                    |ui| {
                        if ui.button(app.tr("Done")).clicked() {
                            close = true;
                        }
                        if ui
                            .add_enabled(
                                allowed,
                                egui::Button::new(format!(
                                    "{}  {}",
                                    icons::ARROW_COUNTER_CLOCKWISE,
                                    app.tr("Reset actions")
                                ))
                                .frame(false),
                            )
                            .clicked()
                        {
                            let enabled = app.toolbar_state.preferences.enabled;
                            app.toolbar_state.preferences = ToolbarPreferences {
                                enabled,
                                ..Default::default()
                            };
                        }
                    },
                );
            });
        if let Some((id, delta)) = moved {
            app.toolbar_state.preferences.move_command(&id, delta);
        }
        if before != app.toolbar_state.preferences {
            app.deferred_config_save = true;
        }
        if close || modal.should_close() {
            app.toolbar_state.editor_open = false;
            app.toolbar_state.editor_was_open = false;
            cancel_drag(ctx, false, true);
            if let Some(previous) = app.toolbar_state.editor_previous_focus.take() {
                ctx.memory_mut(|memory| memory.request_focus(previous));
            }
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

    fn modal_blocked(ctx: &egui::Context, own_id: &str) -> bool {
        let own_layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique(own_id));
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
        if app.toolbar_state.editor_open {
            draw_editor(app, ctx);
            return;
        }
        // An API close has the same focus/drag cleanup as the modal's own close.
        if app.toolbar_state.editor_was_open {
            app.toolbar_state.editor_was_open = false;
            cancel_drag(ctx, false, true);
            let frame = ctx.cumulative_frame_nr();
            ctx.data_mut(|data| data.insert_temp(keyboard_claim_id(), frame));
            if let Some(previous) = app.toolbar_state.editor_previous_focus.take() {
                ctx.memory_mut(|memory| memory.request_focus(previous));
            }
        }
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
        let blocked = modal_blocked(ctx, "workspace-command-palette");
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

    #[cfg(test)]
    mod presentation_tests {
        use super::*;

        #[test]
        fn toolbar_geometry_keeps_menu_at_trailing_edge_and_reserves_separators() {
            for rtl in [false, true] {
                for width in [16.0, 48.0, 180.0, 720.0, 1400.0] {
                    let bar =
                        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(width, 24.0));
                    let layout = geometry(bar, 24.0, 4.0, &commands(), rtl);
                    assert_eq!(
                        if rtl {
                            layout.menu.left()
                        } else {
                            layout.menu.right()
                        },
                        if rtl { bar.left() } else { bar.right() }
                    );
                    assert!(bar.contains_rect(layout.menu));
                    for action in &layout.actions {
                        assert!(bar.contains_rect(*action));
                        assert!(!action.intersects(layout.menu));
                        if let Some(search) = layout.search {
                            assert!(!action.intersects(search));
                        }
                    }
                    for pair in layout.actions.windows(2) {
                        assert!(!pair[0].intersects(pair[1]));
                    }
                    if width >= 720.0 {
                        assert!(!layout.separators.is_empty());
                        assert!(layout.empty.width() > 0.0);
                    }
                }
            }
        }

        #[test]
        fn flat_actions_do_not_paint_inactive_or_hover_outlines_and_never_resize() {
            for dark in [false, true] {
                let ctx = egui::Context::default();
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let rect =
                    egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(28.0, 28.0));
                for active in [false, true] {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            events: vec![egui::Event::PointerMoved(rect.center())],
                            ..Default::default()
                        },
                        |ui| {
                            let response = flat_control(
                                ui,
                                rect,
                                egui::Id::unique("flat-toolbar-fixture"),
                                "P",
                                "Play",
                                true,
                                active,
                                egui::Color32::GREEN,
                            );
                            assert_eq!(response.rect, rect);
                        },
                    );
                    for shape in &output.shapes {
                        if let egui::Shape::Rect(shape) = &shape.shape {
                            if rect.contains_rect(shape.rect) {
                                assert!(
                                    shape.stroke.is_empty(),
                                    "flat action must not acquire a rounded outline"
                                );
                            }
                        }
                    }
                    output.textures_delta.clear();
                }
            }
        }

        #[test]
        fn media_active_style_uses_actual_state_and_not_a_generic_selected_flag() {
            let _guard = crate::app::tests::lock_app_tests();
            let mut app = PealayerApp::default();
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                app.is_paused = false;
                assert!(
                    !Command::PlayPause.active_ink(&app, ui).0,
                    "no media is not playing"
                );
                app.current_video_path = Some(std::path::PathBuf::from("fixture-only.mp4"));
                assert!(Command::PlayPause.active_ink(&app, ui).0);
                assert_eq!(Command::PlayPause.current_icon(&app), icons::PAUSE);
                app.is_paused = true;
                assert!(!Command::PlayPause.active_ink(&app, ui).0);
                app.is_muted = true;
                assert!(Command::Mute.active_ink(&app, ui).0);
                assert_eq!(Command::Mute.current_icon(&app), icons::SPEAKER_SLASH);
                app.is_muted = false;
                assert!(!Command::Mute.active_ink(&app, ui).0);
            });
            output.textures_delta.clear();
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

    #[test]
    fn typed_drop_reorders_without_changing_visibility_or_unknown_identifiers() {
        let mut prefs = ToolbarPreferences::default();
        prefs.order.push("unavailable-plugin".into());
        let hidden = prefs.hidden.clone();
        assert!(prefs.move_to(Command::Mute, Command::OpenMedia, false));
        assert_eq!(prefs.ordered_ids()[0], "mute");
        assert_eq!(prefs.hidden, hidden);
        assert!(prefs.order.iter().any(|id| id == "unavailable-plugin"));
        assert!(!prefs.move_to(Command::Mute, Command::Mute, true));
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
    fn pointer(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    fn toolbar_frame(
        ctx: &egui::Context,
        app: &mut crate::app::PealayerApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                draw(app, ui);
                draw_palette(app, ctx);
                let claimed = owns_keyboard(ctx);
                ctx.data_mut(|data| {
                    data.insert_temp(egui::Id::unique("toolbar-test-frame-claimed"), claimed);
                });
            },
        );
        output.textures_delta.clear();
        output
    }

    #[cfg(feature = "workspace-toolbar")]
    fn text_position(shape: &egui::Shape, needle: &str) -> Option<egui::Pos2> {
        match shape {
            egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            egui::Shape::Vec(shapes) => {
                shapes.iter().find_map(|shape| text_position(shape, needle))
            }
            _ => None,
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn empty_space_context_menu_opens_native_editor_without_changing_workspace() {
        let _guard = crate::app::tests::lock_app_tests();
        for rtl in [false, true] {
            let ctx = egui::Context::default();
            let mut app = crate::app::PealayerApp::default();
            app.rtl = rtl;
            app.toolbar_state.preferences.enabled = true;
            let workspace = app.dock_state.clone();
            let original = app.toolbar_state.preferences.clone();
            toolbar_frame(&ctx, &mut app, vec![]);
            let point = ctx
                .read_response(egui::Id::unique("workspace-toolbar-empty"))
                .unwrap()
                .rect
                .center();
            toolbar_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(point),
                    pointer(point, egui::PointerButton::Secondary, true),
                ],
            );
            toolbar_frame(
                &ctx,
                &mut app,
                vec![pointer(point, egui::PointerButton::Secondary, false)],
            );
            let output = toolbar_frame(&ctx, &mut app, vec![]);
            let edit = output
                .shapes
                .iter()
                .find_map(|shape| text_position(&shape.shape, "Customize toolbar"))
                .expect("empty-space editor action");
            toolbar_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(edit),
                    pointer(edit, egui::PointerButton::Primary, true),
                ],
            );
            toolbar_frame(
                &ctx,
                &mut app,
                vec![pointer(edit, egui::PointerButton::Primary, false)],
            );
            assert!(app.toolbar_state.editor_open);
            for _ in 0..3 {
                toolbar_frame(&ctx, &mut app, vec![]);
            }
            assert!(
                ctx.read_response(egui::Id::unique((
                    "workspace-toolbar-editor-grip",
                    String::from("open_media")
                )))
                .is_some()
            );
            assert_eq!(app.dock_state, workspace);
            assert_eq!(app.toolbar_state.preferences, original);
            toolbar_frame(
                &ctx,
                &mut app,
                vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
            );
            assert!(!app.toolbar_state.editor_open);
            assert!(
                ctx.data(
                    |data| data.get_temp::<bool>(egui::Id::unique("toolbar-test-frame-claimed"))
                )
                .unwrap_or(false),
                "editor closing frame owns Escape before global shortcuts run"
            );
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn toolbar_primary_drag_reorders_but_lock_and_escape_preserve_order_and_workspace() {
        let _guard = crate::app::tests::lock_app_tests();
        for (locked, cancel) in [(false, false), (true, false), (false, true)] {
            let ctx = egui::Context::default();
            let mut app = crate::app::PealayerApp::default();
            app.toolbar_state.preferences.enabled = true;
            app.dock_state.locked = locked;
            let workspace = app.dock_state.clone();
            let original = app.toolbar_state.preferences.clone();
            toolbar_frame(&ctx, &mut app, vec![]);
            let source = ctx
                .read_response(egui::Id::unique(("workspace-toolbar-action", "open_media")))
                .unwrap()
                .rect
                .center();
            let destination = ctx
                .read_response(egui::Id::unique(("workspace-toolbar-action", "stop")))
                .unwrap()
                .rect
                .center()
                + egui::vec2(5.0, 0.0);
            toolbar_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(source),
                    pointer(source, egui::PointerButton::Primary, true),
                ],
            );
            toolbar_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(destination)]);
            if cancel {
                toolbar_frame(
                    &ctx,
                    &mut app,
                    vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
                );
            }
            toolbar_frame(
                &ctx,
                &mut app,
                vec![pointer(destination, egui::PointerButton::Primary, false)],
            );
            if locked || cancel {
                assert_eq!(app.toolbar_state.preferences, original);
            } else {
                assert_eq!(
                    &app.toolbar_state.preferences.ordered_ids()[..4],
                    ["save_timeline", "play_pause", "stop", "open_media"]
                );
            }
            assert_eq!(app.dock_state, workspace);
            assert!(!app.show_about_dialog);
            assert!(!app.show_preferences_dialog);
        }
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn editor_drag_survives_the_disabled_background_toolbar_without_invoking_actions() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = crate::app::PealayerApp::default();
        app.toolbar_state.preferences.enabled = true;
        app.toolbar_state.editor_open = true;
        let workspace = app.dock_state.clone();
        let hidden = app.toolbar_state.preferences.hidden.clone();
        for _ in 0..3 {
            toolbar_frame(&ctx, &mut app, vec![]);
        }
        let source = ctx
            .read_response(egui::Id::unique((
                "workspace-toolbar-editor-grip",
                "open_media",
            )))
            .unwrap()
            .rect
            .center();
        let destination = ctx
            .read_response(egui::Id::unique((
                "workspace-toolbar-editor-grip",
                "save_timeline",
            )))
            .unwrap()
            .rect
            .center()
            + egui::vec2(0.0, 7.0);
        toolbar_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(source),
                pointer(source, egui::PointerButton::Primary, true),
            ],
        );
        let pressed_rect = ctx
            .read_response(egui::Id::unique((
                "workspace-toolbar-editor-grip",
                "open_media",
            )))
            .unwrap()
            .rect;
        assert!(
            pressed_rect.contains(source),
            "editor must stay positioned during press: source={source:?}, current={pressed_rect:?}"
        );
        toolbar_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(destination)]);
        assert!(
            egui::DragAndDrop::has_any_payload(&ctx),
            "editor grip starts a typed drag"
        );
        toolbar_frame(&ctx, &mut app, vec![]);
        assert!(
            egui::DragAndDrop::has_any_payload(&ctx),
            "background toolbar preserves editor drag"
        );
        toolbar_frame(
            &ctx,
            &mut app,
            vec![pointer(destination, egui::PointerButton::Primary, false)],
        );
        assert_eq!(
            &app.toolbar_state.preferences.ordered_ids()[..2],
            ["save_timeline", "open_media"]
        );
        assert_eq!(app.toolbar_state.preferences.hidden, hidden);
        assert_eq!(app.dock_state, workspace);
        assert!(app.toolbar_state.editor_open);
        assert!(!app.show_preferences_dialog);
    }

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn editor_api_close_and_mid_drag_lock_cancel_without_reordering_or_panel_changes() {
        let _guard = crate::app::tests::lock_app_tests();
        for api_close in [true, false] {
            let ctx = egui::Context::default();
            let mut app = crate::app::PealayerApp::default();
            app.toolbar_state.preferences.enabled = true;
            app.toolbar_state.editor_open = true;
            for _ in 0..3 {
                toolbar_frame(&ctx, &mut app, vec![]);
            }
            let original = app.toolbar_state.preferences.clone();
            let source = ctx
                .read_response(egui::Id::unique((
                    "workspace-toolbar-editor-grip",
                    "open_media",
                )))
                .unwrap()
                .rect
                .center();
            let destination = ctx
                .read_response(egui::Id::unique((
                    "workspace-toolbar-editor-grip",
                    "save_timeline",
                )))
                .unwrap()
                .rect
                .center()
                + egui::vec2(0.0, 7.0);
            toolbar_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(source),
                    pointer(source, egui::PointerButton::Primary, true),
                ],
            );
            toolbar_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(destination)]);
            assert!(egui::DragAndDrop::has_any_payload(&ctx));
            if api_close {
                crate::workspace_tools::apply(
                    &mut app,
                    &ctx,
                    crate::workspace_tools::Request::Editor { open: false },
                    "IPC",
                )
                .unwrap();
            } else {
                app.dock_state.locked = true;
            }
            let workspace = app.dock_state.clone();
            toolbar_frame(&ctx, &mut app, vec![]);
            assert!(!egui::DragAndDrop::has_any_payload(&ctx));
            assert!(
                ctx.data(
                    |data| data.get_temp::<bool>(egui::Id::unique("toolbar-test-frame-claimed"))
                )
                .unwrap_or(false)
            );
            toolbar_frame(
                &ctx,
                &mut app,
                vec![pointer(destination, egui::PointerButton::Primary, false)],
            );
            assert_eq!(app.toolbar_state.preferences, original);
            assert_eq!(app.dock_state, workspace);
            assert_eq!(app.toolbar_state.editor_open, !api_close);
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
