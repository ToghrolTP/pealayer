//! Application-owned rail actions. Dragging is a preview, never an activation.
use super::{icons, layout::PealayerTab as Tab, sidebar_rail::SidebarPane as Pane};
use crate::app::PealayerApp;
use eframe::egui::{self, Id, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailAction {
    HardwareMonitor,
    MediaInspector,
    EffectControls,
    EffectsLibrary,
    ProgramMonitor,
    Timeline,
    RfManager,
    EffectsManager,
    BoardInformation,
    Downloads,
    OpenLocation,
    Preferences,
    KeyboardShortcuts,
    WorkspaceProfiles,
    AudioSettings,
    SubtitleSettings,
    About,
}

impl RailAction {
    pub const ALL: [Self; 17] = [
        Self::HardwareMonitor,
        Self::MediaInspector,
        Self::EffectControls,
        Self::EffectsLibrary,
        Self::ProgramMonitor,
        Self::Timeline,
        Self::RfManager,
        Self::EffectsManager,
        Self::BoardInformation,
        Self::Downloads,
        Self::OpenLocation,
        Self::Preferences,
        Self::KeyboardShortcuts,
        Self::WorkspaceProfiles,
        Self::AudioSettings,
        Self::SubtitleSettings,
        Self::About,
    ];
    pub fn tab(self) -> Option<Tab> {
        Some(match self {
            Self::HardwareMonitor => Tab::HardwareMonitor,
            Self::MediaInspector => Tab::MediaInspector,
            Self::EffectControls => Tab::EffectControls,
            Self::EffectsLibrary => Tab::EffectsLibrary,
            Self::ProgramMonitor => Tab::ProgramMonitor,
            Self::Timeline => Tab::Timeline,
            _ => return None,
        })
    }
    pub fn pane(self) -> Option<Pane> {
        self.tab().and_then(|tab| Pane::try_from(tab).ok())
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::HardwareMonitor => "Hardware Monitor",
            Self::MediaInspector => "Media Inspector",
            Self::EffectControls => "Effect Controls",
            Self::EffectsLibrary => "Effects Library",
            Self::ProgramMonitor => "Program Monitor",
            Self::Timeline => "Timeline",
            Self::RfManager => "RF management",
            Self::EffectsManager => "Manage effects",
            Self::BoardInformation => "Board information",
            Self::Downloads => "Downloads",
            Self::OpenLocation => "Open Location / URL",
            Self::Preferences => "Preferences",
            Self::KeyboardShortcuts => "Keyboard Shortcuts...",
            Self::WorkspaceProfiles => "Workspace profiles",
            Self::AudioSettings => "Audio settings",
            Self::SubtitleSettings => "Subtitle settings",
            Self::About => "About",
        }
    }
    pub fn icon(self) -> &'static str {
        if let Some(tab) = self.tab() {
            return tab.icon();
        }
        match self {
            Self::RfManager => icons::BROADCAST,
            Self::EffectsManager => icons::SPARKLE,
            Self::BoardInformation => icons::CIRCUITRY,
            Self::Downloads => egui_phosphor::regular::DOWNLOAD_SIMPLE,
            Self::OpenLocation => icons::LINK_SIMPLE,
            Self::Preferences => icons::GEAR,
            Self::KeyboardShortcuts => icons::KEYBOARD,
            Self::WorkspaceProfiles => icons::GRID_FOUR,
            Self::AudioSettings => icons::SPEAKER_HIGH,
            Self::SubtitleSettings => egui_phosphor::regular::SUBTITLES,
            _ => icons::INFO,
        }
    }
    fn available(self, app: &mut PealayerApp) -> bool {
        if self == Self::RfManager {
            let caps = app.advertised_hardware();
            return super::hardware_monitor::show_rf(app.is_connected, caps.as_ref());
        }
        true
    }
    fn activate(self, app: &mut PealayerApp, ctx: &egui::Context) {
        if !self.available(app) {
            return;
        }
        if self.tab().is_some() && app.dock_state.locked {
            return;
        }
        if let Some(pane) = self.pane() {
            if app.sidebar_rail_state.open == Some(pane) {
                app.sidebar_rail_state.open = None;
            } else if let Err(error) = app.open_sidebar_pane(pane) {
                app.config_status = error;
            }
            return;
        }
        if let Some(tab) = self.tab() {
            app.open_or_focus_tab(tab);
            return;
        }
        use crate::platform::interop::InteropCommand as Command;
        match self {
            Self::RfManager => app.apply_interop_command(ctx, Command::OpenRfManager, "sidebar"),
            Self::Preferences => super::preferences::open(app, ctx),
            Self::BoardInformation => {
                app.apply_interop_command(ctx, Command::OpenBoardInformation { tab: 0 }, "sidebar")
            }
            Self::Downloads => crate::downloads::open(ctx),
            Self::EffectsManager => app.show_effect_library_editor = true,
            Self::OpenLocation => app.show_open_url_dialog = true,
            Self::KeyboardShortcuts => app.show_shortcuts_dialog = true,
            Self::WorkspaceProfiles => app.show_workspace_profiles_dialog = true,
            Self::AudioSettings => app.show_audio_settings = true,
            Self::SubtitleSettings => app.show_sub_settings = true,
            Self::About => app.show_about_dialog = true,
            _ => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RailShortcuts {
    pub order: Vec<RailAction>,
    pub hidden: Vec<RailAction>,
}
impl Default for RailShortcuts {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            hidden: RailAction::ALL[4..].to_vec(),
        }
    }
}
impl RailShortcuts {
    pub fn order(&self) -> Vec<RailAction> {
        let mut result = Vec::new();
        for action in self.order.iter().copied().chain(RailAction::ALL) {
            if !result.contains(&action) {
                result.push(action);
            }
        }
        result
    }
    pub fn set_visible(&mut self, action: RailAction, visible: bool) {
        self.hidden.retain(|candidate| *candidate != action);
        if !visible {
            self.hidden.push(action);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RailDrag {
    action: RailAction,
}
#[derive(Clone)]
struct PendingDrop {
    tab: Tab,
    anchor: Option<Tab>,
    placement: craft_ui::docking::Placement<Tab>,
}
fn drop_id() -> Id {
    Id::unique("pealayer-rail-panel-drop")
}

fn configure(app: &mut PealayerApp, ui: &egui::Ui) {
    super::preferences::navigate(
        app,
        ui.ctx(),
        crate::preferences_contract::PreferencesNavigation {
            section: Some("sidebar_rail".into()),
            scroll_y: 0.0,
        },
    );
}

fn common_menu(app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.menu_button(
        format!("{}  {}", icons::PLUS, app.tr("Show / hide actions")),
        |ui| {
            for action in app.sidebar_shortcuts.order() {
                let mut visible = !app.sidebar_shortcuts.hidden.contains(&action);
                if ui
                    .checkbox(
                        &mut visible,
                        format!("{}  {}", action.icon(), app.tr(action.label())),
                    )
                    .changed()
                {
                    app.sidebar_shortcuts.set_visible(action, visible);
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
        app.sidebar_shortcuts = RailShortcuts::default();
        app.deferred_config_save = true;
        ui.close();
    }
    if ui
        .button(format!("{}  {}", icons::GEAR, app.tr("Preferences...")))
        .clicked()
    {
        configure(app, ui);
        ui.close();
    }
    if ui
        .button(format!(
            "{}  {}",
            icons::EYE_SLASH,
            app.tr("Hide sidebar rail")
        ))
        .clicked()
    {
        app.sidebar_rail_state.enabled = false;
        app.deferred_config_save = true;
        ui.close();
    }
}
fn item_menu(action: RailAction, app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.strong(format!("{}  {}", action.icon(), app.tr(action.label())));
    ui.separator();
    let available = action.available(app);
    let open_label = if action
        .pane()
        .is_some_and(|pane| app.sidebar_rail_state.open == Some(pane))
    {
        "Close sidebar pane"
    } else if action.pane().is_some() {
        "Open in sidebar"
    } else {
        "Open"
    };
    if ui
        .add_enabled(
            available && (action.tab().is_none() || !app.dock_state.locked),
            egui::Button::new(app.tr(open_label)),
        )
        .clicked()
    {
        action.activate(app, ui.ctx());
        ui.close();
    }
    if let Some(tab) = action.tab() {
        if ui
            .add_enabled(
                !app.dock_state.locked,
                egui::Button::new(format!(
                    "{}  {}",
                    icons::GRID_FOUR,
                    app.tr("Dock to Workspace")
                )),
            )
            .clicked()
        {
            app.open_or_focus_tab(tab);
            ui.close();
        }
    }
    let mut order = app.sidebar_shortcuts.order();
    let index = order.iter().position(|item| *item == action).unwrap();
    let previous = (0..index)
        .rev()
        .find(|index| !app.sidebar_shortcuts.hidden.contains(&order[*index]));
    let next = (index + 1..order.len())
        .find(|index| !app.sidebar_shortcuts.hidden.contains(&order[*index]));
    for (enabled, next, icon, label) in [
        (
            previous.is_some(),
            previous.unwrap_or(index),
            icons::ARROW_UP,
            "Move up",
        ),
        (
            next.is_some(),
            next.unwrap_or(index),
            icons::ARROW_DOWN,
            "Move down",
        ),
    ] {
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(format!("{icon}  {}", app.tr(label))),
            )
            .clicked()
        {
            order.swap(index, next);
            app.sidebar_shortcuts.order = order.clone();
            app.deferred_config_save = true;
            ui.close();
        }
    }
    if ui
        .button(format!("{}  {}", icons::EYE_SLASH, app.tr("Hide shortcut")))
        .clicked()
    {
        app.sidebar_shortcuts.set_visible(action, false);
        app.deferred_config_save = true;
        ui.close();
    }
    ui.separator();
    common_menu(app, ui);
}

/// Native Phosphor targets share the application palette and never change size on hover.
pub(super) fn show(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let before = app.sidebar_shortcuts.clone();
    let interactive = ui.is_enabled() && ui.ctx().memory(|m| m.is_above_modal_layer(ui.layer_id()));
    if egui::DragAndDrop::has_payload_of_type::<RailDrag>(ui.ctx())
        && (!interactive
            || ui.input(|input| input.key_pressed(egui::Key::Escape) || !input.focused))
    {
        egui::DragAndDrop::clear_payload(ui.ctx());
    }
    let panel = if app.rtl {
        egui::Panel::left(Id::unique("sidebar-shortcuts"))
    } else {
        egui::Panel::right(Id::unique("sidebar-shortcuts"))
    };
    panel
        .exact_size(super::craft_theme::RAIL_WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::NONE
                .fill(ui.visuals().panel_fill)
                .inner_margin(egui::Margin::symmetric(0, 8)),
        )
        .show_inside(ui, |ui| {
            egui::Panel::bottom(Id::unique("sidebar-shortcuts-footer"))
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let response = ui
                            .button(icons::DOTS_THREE)
                            .on_hover_text(app.tr("Sidebar rail preferences"));
                        if interactive {
                            egui::Popup::menu(&response).show(|ui| common_menu(app, ui));
                        }
                    });
                });
            egui::ScrollArea::vertical()
                .id_salt("sidebar-shortcut-items")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        let mut focusable = Vec::new();
                        for action in app
                            .sidebar_shortcuts
                            .order()
                            .into_iter()
                            .filter(|action| !before.hidden.contains(action))
                        {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::Vec2::splat(super::craft_theme::RAIL_BUTTON),
                                egui::Sense::hover(),
                            );
                            let hardware_available = action.available(app);
                            let available = hardware_available
                                && (action.tab().is_none() || !app.dock_state.locked);
                            let selected = action
                                .pane()
                                .is_some_and(|pane| app.sidebar_rail_state.open == Some(pane));
                            let response = ui
                                .add_enabled_ui(interactive, |ui| {
                                    super::action_customization::control(
                                        ui,
                                        rect,
                                        Id::unique(("sidebar-shortcut", action)),
                                        "",
                                        &app.tr(action.label()),
                                        available,
                                        selected,
                                        false,
                                    )
                                })
                                .inner;
                            let color = if !available {
                                ui.visuals().weak_text_color()
                            } else if selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().text_color()
                            };
                            let galley = ui.painter().layout_no_wrap(
                                action.icon().into(),
                                egui::FontId::proportional(19.0),
                                color,
                            );
                            let origin = rect.center() - galley.mesh_bounds.center().to_vec2();
                            ui.painter()
                                .with_clip_rect(rect)
                                .galley(origin, galley, color);
                            focusable.push(response.id);
                            if interactive {
                                super::action_customization::register_drag(
                                    &response,
                                    super::action_customization::moved_for_drag(ui.ctx()),
                                    |_| RailDrag { action },
                                );
                            }
                            if let Some(drag) = response.dnd_hover_payload::<RailDrag>() {
                                let after = ui.input(|input| {
                                    input
                                        .pointer
                                        .interact_pos()
                                        .is_some_and(|pos| pos.y > rect.center().y)
                                });
                                ui.painter().hline(
                                    rect.x_range(),
                                    if after { rect.bottom() } else { rect.top() },
                                    egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
                                );
                                if super::action_customization::drop_payload::<RailDrag>(&response)
                                    .is_some()
                                {
                                    let mut order = app.sidebar_shortcuts.order();
                                    super::action_customization::move_action(
                                        &mut order,
                                        drag.action,
                                        action,
                                        after,
                                    );
                                    app.sidebar_shortcuts.order = order;
                                }
                            }
                            if response.clicked()
                                && available
                                && !egui::DragAndDrop::has_any_payload(ui.ctx())
                            {
                                action.activate(app, ui.ctx());
                            }
                            if interactive {
                                response
                                    .clone()
                                    .context_menu(|ui| item_menu(action, app, ui));
                            }
                            response
                                .on_hover_text(format!(
                                    "{}\n{}",
                                    app.tr(action.label()),
                                    app.tr(if action.tab().is_some() {
                                        "Drag to reorder or dock in a workspace pane"
                                    } else {
                                        "Drag to reorder; click to open the dialog"
                                    })
                                ))
                                .on_disabled_hover_text(app.tr(if hardware_available {
                                    "Workspace is locked"
                                } else {
                                    "Requires connected RF-capable hardware"
                                }));
                        }
                        if interactive
                            && let Some(index) = focusable
                                .iter()
                                .position(|id| ui.memory(|m| m.has_focus(*id)))
                        {
                            let next = ui.input_mut(|input| {
                                if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                                    Some(index.saturating_sub(1))
                                } else if input
                                    .consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                                {
                                    Some((index + 1).min(focusable.len() - 1))
                                } else if input.consume_key(egui::Modifiers::NONE, egui::Key::Home)
                                {
                                    Some(0)
                                } else if input.consume_key(egui::Modifiers::NONE, egui::Key::End) {
                                    Some(focusable.len() - 1)
                                } else {
                                    None
                                }
                            });
                            if let Some(next) = next {
                                ui.memory_mut(|m| m.request_focus(focusable[next]));
                                if let Some(response) = ui.ctx().read_response(focusable[next]) {
                                    response.scroll_to_me(None);
                                }
                            }
                        }
                        let blank = Rect::from_min_max(ui.cursor().min, ui.max_rect().max)
                            .intersect(ui.clip_rect());
                        let blank = ui.interact(
                            blank,
                            Id::unique("sidebar-shortcuts-empty"),
                            egui::Sense::click(),
                        );
                        if interactive {
                            blank.context_menu(|ui| common_menu(app, ui));
                        }
                        if let Some(drag) = egui::DragAndDrop::payload::<RailDrag>(ui.ctx()) {
                            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                                egui::DragAndDrop::clear_payload(ui.ctx());
                            } else if let Some(pos) = ui.ctx().pointer_interact_pos() {
                                let painter = ui.ctx().layer_painter(egui::LayerId::new(
                                    egui::Order::Tooltip,
                                    Id::unique("rail-drag-preview"),
                                ));
                                painter.text(
                                    pos + egui::vec2(18.0, 16.0),
                                    egui::Align2::LEFT_TOP,
                                    format!(
                                        "{}  {}",
                                        drag.action.icon(),
                                        app.tr(drag.action.label())
                                    ),
                                    egui::FontId::proportional(14.0),
                                    ui.visuals().text_color(),
                                );
                            }
                        }
                    });
                });
        });
    if before != app.sidebar_shortcuts {
        app.deferred_config_save = true;
        ui.ctx().request_repaint();
    }
}

