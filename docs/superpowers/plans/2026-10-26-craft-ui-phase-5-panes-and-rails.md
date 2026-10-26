# Craft UI Phase 5 (Panes & Property Rails) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate Craft UI's `panes::Rail` and `panes::PaneHost` into Pealayer to provide a collapsible, high-density property sidebar for Hardware Monitor, Media Inspector, Effect Controls, and Effects Library with persisted state and smooth resizing.

**Architecture:** Implement a dedicated `src/ui/sidebar_rail.rs` module defining `SidebarPane` and `SidebarPaneContent` implementing `craft_ui::panes::PaneContent<SidebarPane>`. In `src/app.rs`, before `egui::CentralPanel`, render `craft_ui::panes::Rail` and `craft_ui::panes::PaneHost` with `SidePolicy::OneSlot`, synchronizing the open pane, width, and preferences with `PealayerApp`.

**Tech Stack:** Rust, egui 0.37 (storytold fork), craft-ui 0.1 (`panes`, `rail`, `docking::chrome`, `layout`), serde.

**Spec:** `AGENTS.md`, `docs/ui-panes.md`

## Global Constraints

- Standard input/selector/menu density comes from `ui::control_density` and `ui_control_density` (Compact 24, Regular 28, Comfortable 32, Spacious 36).
- Modal corner-close controls must use `ui::dialog::close_button`.
- Confirmation prompt must use `confirmation_modal`.
- Non-reentrant egui context: never call egui Context accessors or repaint callbacks from inside input/data/memory transactions. Snapshot inputs first.
- Isolated persistence: tests exercising persistence must receive an isolated `PEALAYER_CONFIG_FILE=$(mktemp)`. Pure reconciliation must never save config.
- Relay ON/OFF actions use semantic green/red, not theme accent. E-STOP release GUI confirmation path remains intact.

---

### Task 1: `SidebarPane` Data Model, SVG Icons & `PaneContent` Trait

**Files:**
- Create: `src/ui/sidebar_rail.rs`
- Modify: `src/ui/mod.rs:30-40`
- Test: tests in `src/ui/sidebar_rail.rs`

**Interfaces:**
- Consumes:
  - `crate::ui::layout::PealayerTab`
  - `crate::app::PealayerApp`
  - `craft_ui::panes::{Edge, PaneContent, PaneState}`
- Produces:
  - `pub enum SidebarPane { HardwareMonitor, MediaInspector, EffectControls, EffectsLibrary }`
  - `impl PaneContent<SidebarPane> for SidebarPaneContent<'a>`
  - `pub fn icon_source(pane: SidebarPane) -> egui::ImageSource<'static>`

- [x] **Step 1: Write the failing tests for `SidebarPane` and `PaneContent`**

In `src/ui/sidebar_rail.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use craft_ui::panes::{Edge, PaneContent, Place};

    #[test]
    fn sidebar_pane_conversions_and_titles() {
        assert_eq!(SidebarPane::from(SidebarPane::HardwareMonitor), SidebarPane::HardwareMonitor);
        let tab: crate::ui::layout::PealayerTab = SidebarPane::EffectsLibrary.into();
        assert_eq!(tab, crate::ui::layout::PealayerTab::EffectsLibrary);
        assert_eq!(SidebarPane::try_from(crate::ui::layout::PealayerTab::Timeline), Err(()));
        assert_eq!(SidebarPane::try_from(crate::ui::layout::PealayerTab::MediaInspector), Ok(SidebarPane::MediaInspector));
    }

    #[test]
    fn sidebar_pane_icons_are_valid_svg_sources() {
        for pane in SidebarPane::ALL {
            let source = icon_source(pane);
            match source {
                egui::ImageSource::Bytes { uri, bytes } => {
                    assert!(uri.starts_with("bytes://pealayer/sidebar/"));
                    assert!(!bytes.is_empty());
                }
                _ => panic!("Expected ImageSource::Bytes for {pane:?}"),
            }
        }
    }

    #[test]
    fn sidebar_pane_content_defaults_match_craft_spec() {
        let content = SidebarPaneContentMock;
        for pane in SidebarPane::ALL {
            let state = content.default_state(&pane);
            assert_eq!(state.place, Place::Docked(Edge::End));
            assert_eq!(state.size, Some(300.0));
            assert!(!content.movable(&pane));
            assert!(content.closable(&pane));
            let range = content.size_range(&pane);
            assert_eq!(*range.start(), 240.0);
            assert_eq!(*range.end(), 600.0);
        }
    }
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::sidebar_rail`
Expected: FAIL with module/type not found.

