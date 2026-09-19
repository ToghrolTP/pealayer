# Timeline Effect Controls & Handle Ergonomics Design Specification

## Problem Statement

When users interact with effect cue clips on the Pealayer 4D timeline, attempting to resize a clip by dragging its start or end handles frequently fails and instead moves the entire effect. This breaks the expected mental model of timeline editing (Nielsen Heuristics #5: Error Prevention, #6: Recognition Rather Than Recall).

### Root Causes
1. **Misclassification on Drag Start Due to Pointer Displacement**:
   - `egui::Response::drag_started()` fires only after the pointer moves past the drag distance threshold (approx. 4–6 px).
   - In `src/ui/layout.rs`, `hover_mode` was re-evaluated every frame based strictly on `clip_response.hovered()`.
   - When extending a clip outward (rightward for `ResizeRight` or leftward for `ResizeLeft`), the pointer moves outside `clip_rect` before `drag_started()` trips.
   - Once outside, `clip_response.hovered()` returns `false`, causing the code to skip updating `hover_mode` and leave it at `DragMode::Move`.
   - The clip then begins a Move operation rather than resizing.

2. **Microscopic 6px Hitbox**:
   - The 6.0px inward edge hitbox is too narrow for standard mouse interaction, providing no tolerance for slight positional offsets.

3. **Ignoring Press Origin (`pointer.press_origin()`)**:
   - Interaction mode was based on current frame pointer position rather than the point where the user initially clicked down (`press_origin`).

4. **Additional Discovered UX Issues**:
   - **Invisible Handles**: Clips have no visual indicators (brackets, endcaps, or grip notches) indicating where handle zones start.
   - **Missing Real-Time HUD**: No real-time duration, delta, or timecode readout during clip dragging/resizing.
   - **No Undo/Redo for Clip Moves or Resizes**: `active_drag` completion does not push to `undo_stack`.
   - **Shared Template Mutation**: Resizing one instance directly mutates `template.duration_ms`, inadvertently resizing all other instances sharing that template ID.
   - **Choreography Pattern Destruction**: Resizing replaces all template actions with `generate_constant`, destroying custom pulse or strobe patterns.
   - **Inspector Slider Cap & Missing Undo**: Duration slider in Effect Controls is capped at 10s, lacks direct numeric entry, and changes do not record undo snapshots.

---

## Architectural & UX Specification

### 1. Robust Handle Drag Classification via Press Origin
- When `clip_response.drag_started()` fires, determine drag mode (`ResizeLeft`, `ResizeRight`, `Move`) from:
  ```rust
  let press_pos = ui.ctx().input(|i| i.pointer.press_origin())
      .or_else(|| clip_response.interact_pointer_pos());
  ```
- Define handle hitbox width:
  ```rust
  let handle_hitbox_width = (clip_rect.width() * 0.35).min(10.0);
  ```
- If `press_pos.x <= clip_rect.left() + handle_hitbox_width`: `DragMode::ResizeLeft`
- If `press_pos.x >= clip_rect.right() - handle_hitbox_width`: `DragMode::ResizeRight`
- Otherwise: `DragMode::Move`
- Also check hover during non-drag states using the same logic to set cursor icon (`ResizeHorizontal` for handles, `Grab` for body).

### 2. Visual Handle Affordances
- For every clip on the timeline:
  - Render subtle vertical handle end-caps on the left and right borders (e.g. 3px inset lines or pill grips with `Color32::from_rgba_unmultiplied(255, 255, 255, 60)`).
  - When the pointer hovers over the left or right handle hitbox, highlight that handle with a bright cyan accent (`Color32::from_rgb(0, 220, 255)`) and 2px stroke.
  - While actively dragging a resize handle, draw an active grab indicator with bright glow.

### 3. Floating HUD Readout Tooltip
- While `self.app.active_drag` is active:
  - For `ResizeRight` and `ResizeLeft`: render a floating monospace badge above the pointer:
    `[ ⏱ Duration: 1.50s (+250ms) | End: 00:03.750 ]`
  - For `Move`: render:
    `[ ⏱ Start: 00:02.100 | Track: R2 Wind Fan ]`
  - Styling matches keyframe HUD: dark translucent background (`Color32::from_black_alpha(220)`), 1px border, monospace font.

### 4. Complete Undo/Redo History
- Before initiating any clip drag/resize:
  ```rust
  self.app.undo_stack.push(self.app.snapshot_timeline());
  ```
- If a drag is canceled or no changes occurred upon release, avoid redundant pushes.
- Inspector modifications (duration slider, start time, name, icon, target relay) also push to `undo_stack` on commit/change.

### 5. Template Isolation on Resize
- When an instance is resized or relocated:
  - If `self.app.timeline.instances.iter().filter(|i| i.effect_id == instance.effect_id).count() > 1`:
    - Clone the template with a new `Uuid::new_v4()`.
    - Update `instance.effect_id = new_template.id`.
    - Append the cloned template to `self.app.timeline.templates`.
  - Mutate only the isolated template.

### 6. Pattern-Aware Duration Rescaling
- If `template.actions` contains custom pulses (more than 2 actions or alternating states):
  - Scale non-terminal action timestamps proportionally to `new_dur / old_dur`.
  - Update final OFF action offset to `new_dur`.
- If constant (2 actions: ON at 0, OFF at old_dur):
  - Set the second action's `offset_ms` to `new_dur`.

### 7. Effect Controls Inspector Enhancements
- Expand duration slider range up to `60000.0` ms (60 seconds) with log/linear curve or double-click text editing.
- Capture undo snapshot when slider drag begins/stops (`slider.drag_stopped()`).

---

## Verification Criteria
- Unit & integration tests in `tests/timeline_clip_controls_test.rs`.
- Zero compiler warnings (`RUSTFLAGS="-D warnings"`).
- All 104+ existing tests remain 100% passing.
