# 4D Orchestrator - egui Implementation Guide

Moving from a simple standalone video player to a full Non-Linear Editor (NLE) layout (like Premiere Pro) is a significant architectural shift. This guide breaks down the new wireframe into manageable phases for your Rust `egui` developer so that the application can be upgraded iteratively without breaking the existing playback functionality.

## Architectural Overview: The NLE Layout

The new design relies on a multi-panel workspace. In `egui`, achieving this Premiere Pro style layout involves a docking system (which you have successfully implemented via `egui_dock`). This allows the operator to resize, tear off, and rearrange panels (just like Premiere Pro workspaces).

---

## Phase 1: The Foundation & Workspace Grid (Completed)

**Goal:** Establish the empty layout containers before adding complex logic.
*Status: Successfully implemented by the developer. The application now uses `egui_dock` with a dedicated layout mapping the Program Monitor, Effect Controls, Timeline, etc.*

---

## Phase 2: Migrating the Existing Playback (Completed)

**Goal:** Move your existing `mpv` video player code into the new layout, specifically rendering the video texture and transport controls inside the "Program Monitor" tab. 
*Status: Successfully implemented. MPV video texture successfully renders via offscreen GPU allocation. Aspect ratio letterboxing and transport controls are active.*

---

## Phase 3: The Timeline Foundation (Completed)

**Goal:** Create the visual representation of time and hardware tracks.
*Status: Successfully implemented. 10-track layout built, active database integration achieved for real-time cue rendering, and a click-to-seek playhead mapped to the video timecap.*

---

## Phase 4: Hardware Monitoring & Indicators (Completed)

**Goal:** Build a high-density, real-time dashboard reflecting the state of physical relays and DMX devices as the playhead scrubs over active timeline cues.
*Status: Successfully implemented. Real-time diagnostic dashboard displays live relay states with custom emissive LED bulbs and manual Force ON override switches.*

---

## Phase 5: Effect Controls & Interactive Cues (Completed)

**Goal:** Allow the operator to select individual cues on the timeline, inspect their metadata, and edit their parameters via the "Effect Controls" tab.
*Status: Successfully implemented. Selected timeline clips have their timing constraints, hardware targets, names, and icon parameters editable in real-time with automatic rebuilds on the engine thread.*

---

## Phase 6: Effects & Project Library (Asset Browser)

**Goal:** Create a browsable library of pre-configured hardware cues (presets) that the operator can drag and drop into the timeline, focusing on a highly tactile and responsive user experience.

### 1. Library Layout & Data Structure
The "Effects Library" tab serves as the primary asset browser. The UX goal here is immediate discoverability and clear visual hierarchy.
- **Data Model:** Define a library of templates in your Rust backend (e.g., `Vec<EffectTemplate>`). These templates should contain default durations, target relays, names (e.g., "Water Splash", "Mist Spray", "Wind Blast"), and specific icons.
- **Visual Hierarchy & Folders:** Use `egui::CollapsingHeader` to organize presets logically (e.g., "Atmospherics", "Lighting", "Hardware Relays"). 
  - *UX Detail:* Ensure folder icons toggle between open/closed states. Add subtle indentation so the nested items are clearly subordinate.
- **Instant Search:** Add a sticky `egui::TextEdit::singleline` at the top of the panel with a clear "Search effects..." placeholder.
  - *UX Detail:* The search must feel instantaneous. If the user types "wind", instantly filter out non-matching items and auto-expand folders containing matches. 
  - *UX Detail (Empty State):* If a search yields zero results, do not just leave an empty void. Render a centered, muted text label (e.g., "No effects found") to provide clear feedback.

### 2. Implementing Drag-and-Drop (D&D)
This is where the application becomes a true NLE. The user must feel exactly what they are doing during a drag operation.
- **Hover Affordances:** When the user hovers over an effect in the library, change the background color slightly to indicate interactivity, and change the mouse cursor to a `Grab` icon (`egui::CursorIcon::Grab`).
- **Drag Source (Library):** Wrap the template items using `ui.dnd_drag_source()`.
  - *UX Detail (Ghosting/Tooltip):* When the user clicks and drags, change the cursor to `Grabbing`. Attach a visual "ghost" to the cursor—a small tooltip or semi-transparent label that follows the mouse (e.g., showing the name and icon of the effect). This reassures the user that they are actively carrying the payload.