/// Called in the visible workspace body, outside its scroll content. No overlay input widget
/// is allocated: normal fields retain focus and pane gestures keep their existing ownership.
pub(crate) fn drop_target(app: &PealayerApp, ui: &egui::Ui, anchor: Option<Tab>) {
    if app.dock_state.locked
        || !ui.is_enabled()
        || !ui.ctx().memory(|m| m.is_above_modal_layer(ui.layer_id()))
    {
        return;
    }
    if anchor.is_some_and(|tab| !app.dock_state.layout.contains(&tab)) {
        return;
    }
    let Some(drag) = egui::DragAndDrop::payload::<RailDrag>(ui.ctx()) else {
        return;
    };
    let Some(tab) = drag.action.tab() else {
        return;
    }; // dialogs never become fake dock panels
    if Some(tab) == anchor {
        return;
    }
    let rect = ui.max_rect().intersect(ui.clip_rect());
    let Some(pointer) = ui
        .ctx()
        .pointer_interact_pos()
        .filter(|pos| rect.contains(*pos))
    else {
        return;
    };
    if ui.ctx().layer_id_at(pointer) != Some(ui.layer_id()) {
        return;
    }
    let Some(zone) = craft_ui::docking::drop_zone(rect, 0.0, 40.0, pointer) else {
        return;
    };
    let placement = craft_ui::docking::Placement::Split(zone);
    let points = craft_ui::docking::zone_shape(rect, 0.0, 40.0, zone).unwrap_or_default();
    ui.ctx()
        .layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            Id::unique("rail-dock-preview"),
        ))
        .with_clip_rect(rect)
        .add(egui::Shape::convex_polygon(
            points,
            ui.visuals().selection.bg_fill.gamma_multiply(0.3),
            egui::Stroke::new(1.0, ui.visuals().selection.bg_fill),
        ));
    if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
        egui::DragAndDrop::clear_payload(ui.ctx());
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                drop_id(),
                PendingDrop {
                    tab,
                    anchor,
                    placement,
                },
            )
        });
    }
}

