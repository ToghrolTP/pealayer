# Timeline Effect Controls & Handle Ergonomics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix timeline effect handle resize failure (where grabbing handles accidentally moves the effect), add visible handle grips, real-time floating HUD feedback, template isolation on resize, pattern-preserving duration scaling, and full undo/redo support.

**Architecture:** 
- Calculate drag mode from pointer press origin (`ui.ctx().input(|i| i.pointer.press_origin())`) rather than displaced frame-time hover position, expanding handle hitboxes to an ergonomic 10px.
- Render visual handle endcaps with hover/drag state feedback.
- Isolate shared effect templates upon resize to prevent unintended side effects on sibling instances.
- Integrate timeline snapshot history before mutations and render live floating coordinate HUD tooltips during drags.

**Tech Stack:** Rust, egui 0.31, eframe, uuid, serde

**Spec:** `docs/superpowers/specs/2026-09-19-timeline-effect-controls-and-handle-ergonomics-design.md`

## Global Constraints
- Zero compiler warnings (`RUSTFLAGS="-D warnings"` standard).
- All 104+ existing unit and integration tests must remain 100% passing.
- Backward compatibility: full compatibility with existing `.4d` projects and serialize formats.
- Non-destructive pattern handling: preserve pulse/strobe action choreography when resizing.

---

### Task 1: Domain & Logic: Press-Origin Drag Mode Classification, Template Isolation & Pattern Rescaling

**Files:**
- Modify: `src/app.rs`
- Modify: `src/four_d/models.rs`
- Test: `tests/timeline_clip_controls_test.rs`

**Interfaces:**
- Consumes: `DragMode`, `PealayerApp`, `Effect`, `EffectInstance`, `TimelineSnapshot`
- Produces: 
  - `pub fn classify_clip_drag_mode(clip_left: f32, clip_right: f32, press_x: f32) -> DragMode`
  - `pub fn isolate_template_for_instance(&mut self, instance_id: uuid::Uuid) -> Option<uuid::Uuid>`
  - `pub fn update_effect_duration(effect: &mut Effect, new_dur_ms: u64)`

- [ ] **Step 1: Write failing tests in `tests/timeline_clip_controls_test.rs`**

```rust
use pealayer::app::{classify_clip_drag_mode, DragMode, PealayerApp};
use pealayer::four_d::models::{AtomicAction, Effect, EffectInstance, HardwareTarget};
use uuid::Uuid;

#[test]
fn test_classify_clip_drag_mode_left_right_and_center() {
    let clip_left = 100.0;
    let clip_right = 200.0;
    
    // Left edge (within 10px)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 100.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 108.0), DragMode::ResizeLeft);
    
    // Right edge (within 10px)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 200.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 192.0), DragMode::ResizeRight);
    
    // Center (Move)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 150.0), DragMode::Move);
}

#[test]
fn test_classify_clip_drag_mode_short_clip() {
    let clip_left = 100.0;
    let clip_right = 110.0; // 10px wide clip
    // Dynamic handle clamping to 35% of width (3.5px)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 101.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 109.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 105.0), DragMode::Move);
}

#[test]
fn test_template_isolation_when_shared() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
        "Shared Effect".into(),
        "💧".into(),
        1000,
        HardwareTarget::Water,
        vec![AtomicAction { relay_id: 1, state: true, offset_ms: 0 }],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);
    
    let inst1 = EffectInstance::new(tmpl_id, 0);
    let inst2 = EffectInstance::new(tmpl_id, 2000);
    let inst1_id = inst1.id;
    let inst2_id = inst2.id;
    
    app.timeline.instances.push(inst1);
    app.timeline.instances.push(inst2);
    
    // Isolate inst1
    let new_tmpl_id = app.isolate_template_for_instance(inst1_id).expect("Should isolate");
    assert_ne!(new_tmpl_id, tmpl_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == inst1_id).unwrap().effect_id, new_tmpl_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == inst2_id).unwrap().effect_id, tmpl_id);
}

#[test]
fn test_pattern_rescaling_preserves_choreography() {
    let mut effect = Effect::with_target(
        "Strobe".into(),
        "⚡".into(),
        1000,
        HardwareTarget::Auxiliary,
        vec![
            AtomicAction { relay_id: 5, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 5, state: false, offset_ms: 250 },
            AtomicAction { relay_id: 5, state: true, offset_ms: 500 },
            AtomicAction { relay_id: 5, state: false, offset_ms: 1000 },
        ],
    );
    
    pealayer::app::update_effect_duration(&mut effect, 2000);
    assert_eq!(effect.duration_ms, 2000);
    assert_eq!(effect.actions.len(), 4);
    assert_eq!(effect.actions[0].offset_ms, 0);
    assert_eq!(effect.actions[1].offset_ms, 500); // 250 * 2
    assert_eq!(effect.actions[2].offset_ms, 1000); // 500 * 2
    assert_eq!(effect.actions[3].offset_ms, 2000); // 1000 * 2
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test timeline_clip_controls_test`
Expected: FAIL due to missing functions

- [ ] **Step 3: Implement functions in `src/app.rs`**
Add:
- `classify_clip_drag_mode`
- `isolate_template_for_instance`
- `update_effect_duration`

