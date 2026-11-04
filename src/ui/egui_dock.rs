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
        let _ = projection.dock.set_active_tab(path);
        projection
            .dock
            .set_focused_node_and_surface(egui_dock::NodePath::new(path.surface, path.node));
    }
    let before = read_layout(&projection.dock, &projection.source);
    let locked = app.dock_state.locked;
    let layer = ui.layer_id();
    let blocked = !ui.ctx().memory(|m| m.is_above_modal_layer(layer));
    app.workspace_rendering = true;
    ui.add_enabled_ui(!blocked, |ui| {
        let mut viewer = PealayerTabViewer { app };
        egui_dock::DockArea::new(&mut projection.dock)
            .id(egui::Id::new("egui-workspace-dock"))
            .style(egui_dock::Style::from_egui(ui.style()))
            .draggable_tabs(!locked && !blocked)
            .show_close_buttons(!locked)
            .show_window_close_buttons(!locked)
            .show_leaf_close_all_buttons(!locked)
            .show_inside(ui, &mut viewer);
    });
    app.workspace_rendering = false;
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
