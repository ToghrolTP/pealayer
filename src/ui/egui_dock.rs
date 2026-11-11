//! egui Dock presentation of the shared workspace. Panels, hidden locations,
//! profiles and commands have one owner; changing chrome never reloads media.
use super::{
    layout::{PealayerTab as Tab, PealayerTabViewer},
    panel_chrome::HeaderOptions,
};
use crate::app::PealayerApp;
use craft_ui::{
    docking::{Floating, Layout, Node},
    layout::{SplitAxis, SplitSize},
};
use eframe::egui;
use egui_dock::{DockState, NodeIndex, Surface};

#[derive(Clone)]
struct Projection {
    source: Layout<Tab>,
    dock: DockState<Tab>,
}

#[derive(Clone)]
struct TabButton {
    tab: Tab,
    rect: egui::Rect,
    clip_rect: egui::Rect,
    close_rect: Option<egui::Rect>,
    keyboard_focus: bool,
    layer: egui::LayerId,
}

fn tab_buttons_id() -> egui::Id {
    egui::Id::new("egui-dock-tab-buttons")
}

// The pinned egui_dock renderer uses these private, fixed-width hit slots.
// Keep geometry tests coupled to its real tab responses when updating it.
const HEADER_BUTTON_WIDTH: f32 = 24.0;

/// Both renderers consume the live application palette; never reinstall a theme.
fn dock_style(ui: &egui::Ui, options: HeaderOptions) -> egui_dock::Style {
    let t = super::craft_theme::current(ui.ctx());
    let (panel, tabs) = options.colors(&t);
    let corners = options.tab_corners();
    let mut style = egui_dock::Style::from_egui(ui.style());
    style.dock_area_padding = None;
    style.main_surface_border_stroke = egui::Stroke::NONE;
    style.main_surface_border_rounding = egui::CornerRadius::ZERO;
    // egui Dock was 24 points before the CraftUI migration. Its selectable
    // adapter must not inherit CraftUI's independently tuned 32-point chrome.
    style.tab_bar.height = HEADER_BUTTON_WIDTH
        .max(ui.text_style_height(&egui::TextStyle::Button) + 4.0).ceil();
    style.tab_bar.bg_fill = tabs;
    style.tab_bar.corner_radius = egui::CornerRadius::ZERO;
    // The adapter paints one connected tab/body perimeter. Upstream's inset
    // tab rectangle, body rectangle and erasing hlines use different coordinates.
    style.tab_bar.hline_color = egui::Color32::TRANSPARENT;
    style.tab.active.bg_fill = panel;
    style.tab.active.text_color = t.tab_text_active;
    style.tab.active.outline_color = egui::Color32::TRANSPARENT;
    style.tab.active.corner_radius = corners;
    style.tab.inactive.bg_fill = tabs;
    style.tab.inactive.text_color = t.tab_text;
    style.tab.inactive.outline_color = egui::Color32::TRANSPARENT;
    style.tab.inactive.corner_radius = corners;
    style.tab.focused.bg_fill = panel;
    style.tab.focused.text_color = t.tab_text_active;
    style.tab.focused.outline_color = egui::Color32::TRANSPARENT;
    style.tab.focused.corner_radius = corners;
    style.tab.hovered.bg_fill = t.hover;
    style.tab.hovered.text_color = t.tab_text_active;
    style.tab.hovered.corner_radius = corners;
    style.tab.hovered.outline_color = egui::Color32::TRANSPARENT;
    style.tab.tab_body.stroke = egui::Stroke::NONE;
    style.tab.tab_body.bg_fill = panel;
    style.tab.tab_body.corner_radius = egui::CornerRadius::ZERO;
    style.separator.width = 1.0;
    style.separator.color_idle = t.separator;
    style.separator.color_hovered = t.focus;
    style.separator.color_dragged = t.accent_hover;
    style.separator.extra_interact_width = t.gap;
    // Keep upstream hit targets; paint consistently sized Phosphor glyphs below.
    style.buttons.close_tab_color = egui::Color32::TRANSPARENT;
    style.buttons.close_tab_active_color = egui::Color32::TRANSPARENT;
    style.buttons.close_tab_bg_fill = t.hover;
    style.buttons.close_all_tabs_color = egui::Color32::TRANSPARENT;
    style.buttons.close_all_tabs_active_color = egui::Color32::TRANSPARENT;
    style.buttons.close_all_tabs_disabled_color = egui::Color32::TRANSPARENT;
    style.buttons.close_all_tabs_bg_fill = t.hover;
    style.buttons.close_all_tabs_border_color = egui::Color32::TRANSPARENT;
    style.buttons.collapse_tabs_bg_fill = t.hover;
    style.buttons.collapse_tabs_border_color = egui::Color32::TRANSPARENT;
    // Keyboard focus must use the same palette, not the upstream default skin.
    style.tab.inactive_with_kb_focus = style.tab.inactive.clone();
    style.tab.active_with_kb_focus = style.tab.active.clone();
    style.tab.focused_with_kb_focus = style.tab.focused.clone();
    // Preserve the upstream button/hit target, replacing only its solid triangle.
    style.buttons.collapse_tabs_color = egui::Color32::TRANSPARENT;
    style.buttons.collapse_tabs_active_color = egui::Color32::TRANSPARENT;
    style
}

fn disclosure_rect(header: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        header.min,
        egui::vec2(HEADER_BUTTON_WIDTH.min(header.width()), header.height()),
    )
}

fn paint_header_icon(painter: &egui::Painter, rect: egui::Rect, glyph: &str, color: egui::Color32) {
    super::panel_chrome::paint_icon(painter, rect, glyph, color);
}

