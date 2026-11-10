# Workspace Panel Restoration & Layout Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement individual workspace panel restoration via a dedicated `Window` menu, canonical slot insertion, and persistent dock layouts across sessions.

**Architecture:** Enable `egui_dock`'s `serde` feature, persist serialized layout JSON in `AppConfig`, extend `PealayerTab` with metadata, implement canonical slot placement for reopening closed tabs, and provide a top-level `Window` menu with panel checkmarks.

**Tech Stack:** Rust, `egui`, `egui_dock` (0.19.1 with `serde`), `serde_json`, `egui-phosphor`.

**Spec:** [`docs/superpowers/specs/2026-09-30-workspace-panel-restoration-design.md`](file:///home/toghrol/Documents/rust/pealayer/docs/superpowers/specs/2026-09-30-workspace-panel-restoration-design.md)

## Global Constraints

- Do not break existing initial layout defaults in `create_initial_layout()`.
- Malformed or incompatible saved layout JSON must safely fall back to default layout without crashing or corrupting config.
- All 5 tabs (`ProgramMonitor`, `Timeline`, `EffectControls`, `EffectsLibrary`, `HardwareMonitor`) must be closeable, detectable, and reopenable.
- Maintain full localization (`app.tr(...)`) and Phosphor icon styling across all added menu items and labels.

---

### Task 1: Enable `egui_dock` Serde Feature & Configuration Schema

**Files:**
- Modify: `Cargo.toml:41`
- Modify: `src/config.rs:50-130`
- Test: `tests/workspace_dock_test.rs`

**Interfaces:**
- Consumes: `AppConfig` from `src/config.rs`
- Produces: `AppConfig.workspace_dock_layout: Option<String>`

- [ ] **Step 1: Write the failing test for configuration serialization**

Create `tests/workspace_dock_test.rs`:
```rust
use pealayer::config::AppConfig;

#[test]
fn test_config_workspace_dock_layout_field() {
    let mut config = AppConfig::default();
    assert_eq!(config.workspace_dock_layout, None);

    config.workspace_dock_layout = Some(r#"{"dummy": true}"#.to_string());
    let toml_str = toml::to_string(&config).expect("serialize config to TOML");
    let loaded: AppConfig = toml::from_str(&toml_str).expect("deserialize config from TOML");
    assert_eq!(loaded.workspace_dock_layout, Some(r#"{"dummy": true}"#.to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test workspace_dock_test`
Expected: FAIL with missing field `workspace_dock_layout` on `AppConfig`.

- [ ] **Step 3: Update `Cargo.toml` and `src/config.rs`**

In `Cargo.toml`:
```toml
egui_dock = { version = "0.19.1", features = ["serde"] }
```

In `src/config.rs`, add field to `AppConfig`:
```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_dock_layout: Option<String>,
```
In `AppConfig::default()`:
```rust
    workspace_dock_layout: None,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test workspace_dock_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/config.rs tests/workspace_dock_test.rs
git commit -m "feat(config): enable egui_dock serde and add workspace_dock_layout field"
```

---

### Task 2: `PealayerTab` Metadata & DockState Serde Round-Trip

**Files:**
- Modify: `src/ui/layout.rs:303-345`
- Test: `tests/workspace_dock_test.rs`

**Interfaces:**
- Consumes: `egui_dock::DockState`, `crate::ui::icons`
- Produces: `PealayerTab::ALL`, `PealayerTab::title(&self, app)`, `PealayerTab::icon(&self)`

- [ ] **Step 1: Write failing test for `PealayerTab` metadata and `DockState` round-trip**

Append to `tests/workspace_dock_test.rs`:
```rust
use egui_dock::DockState;
use pealayer::ui::layout::{create_initial_layout, PealayerTab};

#[test]
fn test_pealayer_tab_all_contains_five_tabs() {
    assert_eq!(PealayerTab::ALL.len(), 5);
    assert!(PealayerTab::ALL.contains(&PealayerTab::ProgramMonitor));
    assert!(PealayerTab::ALL.contains(&PealayerTab::Timeline));
    assert!(PealayerTab::ALL.contains(&PealayerTab::EffectControls));
    assert!(PealayerTab::ALL.contains(&PealayerTab::EffectsLibrary));
    assert!(PealayerTab::ALL.contains(&PealayerTab::HardwareMonitor));
}

#[test]
fn test_dock_state_serde_roundtrip() {
    let dock_state = create_initial_layout();
    let json_str = serde_json::to_string(&dock_state).expect("serialize dock state to JSON");
    assert!(!json_str.is_empty());

    let deserialized: DockState<PealayerTab> =
        serde_json::from_str(&json_str).expect("deserialize dock state from JSON");

    for tab in PealayerTab::ALL {
        assert!(
            deserialized.find_tab(&tab).is_some(),
            "tab {:?} should be present after round-trip",
            tab
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test workspace_dock_test`
Expected: FAIL with `ALL` not found on `PealayerTab`.

- [ ] **Step 3: Implement metadata on `PealayerTab`**

In `src/ui/layout.rs`:
```rust
impl PealayerTab {
    pub const ALL: [PealayerTab; 5] = [
        PealayerTab::ProgramMonitor,
        PealayerTab::Timeline,
        PealayerTab::EffectControls,
        PealayerTab::EffectsLibrary,
        PealayerTab::HardwareMonitor,
    ];

    pub fn title(self, app: &crate::app::PealayerApp) -> String {
        match self {
            PealayerTab::ProgramMonitor => app.tr("Program Monitor"),
            PealayerTab::Timeline => app.tr("Timeline"),
            PealayerTab::EffectControls => app.tr("Effect Controls"),
            PealayerTab::EffectsLibrary => app.tr("Effects Library"),
            PealayerTab::HardwareMonitor => app.tr("Hardware Monitor"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            PealayerTab::ProgramMonitor => crate::ui::icons::MONITOR_PLAY,
            PealayerTab::Timeline => crate::ui::icons::WAVEFORM,
            PealayerTab::EffectControls => crate::ui::icons::SLIDERS_HORIZONTAL,
            PealayerTab::EffectsLibrary => crate::ui::icons::SPARKLE,
            PealayerTab::HardwareMonitor => crate::ui::icons::GAUGE,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test workspace_dock_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/workspace_dock_test.rs
git commit -m "feat(ui): add PealayerTab catalog and verify DockState serde round-trip"
```

---

### Task 3: Canonical Slot Placement & Dock Mutation Algorithm

**Files:**
- Modify: `src/ui/layout.rs`
- Modify: `src/app.rs`
- Test: `tests/workspace_dock_test.rs`

**Interfaces:**
- Consumes: `DockState<PealayerTab>`, `PealayerTab`
- Produces: `PealayerApp::is_tab_open(&self, tab) -> bool`, `PealayerApp::open_or_focus_tab(&mut self, tab)`, `PealayerApp::toggle_tab(&mut self, tab)`

- [ ] **Step 1: Write failing tests for canonical slot placement and tab operations**

Append to `tests/workspace_dock_test.rs`:
```rust
use pealayer::ui::layout::restore_tab_to_canonical_slot;

#[test]
fn test_restore_timeline_to_canonical_slot() {
    let mut dock_state = create_initial_layout();
    let path = dock_state.find_tab(&PealayerTab::Timeline).expect("find timeline");
    dock_state.remove_tab(path);
    assert!(dock_state.find_tab(&PealayerTab::Timeline).is_none());

    restore_tab_to_canonical_slot(&mut dock_state, PealayerTab::Timeline);
    assert!(dock_state.find_tab(&PealayerTab::Timeline).is_some());
}

#[test]
fn test_restore_controls_next_to_hardware_monitor() {
    let mut dock_state = create_initial_layout();
    let path = dock_state.find_tab(&PealayerTab::EffectControls).expect("find effect controls");
    dock_state.remove_tab(path);
    assert!(dock_state.find_tab(&PealayerTab::EffectControls).is_none());

    restore_tab_to_canonical_slot(&mut dock_state, PealayerTab::EffectControls);
    assert!(dock_state.find_tab(&PealayerTab::EffectControls).is_some());
}

#[test]
fn test_restore_into_empty_dock() {
    let mut dock_state = create_initial_layout();
    for tab in PealayerTab::ALL {
        if let Some(path) = dock_state.find_tab(&tab) {
            dock_state.remove_tab(path);
        }
    }
    assert_eq!(dock_state.iter_all_tabs().count(), 0);

    restore_tab_to_canonical_slot(&mut dock_state, PealayerTab::ProgramMonitor);
    assert!(dock_state.find_tab(&PealayerTab::ProgramMonitor).is_some());
    assert_eq!(dock_state.iter_all_tabs().count(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test workspace_dock_test`
Expected: FAIL with `restore_tab_to_canonical_slot` not found.

- [ ] **Step 3: Implement canonical slot restoration algorithm**

In `src/ui/layout.rs`:
```rust
pub fn restore_tab_to_canonical_slot(dock_state: &mut egui_dock::DockState<PealayerTab>, tab: PealayerTab) {
    if dock_state.find_tab(&tab).is_some() {
        return;
    }

    if dock_state.iter_all_tabs().count() == 0 {
        *dock_state = egui_dock::DockState::new(vec![tab]);
        return;
    }

    match tab {
        PealayerTab::EffectControls => {
            if let Some(sibling_path) = dock_state.find_tab(&PealayerTab::HardwareMonitor) {
                let node_path = sibling_path.node();
                if let Ok(leaf) = dock_state.leaf_mut(node_path) {
                    leaf.tabs.push(tab);
                    return;
                }
            }
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::Timeline))
            {
                let node_index = anchor_path.node_index();
                dock_state.main_surface_mut().split_left(node_index, 0.25, vec![tab]);
                return;
            }
        }
        PealayerTab::HardwareMonitor => {
            if let Some(sibling_path) = dock_state.find_tab(&PealayerTab::EffectControls) {
                let node_path = sibling_path.node();
                if let Ok(leaf) = dock_state.leaf_mut(node_path) {
                    leaf.tabs.push(tab);
                    return;
                }
            }
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::Timeline))
            {
                let node_index = anchor_path.node_index();
                dock_state.main_surface_mut().split_left(node_index, 0.25, vec![tab]);
                return;
            }
        }
        PealayerTab::Timeline => {
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::EffectControls))
                .or_else(|| dock_state.find_tab(&PealayerTab::EffectsLibrary))
            {
                let node_index = anchor_path.node_index();
                dock_state.main_surface_mut().split_below(node_index, 0.7, vec![tab]);
                return;
            }
        }
        PealayerTab::EffectsLibrary => {
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::Timeline))
            {
                let node_index = anchor_path.node_index();
                dock_state.main_surface_mut().split_right(node_index, 0.75, vec![tab]);
                return;
            }
        }
        PealayerTab::ProgramMonitor => {
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::Timeline) {
                let node_index = anchor_path.node_index();
                dock_state.main_surface_mut().split_above(node_index, 0.7, vec![tab]);
                return;
            }
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::EffectControls)
                .or_else(|| dock_state.find_tab(&PealayerTab::HardwareMonitor))
            {
                let node_index = anchor_path.node_index();
                dock_state.main_surface_mut().split_right(node_index, 0.5, vec![tab]);
                return;
            }
        }
    }

    dock_state.push_to_first_leaf(tab);
}
```

In `src/app.rs`, add methods to `PealayerApp`:
```rust
impl PealayerApp {
    pub fn is_tab_open(&self, tab: crate::ui::layout::PealayerTab) -> bool {
        self.dock_state.find_tab(&tab).is_some()
    }

    pub fn open_or_focus_tab(&mut self, tab: crate::ui::layout::PealayerTab) {
        self.show_four_d_editor = true;
        if let Some(path) = self.dock_state.find_tab(&tab) {
            self.dock_state.set_active_tab(path);
        } else {
            crate::ui::layout::restore_tab_to_canonical_slot(&mut self.dock_state, tab);
            if let Some(path) = self.dock_state.find_tab(&tab) {
                self.dock_state.set_active_tab(path);
            }
            self.save_dock_layout();
        }
    }

    pub fn toggle_tab(&mut self, tab: crate::ui::layout::PealayerTab) {
        if let Some(path) = self.dock_state.find_tab(&tab) {
            self.dock_state.remove_tab(path);
            self.save_dock_layout();
        } else {
            self.open_or_focus_tab(tab);
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test workspace_dock_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs src/app.rs tests/workspace_dock_test.rs
git commit -m "feat(ui): implement canonical slot panel restoration and tab toggle"
```

---

### Task 4: Layout Persistence, Launch Recovery & Empty Dock Handling

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Test: `tests/workspace_dock_test.rs`

**Interfaces:**
- Consumes: `AppConfig.workspace_dock_layout`, `PealayerApp::dock_state`
- Produces: `PealayerApp::save_dock_layout(&mut self)`

- [ ] **Step 1: Write test for corrupt JSON fallback and layout save format**

Append to `tests/workspace_dock_test.rs`:
```rust
#[test]
fn test_corrupt_dock_json_fallback() {
    let invalid_json = "{ corrupted invalid json ...";
    let fallback = serde_json::from_str::<DockState<PealayerTab>>(invalid_json)
        .unwrap_or_else(|_| create_initial_layout());

    for tab in PealayerTab::ALL {
        assert!(fallback.find_tab(&tab).is_some());
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test workspace_dock_test`
Expected: PASS.

- [ ] **Step 3: Implement `save_dock_layout` and launch loading**

In `src/app.rs`:
```rust
impl PealayerApp {
    pub fn save_dock_layout(&mut self) {
        if let Ok(json) = serde_json::to_string(&self.dock_state) {
            self.save_config();
        }
    }
}
```
Update `PealayerApp::save_config` in `src/app.rs`:
```rust
if let Ok(json) = serde_json::to_string(&self.dock_state) {
    cfg.workspace_dock_layout = Some(json);
}
```
In `src/main.rs`:
```rust
let dock_state = loaded_config
    .workspace_dock_layout
    .as_deref()
    .and_then(|json| serde_json::from_str::<egui_dock::DockState<crate::ui::layout::PealayerTab>>(json).ok())
    .unwrap_or_else(crate::ui::layout::create_initial_layout);
```
Pass `dock_state` to `PealayerApp` initialization.

In `src/app.rs` CentralPanel rendering when `self.show_four_d_editor` is active:
If `self.dock_state.iter_all_tabs().count() == 0`:
```rust
ui.centered_and_justified(|ui| {
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(crate::ui::icons::TABS).size(36.0));
        ui.add_space(8.0);
        ui.heading(self.tr("All workspace panels are closed"));
        ui.label(self.tr("Open panels from the Window menu above, or reset the workspace."));
        ui.add_space(12.0);
        if ui.button(format!("{} {}", crate::ui::icons::ARROW_COUNTER_CLOCKWISE, self.tr("Reset Workspace to Default"))).clicked() {
            self.dock_state = crate::ui::layout::create_initial_layout();
            self.save_dock_layout();
        }
    });
});
```

- [ ] **Step 4: Verify compilation and tests**

Run: `cargo check --all-targets && cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs tests/workspace_dock_test.rs
git commit -m "feat(ui): serialize dock layout to config and handle empty workspace state"
```

---

### Task 5: Top-Level `Window` Menu & Context Menu Integration

**Files:**
- Modify: `src/ui/menu.rs`
- Modify: `src/app.rs`
- Modify: `src/ui/layout.rs`
- Test: `tests/workspace_dock_test.rs`

**Interfaces:**
- Consumes: `PealayerApp::is_tab_open`, `PealayerApp::toggle_tab`, `PealayerTab::ALL`
- Produces: `Window` menu bar item and dock context menus

- [ ] **Step 1: Write integration check for tab titles and icons**

Append to `tests/workspace_dock_test.rs`:
```rust
#[test]
fn test_pealayer_tab_icons_not_empty() {
    for tab in PealayerTab::ALL {
        assert!(!tab.icon().is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test workspace_dock_test`
Expected: PASS.

- [ ] **Step 3: Implement `Window` menu in `src/ui/menu.rs`**

In `src/ui/menu.rs`, add between `Workspace` and `Help`:
```rust
ui.menu_button(app.tr("Window"), |ui| {
    ui.label(egui::RichText::new(app.tr("Panels")).strong());
    ui.separator();

    for tab in crate::ui::layout::PealayerTab::ALL {
        let is_open = app.is_tab_open(tab);
        let icon = tab.icon();
        let name = tab.title(app);
        let label = format!("{icon}  {name}");

        let mut checked = is_open;
        if ui.checkbox(&mut checked, label).clicked() {
            app.toggle_tab(tab);
            ui.close();
        }
    }

    ui.separator();
    if ui
        .button(format!(
            "{}  {}",
            crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
            app.tr("Reset Workspace to Default")
        ))
        .clicked()
    {
        app.dock_state = crate::ui::layout::create_initial_layout();
        app.save_dock_layout();
        ui.close();
    }
});
```

- [ ] **Step 4: Add Panels submenu to dock and workspace context menus**

In `src/app.rs` (workspace context menu) and `src/ui/layout.rs` (tab context menu):
Add submenu listing panels with checkmarks to toggle directly from the right-click menu.

- [ ] **Step 5: Run full test suite and verify clean builds**

Run: `cargo check --all-targets && cargo test`
Expected: 100% tests pass, zero errors.

- [ ] **Step 6: Commit**

```bash
git add src/ui/menu.rs src/app.rs src/ui/layout.rs tests/workspace_dock_test.rs
git commit -m "feat(ui): add Window menu bar and workspace context menu panel restoration"
```
