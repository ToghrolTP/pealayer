# Craft UI Phase 6: Docking Engine Migration (`egui_dock` $\to$ `craft_ui::docking`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Completely replace the legacy `egui_dock` dependency with `craft_ui::docking`, adopting Craft UI's declarative `Workspace<PealayerTab>`, `DockArea`, `DockStyle`, and `DockContent<PealayerTab>` across Pealayer.

**Architecture:** Implement the `craft_ui::docking::DockContent<PealayerTab>` trait on `PealayerTabViewer`, migrate `create_initial_layout` to declarative `Workspace<PealayerTab>` splits, replace `egui_dock::DockArea` with non-reentrant snapshot-borrowed `craft_ui::docking::DockArea::show_content`, configure `craft_ui::docking::DockStyle` in `craft_theme.rs`, and remove the `egui_dock` dependency from `Cargo.toml`.

**Tech Stack:** Rust (edition 2024), `craft-ui` (`craft_ui::docking::{Workspace, Layout, Node, DockArea, DockContent, DockStyle, Permissions, Action}`), `eframe`/`egui` 0.37.

**Spec:** Craft UI Design System Migration Specification & AGENTS.md alpha contract.

## Global Constraints

- Use isolated config file (`PEALAYER_CONFIG_FILE=$(mktemp)`) for any persistence testing.
- Single current-contract rule: do not keep legacy migration shims or obsolete version aliases for `egui_dock`.
- Preserve all 4 control densities (`UiControlDensity`: Compact 24, Regular 28, Comfortable 32, Spacious 36).
- Modal corner-close controls must stay on `ui::dialog::close_button`.
- Context lock is non-reentrant: snapshot layout before calling `DockArea::show_content`, and apply returned actions to `Workspace` afterwards.
- Relay ON/OFF actions use semantic green/red; E-STOP release confirmation modal remains intact.
- Keep all existing tests green throughout the migration.

---

### Task 1: `DockContent` Trait Implementation on `PealayerTabViewer`

**Files:**
- Modify: `src/ui/layout.rs:13782-14350`
- Modify: `src/ui/sidebar_rail.rs:105-115`
- Test: `src/ui/layout.rs` (unit tests for `DockContent` methods)

**Interfaces:**
- Consumes: `craft_ui::docking::{DockContent, Permissions, Action}`
- Produces: `impl<'a> craft_ui::docking::DockContent<PealayerTab> for PealayerTabViewer<'a>`

- [x] **Step 1: Write the failing unit tests for `DockContent` on `PealayerTabViewer`**

Add unit tests to `src/ui/layout.rs` testing that `PealayerTabViewer` implements `DockContent<PealayerTab>`:
```rust
#[test]
fn pealayer_tab_viewer_implements_dock_content() {
    use craft_ui::docking::DockContent;
    let mut app = PealayerApp::default();
    let mut viewer = PealayerTabViewer { app: &mut app };
    
    assert_eq!(viewer.logical_label(&PealayerTab::ProgramMonitor), "Program Monitor");
    assert_eq!(viewer.logical_label(&PealayerTab::Timeline), "Timeline");
    
    let permissions = viewer.permissions(&PealayerTab::ProgramMonitor);
    assert!(permissions.movable);
    assert!(permissions.closable);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --lib pealayer_tab_viewer_implements_dock_content`
Expected: FAIL to compile or fail because `DockContent` is not yet implemented on `PealayerTabViewer`.

- [x] **Step 3: Implement `craft_ui::docking::DockContent<PealayerTab>` on `PealayerTabViewer`**