/// A single physical-pixel contour joins the selected tab to its body. There
/// are no overlapping rectangles or erase strokes.
fn paint_panel_contour(painter: &egui::Painter, panel: egui::Rect, header: egui::Rect,
    active: Option<egui::Rect>, collapsed: bool, radius: u8, color: egui::Color32) {
    let scale = painter.ctx().pixels_per_point();
    let pixel = scale.recip();
    let snap = |value: f32| (value * scale).round() / scale;
    let left = snap(panel.left()) + pixel * 0.5;
    let right = snap(panel.right()) - pixel * 0.5;
    let top = snap(header.top()) + pixel * 0.5;
    let seam = snap(header.bottom()) - pixel * 0.5;
    let bottom = snap(panel.bottom()) - pixel * 0.5;
    let stroke = egui::Stroke::new(pixel, color);
    if right <= left || bottom <= top { return; }
    let mut points = if collapsed || bottom <= seam {
        vec![egui::pos2(left,top),egui::pos2(right,top),egui::pos2(right,bottom),egui::pos2(left,bottom)]
    } else if let Some(active) = active.filter(|rect| rect.width() > pixel) {
        let tab_left = (snap(active.left()) + pixel * 0.5).clamp(left,right);
        let tab_right = (snap(active.right()) - pixel * 0.5).clamp(tab_left,right);
        let radius = f32::from(radius).min((tab_right - tab_left) / 2.0).min(seam - top).max(0.0);
        let diagonal = radius * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
        vec![egui::pos2(left,seam),egui::pos2(tab_left,seam),
            egui::pos2(tab_left,top + radius),egui::pos2(tab_left + diagonal,top + diagonal),
            egui::pos2(tab_left + radius,top),egui::pos2(tab_right - radius,top),
            egui::pos2(tab_right - diagonal,top + diagonal),egui::pos2(tab_right,top + radius),
            egui::pos2(tab_right,seam),egui::pos2(right,seam),
            egui::pos2(right,bottom),egui::pos2(left,bottom)]
    } else {
        vec![egui::pos2(left,seam),egui::pos2(right,seam),egui::pos2(right,bottom),egui::pos2(left,bottom)]
    };
    points.dedup();
    painter.add(egui::Shape::closed_line(points, stroke));
}

/// Register only genuinely empty header space, not a y-band across unrelated
/// panels. Paint floating chrome on its own layer, never behind the window.
fn decorate_headers(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    dock: &DockState<Tab>,
    height: f32,
    interactive: bool,
    options: HeaderOptions,
) {
    let buttons = ui
        .data(|d| d.get_temp::<Vec<TabButton>>(tab_buttons_id()))
        .unwrap_or_default();
    let t = super::craft_theme::current(ui.ctx());
    let show_disclosure = !app.dock_state.locked && options.dock_collapse;
    for (path, leaf) in dock.iter_leaves() {
        if !leaf.rect.is_positive() || leaf.tabs.is_empty() {
            continue;
        }
        let Some(button) = buttons.iter().find(|b| leaf.tabs.contains(&b.tab)) else {
            continue;
        };
        let header = egui::Rect::from_min_size(
            leaf.rect.min,
            egui::vec2(leaf.rect.width(), height.min(leaf.rect.height())),
        );
        let clip = if path.surface.is_main() {
            ui.clip_rect()
        } else {
            ui.ctx().content_rect()
        };
        let painter = egui::Painter::new(ui.ctx().clone(), button.layer, leaf.rect.intersect(clip));
        if show_disclosure {
            let rect = disclosure_rect(header);
            let hovered = ui
                .ctx()
                .pointer_hover_pos()
                .is_some_and(|p| rect.contains(p))
                && ui.ctx().layer_id_at(rect.center()) == Some(button.layer);
            paint_header_icon(
                &painter, rect,
                if leaf.collapsed {
                    super::icons::CARET_RIGHT
                } else {
                    super::icons::CARET_DOWN
                },
                if hovered { t.tab_text_active } else { t.icon },
            );
        }
        if !app.dock_state.locked {
            let close_all = egui::Rect::from_min_max(egui::pos2(header.right()-HEADER_BUTTON_WIDTH.min(header.width()),header.top()),header.max);
            let secondary = ui.input(|i| i.modifiers.shift);
            let icon_color = |rect: egui::Rect| {
                if !interactive { t.text_dim }
                else if ui.ctx().pointer_hover_pos().is_some_and(|p| rect.contains(p))
                    && ui.ctx().layer_id_at(rect.center()) == Some(button.layer) { t.tab_text_active }
                else { t.icon }
            };
            paint_header_icon(&painter,close_all,
                if secondary && !path.surface.is_main() { super::icons::APP_WINDOW } else { super::icons::X },
                if secondary && path.surface.is_main() { t.text_dim } else { icon_color(close_all) });
            for tab in buttons.iter().filter(|b| leaf.tabs.contains(&b.tab)) {
                let tab_clip = tab.clip_rect.intersect(header).intersect(clip);
                if let Some(close) = tab.close_rect.filter(|rect| rect.intersects(tab_clip)) {
                    // Scrollable tabs must not paint over collapse/close-all.
                    paint_header_icon(&painter.with_clip_rect(tab_clip),close,super::icons::X,icon_color(close));
                }
            }
        }
        let active = leaf.tabs.get(leaf.active.0).and_then(|tab| buttons.iter().find(|button| button.tab == *tab));
        paint_panel_contour(&painter, leaf.rect, header,
            active.map(|button| button.rect.intersect(button.clip_rect).intersect(header)), leaf.collapsed,
            options.tab_corners().nw,
            if dock.focused_leaf() == Some(path) || active.is_some_and(|button|button.keyboard_focus) {t.focus} else {t.separator});
        for button in buttons.iter().filter(|button| leaf.tabs.contains(&button.tab) && button.keyboard_focus
            && active.is_none_or(|active| active.tab != button.tab)) {
            painter.with_clip_rect(button.clip_rect.intersect(header)).rect_stroke(button.rect.shrink(1.0),
                options.tab_corners(), egui::Stroke::new(1.0, t.focus), egui::StrokeKind::Inside);
        }
        let left = buttons
            .iter()
            .filter(|b| leaf.tabs.contains(&b.tab))
            .map(|b| b.rect.right())
            .fold(header.left(), f32::max)
            .max(if show_disclosure { disclosure_rect(header).right() } else { header.left() });
        let right = header.right()
            - if app.dock_state.locked {
                0.0
            } else {
                HEADER_BUTTON_WIDTH
            };
        let empty = egui::Rect::from_min_max(
            egui::pos2(left, header.top()),
            egui::pos2(right, header.bottom()),
        );
        if !interactive || !empty.is_positive() {
            continue;
        }
        let mut header_ui = ui.new_child(
            egui::UiBuilder::new()
                .layer_id(button.layer)
                .max_rect(empty),
        );
        header_ui.set_clip_rect(header.intersect(clip));
        let response = header_ui.interact(
            empty,
            egui::Id::new(("egui-dock-empty-header", leaf.tabs[0])),
            egui::Sense::click(),
        );
        response.context_menu(|ui| {
            if leaf.collapsed && !show_disclosure {
                expand_panel_action(app, ui, leaf.tabs[0]);
                ui.separator();
            }
            ui.strong(app.tr("Panels"));
            ui.separator();
            ui.add_enabled_ui(!app.dock_state.locked, |ui| {
                super::layout::draw_workspace_tab_menu(app, ui)
            });
            ui.separator();
            if ui
                .add_enabled(
                    !app.dock_state.locked,
                    egui::Button::new(format!(
                        "{} {}",
                        super::icons::TABS,
                        app.tr("Reset workspace layout")
                    )),
                )
                .clicked()
            {
                app.dock_state = super::layout::create_initial_layout();
                app.sync_terminal_visibility_from_workspace();
                app.deferred_config_save = true;
                ui.close();
            }
        });
    }
}

