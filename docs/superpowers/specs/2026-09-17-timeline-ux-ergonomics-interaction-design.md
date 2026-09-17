# NLE Timeline UX, Ergonomics & Interaction Design Specification

## 1. Overview & Goals

This specification details the comprehensive user experience (UX), ergonomic interaction model, and visual overhaul of the Pealayer NLE timeline and 4D analog curve editor.

### Goals
- **Industry-Standard Keyframe Manipulation**: 18px interaction hitboxes, active drag lock (no mouse slip), hover visual affordance, floating real-time coordinate HUD, and context menu for interpolation switching.
- **Multi-Selection & Regional Marquee**: Box select for both timeline clips and analog curve keyframes via `Ctrl + Mouse Drag` (or empty-canvas drag); group translation of selected keyframes.
- **Dedicated Time Ruler & Playhead Scrubbing**: Top 26px dedicated time ruler band for smooth, continuous playhead dragging.
- **Native Egui Tooltips**: Comprehensive contextual hover tooltips with keyboard shortcuts across every timeline control, header, and node.
- **Undo / Redo History Engine**: Dedicated timeline undo/redo stack (`Ctrl + Z`, `Ctrl + Y` / `Ctrl + Shift + Z`) tracking keyframe edits, clip moves, deletions, and live recording commits.
- **Standard Keyboard Shortcuts**: `Delete`/`Backspace` for multi-item removal, `Ctrl + A` to select all, `Escape` to clear selection, `1`/`2`/`3` for interpolation modes, `Space` for play/pause.
- **Track Header Responsive Layout**: Expanded 250px column width with dedicated flex slots preventing the live recording fader from clipping or hiding behind the grid.
- **Tier-1 NLE Visual Enhancements**:
  - Zoom & Pan (`Ctrl + Mouse Wheel` time zoom, `Shift + Mouse Wheel` scroll).
  - Magnetic snapping to playhead and grid with visual cyan guides.
  - Curve gradient underlay for instant actuator intensity visualization.
  - Amplitude Y-axis ticks (0%, 50%, 100%).

---

## 2. Architecture & Data Structures

### A. Undo / Redo History Stack
Location: `src/four_d/history.rs` & `src/app.rs`

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineSnapshot {
    pub instances: Vec<EffectInstance>,
    pub analog_tracks: Vec<AnalogTrack>,
}

#[derive(Default, Debug)]
pub struct UndoStack {
    undo_stack: Vec<TimelineSnapshot>,
    redo_stack: Vec<TimelineSnapshot>,
    max_history: usize,
}

impl UndoStack {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
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
}
```

### B. Multi-Selection & Keyframe Drag Lock State
Location: `src/app.rs`

```rust
pub struct KeyframeDragState {
    pub track_id: Uuid,
    pub keyframe_index: usize,
    pub start_pointer_pos: egui::Pos2,
    pub original_time_ms: u64,
    pub original_value: f32,
    /// Pre-drag state of all currently selected keyframes for group translation
    pub group_originals: Vec<(Uuid, usize, u64, f32)>,
}

