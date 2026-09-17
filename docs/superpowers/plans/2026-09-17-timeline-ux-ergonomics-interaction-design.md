# Timeline UX, Ergonomics & Interaction Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the Pealayer NLE timeline and 4D curve editor into a professional-grade interaction canvas with padded hitboxes, drag-lock, floating HUD, multi-selection marquee, dedicated time ruler, undo/redo history, shortcuts, responsive track headers, and visual polish.

**Architecture:**
- Pure Rust `UndoStack` manages timeline history snapshots for instant `Ctrl+Z` / `Ctrl+Y` undo/redo.
- App state upgrades to multi-selection (`selected_keyframes: HashSet<(Uuid, usize)>`) and drag lock (`KeyframeDragState`).
- Timeline grid separates into a top 26px time scrubbing ruler and a tracks interaction canvas with `Ctrl + Drag` marquee selection.
- Track header expands to 250px with dedicated flex slots preventing the live recording fader from clipping or overflowing.
- Analog curve visualizer renders gradient underlays, 16px padded hitboxes, magnetic snap guides, and a floating time/value HUD badge.

**Tech Stack:** Rust 2024 edition, `egui`, `eframe`, `uuid`, `serde`.

**Spec:** `docs/superpowers/specs/2026-09-17-timeline-ux-ergonomics-interaction-design.md`

## Global Constraints
- Zero compiler warnings (`-D warnings` standard).
- All 83 existing unit and integration tests must remain 100% passing across all task gates.
- No heavy external crate additions; use std and existing `egui` / `uuid` / `serde`.
- Fully preserve backward compatibility for project serialization (`.4d` and JSON sidecars).

---

### Task 1: Undo/Redo Engine & Timeline Snapshot History

**Files:**
- Create: `src/four_d/history.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/timeline_history_test.rs`

**Interfaces:**
- Produces:
  - `pub struct TimelineSnapshot`: holds clones of `Vec<EffectInstance>` and `Vec<AnalogTrack>`.
  - `pub struct UndoStack`: `new(max_history: usize)`, `push(snapshot)`, `undo(current) -> Option<TimelineSnapshot>`, `redo(current) -> Option<TimelineSnapshot>`, `can_undo() -> bool`, `can_redo() -> bool`, `clear()`.

- [ ] **Step 1: Write the failing test**

Create `tests/timeline_history_test.rs`:
```rust
use pealayer::four_d::curve::AnalogTrack;
use pealayer::four_d::history::{TimelineSnapshot, UndoStack};
use pealayer::models::EffectInstance;
use uuid::Uuid;

#[test]
fn test_undo_redo_stack_basic_flow() {
    let mut stack = UndoStack::new(10);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());

    let state0 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![AnalogTrack::new("Wind", 0)],
    };
    let state1 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![AnalogTrack::new("Wind", 0), AnalogTrack::new("Water", 1)],
    };
    let state2 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![
            AnalogTrack::new("Wind", 0),
            AnalogTrack::new("Water", 1),
            AnalogTrack::new("Vibe", 2),
        ],
    };

    stack.push(state0.clone());
    stack.push(state1.clone());
    assert!(stack.can_undo());
    assert!(!stack.can_redo());

    // Undo from state2 back to state1
    let undone = stack.undo(state2.clone()).expect("Should undo to state1");
    assert_eq!(undone.analog_tracks.len(), 2);
    assert!(stack.can_redo());

    // Redo back to state2
    let redone = stack.redo(undone).expect("Should redo to state2");
    assert_eq!(redone.analog_tracks.len(), 3);
}

#[test]
fn test_undo_stack_max_depth_cap() {
    let mut stack = UndoStack::new(2);
    for i in 0..5 {
        stack.push(TimelineSnapshot {
            instances: vec![],
            analog_tracks: vec![AnalogTrack::new(&format!("Track{}", i), i as u8)],
        });
    }

    let cur = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![],
    };
    // Should be capped to 2 undos
    let u1 = stack.undo(cur).unwrap();
    assert_eq!(u1.analog_tracks[0].name, "Track4");
    let u2 = stack.undo(u1).unwrap();
    assert_eq!(u2.analog_tracks[0].name, "Track3");
    assert!(!stack.can_undo());
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test timeline_history_test`
Expected: FAIL with missing module `history`.

