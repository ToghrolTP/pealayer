# Effect editor timeline context menus

Scope: the native Effect properties dialog's sequence timeline shown in the user's screenshot. The Web editor is unchanged by this focused pass.

## Use

Right-click a cue to select that exact cue and open its menu:

- Edit cue: brings the existing Selected cue editor into view, including all peripheral-specific parameters. No duplicate editor/modal.
- Action: seats offer Up, Down, Stop; relays offer On, Off. Changes update the corresponding semantic action IDs in the draft.
- PWM cues offer an intensity slider in percent; RGB/addressable cues offer a color picker.
- Timing: edit start in seconds, enable/remove duration, edit duration in seconds, move to zero, or quantize this cue using the current grid.
- Repeat: enable looping, count and interval using the existing repeat controls.
- Duplicate / Delete: apply to the pointer-selected cue, not the previously selected cue.

Right-click empty timeline space for Snap to grid, Quantize all cues and Remove leading delay. Changes use the existing draft and existing Save to PCController path. Nothing in these menus invokes hardware or starts playback. Primary dragging retains movement/resizing; secondary dragging never alters a cue. Arrow-key nudging is suppressed while a popup or input owns the keyboard.

## Verification

- `cargo test --lib --locked --jobs 1 ui::effects_library::tests -- --test-threads=1`: 15 passed. Actual pointer events in both light/dark themes prove right-click selection when another channel was selected, functioning Duplicate, preserved metadata/other cues, and no mutation during secondary dragging. Semantic seat/relay action tests and timing/repeat quantization checks pass.
- `git diff --check`: passed. No full suite or real-output activation.
- Computer-use skill was used to attempt native visual inspection. Windows.Graphics.Capture failed twice with a fresh window binding: `IGraphicsCaptureItemInterop.CreateForMonitor`, `0x8007041D` (capture-service timeout). No screenshot or live native-menu visual acceptance is claimed; no blind UI inputs were sent.
- PR #46 remains unmerged.
