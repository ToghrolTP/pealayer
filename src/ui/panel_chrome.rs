//! Application-owned native panel glyphs. Painting never changes a hit target.
use super::layout::{DockViewState, PealayerTab as Tab, PealayerTabViewer};
use craft_ui::{
    docking::{Layout, Node},
    layout::{SplitAxis, SplitSize},
};
use eframe::egui;

#[derive(Clone, Copy)]
pub(crate) struct HeaderOptions {
    pub close: bool,
    pub collapse: bool,
}
impl HeaderOptions {
    pub fn current() -> Self {
        let config = crate::platform::interop::get_live_config();
        Self {
            close: config.craft_show_close_buttons,
            collapse: config.craft_show_collapse_buttons,
        }
    }
}

pub(crate) fn paint_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    glyph: &str,
    color: egui::Color32,
) {
    let size = 14.0_f32.min((rect.width().min(rect.height()) - 4.0).max(1.0));
    let galley = painter.layout_no_wrap(glyph.into(), egui::FontId::proportional(size), color);
    let origin = rect.center() - galley.mesh_bounds.center().to_vec2();
    painter
        .with_clip_rect(rect.intersect(painter.clip_rect()))
        .galley(origin, galley, color);
}

pub(crate) fn button(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    glyph: &str,
    label: &str,
) -> egui::Response {
    let interactive = ui
        .ctx()
        .memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
    let response = ui
        .add_enabled_ui(interactive, |ui| {
            ui.interact(rect, id, egui::Sense::click())
        })
        .inner
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let tokens = super::craft_theme::current(ui.ctx());
    if response.hovered() || response.is_pointer_button_down_on() || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            3.0,
            if response.is_pointer_button_down_on() {
                tokens.pressed
            } else {
                tokens.hover
            },
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.0, tokens.focus),
            egui::StrokeKind::Inside,
        );
    }
    paint_icon(
        ui.painter(),
        rect,
        glyph,
        if !response.enabled() {
            tokens.text_faint
        } else if response.hovered() || response.has_focus() {
            tokens.tab_text_active
        } else {
            tokens.icon
        },
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, response.enabled(), label));
    response.on_hover_text(label)
}

pub(crate) fn craft_actions(rect: egui::Rect, rtl: bool, collapse: bool) -> Vec<egui::Rect> {
    let count = if collapse { 2 } else { 1 };
    let side = 24.0_f32
        .min(rect.height())
        .min((rect.width() - 4.0).max(0.0) / count as f32);
    let last = rect.right() - 2.0 - side * 0.5;
    (0..count)
        .map(|index| {
            let slot = egui::Rect::from_center_size(
                egui::pos2(last - side * (count - index - 1) as f32, rect.center().y),
                egui::Vec2::splat(side),
            );
            if rtl {
                egui::Rect::from_x_y_ranges(
                    (rect.left() + rect.right() - slot.right())
                        ..=(rect.left() + rect.right() - slot.left()),
                    slot.y_range(),
                )
            } else {
                slot
            }
        })
        .collect()
}

fn tab_id(area: egui::Id, panel: Tab) -> egui::Id {
    area.with(("tab", panel))
}

pub(crate) fn collapsed(ctx: &egui::Context, panel: Tab) -> bool {
    DockViewState::load(ctx).collapsed.contains(&panel)
}

pub(crate) fn group_collapsed(
    app: &crate::app::PealayerApp,
    ctx: &egui::Context,
    panels: &[Tab],
    active: Tab,
) -> bool {
    if panels.iter().all(|panel| collapsed(ctx, *panel)) {
        return true;
    }
    // Native dock groups/floating windows already own minimization. Ask their
    // model rather than inferring it from inactive/hidden presentation labels.
    let mut expanded = app.dock_state.layout.clone();
    expanded
        .apply(craft_ui::docking::Action::SetMinimized {
            panel: active,
            minimized: false,
        })
        .is_ok()
        && expanded != app.dock_state.layout
}