In `src/ui/layout.rs`:
```rust
impl<'a> craft_ui::docking::DockContent<PealayerTab> for PealayerTabViewer<'a> {
    fn body(&mut self, ui: &mut egui::Ui, panel: &PealayerTab) {
        self.draw_tab_body(ui, *panel);
    }

    fn logical_label(&self, panel: &PealayerTab) -> String {
        panel.title(self.app)
    }

    fn label(&mut self, _ui: &egui::Ui, panel: &PealayerTab, logical: &str, _font: &egui::FontId, _color: egui::Color32) -> egui::WidgetText {
        format!("{} {logical}", panel.icon()).into()
    }

    fn permissions(&self, _panel: &PealayerTab) -> craft_ui::docking::Permissions {
        craft_ui::docking::Permissions {
            movable: true,
            floatable: true,
            closable: true,
            accepts_tabs: true,
            accepts_splits: true,
        }
    }

    fn panel_menu(&mut self, ui: &mut egui::Ui, panel: &PealayerTab) {
        self.draw_context_menu(ui, *panel);
    }
}
```
Extract `draw_tab_body` from existing `ui(&mut self, ui, tab)` and `draw_context_menu` from existing `context_menu`.
Keep `TabViewer` temporarily if needed for other code or refactor callers.
In `src/ui/sidebar_rail.rs:109`:
Replace:
```rust
egui_dock::TabViewer::ui(&mut viewer, ui, &mut tab);
```
with:
```rust
craft_ui::docking::DockContent::body(&mut viewer, ui, &tab);
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --lib pealayer_tab_viewer_implements_dock_content`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/ui/layout.rs src/ui/sidebar_rail.rs
git commit -m "feat(docking): implement DockContent on PealayerTabViewer"
```

---

### Task 2: Workspace Layout Model & Helper Migration

**Files:**
- Modify: `src/ui/layout.rs:19956-20185`
- Test: `src/ui/layout.rs` (layout tests for `create_initial_layout` and `reveal_and_focus_tab`)

**Interfaces:**
- Consumes: `craft_ui::docking::{Workspace, Layout, Node, SplitAxis, SplitSize, Action}`
- Produces: `create_initial_layout() -> craft_ui::docking::Workspace<PealayerTab>`, `reveal_and_focus_tab(&mut Workspace<PealayerTab>, PealayerTab) -> bool`

- [x] **Step 1: Write the failing unit tests for `Workspace` layout and `reveal_and_focus_tab`**

Add unit tests in `src/ui/layout.rs`:
```rust
#[test]
fn craft_workspace_initial_layout_hierarchy() {
    let workspace = create_initial_layout();
    assert!(workspace.validate().is_ok());
    assert!(workspace.layout.contains(&PealayerTab::Timeline));
    assert!(workspace.layout.contains(&PealayerTab::ProgramMonitor));
    assert!(workspace.layout.contains(&PealayerTab::EffectControls));
    assert!(workspace.layout.contains(&PealayerTab::HardwareMonitor));
    assert!(workspace.layout.contains(&PealayerTab::EffectsLibrary));
    assert!(!workspace.layout.contains(&PealayerTab::MediaInspector));
}