fn members(node: &Node<Tab>) -> Vec<Tab> {
    Layout {
        root: Some(node.clone()),
        floating: vec![],
    }
    .panels()
    .into_iter()
    .copied()
    .collect()
}

fn fill(tree: &mut egui_dock::Tree<Tab>, index: NodeIndex, node: &Node<Tab>, extent: egui::Vec2) {
    match node {
        Node::Split {
            axis,
            size,
            first,
            second,
        } if !members(first).is_empty() && !members(second).is_empty() => {
            let horizontal = *axis == SplitAxis::Horizontal;
            let available = if horizontal { extent.x } else { extent.y }.max(1.0);
            let ratio = match size {
                SplitSize::Ratio(r) => *r,
                SplitSize::FixedFirst(v) => v / available,
                SplitSize::FixedSecond(v) => 1.0 - v / available,
            }
            .clamp(0.01, 0.99);
            tree[index] = egui_dock::Node::leaf_with(members(first));
            let [a, b] = tree.split(
                index,
                if horizontal {
                    egui_dock::Split::Right
                } else {
                    egui_dock::Split::Below
                },
                ratio,
                egui_dock::Node::leaf_with(members(second)),
            );
            let mut first_size = extent;
            let mut second_size = extent;
            if horizontal {
                first_size.x *= ratio;
                second_size.x *= 1.0 - ratio;
            } else {
                first_size.y *= ratio;
                second_size.y *= 1.0 - ratio;
            }
            fill(tree, a, first, first_size);
            fill(tree, b, second, second_size);
        }
        _ => {
            tree[index] = egui_dock::Node::leaf_with(members(node));
            if let Node::Tabs { active, .. } = node {
                let _ = tree[index].get_leaf_mut().unwrap().set_active_tab(*active);
            }
        }
    }
}

fn project(layout: &Layout<Tab>, extent: egui::Vec2) -> DockState<Tab> {
    let mut dock = DockState::new(vec![]);
    if let Some(root) = &layout.root {
        fill(dock.main_surface_mut(), NodeIndex::root(), root, extent);
    }
    for window in &layout.floating {
        let surface = dock.add_window(members(&window.root));
        fill(
            &mut dock[surface],
            NodeIndex::root(),
            &window.root,
            egui::vec2(window.rect[2], window.rect[3]),
        );
        dock.get_window_state_mut(surface)
            .unwrap()
            .set_position(egui::pos2(window.rect[0], window.rect[1]))
            .set_size(egui::vec2(window.rect[2], window.rect[3]));
    }
    dock
}