pub(crate) fn toggle_collapse(
    app: &mut crate::app::PealayerApp,
    ctx: &egui::Context,
    panels: &[Tab],
) {
    if app.dock_state.locked {
        return;
    }
    let mut view = DockViewState::load(ctx);
    let active = panels
        .iter()
        .copied()
        .find(|panel| {
            app.dock_state.layout.panel_state(panel) == craft_ui::docking::PanelState::Front
        })
        .or_else(|| panels.first().copied());
    let Some(active) = active else {
        return;
    };
    let collapsed = group_collapsed(app, ctx, panels, active);
    view.collapsed.retain(|panel| !panels.contains(panel));
    let mut candidate = app.dock_state.clone();
    if candidate
        .apply_all(
            [craft_ui::docking::Action::SetMinimized {
                panel: active,
                minimized: !collapsed,
            }],
            |_| Default::default(),
        )
        .is_ok()
    {
        app.dock_state = candidate;
    } else if !collapsed {
        view.collapsed.extend_from_slice(panels);
    }
    view.store(ctx);
    app.deferred_config_save = true;
    ctx.request_repaint();
}

pub(crate) fn expand_group(view: &mut DockViewState, layout: &Layout<Tab>, panel: Tab) {
    fn find(node: &Node<Tab>, panel: Tab) -> Option<&[Tab]> {
        match node {
            Node::Tabs { panels, .. } if panels.contains(&panel) => Some(panels),
            Node::Split { first, second, .. } => find(first, panel).or_else(|| find(second, panel)),
            _ => None,
        }
    }
    let group = layout
        .root
        .as_ref()
        .and_then(|node| find(node, panel))
        .or_else(|| {
            layout
                .floating
                .iter()
                .find_map(|window| find(&window.root, panel))
        });
    view.collapsed
        .retain(|tab| !group.map_or(*tab == panel, |group| group.contains(tab)));
}

/// Only view geometry changes: never replace tabs with stacks, drop members,
/// or write collapsed dimensions over the user's normal split ratios.
fn collapse_projection(
    node: &mut Node<Tab>,
    collapsed: &[Tab],
    header: f32,
    gap: f32,
) -> Option<f32> {
    match node {
        Node::Tabs { panels, .. } => panels
            .iter()
            .all(|panel| collapsed.contains(panel))
            .then_some(header),
        Node::Split {
            axis,
            size,
            first,
            second,
        } => {
            let a = collapse_projection(first, collapsed, header, gap);
            let b = collapse_projection(second, collapsed, header, gap);
            if *axis == SplitAxis::Vertical {
                if let Some(height) = a {
                    *size = SplitSize::FixedFirst(height);
                } else if let Some(height) = b {
                    *size = SplitSize::FixedSecond(height);
                }
                a.zip(b).map(|(a, b)| a + b + gap)
            } else {
                a.zip(b).map(|(a, b)| a.max(b))
            }
        }
        _ => None,
    }
}

fn projected_layout(
    layout: &Layout<Tab>,
    view: &DockViewState,
    header: f32,
    gap: f32,
) -> Layout<Tab> {
    let mut projected = layout.clone();
    if let Some(root) = projected.root.as_mut() {
        collapse_projection(root, &view.collapsed, header, gap);
    }
    for window in &mut projected.floating {
        if let Some(height) = collapse_projection(&mut window.root, &view.collapsed, header, gap) {
            window.rect[3] = height;
        }
    }
    projected
}

/// Center the icon and caption ink independently. Craft centers their shared
/// line box, but an icon fallback face and the UI font have different baselines.
/// Work on the final (possibly elided) galley without changing its logical size,
/// text, tab hit target or horizontal glyph positions.
fn center_tab_ink(text: &mut egui::epaint::TextShape, rect: egui::Rect, icon: char) {
    let target = rect.center().y - text.pos.y;
    let galley = std::sync::Arc::make_mut(&mut text.galley);
    galley.mesh_bounds = egui::Rect::NOTHING;
    for placed in &mut galley.rows {
        let row = std::sync::Arc::make_mut(&mut placed.row);
        let mut bounds = [egui::Rect::NOTHING; 2];
        for glyph in &row.glyphs {
            if glyph.uv_rect.is_nothing() {
                continue;
            }
            let start = glyph.first_vertex as usize;
            for vertex in &row.visuals.mesh.vertices[start..start + 4] {
                if vertex.color.a() != 0 {
                    bounds[usize::from(glyph.chr != icon)].extend_with(vertex.pos);
                }
            }
        }
        let offsets = bounds.map(|bounds| {
            if bounds.is_positive() {
                target - placed.pos.y - bounds.center().y
            } else {
                0.0
            }
        });
        for glyph in &mut row.glyphs {
            let offset = offsets[usize::from(glyph.chr != icon)];
            glyph.pos.y += offset;
            if !glyph.uv_rect.is_nothing() {
                let start = glyph.first_vertex as usize;
                for vertex in &mut row.visuals.mesh.vertices[start..start + 4] {
                    vertex.pos.y += offset;
                }
            }
        }
        row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
        galley.mesh_bounds |= row.visuals.mesh_bounds.translate(placed.pos.to_vec2());
    }
}