- [ ] **Step 3: Implement minimal code**

Create `src/four_d/history.rs`:
```rust
use crate::four_d::curve::AnalogTrack;
use crate::models::EffectInstance;

/// Represents a point-in-time state of user-editable timeline elements.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineSnapshot {
    pub instances: Vec<EffectInstance>,
    pub analog_tracks: Vec<AnalogTrack>,
}

/// Bounded undo/redo history stack.
#[derive(Debug, Clone)]
pub struct UndoStack {
    undo_stack: Vec<TimelineSnapshot>,
    redo_stack: Vec<TimelineSnapshot>,
    max_history: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(50)
    }
}

impl UndoStack {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new>,
            max_history: max_history.max(1),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn push(&mut self, snapshot: TimelineSnapshot) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: TimelineSnapshot) -> Option<TimelineSnapshot> {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: TimelineSnapshot) -> Option<TimelineSnapshot> {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current);
            Some(next)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
```

Export in `src/four_d/mod.rs`:
```rust
pub mod curve;
pub mod curve_record;
pub mod engine;
pub mod history;
pub mod input_capture;
pub mod models;
pub mod patterns;
pub mod protocol;
```

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test timeline_history_test`
Expected: PASS (2 passed).

- [ ] **Step 5: Commit**

```bash
git add src/four_d/history.rs src/four_d/mod.rs tests/timeline_history_test.rs
git commit -m "feat(timeline): implement UndoStack and TimelineSnapshot for history management"
```

---

### Task 2: Multi-Selection Model & App State Architecture

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Test: `tests/timeline_selection_test.rs`

**Interfaces:**
- Consumes: `src/four_d/history.rs` (`UndoStack`, `TimelineSnapshot`)
- Produces:
  - `selected_keyframes: std::collections::HashSet<(Uuid, usize)>`
  - `active_keyframe_drag: Option<KeyframeDragState>`
  - `timeline_zoom: f32`
  - `undo_stack: UndoStack`
  - `PealayerApp::snapshot_timeline(&self) -> TimelineSnapshot`
  - `PealayerApp::restore_timeline_snapshot(&mut self, snapshot: TimelineSnapshot)`

- [ ] **Step 1: Write the failing test**

Create `tests/timeline_selection_test.rs`:
```rust
use pealayer::app::PealayerApp;
use pealayer::four_d::curve::AnalogTrack;
use uuid::Uuid;

