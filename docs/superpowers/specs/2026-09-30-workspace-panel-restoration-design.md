# Workspace Panel Restoration & Layout Management Design

## 1. Overview & Motivation

In Pealayer's Non-Linear Editor (NLE) mode, the workspace is partitioned into docking panes managed by `egui_dock::DockState<PealayerTab>`.

Previously, when a user closed a pane (such as the Timeline, Effect Controls, or Hardware Monitor) via its tab `(×)` button, the application provided no mechanism to reopen that specific panel. The only recovery option was a blanket "Restore all workspace tabs" / "Reset workspace layout" action, which destroyed all user customizations, split ratios, and panel arrangements. Furthermore, the dock state was not serialized, causing layout customizations and closed panels to reset across application restarts.

This feature establishes an industry-standard panel management system (aligned with Adobe Premiere Pro and DaVinci Resolve) featuring a top-level **Window** menu, canonical slot restoration, and persistent workspace layouts across sessions.

---

## 2. Invariants & Requirements

1. **Panel Discoverability**: Users must be able to view the open/closed status of every panel in the application from a dedicated top-level `Window` menu.
2. **Individual Restoration**: Any closed panel must be reopenable without resetting other panels or losing existing dock splits.
3. **Canonical Slot Placement**: Reopened panels must intelligently insert into their natural workspace zone (e.g. Timeline at the bottom, Program Monitor in center-top, Effect Controls on left), docking alongside sibling panels when available or splitting surrounding nodes when absent.
4. **All Panels Closeable**: Every workspace panel (including Program Monitor and Timeline) can be closed. If all panels are closed, the workspace renders a clean empty-state placeholder with clear recovery actions.
5. **Session Persistence**: The exact layout (open panels, sizes, split ratios) must persist in `config.toml`. Relaunching the app restores the exact workspace from the previous session.
6. **Robust Error Recovery**: Corrupted or incompatible serialized dock state must safely fall back to `create_initial_layout()` without crashing or corrupting other configuration values.

---

## 3. Data Structures & Schema Changes

### 3.1 Crate Feature Activation (`Cargo.toml`)
Enable the `serde` feature flag for `egui_dock`:
```toml
egui_dock = { version = "0.19.1", features = ["serde"] }
```