fn read_node(tree: &egui_dock::Tree<Tab>, index: NodeIndex) -> Option<Node<Tab>> {
    match &tree[index] {
        egui_dock::Node::Empty => None,
        egui_dock::Node::Leaf(leaf) => (!leaf.tabs.is_empty()).then(|| Node::Tabs {
            panels: leaf.tabs.clone(),
            active: leaf.active.0,
        }),
        egui_dock::Node::Horizontal(split) | egui_dock::Node::Vertical(split) => {
            match (
                read_node(tree, index.left()),
                read_node(tree, index.right()),
            ) {
                (Some(first), Some(second)) => Some(Node::Split {
                    axis: if matches!(tree[index], egui_dock::Node::Horizontal(_)) {
                        SplitAxis::Horizontal
                    } else {
                        SplitAxis::Vertical
                    },
                    size: SplitSize::Ratio(split.fraction),
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (node, None) | (None, node) => node,
            }
        }
    }
}

fn read_layout(dock: &DockState<Tab>, source: &Layout<Tab>) -> Layout<Tab> {
    let mut layout = Layout::default();
    let mut retained = std::collections::HashSet::new();
    for surface in dock.iter_surfaces() {
        match surface {
            Surface::Main(tree) => {
                if !tree.is_empty() {
                    layout.root = read_node(tree, NodeIndex::root());
                }
            }
            Surface::Window(tree, state) => {
                if !tree.is_empty()
                    && let Some(root) = read_node(tree, NodeIndex::root())
                {
                    // A WindowState has no finite rect before its first frame.
                    // Retain window identity/native placement while this presentation
                    // draws it in-process; never serialize Rect::NOTHING.
                    let panels = members(&root);
                    let previous = source
                        .floating
                        .iter()
                        .filter(|w| !retained.contains(&w.id))
                        .max_by_key(|w| panels.iter().filter(|p| w.contains(*p)).count())
                        .filter(|w| panels.iter().any(|p| w.contains(p)));
                    let mut window = previous
                        .cloned()
                        .unwrap_or_else(|| Floating::new(root.clone(), [40.0, 40.0, 640.0, 480.0]));
                    retained.insert(window.id);
                    window.root = root;
                    let r = state.rect();
                    if r.is_finite() && r.width() > 0.0 && r.height() > 0.0 {
                        window.rect = [r.min.x, r.min.y, r.width(), r.height()];
                    }
                    layout.floating.push(window);
                }
            }
            Surface::Empty => {}
        }
    }
    layout
}

// Restore untouched Craft-specific nodes (accordion/dock and fixed sizing).
// Editing a different branch must not flatten the entire saved workspace.
fn retain_unchanged(source: &Node<Tab>, before: &Node<Tab>, after: &mut Node<Tab>) {
    if *after == *before {
        *after = source.clone();
        return;
    }
    if let (
        Node::Split {
            first: s1,
            second: s2,
            size,
            ..
        },
        Node::Split {
            first: b1,
            second: b2,
            size: bs,
            axis: ba,
        },
        Node::Split {
            first: a1,
            second: a2,
            size: az,
            axis: aa,
        },
    ) = (source, before, after)
        && ba == aa
    {
        if *az == *bs {
            *az = *size;
        }
        retain_unchanged(s1, b1, a1);
        retain_unchanged(s2, b2, a2);
    }
}

// Keep Dock-only title metrics out of the shared CraftUI panel viewer.
struct DockTabViewer<'a> {
    viewer: PealayerTabViewer<'a>,
    context: egui::Context,
    title_font: egui::FontId,
    collapsed_without_chevron: Vec<Tab>,
}

fn expand_panel_action(app: &mut PealayerApp, ui: &mut egui::Ui, tab: Tab) {
    if ui.add_enabled(!app.dock_state.locked, egui::Button::new(app.tr("Expand panel"))).clicked() {
        app.pending_tab_reveals.push(tab);
        ui.ctx().request_repaint();
        ui.close();
    }
}

impl egui_dock::TabViewer for DockTabViewer<'_> {
    type Tab = Tab;
    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        let mut galley = self.context.fonts_mut(|fonts| fonts.layout_no_wrap(
            format!("{} {}", tab.icon(), tab.title(self.viewer.app)),
            self.title_font.clone(), egui::Color32::PLACEHOLDER));
        // Upstream centers the line box. Center its actual ink instead, without
        // changing text, width, hit targets, accessibility or drag geometry.
        let delta = galley.rect.center().y - galley.mesh_bounds.center().y;
        let normalized = std::sync::Arc::make_mut(&mut galley);
        for row in &mut normalized.rows { row.pos.y += delta; }
        normalized.mesh_bounds = normalized.mesh_bounds.translate(egui::vec2(0.0, delta));
        egui::WidgetText::Galley(galley)
    }
    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(("workspace-panel", *tab))
    }
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        PealayerTabViewer::ui(&mut self.viewer, ui, tab);
    }
    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        PealayerTabViewer::scroll_bars(&self.viewer, tab)
    }
    fn is_closeable(&self, _tab: &Tab) -> bool {
        !self.viewer.app.dock_state.locked
    }
    fn allowed_in_windows(&self, _tab: &mut Tab) -> bool {
        !self.viewer.app.dock_state.locked
    }
    fn on_tab_button(&mut self, tab: &mut Tab, response: &egui::Response) {
        // Read the real upstream target before entering the data transaction.
        let close_rect = response.ctx.read_response(response.id.with("close-button")).map(|close| close.rect);
        let keyboard_focus = response.has_focus();
        response.ctx.data_mut(|data| {
            let buttons = data.get_temp_mut_or_default::<Vec<TabButton>>(tab_buttons_id());
            buttons.push(TabButton {
                tab: *tab,
                rect: response.rect,
                clip_rect: response.interact_rect,
                close_rect,
                keyboard_focus,
                layer: response.layer_id,
            });
        });
    }
    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Tab, _node: egui_dock::NodePath) {
        self.viewer.draw_context_menu(ui, *tab);
        if self.collapsed_without_chevron.contains(tab) {
            ui.separator();
            expand_panel_action(self.viewer.app, ui, *tab);
        }
    }
}