#[test]
fn test_app_state_multi_keyframe_selection() {
    let mut app = PealayerApp::default();
    let track_id = Uuid::new_v4();

    assert!(app.selected_keyframes.is_empty());
    assert_eq!(app.timeline_zoom, 100.0);

    // Insert multiple selections
    app.selected_keyframes.insert((track_id, 0));
    app.selected_keyframes.insert((track_id, 1));
    assert_eq!(app.selected_keyframes.len(), 2);

    // Snapshot and restore
    let snapshot = app.snapshot_timeline();
    assert_eq!(snapshot.analog_tracks.len(), app.timeline.analog_tracks.len());
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test timeline_selection_test`
Expected: FAIL due to missing `selected_keyframes`, `timeline_zoom`, `snapshot_timeline`.

- [ ] **Step 3: Implement minimal code**

In `src/app.rs`:
Define `KeyframeDragState`:
```rust
#[derive(Debug, Clone)]
pub struct KeyframeDragState {
    pub track_id: uuid::Uuid,
    pub keyframe_index: usize,
    pub start_pointer_pos: egui::Pos2,
    pub original_time_ms: u64,
    pub original_value: f32,
    pub group_originals: Vec<(uuid::Uuid, usize, u64, f32)>,
}
```

In `PealayerApp`:
Replace `selected_keyframe: Option<(uuid::Uuid, usize)>` with:
```rust
    pub(crate) selected_keyframes: std::collections::HashSet<(uuid::Uuid, usize)>,
    pub(crate) active_keyframe_drag: Option<KeyframeDragState>,
    pub(crate) timeline_zoom: f32,
    pub(crate) undo_stack: crate::four_d::history::UndoStack,
```

Add methods on `PealayerApp`:
```rust
    pub fn snapshot_timeline(&self) -> crate::four_d::history::TimelineSnapshot {
        crate::four_d::history::TimelineSnapshot {
            instances: self.timeline.instances.clone(),
            analog_tracks: self.timeline.analog_tracks.clone(),
        }
    }

    pub fn restore_timeline_snapshot(&mut self, snapshot: crate::four_d::history::TimelineSnapshot) {
        self.timeline.instances = snapshot.instances;
        self.timeline.analog_tracks = snapshot.analog_tracks;
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                self.timeline.analog_tracks.clone(),
            ),
        );
    }
```

In `src/main.rs`: initialize fields in `PealayerApp::default()`:
```rust
    selected_keyframes: std::collections::HashSet::new(),
    active_keyframe_drag: None,
    timeline_zoom: 100.0,
    undo_stack: crate::four_d::history::UndoStack::default(),
```

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test timeline_selection_test`
Expected: PASS (1 passed).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs tests/timeline_selection_test.rs
git commit -m "feat(app): add multi-selection, drag lock, zoom, and undo stack to PealayerApp"
```

---

### Task 3: Track Header Layout Overhaul & Live Fader Isolation

**Files:**
- Modify: `src/ui/layout.rs:560-640`

**Interfaces:**
- Expands track header width from 180px to 250px.
- Organizes controls into fixed flex slots:
  - Track label: `0 .. 85px`
  - Mute `[M]`: `22px`
  - Record Arm `[●]`: `22px`
  - Live Fader (when armed): `70px` width
  - Add Keyframe `[+]`: `22px`
- Renders amplitude Y-axis tick labels (`100%`, `50%`, `0%`) on track header right margin.
- Adds comprehensive native egui tooltips on all track header controls.

- [ ] **Step 1: Check existing layout code**

Inspect lines 560–635 in `src/ui/layout.rs`.

- [ ] **Step 2: Update track header column and layout sizing**

In `src/ui/layout.rs`:
Change left column allocation from `180.0` to `250.0`:
```rust
let (rect, _response) = ui.allocate_exact_size(egui::vec2(250.0, 40.0), egui::Sense::hover());
```
Render Y-ruler ticks on the right edge of the header:
```rust
painter.text(
    egui::pos2(rect.max.x - 3.0, rect.min.y + 5.0),
    egui::Align2::RIGHT_TOP,
    "100%",
    egui::FontId::monospace(7.5),
    egui::Color32::from_rgb(90, 90, 90),
);
painter.text(
    egui::pos2(rect.max.x - 3.0, rect.center().y),
    egui::Align2::RIGHT_CENTER,
    "50%",
    egui::FontId::monospace(7.5),
    egui::Color32::from_rgb(70, 70, 70),
);
painter.text(
    egui::pos2(rect.max.x - 3.0, rect.max.y - 5.0),
    egui::Align2::RIGHT_BOTTOM,
    "0%",
    egui::FontId::monospace(7.5),
    egui::Color32::from_rgb(90, 90, 90),
);
```

Add rich tooltips:
```rust
let m_btn = ui.selectable_label(track.muted, egui::RichText::new("M").strong().size(10.0))
    .on_hover_text("Mute Track (M)\nMutes actuator physical output during playback.");