- [ ] **Step 3: Implement `SidebarPane` and `SidebarPaneContent`**

Create `src/ui/sidebar_rail.rs`:
```rust
use std::ops::RangeInclusive;
use egui::load::Bytes;
use egui::{ImageSource, Ui};
use craft_ui::panes::{Edge, PaneContent, PaneState};
use crate::app::PealayerApp;
use crate::ui::layout::PealayerTab;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SidebarPane {
    HardwareMonitor,
    MediaInspector,
    EffectControls,
    EffectsLibrary,
}

impl SidebarPane {
    pub const ALL: [SidebarPane; 4] = [
        SidebarPane::HardwareMonitor,
        SidebarPane::MediaInspector,
        SidebarPane::EffectControls,
        SidebarPane::EffectsLibrary,
    ];

    pub fn title(self, language: crate::config::AppLanguage) -> String {
        PealayerTab::from(self).title_for_language(language)
    }
}

impl From<SidebarPane> for PealayerTab {
    fn from(pane: SidebarPane) -> Self {
        match pane {
            SidebarPane::HardwareMonitor => PealayerTab::HardwareMonitor,
            SidebarPane::MediaInspector => PealayerTab::MediaInspector,
            SidebarPane::EffectControls => PealayerTab::EffectControls,
            SidebarPane::EffectsLibrary => PealayerTab::EffectsLibrary,
        }
    }
}

impl TryFrom<PealayerTab> for SidebarPane {
    type Error = ();
    fn try_from(tab: PealayerTab) -> Result<Self, Self::Error> {
        match tab {
            PealayerTab::HardwareMonitor => Ok(SidebarPane::HardwareMonitor),
            PealayerTab::MediaInspector => Ok(SidebarPane::MediaInspector),
            PealayerTab::EffectControls => Ok(SidebarPane::EffectControls),
            PealayerTab::EffectsLibrary => Ok(SidebarPane::EffectsLibrary),
            _ => Err(()),
        }
    }
}

// Embedded clean 24x24 SVG vectors for high-density rail rendering
const SVG_CIRCUITRY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 9h.01"/><path d="M9 9h.01"/><path d="M9 15h.01"/><path d="M15 15h.01"/><path d="M9 12h6"/><path d="M12 9v6"/></svg>"##;
const SVG_INFO: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>"##;
const SVG_SLIDERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" x2="4" y1="21" y2="14"/><line x1="4" x2="4" y1="10" y2="3"/><line x1="12" x2="12" y1="21" y2="12"/><line x1="12" x2="12" y1="8" y2="3"/><line x1="20" x2="20" y1="21" y2="16"/><line x1="20" x2="20" y1="12" y2="3"/><line x1="1" x2="7" y1="14" y2="14"/><line x1="9" x2="15" y1="8" y2="8"/><line x1="17" x2="23" y1="16" y2="16"/></svg>"##;
const SVG_SPARKLE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"/></svg>"##;

pub fn icon_source(pane: SidebarPane) -> ImageSource<'static> {
    let (uri, bytes) = match pane {
        SidebarPane::HardwareMonitor => ("bytes://pealayer/sidebar/hardware.svg", SVG_CIRCUITRY),
        SidebarPane::MediaInspector => ("bytes://pealayer/sidebar/inspector.svg", SVG_INFO),
        SidebarPane::EffectControls => ("bytes://pealayer/sidebar/controls.svg", SVG_SLIDERS),
        SidebarPane::EffectsLibrary => ("bytes://pealayer/sidebar/library.svg", SVG_SPARKLE),
    };
    ImageSource::Bytes { uri: uri.into(), bytes: Bytes::Static(bytes) }
}

pub struct SidebarPaneContent<'a> {
    pub app: &'a mut PealayerApp,
}

impl<'a> PaneContent<SidebarPane> for SidebarPaneContent<'a> {
    fn title(&self, pane: &SidebarPane) -> String {
        pane.title(self.app.language)
    }

    fn body(&mut self, ui: &mut Ui, pane: &SidebarPane) {
        let mut tab = PealayerTab::from(*pane);
        let mut viewer = crate::ui::layout::PealayerTabViewer { app: self.app };
        egui_dock::TabViewer::ui(&mut viewer, ui, &mut tab);
    }

    fn default_state(&self, _pane: &SidebarPane) -> PaneState {
        PaneState::docked(Edge::End, 300.0)
    }

    fn size_range(&self, _pane: &SidebarPane) -> RangeInclusive<f32> {
        240.0..=600.0
    }

    fn movable(&self, _pane: &SidebarPane) -> bool {
        false
    }

    fn closable(&self, _pane: &SidebarPane) -> bool {
        true
    }
}
```
Expose `pub mod sidebar_rail;` in `src/ui/mod.rs`.

