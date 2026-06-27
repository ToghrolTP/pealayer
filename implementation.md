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

**Goal:** Allow the operator to modify cues directly on the timeline by dragging them, rather than relying solely on the Effect Controls parameter sliders.

### 1. Horizontal Dragging (Move Cue)
Users expect to be able to click and drag a clip left or right to change its timing.
- **Hit Detection:** When the user clicks on a cue rectangle, check if they are initiating a drag (`response.dragged()`).
- **Translation:** Calculate the horizontal mouse delta in screen space, convert that delta into seconds (based on your timeline's zoom/scale factor), and apply it to the cue's `start_time`.
- **Constraint:** Ensure the `start_time` cannot go below `0.0`.

### 2. Edge Dragging (Resize Cue)
Users expect to hover over the left or right edge of a clip to change its duration.
- **Edge Hitboxes:** Instead of a single bounding box for the whole cue, define three interaction zones: the left edge (e.g., 5 pixels wide), the main body, and the right edge.
- **Cursor Icon:** When hovering over an edge, change the mouse cursor (`ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal)`) to indicate it can be resized.
- **Duration Mutation:** If the user drags the right edge, modify the cue's `duration`. If they drag the left edge, modify both the `start_time` and `duration` simultaneously so the right edge remains visually anchored.

### 3. Track Controls (Mute/Solo/Lock)
Update the Track Headers (the fixed left column of the timeline).
- Add small toggle buttons for Mute (M), Solo (S), and Lock (L) on each track.
- **Locking:** If a track is locked, disable drag interactions and drop events for that specific row.
- **Muting:** If muted, instruct the backend hardware engine to ignore cues on this track during playback.

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