let arm_btn = ui.selectable_label(track.armed, egui::RichText::new("●").size(12.0).color(arm_color))
    .on_hover_text("Record Arm (R)\nArms this track for real-time motion capture gesture recording.");

if track.armed {
    let mut val = self.app.input_capture.current_throttle;
    let slider = egui::Slider::new(&mut val, 0.0..=1.0)
        .show_value(false)
        .text("");
    if ui.add_sized([65.0, 16.0], slider)
        .on_hover_text("Live Actuator Fader\nControl actuator intensity in real time (0% - 100%).")
        .changed() { ... }
}

let add_btn = ui.button(egui::RichText::new("+").size(10.0))
    .on_hover_text("Add Keyframe\nInserts a keyframe at the current playhead position.");
```

- [ ] **Step 3: Run compiler checks and test suite**

Run: `cargo check --all-targets`
Run: `cargo test`
Expected: PASS with 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add src/ui/layout.rs
git commit -m "feat(ui): expand track header to 250px with responsive fader slots and Y-axis scale"
```

---

### Task 4: Dedicated Time Ruler, Playhead Scrubbing & Zoom/Pan Interaction

**Files:**
- Modify: `src/ui/layout.rs:635-720`, `1430-1490`

**Interfaces:**
- Allocates top 26px band of timeline grid as a dedicated time ruler.
- Hovering ruler sets cursor to `egui::CursorIcon::ResizeHorizontal`.
- Primary click or drag on ruler scrubs `playback_time` continuously and updates video seek.
- Connects `self.app.timeline_zoom` (pixels per second) to time-to-pixel conversions throughout the grid.
- Intercepts `Ctrl + Mouse Wheel` to scale `timeline_zoom` in range `20.0 ..= 500.0`.

- [ ] **Step 1: Implement Time Ruler Geometry and Interaction**

In `src/ui/layout.rs`:
Replace hardcoded `100.0` with `let zoom = self.app.timeline_zoom;` across grid coordinate calculations.
Ruler rect:
```rust
let ruler_rect = egui::Rect::from_min_max(
    egui::pos2(rect.min.x, rect.min.y),
    egui::pos2(rect.max.x, rect.min.y + 26.0),
);
painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_rgb(25, 25, 25));
painter.line_segment(
    [egui::pos2(ruler_rect.min.x, ruler_rect.max.y), egui::pos2(ruler_rect.max.x, ruler_rect.max.y)],
    egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)),
);
```

Scrubbing logic:
```rust
if let Some(pos) = pointer_pos {
    if ruler_rect.contains(pos) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        if ui.input(|i| i.pointer.primary_down()) {
            let relative_x = (pos.x - rect.min.x).max(0.0);
            let target_time = ((relative_x / zoom) as f64).clamp(0.0, total_seconds);
            self.app.playback_time = target_time;
            self.app.commit_recorded_samples();
            let _ = self.app.mpv.command("seek", &[&target_time.to_string(), "absolute"]);
        }
    }
}
```

Zoom with `Ctrl + Wheel`:
```rust
let scroll_delta = ui.input(|i| i.raw_scroll_delta);
if ui.input(|i| i.modifiers.ctrl) && scroll_delta.y != 0.0 {
    self.app.timeline_zoom = (self.app.timeline_zoom + scroll_delta.y * 0.2).clamp(20.0, 500.0);
}
```

- [ ] **Step 2: Run compiler checks and test suite**