- [x] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::sidebar_rail`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/ui/sidebar_rail.rs src/ui/mod.rs
git commit -m "feat(ui): add SidebarPane model and PaneContent implementation"
```

---

### Task 2: `SidebarRail` UI Component & Interaction Handling

**Files:**
- Modify: `src/ui/sidebar_rail.rs`
- Test: tests in `src/ui/sidebar_rail.rs`

**Interfaces:**
- Consumes:
  - `craft_ui::panes::{Edge, PaneHost, PaneOutput, PaneRequest, Rail, RailItem, RailOutput, RailRequest, SidePolicy}`
  - `crate::ui::craft_theme`
- Produces:
  - `pub struct SidebarRailState { pub open: Option<SidebarPane>, pub width: f32, pub enabled: bool }`
  - `pub fn show_sidebar_rail(app: &mut PealayerApp, ui: &mut egui::Ui, state: &mut SidebarRailState) -> (Option<RailRequest<SidebarPane>>, Option<PaneOutput<SidebarPane>>)`

- [x] **Step 1: Write failing tests for `SidebarRailState` and `show_sidebar_rail`**

In `src/ui/sidebar_rail.rs`:
```rust
#[test]
fn sidebar_rail_state_request_updates() {
    let mut state = SidebarRailState::default();
    assert_eq!(state.open, None);
    assert_eq!(state.width, 300.0);
    assert!(state.enabled);

    state.apply_rail_request(RailRequest::Open(SidebarPane::HardwareMonitor));
    assert_eq!(state.open, Some(SidebarPane::HardwareMonitor));

    state.apply_rail_request(RailRequest::Close(SidebarPane::HardwareMonitor));
    assert_eq!(state.open, None);

    state.apply_pane_request(PaneRequest::Resize(SidebarPane::HardwareMonitor, 360.0));
    assert_eq!(state.width, 360.0);

    state.open = Some(SidebarPane::MediaInspector);
    state.apply_pane_request(PaneRequest::Close(SidebarPane::MediaInspector));
    assert_eq!(state.open, None);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib sidebar_rail_state_request_updates`
Expected: FAIL with unresolved methods on `SidebarRailState`.

- [x] **Step 3: Implement `SidebarRailState` and `show_sidebar_rail`**

In `src/ui/sidebar_rail.rs`:
```rust
use craft_ui::panes::{Edge, PaneHost, PaneOutput, PaneRequest, Rail, RailItem, RailOutput, RailRequest, SidePolicy};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SidebarRailState {
    pub open: Option<SidebarPane>,
    pub width: f32,
    pub enabled: bool,
}

impl Default for SidebarRailState {
    fn default() -> Self {
        Self {
            open: None,
            width: 300.0,
            enabled: true,
        }
    }
}

impl SidebarRailState {
    pub fn apply_rail_request(&mut self, request: RailRequest<SidebarPane>) {
        match request {
            RailRequest::Open(pane) => self.open = Some(pane),
            RailRequest::Close(_) => self.open = None,
        }
    }

    pub fn apply_pane_request(&mut self, request: PaneRequest<SidebarPane>) {
        match request {
            PaneRequest::Close(_) => self.open = None,
            PaneRequest::Resize(_, size) => self.width = size.clamp(240.0, 600.0),
            _ => {}
        }
    }
}

pub fn show_sidebar_rail(
    app: &mut PealayerApp,
    ui: &mut Ui,
    state: &mut SidebarRailState,
) -> (Option<RailRequest<SidebarPane>>, Option<PaneOutput<SidebarPane>>) {
    if !state.enabled {
        return (None, None);
    }

    let items: Vec<RailItem<SidebarPane>> = SidebarPane::ALL
        .iter()
        .map(|pane| {
            RailItem::new(*pane, icon_source(*pane), pane.title(app.language))
        })
        .collect();

    let rail_out = Rail::new(egui::Id::unique("pealayer-sidebar-rail"))
        .edge(Edge::End)
        .rtl(app.rtl)
        .width(crate::ui::craft_theme::RAIL_WIDTH)
        .button_size(crate::ui::craft_theme::RAIL_BUTTON)
        .show(ui, &items, state.open.as_ref(), |_| {});

    if let Some(ref req) = rail_out.request {
        state.apply_rail_request(req.clone());
    }

    let pane_out = if let Some(pane) = state.open {
        let open_panes = [pane];
        let mut content = SidebarPaneContent { app };
        let out = PaneHost::new(egui::Id::unique("pealayer-sidebar-pane-host"))
            .rtl(content.app.rtl)
            .policy(SidePolicy::OneSlot)
            .show(
                ui,
                &open_panes,
                |_| Some(craft_ui::panes::PaneState::docked(Edge::End, state.width)),
                &mut content,
            );
        for req in &out.requests {
            state.apply_pane_request(req.clone());
        }
        Some(out)
    } else {
        None
    };

    (rail_out.request, pane_out)
}
```