/// Align visible tab ink and add application-owned close controls on each
/// actual tab. Overflowed tabs never acquire invisible paint or hit targets.
fn decorate_tabs(
    app: &mut crate::app::PealayerApp,
    ui: &mut egui::Ui,
    area: egui::Id,
    blocked: bool,
    options: HeaderOptions,
) {
    let tabs: Vec<_> = app
        .dock_state
        .layout
        .panels()
        .into_iter()
        .copied()
        .collect();
    let mut close = None;
    for panel in tabs {
        let Some(tab) = ui.ctx().read_response(tab_id(area, panel)) else {
            continue;
        };
        let rect = tab.rect;
        if rect.width() < 28.0 || rect.height() < 16.0 {
            continue;
        }
        // read_response can retain an overflowed tab from the previous pass.
        // Require its actual title paint in this pass and use its own layer
        // and clip (floating panes do not share the docked parent's clip).
        let title = format!("{} {}", panel.icon(), panel.title(app));
        let mut clip = None;
        let mut fill = super::craft_theme::current(ui.ctx()).tab_bg;
        ui.ctx().graphics_mut(|graphics| {
            let Some(list) = graphics.get_mut(tab.layer_id) else {
                return;
            };
            let indices: Vec<_> = list
                .all_entries()
                .enumerate()
                .filter_map(|(index, shape)| {
                    matches!(&shape.shape, egui::Shape::Text(text)
                    if text.galley.text().contains(&title)
                        && text.visual_bounding_rect().intersects(rect)
                        && shape.clip_rect.intersects(rect))
                    .then_some(egui::layers::ShapeIdx(index))
                })
                .collect();
            for index in indices {
                list.mutate_shape(index, |shape| {
                    if let egui::Shape::Text(text) = &mut shape.shape {
                        center_tab_ink(text, rect, panel.icon().chars().next().unwrap());
                    }
                });
            }
        });
        ui.ctx()
            .layer_painter(tab.layer_id)
            .for_each_shape(|shape| {
                if let egui::Shape::Rect(shape) = &shape.shape
                    && shape.rect == rect
                {
                    fill = shape.fill;
                }
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text().contains(&title)
                    && text.visual_bounding_rect().intersects(rect)
                    && shape.clip_rect.intersects(rect)
                {
                    clip = Some(shape.clip_rect.intersect(rect));
                }
            });
        let Some(clip) = clip else {
            continue;
        };
        if !options.close {
            continue;
        }
        let side = rect.height().min(24.0);
        let center = egui::pos2(
            if app.rtl {
                rect.left() + side * 0.5
            } else {
                rect.right() - side * 0.5
            },
            rect.center().y,
        );
        let slot = egui::Rect::from_center_size(center, egui::Vec2::splat(side));
        // The title reserves this gutter; opaque coverage also keeps a very
        // narrow elided title out of the close target.
        let mut child = ui.new_child(egui::UiBuilder::new().layer_id(tab.layer_id).max_rect(slot));
        child.set_clip_rect(clip);
        child.painter().rect_filled(slot, 0.0, fill);
        let response = child
            .add_enabled_ui(!blocked && !app.dock_state.locked, |ui| {
                button(
                    ui,
                    slot,
                    area.with(("close-panel", panel)),
                    crate::ui::icons::X,
                    &format!("{}: {}", app.tr("Close panel"), panel.title(app)),
                )
            })
            .inner;
        if response.clicked() {
            close = Some(panel);
        }
    }
    if let Some(panel) = close {
        match app
            .dock_state
            .set_visible(panel, false, Some(Tab::ProgramMonitor), |_| {
                Default::default()
            }) {
            Ok(()) => {
                app.sync_terminal_visibility_from_workspace();
                app.deferred_config_save = true;
                ui.ctx().request_repaint();
            }
            Err(error) => log::warn!("Cannot close workspace panel: {error:?}"),
        }
    }
}