#[test]
fn craft_workspace_reveal_and_focus_tab() {
    let mut workspace = create_initial_layout();
    // Close EffectControls
    let _ = workspace.set_visible(PealayerTab::EffectControls, false, None, |_| craft_ui::docking::Permissions::default());
    assert!(!workspace.layout.contains(&PealayerTab::EffectControls));
    
    // Reveal and focus
    let changed = reveal_and_focus_tab(&mut workspace, PealayerTab::EffectControls);
    assert!(changed);
    assert!(workspace.layout.contains(&PealayerTab::EffectControls));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --lib craft_workspace_initial_layout_hierarchy`
Expected: FAIL with compilation errors on return types.

- [x] **Step 3: Implement `create_initial_layout` and `reveal_and_focus_tab` using `craft_ui::docking::Workspace`**

In `src/ui/layout.rs`:
```rust
use craft_ui::docking::{Workspace, Layout, Node, Action, Permissions};
use craft_ui::layout::{SplitAxis, SplitSize};

pub fn create_initial_layout() -> Workspace<PealayerTab> {
    let left_tabs = Node::Tabs {
        panels: vec![PealayerTab::EffectControls, PealayerTab::HardwareMonitor],
        active: 0,
    };
    let center_tab = Node::Tabs {
        panels: vec![PealayerTab::ProgramMonitor],
        active: 0,
    };
    let right_tab = Node::Tabs {
        panels: vec![PealayerTab::EffectsLibrary],
        active: 0,
    };
    let center_right_split = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::Ratio(0.70),
        first: Box::new(center_tab),
        second: Box::new(right_tab),
    };
    let top_split = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::Ratio(0.25),
        first: Box::new(left_tabs),
        second: Box::new(center_right_split),
    };
    let bottom_tab = Node::Tabs {
        panels: vec![PealayerTab::Timeline],
        active: 0,
    };
    let root = Node::Split {
        axis: SplitAxis::Vertical,
        size: SplitSize::Ratio(0.70),
        first: Box::new(top_split),
        second: Box::new(bottom_tab),
    };

    let layout = Layout {
        root: Some(root),
        floating: Vec::new(),
    };

    let mut workspace = Workspace::new(layout);
    // Hidden MediaInspector default location
    workspace.hidden = vec![
        (PealayerTab::MediaInspector, craft_ui::docking::Location {
            anchor: Some(PealayerTab::EffectsLibrary),
            group: None,
            placement: craft_ui::docking::Placement::Tab { before: None },
        })
    ];
    workspace
}