- [x] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::sidebar_rail`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/ui/sidebar_rail.rs
git commit -m "feat(ui): add show_sidebar_rail component and request processing"
```

---

### Task 3: App Configuration & Central Workspace Integration

**Files:**
- Modify: `src/config.rs:250-320`
- Modify: `src/app.rs:690-710, 1980-2010`
- Modify: `src/ui/menu.rs:500-540`
- Test: tests in `src/app.rs` / `src/config.rs`

**Interfaces:**
- Consumes:
  - `src/ui/sidebar_rail::{SidebarRailState, show_sidebar_rail}`
- Produces:
  - `app.sidebar_rail_state: SidebarRailState`
  - Workspace layout rendering with sidebar rail

- [x] **Step 1: Write failing tests for sidebar persistence**

In `src/config.rs` tests:
```rust
#[test]
fn config_serializes_and_deserializes_sidebar_rail_state() {
    let mut config = AppConfig::default();
    assert!(config.ui.sidebar_rail.enabled);
    config.ui.sidebar_rail.width = 380.0;
    config.ui.sidebar_rail.open = Some(crate::ui::sidebar_rail::SidebarPane::HardwareMonitor);

    let json = serde_json::to_string(&config).expect("serialize");
    let restored: AppConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.ui.sidebar_rail.width, 380.0);
    assert_eq!(restored.ui.sidebar_rail.open, Some(crate::ui::sidebar_rail::SidebarPane::HardwareMonitor));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib config_serializes_and_deserializes_sidebar_rail_state`
Expected: FAIL with `sidebar_rail` field missing on `UiPreferences`.

- [x] **Step 3: Implement persistence and workspace integration**

1. In `src/config.rs`:
Add `sidebar_rail: crate::ui::sidebar_rail::SidebarRailState` to `UiPreferences` with `#[serde(default)]`.

2. In `src/app.rs`:
Add `pub(crate) sidebar_rail_state: crate::ui::sidebar_rail::SidebarRailState` to `PealayerApp`. Initialize from `config.ui.sidebar_rail`.

3. In `src/app.rs` in `show_four_d_editor` rendering:
Right before `egui::CentralPanel::default().frame(frame).show(ui, |ui| {`:
Invoke `crate::ui::sidebar_rail::show_sidebar_rail(self, ui, &mut self.sidebar_rail_state)`.

4. In `src/ui/menu.rs` in the View / Workspace menu:
Add a toggle `app.sidebar_rail_state.enabled` ("Sidebar Rail") and actions to quick-toggle any sidebar pane.

- [x] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib config_serializes_and_deserializes_sidebar_rail_state`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/config.rs src/app.rs src/ui/menu.rs
git commit -m "feat(ui): integrate sidebar rail state into app configuration and workspace"
```

---

### Task 4: Full Verification Gate & Regression Suite

**Files:**
- Full repository verification

- [ ] **Step 1: Run full unit test suite**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib`
Expected: All 947+ tests PASS with 0 failures.

- [ ] **Step 2: Run downloader crate tests**

Run: `cargo test -p pealayer-downloader`
Expected: PASS with 0 failures.

- [ ] **Step 3: Run compiler checks across all targets**

Run: `cargo check --lib --bins`
Expected: Clean exit 0 with 0 errors.

- [ ] **Step 4: Commit verification updates**

```bash
git commit --allow-empty -m "chore(verification): verify Phase 5 test suite passes cleanly"
```