pub(crate) fn show(app: &mut crate::app::PealayerApp, ui: &mut egui::Ui, raise: Option<Tab>) {
    let area = egui::Id::new("workspace_dock_area");
    let mut view = DockViewState::load(ui.ctx());
    let before = view.clone();
    let mut style = craft_ui::docking::DockStyle::from_ui(ui);
    super::craft_theme::apply_dock_style(&mut style, &super::craft_theme::current(ui.ctx()));
    style.right_to_left = app.rtl;
    // Normal panels keep Craft's original 40-point body minimum through
    // DockContent::limits. Collapsed panels need a true zero-height body.
    let minimum_body = style.min_pane;
    style.min_pane = 0.0;
    let layout = projected_layout(&app.dock_state.layout, &view, style.tab_height, style.gap);
    let blocked = !ui
        .ctx()
        .memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
    app.workspace_rendering = true;
    ui.ctx()
        .data_mut(|data| data.insert_temp(area.with("collapsed-bodies"), Vec::<egui::Rect>::new()));
    let mut renderer = craft_ui::docking::DockArea::new(area)
        .locked(app.dock_state.locked)
        .input_blocked(blocked)
        .maximized(view.maximized)
        .auto_collapse_iconic(view.auto_collapse);
    if let Some(panel) = raise {
        renderer = renderer.raise_panel(panel);
    }
    let options = HeaderOptions::current();
    let output = renderer.show_content(
        ui,
        &layout,
        &style,
        &mut PealayerTabViewer {
            app,
            collapsed: view.collapsed.clone(),
            header_options: options,
            minimum_body,
        },
    );
    #[cfg(test)]
    assert!(
        output.error.is_none(),
        "invalid CraftUI projection: {:?}",
        output.error
    );
    // Collapsed grid cells keep their columns, but expose the workspace rather
    // than an empty bordered body. Fully collapsed rows give their height back.
    let bodies = ui
        .ctx()
        .data_mut(|data| data.remove_temp::<Vec<egui::Rect>>(area.with("collapsed-bodies")))
        .unwrap_or_default();
    let background = super::craft_theme::current(ui.ctx()).app_bg;
    for rect in bodies {
        ui.painter()
            .rect_filled(rect.expand(0.5).intersect(ui.clip_rect()), 0.0, background);
    }
    decorate_tabs(app, ui, area, blocked, options);
    app.workspace_rendering = false;
    view = DockViewState::load(ui.ctx());
    for action in &output.actions {
        if let craft_ui::docking::Action::Activate { panel } = action {
            expand_group(&mut view, &app.dock_state.layout, *panel);
        }
    }
    if !output.actions.is_empty() {
        let before = app.dock_state.clone();
        if let Err(error) = app
            .dock_state
            .apply_all(output.actions, |_| Default::default())
        {
            log::warn!("Workspace action rejected: {error:?}");
        }
        if app.dock_state != before {
            app.sync_terminal_visibility_from_workspace();
            app.deferred_config_save = true;
            ui.ctx().request_repaint();
        }
    }
    // Header callbacks can update collapse state. Merge events into that fresh
    // view, not the stale pre-paint clone.
    view.apply(output.events, &app.dock_state);
    if view != before {
        view.store(ui.ctx());
        app.deferred_config_save = true;
        ui.ctx().request_repaint();
    }
    if !app.pending_tab_reveals.is_empty() {
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::PealayerApp,
        ui::layout::{PealayerTab as Tab, PealayerTabViewer},
    };

    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn terminal_uses_craft_individual_close_with_modal_and_lock_guards() {
        let _guard = crate::app::tests::lock_app_tests();
        for guarded in ["none", "modal", "locked"] {
            let (ctx, mut app) = fixture(false);
            app.dock_state = craft_ui::docking::Workspace::new(craft_ui::docking::Layout {
                root: Some(craft_ui::docking::Node::Tabs { panels: vec![Tab::Timeline, Tab::Terminal], active: 1 }),
                floating: Vec::new(),
            });
            app.sync_terminal_visibility_from_workspace();
            for _ in 0..3 { frame(&ctx, &mut app, vec![], false); }
            let close_id = egui::Id::new("workspace_dock_area").with(("close-panel", Tab::Terminal));
            let point = ctx.read_response(close_id).expect("Terminal shares native individual close target").rect.center();
            let sessions = app.terminal_state.status()["sessions"].clone();
            app.dock_state.locked = guarded == "locked";
            let blocked = guarded == "modal";
            frame(&ctx, &mut app, vec![egui::Event::PointerMoved(point)], blocked);
            for pressed in [true, false] {
                frame(&ctx, &mut app, vec![egui::Event::PointerButton {
                    pos: point, button: egui::PointerButton::Primary, pressed, modifiers: Default::default(),
                }], blocked);
            }
            assert_eq!(app.dock_state.layout.contains(&Tab::Terminal), guarded != "none");
            assert_eq!(app.terminal_state.preferences.enabled, guarded != "none");
            assert!(app.dock_state.layout.contains(&Tab::Timeline));
            assert_eq!(app.terminal_state.status()["sessions"], sessions);
        }
    }

    fn frame(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        events: Vec<egui::Event>,
        blocked: bool,
    ) -> egui::FullOutput {
        frame_width(ctx, app, events, blocked, 700.0)
    }

    fn frame_width(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        events: Vec<egui::Event>,
        blocked: bool,
        width: f32,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                if blocked {
                    egui::Modal::new(egui::Id::new("panel-chrome-modal")).show(ctx, |ui| {
                        ui.label("Modal");
                    });
                }
                show(app, ui, None);
            },
        );
        output.textures_delta.clear();
        output
    }

    fn click(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        pos: egui::Pos2,
        blocked: bool,
    ) -> egui::FullOutput {
        frame(ctx, app, vec![egui::Event::PointerMoved(pos)], blocked);
        frame(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            blocked,
        );
        frame(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            blocked,
        )
    }

    fn fixture(rtl: bool) -> (egui::Context, PealayerApp) {
        let ctx = egui::Context::default();
        crate::ui::i18n::configure_ui_fonts(&ctx, rtl);
        let mut app = PealayerApp::default();
        app.engine_handle = crate::four_d::engine::memory_only_test_engine();
        app.rtl = rtl;
        app.dock_state.layout = craft_ui::docking::Layout {
            root: Some(craft_ui::docking::Node::Tabs {
                panels: vec![Tab::EffectControls, Tab::MediaInspector],
                active: 0,
            }),
            floating: vec![],
        };
        app.dock_state.hidden.clear();
        (ctx, app)
    }

    fn control_centers(output: &egui::FullOutput) -> [egui::Pos2; 2] {
        [crate::ui::icons::DOTS_THREE, crate::ui::icons::X].map(|glyph| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == glyph => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .expect("real CraftUI header glyph missing")
        })
    }

    #[test]
    fn craft_tab_icon_caption_and_close_share_the_actual_header_center() {
        let _guard = crate::app::tests::lock_app_tests();
        let previous = crate::platform::interop::get_live_config();
        struct RestoreConfig(crate::config::AppConfig);
        impl Drop for RestoreConfig {
            fn drop(&mut self) {
                crate::platform::interop::set_live_config(self.0.clone());
            }
        }
        let _restore = RestoreConfig(previous.clone());
        for close in [false, true] {
            let mut config = previous.clone();
            config.craft_show_close_buttons = close;
            crate::platform::interop::set_live_config(config);
            for rtl in [false, true] {
                for scale in [1.0, 1.25, 1.5, 2.0] {
                    for dark in [false, true] {
                        for floating in [false, true] {
                            let (ctx, mut app) = fixture(rtl);
                            app.language = if rtl {
                                crate::config::AppLanguage::Persian
                            } else {
                                crate::config::AppLanguage::English
                            };
                            ctx.set_pixels_per_point(scale);
                            ctx.set_visuals(if dark {
                                egui::Visuals::dark()
                            } else {
                                egui::Visuals::light()
                            });
                            if floating {
                                app.dock_state.layout = Layout {
                                    root: None,
                                    floating: vec![craft_ui::docking::Floating::single(
                                        vec![Tab::EffectControls, Tab::MediaInspector],
                                        0,
                                        [30.0, 40.0, 600.0, 350.0],
                                    )],
                                };
                            }
                            for _ in 0..3 {
                                frame(&ctx, &mut app, vec![], false);
                            }
                            let output = frame(&ctx, &mut app, vec![], false);
                            for panel in [Tab::EffectControls, Tab::MediaInspector] {
                                let tab = ctx
                                    .read_response(tab_id(
                                        egui::Id::new("workspace_dock_area"),
                                        panel,
                                    ))
                                    .expect("actual visible upstream tab");
                                let title = format!("{} {}", panel.icon(), panel.title(&app));
                                let text = output.shapes.iter().find_map(|shape| match &shape.shape {
                                    egui::Shape::Text(text) if text.galley.text().contains(&title)
                                        && text.visual_bounding_rect().intersects(tab.rect) => Some(text),
                                    _ => None,
                                }).unwrap_or_else(|| panic!("painted {panel:?} caption missing: close={close}, RTL={rtl}, scale={scale}, dark={dark}, floating={floating}; title={title:?}; tab={:?}; text={:?}", tab.rect,
                                    output.shapes.iter().filter_map(|shape| match &shape.shape { egui::Shape::Text(t) => Some((t.galley.text(), t.visual_bounding_rect())), _=>None }).take(8).collect::<Vec<_>>()));
                                let mut ink = [egui::Rect::NOTHING; 2];
                                for placed in &text.galley.rows {
                                    for glyph in &placed.row.glyphs {
                                        if glyph.uv_rect.is_nothing() {
                                            continue;
                                        }
                                        let start = glyph.first_vertex as usize;
                                        let group = usize::from(
                                            glyph.chr != panel.icon().chars().next().unwrap(),
                                        );
                                        for vertex in
                                            &placed.row.visuals.mesh.vertices[start..start + 4]
                                        {
                                            if vertex.color.a() != 0 {
                                                ink[group].extend_with(
                                                    vertex.pos
                                                        + placed.pos.to_vec2()
                                                        + text.pos.to_vec2(),
                                                );
                                            }
                                        }
                                    }
                                }
                                for (part, ink) in ["icon", "caption"].into_iter().zip(ink) {
                                    assert!(ink.is_positive(), "missing {part}");
                                    assert!(
                                        (ink.center().y - tab.rect.center().y).abs() < 0.1,
                                        "{part} off center at RTL={rtl}, scale={scale}, dark={dark}, floating={floating}: {ink:?} vs {:?}",
                                        tab.rect
                                    );
                                }
                                if close {
                                    let button = ctx
                                        .read_response(
                                            egui::Id::new("workspace_dock_area")
                                                .with(("close-panel", panel)),
                                        )
                                        .expect("individual close target");
                                    assert!(
                                        (button.rect.center().y - tab.rect.center().y).abs() < 0.1
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        crate::platform::interop::set_live_config(previous);
    }

    #[test]
    fn craft_header_menu_and_close_are_real_stable_controls() {
        let _guard = crate::app::tests::lock_app_tests();
        for rtl in [false, true] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                for dark in [false, true] {
                    let (ctx, mut app) = fixture(rtl);
                    ctx.set_pixels_per_point(scale);
                    ctx.set_visuals(if dark {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    });
                    frame(&ctx, &mut app, vec![], false);
                    let output = frame(&ctx, &mut app, vec![], false);
                    let [menu, close] = control_centers(&output);
                    assert!((menu.y - close.y).abs() < 0.1);
                    assert_eq!(
                        menu.x < close.x,
                        rtl,
                        "tab close and trailing group menu must mirror"
                    );
                    let before = app.dock_state.clone();
                    let hover = frame(&ctx, &mut app, vec![egui::Event::PointerMoved(menu)], false);
                    assert_eq!(
                        control_centers(&hover),
                        [menu, close],
                        "hover must not move glyphs"
                    );
                    assert_eq!(
                        hover.platform_output.cursor_icon,
                        egui::CursorIcon::PointingHand
                    );
                    click(&ctx, &mut app, menu, false);
                    let open = frame(&ctx, &mut app, vec![], false);
                    assert!(open.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Preferences"))),"three dots failed to open the panel menu");
                    assert_eq!(app.dock_state, before, "opening menu changed the workspace");
                    frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::Key {
                            key: egui::Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Default::default(),
                        }],
                        false,
                    );
                    click(&ctx, &mut app, close, false);
                    assert!(!app.dock_state.layout.contains(&Tab::EffectControls));
                    assert!(app.dock_state.layout.contains(&Tab::MediaInspector));
                    assert!(
                        app.dock_state
                            .hidden
                            .iter()
                            .any(|(tab, _)| *tab == Tab::EffectControls)
                    );
                    assert!(app.dock_state.validate().is_ok());
                }
            }
        }
    }

    #[test]
    fn craft_close_respects_lock_and_modal_without_hiding_its_glyph() {
        let _guard = crate::app::tests::lock_app_tests();
        for (locked, blocked) in [(true, false), (false, true)] {
            let (ctx, mut app) = fixture(false);
            app.dock_state.locked = locked;
            frame(&ctx, &mut app, vec![], blocked);
            let output = frame(&ctx, &mut app, vec![], blocked);
            let close = control_centers(&output)[1];
            let before = app.dock_state.clone();
            click(&ctx, &mut app, close, blocked);
            assert_eq!(
                app.dock_state, before,
                "locked/modal header changed workspace"
            );
        }
    }

    #[test]
    fn craft_defaults_and_visibility_controls_do_not_change_workspace() {
        let _guard = crate::app::tests::lock_app_tests();
        let previous = crate::platform::interop::get_live_config();
        let mut config = previous.clone();
        config.craft_show_close_buttons = true;
        config.craft_show_collapse_buttons = false;
        crate::platform::interop::set_live_config(config.clone());
        let (ctx, mut app) = fixture(false);
        let before = app.dock_state.clone();
        frame(&ctx, &mut app, vec![], false);
        let output = frame(&ctx, &mut app, vec![], false);
        assert_eq!(output.shapes.iter().filter(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==crate::ui::icons::X)).count(),2,"each visible tab needs its own close button");
        assert!(
            ctx.read_response(
                egui::Id::new("workspace_dock_area").with(("close-panel", Tab::MediaInspector))
            )
            .is_some()
        );
        config.craft_show_close_buttons = false;
        config.craft_show_collapse_buttons = true;
        crate::platform::interop::set_live_config(config);
        let output = frame(&ctx, &mut app, vec![], false);
        assert!(!output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==crate::ui::icons::X)));
        assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==crate::ui::icons::CARET_DOWN)));
        assert_eq!(app.dock_state, before);
        crate::platform::interop::set_live_config(previous);
    }

    #[test]
    fn collapse_and_expand_preserve_exact_layout_and_panel_members() {
        let _guard = crate::app::tests::lock_app_tests();
        let previous = crate::platform::interop::get_live_config();
        let mut config = previous.clone();
        config.craft_show_collapse_buttons = true;
        crate::platform::interop::set_live_config(config);
        for rtl in [false, true] {
            let (ctx, mut app) = fixture(rtl);
            app.dock_state.layout.root = Some(Node::Split {
                axis: SplitAxis::Vertical,
                size: SplitSize::Ratio(0.6),
                first: Box::new(Node::Tabs {
                    panels: vec![Tab::EffectControls, Tab::MediaInspector],
                    active: 0,
                }),
                second: Box::new(Node::Tabs {
                    panels: vec![Tab::EffectsLibrary],
                    active: 0,
                }),
            });
            let before = app.dock_state.clone();
            frame(&ctx, &mut app, vec![], false);
            let output = frame(&ctx, &mut app, vec![], false);
            let caret = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.text() == crate::ui::icons::CARET_DOWN => {
                        Some(t.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .unwrap();
            click(&ctx, &mut app, caret, false);
            let collapsed_output = frame(&ctx, &mut app, vec![], false);
            let view = DockViewState::load(&ctx);
            assert!(view.collapsed.contains(&Tab::EffectControls));
            assert!(view.collapsed.contains(&Tab::MediaInspector));
            assert_eq!(
                app.dock_state, before,
                "collapse must not rewrite the saved split ratios"
            );
            let projected = projected_layout(&app.dock_state.layout, &view, 32.0, 4.0);
            assert!(
                matches!(projected.root,Some(Node::Split{size:SplitSize::FixedFirst(height),..}) if height==32.0)
            );
            let painted_title_y = |output: &egui::FullOutput| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t)
                            if t.galley.text().contains(&format!(
                                "{} Effects Library",
                                Tab::EffectsLibrary.icon()
                            )) =>
                        {
                            Some(t.visual_bounding_rect().center().y)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            assert!(
                painted_title_y(&collapsed_output) < 70.0,
                "collapsed upper row did not return its height"
            );
            let output = frame(&ctx, &mut app, vec![], false);
            let glyph = if rtl {
                crate::ui::icons::CARET_LEFT
            } else {
                crate::ui::icons::CARET_RIGHT
            };
            let caret = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.text() == glyph => {
                        Some(t.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .unwrap();
            click(&ctx, &mut app, caret, false);
            let expanded = frame(&ctx, &mut app, vec![], false);
            assert!(DockViewState::load(&ctx).collapsed.is_empty());
            assert_eq!(app.dock_state, before);
            assert!(painted_title_y(&expanded) > 200.0);
        }
        crate::platform::interop::set_live_config(previous);
    }

    #[test]
    fn closing_an_inactive_tab_does_not_close_its_selected_neighbor() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture(false);
        frame(&ctx, &mut app, vec![], false);
        frame(&ctx, &mut app, vec![], false);
        let response = ctx
            .read_response(
                egui::Id::new("workspace_dock_area").with(("close-panel", Tab::MediaInspector)),
            )
            .unwrap();
        click(&ctx, &mut app, response.rect.center(), false);
        assert!(app.dock_state.layout.contains(&Tab::EffectControls));
        assert!(!app.dock_state.layout.contains(&Tab::MediaInspector));
        assert!(app.dock_state.validate().is_ok());
    }

    #[test]
    fn explicit_reveal_expands_all_members_and_floating_projection_preserves_saved_bounds() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture(false);
        let mut view = DockViewState {
            collapsed: vec![Tab::EffectControls, Tab::MediaInspector],
            ..Default::default()
        };
        view.store(&ctx);
        app.pending_tab_reveals.push(Tab::MediaInspector);
        app.prepare_workspace_reveals(&ctx);
        assert!(DockViewState::load(&ctx).collapsed.is_empty());
        let layout = Layout {
            root: None,
            floating: vec![craft_ui::docking::Floating::single(
                vec![Tab::EffectControls],
                0,
                [30.0, 40.0, 300.0, 250.0],
            )],
        };
        view.collapsed = vec![Tab::EffectControls];
        let projected = projected_layout(&layout, &view, 32.0, 4.0);
        assert_eq!(projected.floating[0].rect, [30.0, 40.0, 300.0, 32.0]);
        assert_eq!(layout.floating[0].rect, [30.0, 40.0, 300.0, 250.0]);
    }

    #[test]
    fn narrow_overflow_has_no_ghost_close_buttons_and_never_overlaps_menu() {
        let _guard = crate::app::tests::lock_app_tests();
        for rtl in [false, true] {
            for width in [160.0, 240.0, 420.0, 900.0, 1440.0] {
                let (ctx, mut app) = fixture(rtl);
                frame_width(&ctx, &mut app, vec![], false, width);
                let output = frame_width(&ctx, &mut app, vec![], false, width);
                let [menu, _] = control_centers(&output);
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == crate::ui::icons::X
                    {
                        let bounds = text.visual_bounding_rect();
                        assert!(shape.clip_rect.contains(bounds.center()));
                        assert!(bounds.center().x >= 0.0 && bounds.center().x <= width);
                        assert!(
                            (bounds.center().x - menu.x).abs() > 20.0,
                            "tab close overlaps group menu"
                        );
                    }
                }
                assert!(app.dock_state.validate().is_ok());
            }
        }
    }

    #[test]
    fn floating_collapse_uses_native_model_and_restores_bounds_and_members() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture(false);
        app.dock_state.layout = Layout {
            root: None,
            floating: vec![craft_ui::docking::Floating::single(
                vec![Tab::EffectControls, Tab::MediaInspector],
                0,
                [30.0, 40.0, 300.0, 250.0],
            )],
        };
        let before = app.dock_state.clone();
        toggle_collapse(&mut app, &ctx, &[Tab::EffectControls, Tab::MediaInspector]);
        assert!(app.dock_state.layout.floating[0].minimized);
        assert!(group_collapsed(
            &app,
            &ctx,
            &[Tab::EffectControls, Tab::MediaInspector],
            Tab::EffectControls
        ));
        assert!(DockViewState::load(&ctx).collapsed.is_empty());
        toggle_collapse(&mut app, &ctx, &[Tab::EffectControls, Tab::MediaInspector]);
        assert_eq!(app.dock_state, before);
    }
}
