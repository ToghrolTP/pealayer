//! egui Dock presentation of the shared workspace. Panels, hidden locations,
//! profiles and commands have one owner; changing chrome never reloads media.
use super::layout::{PealayerTab as Tab, PealayerTabViewer};
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
    layer: egui::LayerId,
}

fn tab_buttons_id() -> egui::Id {
    egui::Id::new("egui-dock-tab-buttons")
}

// The pinned egui_dock renderer uses these private, fixed-width hit slots.
// Keep geometry tests coupled to its real tab responses when updating it.
const HEADER_BUTTON_WIDTH: f32 = 24.0;

/// Both renderers consume the live application palette; never reinstall a theme.
fn dock_style(ui: &egui::Ui) -> egui_dock::Style {
    let t = super::craft_theme::current(ui.ctx());
    let mut style = egui_dock::Style::from_egui(ui.style());
    style.dock_area_padding = None;
    style.main_surface_border_stroke = egui::Stroke::NONE;
    style.main_surface_border_rounding = egui::CornerRadius::ZERO;
    style.tab_bar.height = t
        .tab_h
        .max(ui.text_style_height(&egui::TextStyle::Body) + 6.0);
    style.tab_bar.bg_fill = t.tab_bg;
    style.tab_bar.hline_color = t.separator;
    style.tab.active.bg_fill = t.panel_bg;
    style.tab.active.text_color = t.tab_text_active;
    style.tab.active.outline_color = t.separator;
    style.tab.active.corner_radius = egui::CornerRadius::ZERO;
    style.tab.inactive.bg_fill = t.tab_bg;
    style.tab.inactive.text_color = t.tab_text;
    style.tab.inactive.outline_color = egui::Color32::TRANSPARENT;
    style.tab.inactive.corner_radius = egui::CornerRadius::ZERO;
    style.tab.focused.bg_fill = t.panel_bg;
    style.tab.focused.text_color = t.tab_text_active;
    style.tab.focused.outline_color = t.focus;
    style.tab.focused.corner_radius = egui::CornerRadius::ZERO;
    style.tab.hovered.bg_fill = t.hover;
    style.tab.hovered.text_color = t.tab_text_active;
    style.tab.hovered.corner_radius = egui::CornerRadius::ZERO;
    style.tab.tab_body.stroke = egui::Stroke::new(1.0, t.separator);
    style.tab.tab_body.corner_radius = egui::CornerRadius::ZERO;
    style.separator.width = 1.0;
    style.separator.color_idle = t.separator;
    style.separator.color_hovered = t.focus;
    style.separator.color_dragged = t.accent_hover;
    style.separator.extra_interact_width = t.gap;
    style.buttons.close_tab_color = t.icon;
    style.buttons.close_tab_active_color = t.tab_text_active;
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

/// Register only genuinely empty header space, not a y-band across unrelated
/// panels. Paint floating chrome on its own layer, never behind the window.
fn decorate_headers(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    dock: &DockState<Tab>,
    height: f32,
    interactive: bool,
) {
    let buttons = ui
        .data(|d| d.get_temp::<Vec<TabButton>>(tab_buttons_id()))
        .unwrap_or_default();
    let t = super::craft_theme::current(ui.ctx());
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
        if !app.dock_state.locked {
            let rect = disclosure_rect(header);
            let hovered = ui
                .ctx()
                .pointer_hover_pos()
                .is_some_and(|p| rect.contains(p))
                && ui.ctx().layer_id_at(rect.center()) == Some(button.layer);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if leaf.collapsed {
                    super::icons::CARET_RIGHT
                } else {
                    super::icons::CARET_DOWN
                },
                egui::FontId::proportional(13.0),
                if hovered { t.tab_text_active } else { t.icon },
            );
        }
        if dock.focused_leaf() == Some(path) {
            super::craft_theme::paint_active_panel_outline(&painter, leaf.rect, &t);
        }
        let left = buttons
            .iter()
            .filter(|b| leaf.tabs.contains(&b.tab))
            .map(|b| b.rect.right())
            .fold(header.left(), f32::max)
            .max(disclosure_rect(header).right());
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

impl egui_dock::TabViewer for PealayerTabViewer<'_> {
    type Tab = Tab;
    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        format!("{} {}", tab.icon(), tab.title(self.app)).into()
    }
    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(("workspace-panel", *tab))
    }
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        PealayerTabViewer::ui(self, ui, tab);
    }
    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        PealayerTabViewer::scroll_bars(self, tab)
    }
    fn is_closeable(&self, _tab: &Tab) -> bool {
        !self.app.dock_state.locked
    }
    fn allowed_in_windows(&self, _tab: &mut Tab) -> bool {
        !self.app.dock_state.locked
    }
    fn on_tab_button(&mut self, tab: &mut Tab, response: &egui::Response) {
        response.ctx.data_mut(|data| {
            let buttons = data.get_temp_mut_or_default::<Vec<TabButton>>(tab_buttons_id());
            buttons.push(TabButton {
                tab: *tab,
                rect: response.rect,
                layer: response.layer_id,
            });
        });
    }
    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Tab, _node: egui_dock::NodePath) {
        self.draw_context_menu(ui, *tab);
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
    let layer = ui.layer_id();
    let blocked = !ui.ctx().memory(|m| m.is_above_modal_layer(layer));
    ui.data_mut(|d| d.insert_temp(tab_buttons_id(), Vec::<TabButton>::new()));
    let style = dock_style(ui);
    let header_height = style.tab_bar.height;
    app.workspace_rendering = true;
    ui.add_enabled_ui(!blocked, |ui| {
        let mut viewer = PealayerTabViewer { app };
        egui_dock::DockArea::new(&mut projection.dock)
            .id(egui::Id::new("egui-workspace-dock"))
            .style(style)
            .draggable_tabs(!locked && !blocked)
            .show_close_buttons(!locked)
            .show_window_close_buttons(!locked)
            .show_leaf_close_all_buttons(!locked)
            .show_leaf_collapse_buttons(!locked)
            .show_inside(ui, &mut viewer);
    });
    decorate_headers(
        app,
        ui,
        &projection.dock,
        header_height,
        !blocked && ui.is_enabled(),
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
    fn restored_header_chevrons_and_palette_match_live_renderer_geometry() {
        let _guard = crate::app::tests::lock_app_tests();
        for dark in [false, true] {
            for scale in [1.0, 1.25, 2.0] {
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
                    let style = dock_style(ui);
                    let tokens = super::super::craft_theme::current(&ctx);
                    assert_eq!(style.tab.focused.outline_color, tokens.focus);
                    assert_eq!(style.tab_bar.bg_fill, tokens.tab_bg);
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