pub fn reveal_and_focus_tab(
    workspace: &mut Workspace<PealayerTab>,
    tab: PealayerTab,
) -> bool {
    let was_open = workspace.layout.contains(&tab);
    let fallback = Some(PealayerTab::ProgramMonitor);
    let _ = workspace.set_visible(tab, true, fallback, |_| Permissions::default());
    !was_open
}
```
Update existing unit tests in `src/ui/layout.rs` that referenced `DockState` methods to use `Workspace` methods.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --lib craft_workspace_initial_layout_hierarchy` and `cargo test --lib craft_workspace_reveal_and_focus_tab`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/ui/layout.rs
git commit -m "feat(docking): migrate layout hierarchy and tab visibility to Workspace"
```

---

### Task 3: `PealayerApp` Integration & `DockArea` Rendering

**Files:**
- Modify: `src/app.rs`
- Modify: `src/startup.rs:615-635`
- Modify: `src/ui/craft_theme.rs:575-615, 715-730`
- Modify: `src/ui/menu.rs:540-550`
- Test: `src/app.rs` (workspace and dock tests)

**Interfaces:**
- Consumes: `craft_ui::docking::{Workspace, DockArea, DockStyle, DockOutput}`
- Produces: `app.dock_state: craft_ui::docking::Workspace<crate::ui::layout::PealayerTab>`, `apply_dock_style(&mut DockStyle, &Tokens)`

- [x] **Step 1: Write failing test in `src/app.rs` for `Workspace` serialization and rendering state**

Write a test in `src/app.rs` verifying that `app.dock_state` is a valid `Workspace` and serializes/deserializes cleanly in `WorkspaceProfile`:
```rust
#[test]
fn craft_app_workspace_profile_preserves_layout() {
    let mut app = PealayerApp::default();
    assert!(app.dock_state.validate().is_ok());
    assert!(app.is_tab_open(crate::ui::layout::PealayerTab::Timeline));
    
    app.toggle_tab(crate::ui::layout::PealayerTab::Timeline);
    assert!(!app.is_tab_open(crate::ui::layout::PealayerTab::Timeline));
    
    let profile = app.capture_workspace_profile(&egui::Context::default());
    assert!(profile.dock_layout.is_some());
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --lib craft_app_workspace_profile_preserves_layout`
Expected: FAIL to compile or fail because `dock_state` is still `egui_dock::DockState`.

- [x] **Step 3: Update `PealayerApp`, `startup.rs`, `craft_theme.rs`, and `app.rs`**

1. In `src/app.rs`:
   - Change `dock_state: craft_ui::docking::Workspace<crate::ui::layout::PealayerTab>`.
   - Update `is_tab_open(&self, tab)` -> `self.dock_state.layout.contains(&tab)`.
   - Update `open_or_focus_tab` -> calls `reveal_and_focus_tab(&mut self.dock_state, tab)`.
   - Update `toggle_tab(&mut self, tab)`:
     ```rust
     let is_open = self.dock_state.layout.contains(&tab);
     let _ = self.dock_state.set_visible(tab, !is_open, Some(crate::ui::layout::PealayerTab::ProgramMonitor), |_| craft_ui::docking::Permissions::default());
     self.save_dock_layout();
     ```
   - Update `show_four_d_editor`:
     Snapshot layout:
     ```rust
     let layout_snapshot = self.dock_state.layout.clone();
     let mut dock_style = craft_ui::docking::DockStyle::from_ui(ui);
     crate::ui::craft_theme::apply_dock_style(&mut dock_style, &craft_tokens);
     let output = ui.scope(|ui| {
         let mut tab_viewer = crate::ui::layout::PealayerTabViewer { app: self };
         craft_ui::docking::DockArea::new(egui::Id::new("workspace_dock_area"))
             .locked(self.dock_state.locked)
             .show_content(ui, &layout_snapshot, &dock_style, &mut tab_viewer)
     }).inner;
     let _ = self.dock_state.apply_all(output.actions, |_| craft_ui::docking::Permissions::default());
     ```
   - Update `save_dock_layout`:
     ```rust
     pub fn save_dock_layout(&mut self) {
         self.save_config();
     }
     ```
   - Update `capture_workspace_profile` and `apply_workspace_profile` to serialize/deserialize `Workspace<PealayerTab>`.
2. In `src/ui/craft_theme.rs`:
   Update `apply_dock_style`:
   ```rust
   pub fn apply_dock_style(style: &mut craft_ui::docking::DockStyle, t: &Tokens) {
       style.tab_height = t.tab_h;
       style.background = t.panel_bg;
       style.tab_background = t.tab_bg;
       style.active_background = t.panel_bg;
       style.text = t.tab_text_active;
       style.inactive_text = t.tab_text;
       style.border = Stroke::new(1.0, t.separator);
       style.accent = t.focus;
       style.floating_background = t.panel_bg;
       style.floating_stroke = Stroke::new(1.0, t.border);
   }
   ```
3. In `src/startup.rs`:
   Deserialize `Workspace<PealayerTab>`, fall back to `create_initial_layout()`.
4. Update remaining tests in `src/app.rs`.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --lib craft_app_workspace_profile_preserves_layout`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/app.rs src/startup.rs src/ui/craft_theme.rs src/ui/menu.rs
git commit -m "feat(docking): integrate Workspace and DockArea in PealayerApp"
```

---

### Task 4: Remove `egui_dock` Dependency & Full Verification

**Files:**
- Modify: `Cargo.toml:29`
- Modify: `src/ui/about.rs:340`
- Modify: Any remaining references in `src/`
- Test: Full test suite

**Interfaces:**
- Consumes: None
- Produces: Clean build without `egui_dock`

- [x] **Step 1: Check for any remaining occurrences of `egui_dock`**

Run: `grep -rn "egui_dock" src/`
Expected: Only lines to be removed/updated.

- [x] **Step 2: Remove `egui_dock` from `Cargo.toml` and clean up `src/ui/about.rs`**

Remove line 29 from `Cargo.toml`.
Update `src/ui/about.rs:340` to credit `craft-ui` instead of `egui_dock`.

- [x] **Step 3: Run full verification suite**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib`
Expected: All tests PASS (0 failures).

- [x] **Step 4: Run cargo check on all targets**

Run: `cargo check --lib --bins`
Expected: Clean compilation, 0 warnings/errors.

- [x] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/ui/about.rs src/
git commit -m "chore(docking): remove egui_dock dependency"
```