pub fn show(app: &mut PealayerApp, ui: &mut egui::Ui, reveal: Option<Tab>) {
    let key = egui::Id::new("egui-dock-projection");
    let source = app.dock_state.layout.clone();
    let mut projection = ui
        .data_mut(|d| d.get_temp::<Projection>(key))
        .filter(|p| p.source == source)
        .unwrap_or_else(|| Projection {
            dock: project(&source, ui.available_size()),
            source,
        });
    if let Some(tab) = reveal
        && let Some(path) = projection.dock.find_tab(&tab)
    {
        // An explicit reveal must also open collapsed ancestors, as it did in
        // the original native dock. Merely selecting the tab leaves it hidden.
        let mut ancestor = Some(path.node);
        while let Some(node) = ancestor {
            projection.dock[path.surface][node].set_collapsed(false);
            ancestor = node.parent();
        }
        let _ = projection.dock.set_active_tab(path);
        projection
            .dock
            .set_focused_node_and_surface(egui_dock::NodePath::new(path.surface, path.node));
    }
    let before = read_layout(&projection.dock, &projection.source);
    let locked = app.dock_state.locked;
    let options = HeaderOptions::current();
    let show_disclosure = !locked && options.dock_collapse;
    let layer = ui.layer_id();
    let blocked = !ui.ctx().memory(|m| m.is_above_modal_layer(layer));
    ui.data_mut(|d| d.insert_temp(tab_buttons_id(), Vec::<TabButton>::new()));
    let style = dock_style(ui, options);
    let header_height = style.tab_bar.height;
    app.workspace_rendering = true;
    ui.add_enabled_ui(!blocked, |ui| {
        let mut viewer = DockTabViewer {
            viewer: PealayerTabViewer::new(app),
            context: ui.ctx().clone(),
            title_font: egui::TextStyle::Button.resolve(ui.style()),
            collapsed_without_chevron: projection.dock.iter_leaves()
                .filter(|(_, leaf)| leaf.collapsed && !show_disclosure)
                .flat_map(|(_, leaf)| leaf.tabs.iter().copied())
                .collect(),
        };
        egui_dock::DockArea::new(&mut projection.dock)
            .id(egui::Id::new("egui-workspace-dock"))
            .style(style)
            .draggable_tabs(!locked && !blocked)
            .show_close_buttons(!locked)
            .show_window_close_buttons(!locked)
            .show_leaf_close_all_buttons(!locked)
            .show_leaf_collapse_buttons(show_disclosure)
            .show_inside(ui, &mut viewer);
    });
    decorate_headers(
        app,
        ui,
        &projection.dock,
        header_height,
        !blocked && ui.is_enabled(),
        options,
    );
    app.workspace_rendering = false;
    if !app.pending_tab_reveals.is_empty() {
        ui.ctx().request_repaint();
    }
    let mut after = read_layout(&projection.dock, &projection.source);
    // View geometry is intentionally not serialized every frame. Retain Craft
    // stacks/docks unchanged until an actual egui Dock edit is made.
    if !locked && !blocked && after != before && app.dock_state.layout == projection.source {
        let mut candidate = app.dock_state.clone();
        for tab in before
            .panels()
            .into_iter()
            .copied()
            .filter(|tab| !after.contains(tab))
        {
            let _ = candidate.set_visible(tab, false, Some(Tab::ProgramMonitor), |_| {
                Default::default()
            });
        }
        if let (Some(source), Some(before), Some(after)) =
            (&projection.source.root, &before.root, &mut after.root)
        {
            retain_unchanged(source, before, after);
        }
        for window in &mut after.floating {
            if let Some(source) = projection
                .source
                .floating
                .iter()
                .find(|w| w.id == window.id)
                && let Some(before) = before.floating.iter().find(|w| w.id == window.id)
            {
                retain_unchanged(&source.root, &before.root, &mut window.root);
            }
        }
        candidate.layout = after;
        if candidate.validate().is_ok() {
            app.dock_state = candidate;
            app.sync_terminal_visibility_from_workspace();
            projection.source = app.dock_state.layout.clone();
            app.deferred_config_save = true;
        }
    } else if locked || blocked {
        projection.dock = project(&projection.source, ui.available_size());
    }
    ui.data_mut(|d| d.insert_temp(key, projection));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn terminal_uses_native_dock_close_and_preserves_session_owner_and_siblings() {
        let _guard = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = PealayerApp::default();
        app.dock_state = craft_ui::docking::Workspace::new(Layout {
            root: Some(Node::Tabs { panels: vec![Tab::Timeline, Tab::Terminal], active: 1 }),
            floating: Vec::new(),
        });
        app.sync_terminal_visibility_from_workspace();
        for _ in 0..3 { frame(&ctx, &mut app, vec![]); }
        let close = ctx.data(|data| data.get_temp::<Vec<TabButton>>(tab_buttons_id())).unwrap()
            .into_iter().find(|button| button.tab == Tab::Terminal).unwrap().close_rect.unwrap().center();
        let sessions = app.terminal_state.status()["sessions"].clone();
        assert!(app.terminal_state.preferences.enabled);
        app.dock_state.locked = true;
        click(&ctx, &mut app, close, egui::PointerButton::Primary);
        assert!(app.dock_state.layout.contains(&Tab::Terminal));
        app.dock_state.locked = false;
        for _ in 0..2 { frame(&ctx, &mut app, vec![]); }
        let close = ctx.data(|data| data.get_temp::<Vec<TabButton>>(tab_buttons_id())).unwrap()
            .into_iter().find(|button| button.tab == Tab::Terminal).unwrap().close_rect.unwrap().center();
        click(&ctx, &mut app, close, egui::PointerButton::Primary);
        assert!(!app.dock_state.layout.contains(&Tab::Terminal));
        assert!(app.dock_state.layout.contains(&Tab::Timeline));
        assert!(!app.terminal_state.preferences.enabled);
        assert_eq!(app.terminal_state.status()["sessions"], sessions, "closing pane does not destroy terminal views");
    }

    fn frame(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 650.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
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
        button: egui::PointerButton,
    ) -> egui::FullOutput {
        frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
        frame(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: Default::default(),
            }],
        );
        frame(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers: Default::default(),
            }],
        )
    }

    fn has_text(output: &egui::FullOutput, text: &str) -> bool {
        output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains(text)))
    }

    fn fixture() -> (egui::Context, PealayerApp) {
        let ctx = egui::Context::default();
        super::super::i18n::configure_ui_fonts(&ctx, false);
        let mut app = PealayerApp::default();
        app.engine_handle = crate::four_d::engine::memory_only_test_engine();
        app.dock_state.layout = Layout {
            root: Some(Node::Tabs {
                panels: vec![Tab::EffectControls, Tab::MediaInspector],
                active: 0,
            }),
            floating: vec![],
        };
        app.dock_state.hidden.clear();
        assert!(app.dock_state.validate().is_ok());
        (ctx, app)
    }

    #[test]
    fn header_and_body_have_one_connected_pixel_aligned_contour() {
        let _guard = crate::app::tests::lock_app_tests();
        for dark in [false, true] { for scale in [1.0, 1.25, 1.5, 2.0] {
            let (ctx, mut app) = fixture();
            ctx.set_pixels_per_point(scale);
            ctx.set_visuals(if dark {egui::Visuals::dark()} else {egui::Visuals::light()});
            frame(&ctx, &mut app, vec![]);
            let output = frame(&ctx, &mut app, vec![egui::Event::PointerMoved(egui::pos2(45.0,12.0))]);
            let paths: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
                egui::Shape::Path(path) if path.closed && path.points.len() == 8 => Some(path), _=>None
            }).collect();
            assert_eq!(paths.len(),1,"selected tab and body must have one perimeter, not stacked frames");
            for path in paths {
                assert!((path.stroke.width * scale - 1.0).abs()<0.01);
                for point in &path.points {
                    for coordinate in [point.x,point.y] {
                        assert!(((coordinate * scale).fract()-0.5).abs()<0.02,"stroke not on a physical pixel center at {scale}");
                    }
                }
            }
            let header = ctx.data(|data|data.get_temp::<Vec<TabButton>>(tab_buttons_id())).unwrap()[0].rect;
            assert!(!output.shapes.iter().any(|shape|matches!(&shape.shape,
                egui::Shape::Rect(rect) if rect.stroke.color.a()>0 && rect.stroke.width>0.0
                    && rect.rect.top() < header.bottom() && rect.rect.bottom() > header.top())),
                "upstream inset tab/body rectangle returned beside the connected perimeter");
        } }
    }

    #[test]
    fn dock_caption_ink_and_compact_height_match_header_controls() {
        let _guard = crate::app::tests::lock_app_tests();
        for dark in [false, true] {
          for scale in [1.0, 1.25, 1.5, 2.0] {
           for language in [crate::config::AppLanguage::English, crate::config::AppLanguage::Persian] {
            for size in [14.0, 22.0] {
                let (ctx, mut app) = fixture();
                app.language = language;
                app.rtl = language.is_rtl();
                super::super::i18n::configure_ui_fonts(&ctx, language.is_rtl());
                ctx.set_pixels_per_point(scale);
                ctx.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                ctx.all_styles_mut(|style| {
                    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(size));
                    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(size));
                });
                frame(&ctx, &mut app, vec![]);
                let output = frame(&ctx, &mut app, vec![]);
                let buttons = ctx.data(|data| data.get_temp::<Vec<TabButton>>(tab_buttons_id())).unwrap();
                let header = buttons[0].rect;
                if size == 14.0 && scale == 1.0 && language == crate::config::AppLanguage::English {
                    assert_eq!(header.height(), 24.0, "egui Dock must retain its pre-CraftUI compact header, not CraftUI's 32-point header");
                }
                assert!(header.height() >= 24.0, "non-Latin/large-font metrics can grow, never shrink the compact baseline");
                let close = buttons[0].close_rect.expect("real upstream close hit target");
                assert_eq!(close.width(), HEADER_BUTTON_WIDTH);
                assert_eq!(close.width(), close.height(), "large fonts must not stretch the close target");
                assert!((close.center().y - header.center().y).abs() < 0.01);
                let title = Tab::EffectControls.title(&app);
                let caption = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text().contains(&title) => Some(text.visual_bounding_rect()),
                    _ => None,
                }).expect("actual tab caption");
                assert!(caption.height() <= header.height()-2.0, "font-aware height must contain the caption ink");
                assert!((caption.center().y - header.center().y).abs() < 0.5,
                    "caption ink must align with header glyphs at font {size}: {caption:?} vs {header:?}");
            }
           }
          }
        }
    }

    #[test]
    fn overflow_close_glyphs_stay_clipped_and_no_vertical_close_rule_is_painted() {
        let _guard = crate::app::tests::lock_app_tests();
        for dark in [false, true] {
            let (ctx, mut app) = fixture();
            ctx.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
            let draw = |app: &mut PealayerApp, events| {
                let mut output = ctx.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(220.0, 500.0))),
                    events, ..Default::default()
                }, |ui| show(app, ui, None));
                output.textures_delta.clear(); output
            };
            draw(&mut app, vec![]);
            draw(&mut app, vec![]);
            let projection = ctx.data(|data| data.get_temp::<Projection>(egui::Id::new("egui-dock-projection"))).unwrap();
            let leaf = projection.dock.main_surface().root_node().unwrap().get_leaf().unwrap();
            let close_left = leaf.rect.right() - HEADER_BUTTON_WIDTH;
            for pointer in [egui::Pos2::ZERO, egui::pos2(leaf.rect.right()-12.0,leaf.rect.top()+12.0)] {
                let output = draw(&mut app, vec![egui::Event::PointerMoved(pointer)]);
                let buttons = ctx.data(|data| data.get_temp::<Vec<TabButton>>(tab_buttons_id())).unwrap();
                assert!(buttons.iter().any(|button| button.rect.right() > close_left), "fixture must overflow");
                for shape in &output.shapes {
                    match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == super::super::icons::X
                            && text.visual_bounding_rect().center().x < close_left => {
                            assert!(shape.clip_rect.right() <= close_left+0.1, "tab close glyph must not bleed over close-all");
                            assert!(shape.clip_rect.left() >= leaf.rect.left()+HEADER_BUTTON_WIDTH-0.1, "tab close glyph must not bleed over disclosure");
                        }
                        egui::Shape::LineSegment {points,stroke} if stroke.color.a() > 0 => {
                            assert!(!(points.iter().all(|point| (point.x-close_left).abs()<0.1)
                                && (points[1].y-points[0].y).abs()>12.0), "unwanted visible close-all left border");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn restored_header_chevrons_and_palette_match_live_renderer_geometry() {
        let _guard = crate::app::tests::lock_app_tests();
        for dark in [false, true] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let (ctx, mut app) = fixture();
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                ctx.set_pixels_per_point(scale);
                frame(&ctx, &mut app, vec![]);
                let output = frame(&ctx, &mut app, vec![]);
                let projection = ctx
                    .data(|d| d.get_temp::<Projection>(egui::Id::new("egui-dock-projection")))
                    .unwrap();
                let leaf = projection
                    .dock
                    .main_surface()
                    .root_node()
                    .unwrap()
                    .get_leaf()
                    .unwrap();
                let buttons = ctx
                    .data(|d| d.get_temp::<Vec<TabButton>>(tab_buttons_id()))
                    .unwrap();
                let header = egui::Rect::from_min_max(
                    leaf.rect.min,
                    egui::pos2(leaf.rect.right(), buttons[0].rect.bottom()),
                );
                let disclosure = disclosure_rect(header);
                assert!(
                    buttons[0].rect.left() >= disclosure.right(),
                    "disclosure overlaps tab"
                );
                assert!(
                    has_text(&output, super::super::icons::CARET_DOWN),
                    "custom caret not rendered"
                );
                assert!(has_text(&output,super::super::icons::X),"custom close glyphs must render");
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape {
                        if [super::super::icons::X,super::super::icons::CARET_DOWN].contains(&text.galley.text())
                            && header.contains(text.visual_bounding_rect().center()) {
                            assert!((text.visual_bounding_rect().center().y-header.center().y).abs()<0.15,
                                "header icon ink must be vertically centered at {scale}: {:?}",text.visual_bounding_rect());
                        }
                    }
                }
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape {
                        if text.galley.text() == super::super::icons::CARET_DOWN {
                            assert!(
                                disclosure
                                    .expand(1.0)
                                    .contains_rect(text.visual_bounding_rect()),
                                "caret bleeds outside hit slot"
                            );
                        }
                    }
                }
                let before = app.dock_state.layout.clone();
                click(
                    &ctx,
                    &mut app,
                    disclosure.center(),
                    egui::PointerButton::Primary,
                );
                let output = frame(&ctx, &mut app, vec![]);
                assert!(
                    has_text(&output, super::super::icons::CARET_RIGHT),
                    "collapse did not update caret"
                );
                assert_eq!(
                    before, app.dock_state.layout,
                    "collapse must not delete or rearrange panels"
                );
                ctx.run_ui(Default::default(), |ui| {
                    let style = dock_style(ui, HeaderOptions::current());
                    let tokens = super::super::craft_theme::current(&ctx);
                    assert_eq!(style.tab.focused.outline_color, egui::Color32::TRANSPARENT);
                    assert_eq!(style.tab_bar.bg_fill, tokens.tab_bg);
                    assert_eq!(style.tab_bar.corner_radius, egui::CornerRadius::ZERO);
                    assert_eq!(style.tab.tab_body.bg_fill, style.tab.active.bg_fill);
                    assert_eq!(style.buttons.close_all_tabs_border_color,egui::Color32::TRANSPARENT);
                    assert_eq!(style.buttons.close_all_tabs_bg_fill,tokens.hover);
                    assert_eq!(style.tab.active_with_kb_focus.bg_fill,style.tab.active.bg_fill);
                    assert_eq!(
                        style.buttons.collapse_tabs_color,
                        egui::Color32::TRANSPARENT
                    );
                })
                .textures_delta
                .clear();
            }
        }
    }

    #[test]
    fn dock_header_tweaks_swap_surface_colors_round_tabs_and_hide_chevron() {
        let _guard = crate::app::tests::lock_app_tests();
        let previous = crate::platform::interop::get_live_config();
        let mut config = previous.clone();
        config.egui_dock_show_collapse_buttons = false;
        config.workspace_swap_panel_colors = true;
        config.workspace_round_panel_tabs = true;
        config.color_palette = crate::config::ColorPalette::Studio;
        config.theme = crate::config::AppTheme::Dark;
        let (ctx, mut app) = fixture();
        crate::ui::configure_native_appearance(&ctx, &config);
        crate::platform::interop::set_live_config(config);
        let before = app.dock_state.clone();
        frame(&ctx, &mut app, vec![]);
        let output = frame(&ctx, &mut app, vec![]);
        crate::platform::interop::set_live_config(previous);
        assert!(!has_text(&output, super::super::icons::CARET_DOWN));
        assert!(has_text(&output, super::super::icons::X));
        let tokens = super::super::craft_theme::current(&ctx);
        assert_ne!(tokens.panel_bg, tokens.tab_bg);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect)
            if rect.fill == tokens.tab_bg && rect.corner_radius.nw == 5 && rect.corner_radius.sw == 0)));
        assert_eq!(app.dock_state, before);
    }

    #[test]
    fn empty_header_and_tab_menus_open_without_stealing_panel_body_clicks() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture();
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        click(
            &ctx,
            &mut app,
            egui::pos2(700.0, 12.0),
            egui::PointerButton::Secondary,
        );
        let output = frame(&ctx, &mut app, vec![]);
        assert!(
            has_text(&output, "Panels"),
            "empty header lost its context menu"
        );
        assert!(has_text(&output, "Reset workspace layout"));
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
        );
        let buttons = ctx
            .data(|d| d.get_temp::<Vec<TabButton>>(tab_buttons_id()))
            .unwrap();
        click(
            &ctx,
            &mut app,
            buttons[0].rect.center(),
            egui::PointerButton::Secondary,
        );
        let output = frame(&ctx, &mut app, vec![]);
        assert!(
            has_text(&output, "Preferences..."),
            "tab-specific menu was stolen by header menu"
        );
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
        );
        click(
            &ctx,
            &mut app,
            egui::pos2(700.0, 300.0),
            egui::PointerButton::Secondary,
        );
        assert!(!has_text(
            &frame(&ctx, &mut app, vec![]),
            "Reset workspace layout"
        ));
    }

    #[test]
    fn floating_chrome_and_menu_use_the_floating_layer() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture();
        app.dock_state.layout.root = Some(Node::Tabs {
            panels: vec![Tab::EffectControls],
            active: 0,
        });
        app.dock_state.layout.floating.push(Floating::single(
            vec![Tab::MediaInspector],
            0,
            [80.0, 100.0, 650.0, 300.0],
        ));
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        let buttons = ctx
            .data(|d| d.get_temp::<Vec<TabButton>>(tab_buttons_id()))
            .unwrap();
        let floating = buttons
            .iter()
            .find(|b| b.tab == Tab::MediaInspector)
            .unwrap();
        let main = buttons
            .iter()
            .find(|b| b.tab == Tab::EffectControls)
            .unwrap();
        assert_ne!(floating.layer, main.layer);
        let output = frame(&ctx, &mut app, vec![]);
        assert!(
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t)
            if t.galley.text() == super::super::icons::CARET_DOWN && t.pos.y > 80.0)),
            "floating caret missing"
        );
        let pos = egui::pos2(floating.rect.right() + 50.0, floating.rect.center().y);
        click(&ctx, &mut app, pos, egui::PointerButton::Secondary);
        assert!(has_text(
            &frame(&ctx, &mut app, vec![]),
            "Reset workspace layout"
        ));
    }

    #[test]
    fn menu_hides_only_selected_panel_and_preserves_its_restore_location() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture();
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        click(
            &ctx,
            &mut app,
            egui::pos2(700.0, 12.0),
            egui::PointerButton::Secondary,
        );
        let output = frame(&ctx, &mut app, vec![]);
        let title = Tab::MediaInspector.title(&app);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.text().contains(&title) && text.pos.y > 30.0 =>
                {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("visible panel menu row");
        click(&ctx, &mut app, position, egui::PointerButton::Primary);
        frame(&ctx, &mut app, vec![]);
        assert!(app.dock_state.layout.contains(&Tab::EffectControls));
        assert!(!app.dock_state.layout.contains(&Tab::MediaInspector));
        assert!(
            app.dock_state
                .hidden
                .iter()
                .any(|(tab, _)| *tab == Tab::MediaInspector)
        );
        assert!(app.deferred_config_save);
        app.open_or_focus_tab(Tab::MediaInspector);
        frame(&ctx, &mut app, vec![]);
        assert!(app.dock_state.layout.contains(&Tab::MediaInspector));
        assert!(app.dock_state.validate().is_ok());
    }

    #[test]
    fn modal_blocks_header_interaction_but_keeps_custom_chrome_visible() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture();
        let mut draw = |events: Vec<egui::Event>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 650.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    show(&mut app, ui, None);
                    egui::Modal::new(egui::Id::unique("dock-test-modal")).show(&ctx, |ui| {
                        ui.label("Modal owns input");
                    });
                },
            );
            output.textures_delta.clear();
            output
        };
        draw(vec![]);
        draw(vec![]);
        for (pos, button) in [
            (egui::pos2(700.0, 12.0), egui::PointerButton::Secondary),
            (egui::pos2(12.0, 12.0), egui::PointerButton::Primary),
        ] {
            draw(vec![egui::Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                draw(vec![egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: Default::default(),
                }]);
            }
        }
        let output = draw(vec![]);
        assert!(!has_text(&output, "Reset workspace layout"));
        assert!(has_text(&output, super::super::icons::CARET_DOWN));
    }

    #[test]
    fn reveal_expands_a_collapsed_panel_without_changing_workspace_members() {
        let _guard = crate::app::tests::lock_app_tests();
        let (ctx, mut app) = fixture();
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        click(
            &ctx,
            &mut app,
            egui::pos2(12.0, 12.0),
            egui::PointerButton::Primary,
        );
        assert!(has_text(
            &frame(&ctx, &mut app, vec![]),
            super::super::icons::CARET_RIGHT
        ));
        let before = app.dock_state.layout.clone();
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 650.0),
                )),
                ..Default::default()
            },
            |ui| {
                show(&mut app, ui, Some(Tab::EffectControls));
            },
        )
        .textures_delta
        .clear();
        assert!(has_text(
            &frame(&ctx, &mut app, vec![]),
            super::super::icons::CARET_DOWN
        ));
        assert_eq!(before, app.dock_state.layout);
    }
    #[test]
    fn both_presentations_share_tabs_active_splits_and_hidden_state() {
        let workspace = super::super::layout::create_initial_layout();
        let projected = project(&workspace.layout, egui::vec2(1200.0, 800.0));
        assert_eq!(read_layout(&projected, &workspace.layout), workspace.layout);
        let mut edited = projected;
        let path = edited.find_tab(&Tab::HardwareMonitor).unwrap();
        edited.set_active_tab(path).unwrap();
        let after = read_layout(&edited, &workspace.layout);
        assert_ne!(after, workspace.layout);
        assert_eq!(after.panels(), workspace.layout.panels());
        let mut round_trip = workspace.clone();
        round_trip.layout = after;
        assert!(round_trip.validate().is_ok());
        assert_eq!(round_trip.hidden, workspace.hidden);
    }

    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn terminal_float_redock_and_renderer_roundtrip_preserve_workspace_members() {
        use craft_ui::docking::{Action, Placement};
        let mut workspace = super::super::layout::create_initial_layout();
        super::super::layout::reveal_and_focus_tab(&mut workspace, Tab::Terminal);
        let original = workspace.layout.clone();
        workspace.apply_all([Action::Float { panel: Tab::Terminal, rect: [40.0, 50.0, 600.0, 300.0] }], |_| Default::default()).unwrap();
        let projected = project(&workspace.layout, egui::vec2(1200.0, 800.0));
        assert_eq!(read_layout(&projected, &workspace.layout), workspace.layout);
        assert!(workspace.layout.floating.iter().any(|window| window.contains(&Tab::Terminal)));
        workspace.apply_all([Action::Move { panel: Tab::Terminal, anchor: Tab::Timeline,
            placement: Placement::Tab { before: None } }], |_| Default::default()).unwrap();
        assert!(workspace.layout.floating.is_empty());
        assert_eq!(workspace.layout.panels().len(), original.panels().len());
        for panel in original.panels() { assert!(workspace.layout.contains(panel)); }
        assert!(workspace.validate().is_ok());
    }

    #[test]
    fn floating_projection_preserves_identity_and_placement_before_first_paint() {
        let mut layout = Layout::default();
        let mut floating =
            Floating::single(vec![Tab::EffectControls], 0, [60.0, 80.0, 400.0, 300.0]);
        floating.native = Some([120.0, 90.0, 500.0, 400.0]);
        floating.minimized = true;
        let id = floating.id;
        layout.floating.push(floating);
        let projected = project(&layout, egui::vec2(1200.0, 800.0));
        let restored = read_layout(&projected, &layout);
        assert_eq!(restored, layout);
        assert_eq!(restored.floating[0].id, id);
        assert!(restored.validate().is_ok());
    }

    #[test]
    fn editing_one_branch_retains_other_accordion_and_fixed_split() {
        let stack = Node::Stack {
            entries: vec![craft_ui::docking::StackEntry {
                panel: Tab::HardwareMonitor,
                open: true,
                height: Some(300.0),
            }],
        };
        let source = Node::Split {
            axis: SplitAxis::Horizontal,
            size: SplitSize::FixedFirst(320.0),
            first: Box::new(stack.clone()),
            second: Box::new(Node::Tabs {
                panels: vec![Tab::ProgramMonitor, Tab::Timeline],
                active: 0,
            }),
        };
        let layout = Layout {
            root: Some(source.clone()),
            floating: vec![],
        };
        let mut dock = project(&layout, egui::vec2(1200.0, 800.0));
        let before = read_layout(&dock, &layout).root.unwrap();
        dock.set_active_tab(dock.find_tab(&Tab::Timeline).unwrap())
            .unwrap();
        let mut after = read_layout(&dock, &layout).root.unwrap();
        retain_unchanged(&source, &before, &mut after);
        let Node::Split {
            first,
            second,
            size,
            ..
        } = after
        else {
            panic!("split lost")
        };
        assert_eq!(*first, stack);
        assert_eq!(size, SplitSize::FixedFirst(320.0));
        assert!(matches!(*second, Node::Tabs { active: 1, .. }));
    }
}