### 3.2 Configuration Schema (`src/config.rs`)
Add `workspace_dock_layout` to `AppConfig`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_dock_layout: Option<String>,
}
```
* **Default**: `None`.
* When `None`, `PealayerApp` initializes with `crate::ui::layout::create_initial_layout()`.
* When `Some(json_str)`, `PealayerApp` attempts `serde_json::from_str::<egui_dock::DockState<PealayerTab>>(&json_str)`. If deserialization fails, it falls back to default.

### 3.3 `PealayerTab` Extensions (`src/ui/layout.rs`)
Expand `PealayerTab` with metadata methods:
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

---

## 4. Dock Manipulation & Placement Logic

### 4.1 State Query
```rust
impl PealayerApp {
    pub fn is_tab_open(&self, tab: PealayerTab) -> bool {
        self.dock_state.find_tab(&tab).is_some()
    }
}
```

### 4.2 Canonical Slot Reopen (`open_or_focus_tab`)
When `open_or_focus_tab(tab)` is invoked:
1. Ensure NLE workspace is active: `self.show_four_d_editor = true`.
2. Check if already open:
   ```rust
   if let Some(path) = self.dock_state.find_tab(&tab) {
       self.dock_state.set_active_tab(path);
       return;
   }
   ```
3. If dock is completely empty (`self.dock_state.surfaces_count() == 0` or all trees empty):
   ```rust
   self.dock_state = egui_dock::DockState::new(vec![tab]);
   self.save_dock_layout();
   return;
   ```
4. Canonical Insertion:
   * **`EffectControls`**: If `HardwareMonitor` is open, insert into the same leaf node next to it. Otherwise, split the primary top/center node to the left (fraction `0.25`).
   * **`HardwareMonitor`**: If `EffectControls` is open, insert into the same leaf node next to it. Otherwise, split the primary top/center node to the left (fraction `0.25`).
   * **`Timeline`**: If any upper node exists (`ProgramMonitor`, `EffectControls`, etc.), split below it (fraction `0.7`, placing Timeline at bottom 30%).
   * **`EffectsLibrary`**: If center node exists, split right (fraction `0.75`).
   * **`ProgramMonitor`**: If `Timeline` exists, split above it. If left/right nodes exist, split between them.
   * **Fallback**: If specific node search fails, call `self.dock_state.push_to_first_leaf(tab)`.
5. Set the newly added tab active:
   ```rust
   if let Some(path) = self.dock_state.find_tab(&tab) {
       self.dock_state.set_active_tab(path);
   }
   self.save_dock_layout();
   ```

### 4.3 Tab Toggling (`toggle_tab`)
```rust
impl PealayerApp {
    pub fn toggle_tab(&mut self, tab: PealayerTab) {
        if let Some(path) = self.dock_state.find_tab(&tab) {
            self.dock_state.remove_tab(path);
            self.save_dock_layout();
        } else {
            self.open_or_focus_tab(tab);
        }
    }
}
```

### 4.4 Tab Closing Callback
In `PealayerTabViewer`:
```rust
fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
    // Return true to allow egui_dock to remove the tab.
    // Post-close layout save is triggered in app frame processing.
    true
}
```

---

## 5. UI Integration

### 5.1 Menu Bar (`src/ui/menu.rs`)
Add a dedicated `Window` top-level menu button between `Subtitles`/`Workspace` and `Help`:
```rust
ui.menu_button(app.tr("Window"), |ui| {
    ui.label(egui::RichText::new(app.tr("Panels")).strong());
    ui.separator();

    for tab in PealayerTab::ALL {
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
    if ui.button(format!("{}  {}", crate::ui::icons::TABS, app.tr("Reset Workspace to Default"))).clicked() {
        app.dock_state = crate::ui::layout::create_initial_layout();
        app.save_dock_layout();
        ui.close();
    }
});
```

### 5.2 Context Menus
Update the context menus in `src/app.rs` (workspace header) and `src/ui/layout.rs` (tab header) to include a "Panels" submenu listing all tabs with checkmarks, allowing operators to restore closed panes without leaving the work surface.

### 5.3 Empty Dock Canvas State
In `src/app.rs`, if `self.show_four_d_editor` is active but `self.dock_state.iter_all_tabs().count() == 0`:
Display an aesthetic, centered placeholder:
* Phosphor icon: `crate::ui::icons::TABS`
* Title: `app.tr("All workspace panels are closed")`
* Description: `app.tr("Open panels from the Window menu above, or reset to the default layout.")`
* Action button: `Reset Workspace to Default`

---

## 6. Serialization & Persistence Architecture

1. `PealayerApp::save_dock_layout(&mut self)`:
   Serializes `self.dock_state` to JSON via `serde_json::to_string(&self.dock_state)`. Stores in `self.workspace_dock_layout` and triggers `self.save_config()`.
2. App Launch in `main.rs`:
   ```rust
   let initial_dock_state = loaded_config
       .workspace_dock_layout
       .as_deref()
       .and_then(|json| serde_json::from_str::<egui_dock::DockState<PealayerTab>>(json).ok())
       .unwrap_or_else(crate::ui::layout::create_initial_layout);
   ```

---

## 7. Verification & Testing Plan

### 7.1 Automated Integration Tests (`tests/workspace_dock_test.rs`)
1. `test_default_layout_all_tabs_open`:
   Verify `create_initial_layout()` produces all 5 tabs and `is_tab_open` is true for each.
2. `test_close_and_reopen_single_tab`:
   Remove `PealayerTab::Timeline`, verify `is_tab_open(Timeline)` is false, then call `open_or_focus_tab(Timeline)`, verify it is restored and focused.
3. `test_close_all_tabs_and_recover`:
   Remove all 5 tabs until dock is empty. Verify dock has 0 tabs. Call `open_or_focus_tab(ProgramMonitor)`, verify dock tree reinitializes without error.
4. `test_canonical_placement_grouping`:
   Close `HardwareMonitor`, reopen it while `EffectControls` is open, verify `HardwareMonitor` docks into the same leaf node as `EffectControls`.
5. `test_serde_roundtrip_preservation`:
   Serialize custom dock state to JSON, deserialize back, verify node topology, tab count, and tab identities match.
6. `test_corrupted_json_fallback`:
   Provide invalid JSON to deserializer, verify it safely returns default layout without panic.

### 7.2 Manual Quality Checklist
1. Close each of the 5 panels in NLE mode; confirm checkbox in `Window` menu becomes unchecked.
2. Reopen each closed panel from `Window` menu; confirm it returns to its expected natural split.
3. Close all 5 panels; confirm empty state displays and "Reset Workspace to Default" restores all 5.
4. Close 2 panels, adjust split sizes, restart application; confirm exact layout is remembered.