- [ ] **Step 4: Run tests to verify they pass**
Run: `RUSTFLAGS="-D warnings" cargo test --test timeline_clip_controls_test`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/app.rs tests/timeline_clip_controls_test.rs
git commit -m "feat(timeline): add press-origin drag classifier, template isolation, and pattern-aware duration scaler"
```

---

### Task 2: UI Interaction: Fix Drag Started Misclassification & Add Visual Handle Grips

**Files:**
- Modify: `src/ui/layout.rs:880-1080`

**Interfaces:**
- Consumes: `classify_clip_drag_mode`, `isolate_template_for_instance`, `update_effect_duration`
- Produces: Visual grip bars on clip edges, press-origin drag resolution, proper cursor icons on hover/drag.

- [ ] **Step 1: Write integration tests for clip handle drag detection**
Add integration test in `tests/timeline_clip_controls_test.rs` ensuring drag initiation correctly respects press positions.

- [ ] **Step 2: Implement visual handles in `src/ui/layout.rs`**
Render vertical handle grip brackets on both sides of each clip:
```rust
let handle_w = (clip_rect.width() * 0.35).min(10.0);
let left_handle_rect = egui::Rect::from_min_max(
    clip_rect.left_top(),
    egui::pos2(clip_rect.left() + handle_w, clip_rect.bottom()),
);
let right_handle_rect = egui::Rect::from_min_max(
    egui::pos2(clip_rect.right() - handle_w, clip_rect.top()),
    clip_rect.right_bottom(),
);
```
Draw grip notches/bars with subtle alpha when inactive, brightening on hover.

- [ ] **Step 3: Fix `started_drag` to query press origin**
When `clip_response.drag_started()` trips:
```rust
let press_pos = ui.ctx().input(|i| i.pointer.press_origin())
    .or_else(|| clip_response.interact_pointer_pos())
    .unwrap_or(clip_rect.center());
let mode = classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), press_pos.x);
```
This guarantees that outward mouse movement past the drag threshold never causes the mode to fall back to `Move`!

- [ ] **Step 4: Run tests and compiler checks**
Run: `RUSTFLAGS="-D warnings" cargo check --all-targets && cargo test`
Expected: PASS with 0 warnings.

- [ ] **Step 5: Commit**
```bash
git add src/ui/layout.rs tests/timeline_clip_controls_test.rs
git commit -m "feat(ui): resolve clip drag mode from press origin and render visual handle grips"
```

---

### Task 3: Live Drag HUD Readout, Undo Integration & Inspector Controls Upgrades

**Files:**
- Modify: `src/ui/layout.rs:1080-1250` (timeline drag loop)
- Modify: `src/ui/layout.rs:115-260` (Effect Controls inspector)

**Interfaces:**
- Consumes: `UndoStack`, `TimelineSnapshot`, `update_effect_duration`
- Produces: Floating HUD tooltip during drag, automatic undo snapshots on clip moves/resizes, expanded inspector duration slider up to 60s with undo tracking.

- [ ] **Step 1: Write integration tests for undo on clip resize/move**
Add tests in `tests/timeline_clip_controls_test.rs` verifying that resizing and moving clips correctly pushes to the undo stack and can be reverted with `undo()`.

- [ ] **Step 2: Add Undo snapshot on drag start and drag end**
In `src/ui/layout.rs`:
- When `started_drag` is processed, push `self.app.undo_stack.push(self.app.snapshot_timeline())`.
- If an instance is resized, call `self.app.isolate_template_for_instance(drag_id)` so sibling clips are isolated.
- In `ResizeRight` and `ResizeLeft` loops, call `update_effect_duration` to preserve custom action sequences.

- [ ] **Step 3: Implement floating HUD coordinate badge**
During active drag:
- If `mode == ResizeRight || mode == ResizeLeft`:
  Render floating badge above mouse:
  `[ ⏱ Dur: 1.50s (+250ms) | End: 00:03.750 ]`
- If `mode == Move`:
  Render floating badge above mouse:
  `[ ⏱ Start: 00:02.100 | Track: R2 Wind Fan ]`

- [ ] **Step 4: Upgrade Effect Controls Inspector**
- Increase duration slider range from `100.0..=10000.0` to `50.0..=60000.0` (up to 60s).
- On slider change or text edit, isolate template if shared and push undo snapshot.

- [ ] **Step 5: Run tests and compiler checks**
Run: `RUSTFLAGS="-D warnings" cargo check --all-targets && cargo test`
Expected: PASS with 0 warnings.

- [ ] **Step 6: Commit**
```bash
git add src/ui/layout.rs tests/timeline_clip_controls_test.rs
git commit -m "feat(ui): add live drag HUD badge, undo recording for clip edits, and 60s inspector slider"
```

---

### Task 4: End-to-End Verification & Edge Cases

**Files:**
- Test: `tests/timeline_clip_controls_test.rs`

**Interfaces:**
- Comprehensive verification across multi-clip selection, zooming interaction, snapping boundaries, and rapid resizing.

- [ ] **Step 1: Write comprehensive test scenarios**
- Rapid outward drag beyond clip bounds registers as resize, not move.
- Single-instance vs multi-instance template isolation verification.
- Duration change with snapping to playhead and cue boundaries.
- Undo/redo cycle restores previous duration and template states.

- [ ] **Step 2: Run full test suite with all targets**
Run: `RUSTFLAGS="-D warnings" cargo test`
Expected: All unit, integration, and doc tests pass with 0 warnings.

- [ ] **Step 3: Commit**
```bash
git add tests/timeline_clip_controls_test.rs
git commit -m "test(timeline): add comprehensive e2e test suite for effect handle controls and undo"
```