- **Drop Target (Timeline):** Update the timeline widget (from Phase 3) to act as a drop zone using `ui.dnd_drop_zone()`.

### 3. Drop Validation & Feedback
Dropping an effect must provide immediate visual confirmation and prevent user errors gracefully.
- **Target Highlighting:** As the user drags the ghosted effect over the timeline, highlight the specific track (row) they are currently hovering over. This tells the operator exactly which relay the effect will bind to.
- **Validation (Error Prevention):** If the user drags an effect over an incompatible track (e.g., dragging a relay effect onto the Audio track), change the cursor to `NotAllowed` and highlight the track in a faint red. Prevent the drop event in your code if released here.
- **Position Mapping & Snap to Grid:** On a successful drop, calculate the X (start time) and Y (track) coordinates. 
  - *UX Detail (Snapping):* If you implement a playhead or grid snapping feature, snap the calculated start time to the nearest logical beat or the playhead's current position to aid precision.
- **Instantiation & Auto-Selection:** Clone the template data into the `app.timeline` database.
  - *UX Detail (Workflow Speed):* Once the new cue rectangle is instantiated on the timeline, **automatically select it**. By instantly making it the "active selection," the Effect Controls tab (built in Phase 5) immediately populates with the new cue's parameters, saving the operator an unnecessary secondary click.

---

## Phase 7: Advanced Timeline Editing (Direct Manipulation)

**Goal:** Transform the timeline from a read-only visualizer into a tactile, interactive canvas. The operator must be able to move, trim, slice, and snap cues directly on the grid, matching the fluid UX of professional NLEs like Premiere Pro or Resolve.

### 1. Interaction Foundation (`egui::Sense`)
To handle complex mouse interactions without conflicting with the global scrolling of the timeline, use `egui::Sense` on each cue.
- **Click and Drag:** When allocating the `Rect` for a cue on the timeline, interact with it using `ui.interact(rect, id, egui::Sense::click_and_drag())`. 
- **Z-Ordering & Ghosting (UX Detail):** When `response.dragged()` is true, the active cue must appear to "lift" off the canvas. 
  - Render the actively dragged cue *last* (so it appears on top of all other clips).
  - Add a subtle drop-shadow (using `ui.painter().rect_filled` with a soft blurred color offset) and slightly increase its brightness so it visually detaches from the grid.

### 2. Time Translation (Moving Cues)
Users expect to click the center body of a cue and drag it horizontally to change timing, or vertically to change tracks.
- **Horizontal Translation (Time):** Calculate the delta (`response.drag_delta().x`) in screen-space pixels. Convert this delta back into "seconds" based on your timeline's current zoom scale (e.g., `pixels_per_second`). Apply this delta to the cue's `start_time`.
- **Vertical Translation (Track Switching):** If the user drags a cue far enough up or down (beyond the Y-bounds of its current track row), snap the cue to the adjacent track. 
  - *UX Detail (Visual Feedback):* While holding the clip over a new track, highlight the destination row background slightly so the user knows exactly where it will land upon release.
- **Bounds Constraint:** Implement a strict `f32::max(0.0, new_start_time)` constraint so the operator can never drag a cue into "negative time" (before `00:00:00:00`).

### 3. Edge Trimming (Resizing Cues)
Trimming is the most frequent action in an NLE. Instead of a single bounding box, you must subdivide the cue's `Rect` into three invisible interaction zones.
- **Hitbox Zones:** 
  - **Left Edge (In-Point):** The first 4-6 pixels of the cue.
  - **Right Edge (Out-Point):** The last 4-6 pixels of the cue.
  - **Body:** The remaining center area.