Run: `cargo check --all-targets`
Run: `cargo test`
Expected: PASS with 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/ui/layout.rs
git commit -m "feat(ui): add dedicated time ruler scrubbing bar and mouse wheel timeline zoom"
```

---

### Task 5: Keyframe Ergonomics: Padded Hitboxes, Drag Lock, Snapping, HUD & Context Menu

**Files:**
- Modify: `src/ui/layout.rs:1150-1280`

**Interfaces:**
- Keyframe hit detection: 16px proximity threshold.
- Cursor changes: `Grab` on hover, `Grabbing` during active drag.
- Drag Lock: sets `active_keyframe_drag` so fast pointer movement never slips off.
- Floating HUD: draws `[ Time | Value% ]` badge above cursor.
- Magnetic snapping: snaps keyframe to playhead / 1s grid mark within 5px.
- Curve gradient underlay: fills subtle transparent cyan gradient beneath curve.
- Right-click context menu: `Step`, `Linear`, `Smooth`, `Delete Keyframe`.

- [ ] **Step 1: Implement Padded Hitboxes & Cursor Icon**

In `src/ui/layout.rs`:
```rust
let hover_dist = 16.0;
let is_hovered = pointer_pos.map_or(false, |pos| pos.distance(center) <= hover_dist);
if is_hovered {
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
}
```

- [ ] **Step 2: Implement Active Drag Lock & Floating HUD**

When primary clicked on keyframe:
```rust
if is_hovered && ui.input(|i| i.pointer.primary_clicked()) {
    let mut group_originals = Vec::new();
    for &(tid, kid) in &self.app.selected_keyframes {
        if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
            if let Some(k) = t.keyframes.get(kid) {
                group_originals.push((tid, kid, k.time_ms, k.value));
            }
        }
    }
    self.app.active_keyframe_drag = Some(crate::app::KeyframeDragState {
        track_id: track.id,
        keyframe_index: k_idx,
        start_pointer_pos: pos,
        original_time_ms: kf.time_ms,
        original_value: kf.value,
        group_originals,
    });
}
```

When dragging:
Draw floating HUD above cursor:
```rust
let hud_pos = egui::pos2(pos.x, pos.y - 20.0);
let hud_text = format!("{} | {:.1}%", format_timecode(new_t as f64 / 1000.0), new_v * 100.0);
painter.rect_filled(
    egui::Rect::from_center_size(hud_pos, egui::vec2(100.0, 18.0)),
    4.0,
    egui::Color32::from_black_alpha(200),
);
painter.text(hud_pos, egui::Align2::CENTER_CENTER, hud_text, egui::FontId::monospace(10.0), egui::Color32::WHITE);
```

- [ ] **Step 3: Implement Curve Gradient Underlay**

Fill polygon under curve points down to `row_y + 36.0` with `Color32::from_rgba_unmultiplied(0, 220, 255, 25)`.

- [ ] **Step 4: Implement Right-Click Context Menu**

```rust
if is_hovered && ui.input(|i| i.pointer.secondary_clicked()) {
    ui.menu_button("Interpolation", |ui| {
        if ui.button("Linear").clicked() {
            track.keyframes[k_idx].interpolation = crate::four_d::curve::Interpolation::Linear;
            curve_updated = true;
        }
        if ui.button("Smooth (Hermite)").clicked() {
            track.keyframes[k_idx].interpolation = crate::four_d::curve::Interpolation::Smooth;
            curve_updated = true;
        }
        if ui.button("Step").clicked() {
            track.keyframes[k_idx].interpolation = crate::four_d::curve::Interpolation::Step;
            curve_updated = true;
        }
    });
}
```

- [ ] **Step 5: Run compiler checks and test suite**

Run: `cargo check --all-targets`
Run: `cargo test`
Expected: PASS with 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add src/ui/layout.rs
git commit -m "feat(ui): implement keyframe drag lock, 16px hitboxes, floating HUD, and curve gradients"
```

---

### Task 6: Marquee Regional Select & Timeline Shortcuts Dispatcher

**Files:**
- Modify: `src/ui/layout.rs:1390-1450`
- Test: `tests/timeline_ux_integration_test.rs`