pub(crate) fn commit_drop(app: &mut PealayerApp, ctx: &egui::Context) {
    let Some(drop) = ctx.data_mut(|data| {
        let pending = data.get_temp::<PendingDrop>(drop_id());
        data.remove::<PendingDrop>(drop_id());
        pending
    }) else {
        return;
    };
    if app.dock_state.locked {
        return;
    }
    use craft_ui::docking::{Action, Permissions};
    let action = match drop.anchor {
        Some(anchor) if app.dock_state.layout.contains(&drop.tab) => Action::Move {
            panel: drop.tab,
            anchor,
            placement: drop.placement,
        },
        Some(anchor) => Action::OpenAt {
            panel: drop.tab,
            anchor,
            placement: drop.placement,
        },
        None => Action::Open {
            panel: drop.tab,
            anchor: None,
        },
    };
    match app
        .dock_state
        .apply_all([action, Action::Activate { panel: drop.tab }], |_| {
            Permissions::default()
        }) {
        Ok(_) => {
            if Pane::try_from(drop.tab).ok() == app.sidebar_rail_state.open {
                app.sidebar_rail_state.open = None;
            }
            app.pending_tab_reveals.push(drop.tab);
            app.deferred_config_save = true;
            ctx.request_repaint();
        }
        Err(error) => app.config_status = format!("Cannot dock shortcut: {error:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 650.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                show(app, ui);
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    drop_target(app, ui, Some(Tab::ProgramMonitor));
                });
                commit_drop(app, ctx);
            },
        );
        output.textures_delta.clear();
        output
    }
    fn button(pos: egui::Pos2, which: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: which,
            pressed,
            modifiers: Default::default(),
        }
    }
    fn point(ctx: &egui::Context, action: RailAction) -> egui::Pos2 {
        ctx.read_response(Id::unique(("sidebar-shortcut", action)))
            .unwrap()
            .rect
            .center()
    }
    fn text(shape: &egui::Shape, needle: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text().contains(needle),
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| self::text(shape, needle)),
            _ => false,
        }
    }
    #[test]
    fn sidebar_shortcut_defaults_are_bounded_unique_and_round_trip() {
        let shortcuts = RailShortcuts::default();
        assert_eq!(shortcuts.order().len(), RailAction::ALL.len());
        assert_eq!(
            shortcuts
                .order()
                .into_iter()
                .filter(|action| !shortcuts.hidden.contains(action))
                .collect::<Vec<_>>(),
            RailAction::ALL[..4]
        );
        let mut malformed = shortcuts.clone();
        malformed.order = vec![RailAction::Timeline, RailAction::Timeline];
        assert_eq!(malformed.order().len(), RailAction::ALL.len());
        let restored: RailShortcuts =
            serde_json::from_value(serde_json::to_value(&malformed).unwrap()).unwrap();
        assert_eq!(restored, malformed);
        assert_eq!(restored.order()[0], RailAction::Timeline);
        assert!(
            RailAction::ALL
                .iter()
                .filter(|action| action.tab().is_none())
                .all(|action| shortcuts.hidden.contains(action))
        );
    }
    #[test]
    fn sidebar_shortcut_context_menus_work_for_icons_and_empty_space_without_opening_anything() {
        let _guard = crate::app::tests::lock_app_tests();
        for width in [390.0, 800.0, 1200.0] {
            for rtl in [false, true] {
                for icon in [false, true] {
                    let ctx = egui::Context::default();
                    let mut app = PealayerApp::default();
                    app.rtl = rtl;
                    let original = app.dock_state.clone();
                    frame(&ctx, &mut app, width, vec![]);
                    let target = if icon {
                        point(&ctx, RailAction::MediaInspector)
                    } else {
                        ctx.read_response(Id::unique("sidebar-shortcuts-empty"))
                            .unwrap()
                            .rect
                            .center()
                    };
                    frame(
                        &ctx,
                        &mut app,
                        width,
                        vec![
                            egui::Event::PointerMoved(target),
                            button(target, egui::PointerButton::Secondary, true),
                        ],
                    );
                    frame(
                        &ctx,
                        &mut app,
                        width,
                        vec![button(target, egui::PointerButton::Secondary, false)],
                    );
                    let output = frame(&ctx, &mut app, width, vec![]);
                    assert!(
                        output
                            .shapes
                            .iter()
                            .any(|shape| text(&shape.shape, "Show / hide actions")),
                        "missing context menu width={width} rtl={rtl} icon={icon}"
                    );
                    assert!(
                        output
                            .shapes
                            .iter()
                            .any(|shape| text(&shape.shape, "Preferences..."))
                    );
                    if icon {
                        assert!(
                            output
                                .shapes
                                .iter()
                                .any(|shape| text(&shape.shape, "Dock to Workspace"))
                        );
                    }
                    assert_eq!(app.dock_state, original);
                    assert_eq!(app.sidebar_rail_state.open, None);
                    assert!(!app.rf.open);
                }
            }
        }
    }
    #[test]
    fn sidebar_shortcut_primary_drag_reorders_without_opening_or_changing_workspace() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = PealayerApp::default();
        let original = app.dock_state.clone();
        frame(&ctx, &mut app, 800.0, vec![]);
        let source = point(&ctx, RailAction::HardwareMonitor);
        let destination = point(&ctx, RailAction::EffectsLibrary) + egui::vec2(0.0, 10.0);
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![
                egui::Event::PointerMoved(source),
                button(source, egui::PointerButton::Primary, true),
            ],
        );
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![egui::Event::PointerMoved(destination)],
        );
        assert!(egui::DragAndDrop::has_payload_of_type::<RailDrag>(&ctx));
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![button(destination, egui::PointerButton::Primary, false)],
        );
        assert_eq!(
            app.sidebar_shortcuts.order()[3],
            RailAction::HardwareMonitor
        );
        assert_eq!(app.dock_state, original);
        assert_eq!(app.sidebar_rail_state.open, None);
    }
    #[test]
    fn sidebar_shortcut_cancel_and_secondary_movement_do_not_change_layout_or_order() {
        let _guard = crate::app::tests::lock_app_tests();
        for secondary in [true, false] {
            let ctx = egui::Context::default();
            let mut app = PealayerApp::default();
            frame(&ctx, &mut app, 800.0, vec![]);
            let original = app.dock_state.clone();
            let shortcuts = app.sidebar_shortcuts.clone();
            let pos = point(&ctx, RailAction::HardwareMonitor);
            let which = if secondary {
                egui::PointerButton::Secondary
            } else {
                egui::PointerButton::Primary
            };
            frame(
                &ctx,
                &mut app,
                800.0,
                vec![egui::Event::PointerMoved(pos), button(pos, which, true)],
            );
            frame(
                &ctx,
                &mut app,
                800.0,
                vec![egui::Event::PointerMoved(pos + egui::vec2(-180.0, 100.0))],
            );
            if !secondary {
                frame(
                    &ctx,
                    &mut app,
                    800.0,
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    }],
                );
            }
            assert!(!egui::DragAndDrop::has_payload_of_type::<RailDrag>(&ctx));
            frame(&ctx, &mut app, 800.0, vec![button(pos, which, false)]);
            assert_eq!(app.dock_state, original);
            assert_eq!(app.sidebar_shortcuts, shortcuts);
        }
    }
    #[test]
    fn sidebar_shortcut_drop_opens_hidden_panel_at_destination_atomically_and_closes_sidebar_copy()
    {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = PealayerApp::default();
        app.dock_state
            .set_visible(
                Tab::EffectsLibrary,
                false,
                Some(Tab::ProgramMonitor),
                |_| Default::default(),
            )
            .unwrap();
        app.sidebar_rail_state.open = Some(Pane::EffectsLibrary);
        let untouched: Vec<_> = app
            .dock_state
            .layout
            .panels()
            .into_iter()
            .copied()
            .collect();
        let destination = egui::pos2(200.0, 250.0);
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![egui::Event::PointerMoved(destination)],
        );
        egui::DragAndDrop::set_payload(
            &ctx,
            RailDrag {
                action: RailAction::EffectsLibrary,
            },
        );
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![button(destination, egui::PointerButton::Primary, true)],
        );
        assert!(
            !app.dock_state.layout.contains(&Tab::EffectsLibrary),
            "hover must not reveal a panel"
        );
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![button(destination, egui::PointerButton::Primary, false)],
        );
        assert!(app.dock_state.layout.contains(&Tab::EffectsLibrary));
        assert_eq!(
            app.dock_state
                .layout
                .panels()
                .iter()
                .filter(|tab| ***tab == Tab::EffectsLibrary)
                .count(),
            1
        );
        assert!(
            untouched
                .into_iter()
                .all(|tab| app.dock_state.layout.contains(&tab))
        );
        assert_eq!(app.sidebar_rail_state.open, None);
        app.dock_state.validate().unwrap();
    }
    #[test]
    fn sidebar_shortcut_dialog_drop_and_workspace_lock_cannot_dock_or_actuate() {
        let _guard = crate::app::tests::lock_app_tests();
        for (action, locked) in [
            (RailAction::RfManager, false),
            (RailAction::MediaInspector, true),
        ] {
            let ctx = egui::Context::default();
            let mut app = PealayerApp::default();
            app.dock_state.locked = locked;
            let original = app.dock_state.clone();
            let pos = egui::pos2(200.0, 200.0);
            frame(&ctx, &mut app, 800.0, vec![egui::Event::PointerMoved(pos)]);
            egui::DragAndDrop::set_payload(&ctx, RailDrag { action });
            frame(
                &ctx,
                &mut app,
                800.0,
                vec![button(pos, egui::PointerButton::Primary, true)],
            );
            frame(
                &ctx,
                &mut app,
                800.0,
                vec![button(pos, egui::PointerButton::Primary, false)],
            );
            assert_eq!(app.dock_state, original);
            assert!(!app.rf.open);
            assert_eq!(app.sidebar_rail_state.open, None);
        }
    }
    #[test]
    fn sidebar_shortcut_modal_cancels_drag_and_blocks_activation() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = PealayerApp::default();
        let original = app.dock_state.clone();
        for _ in 0..2 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 650.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    show(&mut app, ui);
                    egui::Modal::new(Id::unique("rail-test-modal")).show(&ctx, |ui| {
                        ui.label("Modal owns input");
                    });
                },
            )
            .drop_without_applying_deltas();
        }
        egui::DragAndDrop::set_payload(
            &ctx,
            RailDrag {
                action: RailAction::MediaInspector,
            },
        );
        frame(&ctx, &mut app, 800.0, vec![]);
        assert!(!egui::DragAndDrop::has_payload_of_type::<RailDrag>(&ctx));
        assert_eq!(app.dock_state, original);
        assert_eq!(app.sidebar_rail_state.open, None);
    }
    #[test]
    fn sidebar_shortcut_dialog_activation_and_keyboard_navigation_reuse_existing_state() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = PealayerApp::default();
        app.sidebar_shortcuts
            .set_visible(RailAction::KeyboardShortcuts, true);
        frame(&ctx, &mut app, 800.0, vec![]);
        ctx.memory_mut(|m| {
            m.request_focus(Id::unique((
                "sidebar-shortcut",
                RailAction::HardwareMonitor,
            )))
        });
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![egui::Event::Key {
                key: egui::Key::End,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        assert!(ctx.memory(|m| m.has_focus(Id::unique((
            "sidebar-shortcut",
            RailAction::KeyboardShortcuts
        )))));
        let pos = point(&ctx, RailAction::KeyboardShortcuts);
        let original = app.dock_state.clone();
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![
                egui::Event::PointerMoved(pos),
                button(pos, egui::PointerButton::Primary, true),
            ],
        );
        frame(
            &ctx,
            &mut app,
            800.0,
            vec![button(pos, egui::PointerButton::Primary, false)],
        );
        assert!(app.show_shortcuts_dialog);
        assert_eq!(app.dock_state, original);
        RailAction::RfManager.activate(&mut app, &ctx);
        assert!(
            !app.rf.open,
            "disconnected RF shortcut cannot dispatch a catalog read"
        );
    }
}