// In PealayerApp:
pub selected_keyframes: std::collections::HashSet<(Uuid, usize)>,
pub active_keyframe_drag: Option<KeyframeDragState>,
pub timeline_zoom: f32, // pixels per second, default 100.0 (range 20.0 ..= 500.0)
pub undo_stack: crate::four_d::history::UndoStack,
```

---

## 3. Detailed UI & Interaction Behavior

### A. Keyframe Hit-Testing & Active Drag Lock
1. **Padded Proximity**:
   - Each keyframe is drawn as a 6px diamond.
   - Mouse hover is detected if `pos.distance(center) <= 16.0`.
   - On hover: cursor changes to `egui::CursorIcon::Grab`, outline strokes in bright white/yellow, and tooltip shows:
     `Keyframe\nTime: {timecode}\nValue: {val * 100:.1f}%\nInterpolation: {mode}\n(Drag to move, Right-click for options)`.
2. **Drag Lock Acquisition**:
   - On primary mouse down within 16px of a keyframe:
     - If keyframe is not already in `selected_keyframes` and `Shift` is not held, clear selection and select this keyframe.
     - Store `active_keyframe_drag` with snapshot of all selected keyframe positions.
     - Save current timeline state to `undo_stack`.
     - Set cursor to `egui::CursorIcon::Grabbing`.
3. **Group Translation & Snapping**:
   - While dragging, calculate `delta_time = (current_pointer.x - start_pointer.x) / zoom * 1000.0` and `delta_val = (start_pointer.y - current_pointer.y) / 32.0`.
   - Apply magnetic snap: if keyframe is within 5px of playhead or 1s grid mark, snap `delta_time` to align exactly.
   - Translate all selected keyframes by `(delta_time, delta_val)`.
   - Draw floating HUD pill directly above cursor: `[ 00:01:23.450 | 75.0% ]`.
4. **Drag Release**:
   - On pointer release, clear `active_keyframe_drag`.
   - Sort keyframes by `time_ms`, deduplicate, and broadcast `EngineMessage::UpdateAnalogTracks`.

### B. Regional Marquee (Box Selection)
1. **Trigger Condition**:
   - `Ctrl + Left Drag` anywhere on timeline canvas, OR left dragging on empty background space (outside clips, keyframes, and top ruler).
2. **Visual Feedback**:
   - Renders filled rectangle with `Color32::from_rgba_unmultiplied(0, 200, 255, 30)` and crisp `1.0px` border in cyan `Color32::from_rgb(0, 220, 255)`.
3. **Selection Query**:
   - Checks intersection against:
     - All timeline clips (`EffectInstance`).
     - All keyframe centers across all analog tracks.
   - If `Shift` is held, unions with existing selection; otherwise replaces it.

### C. Dedicated Time Ruler & Scrubbing Bar
1. **Geometry**: Top `26px` of timeline canvas (`rect.min.y ..= rect.min.y + 26.0`).
2. **Visuals**:
   - Dark header background (`Color32::from_rgb(25, 25, 25)`).
   - Major second ticks with timestamps (e.g. `00:00:01`), sub-second frame notches.
   - Large playhead grab triangle (14px wide, bright red) extending into timeline.
3. **Scrub Interaction**:
   - Hovering top 26px changes cursor to `egui::CursorIcon::ResizeHorizontal`.
   - Primary click or drag anywhere in ruler immediately seeks `playback_time` to mouse position.
   - Emits punch-out if recording is active.
   - Seeks video via MPV continuously without dropping audio/video sync.

### D. Native Egui Tooltips
Every interactive element receives dedicated `.on_hover_text(...)` or `.on_hover_ui(...)`:
- Track Mute button `[M]`: `"Mute Track\nSilences actuator output during playback."`
- Track Record Arm button `[●]`: `"Record Arm (Motion Capture)\nArms track for live fader/keyboard gesture recording."`
- Track Live Fader: `"Live Actuator Fader\nControl actuator intensity in real time (0% - 100%)."`
- Track Add Keyframe `[+]`: `"Add Keyframe\nCreates keyframe at current playhead position."`
- Relay Track Row: `"Relay Output {id}: {name}\nDrag effects from library to sequence."`
- Time Ruler: `"Timeline Ruler\nClick or drag to scrub playhead. Ctrl+Drag below to box-select."`

### E. Track Header Layout & Live Fader Fix
1. **Width Expansion**: Header column width widened from `180px` to **`250px`**.
2. **Slot Allocation**:
   - `0px .. 90px`: Channel tag (`P{channel}`) + Track Name (elided with tooltip).
   - `90px .. 115px`: Mute `[M]` button (22px).
   - `115px .. 140px`: Record Arm `[●]` button (22px).
   - `140px .. 215px`: Live Fader Slider (70px, rendered only when `armed`).
   - `220px .. 245px`: Add Keyframe `[+]` button (22px).
3. **Isolation**: Track headers occupy a dedicated non-scrolling UI column with right border stroke, completely isolated from horizontal timeline scrolling.

### F. Visual Polish: Gradient, Snapping & Y-Ruler
1. **Curve Gradient Underlay**:
   - Under each analog curve, tessellate polygon from curve points down to `row_y + 36.0` (0% baseline).
   - Fill with vertical linear gradient: `Color32::from_rgba_unmultiplied(0, 220, 255, 35)` at top fading to `0` opacity at bottom.
2. **Normalized Amplitude Y-Ruler**:
   - On the right edge of each track header, render mini tick labels: `100%` at top, `50%` at middle, `0%` at bottom in faint gray text.
3. **Magnetic Snap Guideline**:
   - When snapping occurs, draw a vertical cyan line (`Color32::from_rgb(0, 255, 255)`) through the full height of the timeline canvas.
4. **Zoom & Pan**:
   - In timeline scroll area, intercept `ui.input(|i| i.raw_scroll_delta)`:
     - If `modifiers.ctrl`: update `timeline_zoom = (timeline_zoom + delta_y * 0.2).clamp(20.0, 500.0)`.
     - Standard mouse wheel scrolls vertically or horizontally (`Shift + Wheel`).

---

## 4. Keyboard Shortcut Map

| Key | Action | Scope |
|---|---|---|
| `Delete` / `Backspace` | Delete all selected clips and keyframes | Timeline |
| `Ctrl + Z` | Undo last edit (keyframe move, delete, clip drag, record) | Global |
| `Ctrl + Y` / `Ctrl + Shift + Z` | Redo | Global |
| `Ctrl + A` | Select all keyframes and clips in project | Timeline |
| `Escape` | Deselect all | Timeline |
| `1` | Set selected keyframe(s) to Step interpolation | Timeline |
| `2` | Set selected keyframe(s) to Linear interpolation | Timeline |
| `3` | Set selected keyframe(s) to Smooth (Hermite) interpolation | Timeline |
| `Space` | Toggle Play / Pause | Global |
| `W` / `Up` | Ramp live throttle up | When track armed |
| `S` / `Down` | Ramp live throttle down | When track armed |

---

## 5. Verification & Testing Strategy

1. **Unit Tests (`tests/timeline_history_test.rs`)**:
   - `UndoStack`: push, undo, redo, max limit cap, redo stack clearing on new push.
   - Multi-selection bounds and group translation calculation.
2. **Integration Verification**:
   - Verify `cargo test` passes across all existing test suites.
   - Run interactive testing verifying drag lock, marquee selection, tooltips, and live fader layout.