**Interfaces:**
- `Ctrl + Drag` or empty canvas drag creates `lasso_rect` selecting both clips (`EffectInstance`) and keyframes (`(Uuid, usize)`).
- Global keyboard shortcuts in timeline:
  - `Delete` / `Backspace`: deletes selected clips & keyframes with undo recording.
  - `Ctrl + Z`: Undo.
  - `Ctrl + Y` / `Ctrl + Shift + Z`: Redo.
  - `Ctrl + A`: Select all clips and keyframes.
  - `Escape`: Deselect all.
  - `1`, `2`, `3`: Quick change selected keyframes interpolation.

- [ ] **Step 1: Write integration test**

Create `tests/timeline_ux_integration_test.rs`:
```rust
use pealayer::app::PealayerApp;
use pealayer::four_d::curve::{AnalogTrack, Interpolation, Keyframe};

#[test]
fn test_undo_redo_timeline_keyframe_deletion() {
    let mut app = PealayerApp::default();
    let mut track = AnalogTrack::new("Wind Fan", 0);
    track.add_keyframe(Keyframe::new(1000, 0.5, Interpolation::Linear));
    let track_id = track.id;
    app.timeline.analog_tracks.push(track);

    // Save initial state
    let snap0 = app.snapshot_timeline();
    app.undo_stack.push(snap0);

    // Modify (delete keyframe)
    app.timeline.analog_tracks[0].keyframes.clear();
    assert_eq!(app.timeline.analog_tracks[0].keyframes.len(), 0);

    // Undo
    let snap1 = app.snapshot_timeline();
    if let Some(prev) = app.undo_stack.undo(snap1) {
        app.restore_timeline_snapshot(prev);
    }
    assert_eq!(app.timeline.analog_tracks[0].keyframes.len(), 1);
    assert_eq!(app.timeline.analog_tracks[0].keyframes[0].time_ms, 1000);

    // Redo
    let snap2 = app.snapshot_timeline();
    if let Some(next) = app.undo_stack.redo(snap2) {
        app.restore_timeline_snapshot(next);
    }
    assert_eq!(app.timeline.analog_tracks[0].keyframes.len(), 0);
}
```

- [ ] **Step 2: Implement Marquee Box Selection for Keyframes**

In `src/ui/layout.rs`:
Extend `lasso_rect` query to check keyframe centers:
```rust
for track in &self.app.timeline.analog_tracks {
    let t_y = rect.min.y + 320.0 + (t_idx as f32 * 40.0);
    for (k_idx, kf) in track.keyframes.iter().enumerate() {
        let k_pos = egui::pos2(rect.min.x + (kf.time_ms as f32 / 1000.0) * zoom, (t_y + 36.0) - (kf.value * 32.0));
        if lasso_rect.contains(k_pos) {
            self.app.selected_keyframes.insert((track.id, k_idx));
        }
    }
}
```

- [ ] **Step 3: Implement Keyboard Shortcut Dispatcher**

In `src/ui/layout.rs`:
```rust
let delete_pressed = ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
let undo_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift);
let redo_pressed = ui.input(|i| (i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)));
let select_all_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A));
let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

if undo_pressed {
    let current = self.app.snapshot_timeline();
    if let Some(prev) = self.app.undo_stack.undo(current) {
        self.app.restore_timeline_snapshot(prev);
    }
} else if redo_pressed {
    let current = self.app.snapshot_timeline();
    if let Some(next) = self.app.undo_stack.redo(current) {
        self.app.restore_timeline_snapshot(next);
    }
} else if delete_pressed {
    self.app.undo_stack.push(self.app.snapshot_timeline());
    // Delete selected clips and selected keyframes
    ...
}
```

- [ ] **Step 4: Run full workspace test suite**

Run: `cargo test`
Expected: ALL 85+ unit & integration tests pass with 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/timeline_ux_integration_test.rs
git commit -m "feat(ui): implement marquee box selection and keyboard shortcuts dispatcher"
```

---

## Execution Handoff
Plan complete and saved to `docs/superpowers/plans/2026-09-17-timeline-ux-ergonomics-interaction-design.md`.
Two execution options:
1. **Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