- **Cursor Affordances:** When `ui.rect_contains_pointer(left_edge_rect)`, instantly change the mouse cursor using `ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal)`. This tells the operator they are in the trim zone.
- **Trim Logic:**
  - **Right Edge Drag:** Only modifies the `duration` of the cue. (e.g., `duration += delta_x_in_seconds`).
  - **Left Edge Drag:** Modifies *both* `start_time` and `duration` inversely. If the user drags the left edge 1 second to the right, `start_time` increases by 1, and `duration` decreases by 1. This ensures the right edge stays visually anchored in place.
- **Minimum Duration Limit:** Enforce a hard minimum duration (e.g., `0.1` seconds) during trimming so a cue cannot be inverted or crushed out of existence.

### 4. Magnetic Snapping (Precision UX)
Free-dragging is chaotic. Professional operators rely on "snapping" to align cues perfectly.
- **Snap Targets:** Build a temporary array of all potential snap points: the current playhead position, the start/end edges of all other cues on the timeline, and the 0.0 origin.
- **Proximity Detection:** As the user drags a cue (or trims its edge), compare its new timestamp against the snap array. If the timestamp comes within a threshold (e.g., `10 pixels` converted to time), override the dragged value and lock it precisely to the snap target.
- **Snap Feedback:** 
  - *Visual:* Briefly draw a vertical, bright-colored "snap line" that intersects the snapped points, proving to the user that alignment was successful.
  - *Tactile:* Allow the user to hold the `Shift` or `Alt` key while dragging to temporarily bypass/disable snapping for fine-tuning.

### 5. Multi-Selection & Bulk Edits
Live-show programming requires moving groups of effects together.
- **Box Selection (Lasso):** If the user clicks and drags on an *empty* space in the timeline grid, initiate a selection box. 
  - Draw a semi-transparent blue rectangle (`ui.painter().rect()`) tracking the drag.
  - Any cue whose bounding box intersects with the lasso rectangle is added to a `HashSet` of selected IDs.
- **Modifier Keys:** Support `Ctrl+Click` (Windows/Linux) or `Cmd+Click` (macOS) to add/remove individual cues from the active selection.
- **Bulk Translation:** If multiple cues are selected, dragging any *one* of them must apply the identical time delta to *all* of them.

### 6. Track Header Overrides (Mute & Lock)
The track headers (the static left column) need functional controls to protect programmed work.
- **Lock (L):** Add a padlock icon button. If toggled, the track row darkens slightly. In your interaction logic, if `track.is_locked == true`, ignore all `egui::Sense::click_and_drag()` checks for cues on this row.
- **Mute (M) / Solo (S):** Add mute toggles. If muted, the hardware execution engine completely ignores cues on this track during playback, and the cues are rendered with a 50% opacity multiplier on the timeline to signify their inactive state.

---

## Phase 8: Global Safety & Serial Connection

**Goal:** Implement the top-level safety controls and hardware connection management necessary for physical rigs.

### 1. Hardware Connection Status
- In the top menu bar or status bar, add a visual indicator for the connection state (e.g., a green "COM3 Connected" or a red "Hardware Disconnected").
- Provide a dropdown or dialog to select the active serial/COM port to connect to the physical relays.

### 2. Emergency Kill Switch (E-STOP)
When controlling physical hardware (water pumps, air cannons), software must have an immediate cutoff.
- Create a prominent, large red "KILL ALL RELAYS" or "E-STOP" button in the global UI (often in the top menu bar or within the Hardware Monitor tab).
- **Behavior:** Clicking this should immediately bypass the timeline and send a global `OFF` signal to all relays in the hardware abstraction layer, and automatically pause the video playback.

---

## UI Styling Tips for `egui`

To match the high-density, dark Premiere Pro aesthetic wireframed:

- **Theme Override:** Ensure the global visual theme is set to dark mode.
- **Colors:** Override the default panel background colors to `#212121` and inner frame backgrounds to `#1a1a1a` to create depth.
- **Fonts & Density:** Keep the global font sizes capped at 11.0 or 12.0. Use a clear Monospace font for timecodes and hardware logs, and a clean Sans-Serif for standard UI labels to maximize information density without clutter.
- **Custom Tabs:** Customize the appearance of selectable labels to mimic NLE tabs (flat, uppercase, with colored top borders for active states) rather than standard rounded buttons.

