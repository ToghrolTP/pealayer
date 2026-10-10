# Timeline toolbar selection and drag feedback

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

Selected and pressed native timeline actions use the configured accent for
their outline, not the selection text-contrast color. Stroke width and hit-box
geometry stay invariant.

Toolbar dragging shares the effect cards' mouse-down grab-offset capture and
translation. The source geometry and offset are frozen in the drag payload;
the preview no longer jumps above the pointer or drifts after source reflow.
Its painter-only overlay remains visible outside the toolbar without blocking
drop-target hit testing.

Hovering a toolbar drag over Trash transitions the preview to faded,
red-outlined, struck-through removal styling over 150 ms. Leaving Trash
reverses it. Hover alone changes no configuration; a primary-button drop
hides the control, which can be restored from the toolbar menu. Escape cancels
without republishing the same held drag. Secondary gestures cannot start it.

## Verification

Focused headless egui tests cover exact noncentral mouse-down offset, source
reflow, actual Trash hover/leave/drop, Escape cancellation, secondary gestures,
accent outlines in both palettes and light/dark themes, removal preview paint,
stable geometry, reordering and held-pan bounds. Existing effect-card drag,
grip, ScrollArea and same-frame payload tests are rerun as regressions.

Package and Cafe delivery evidence will be added after verification. These
tests are not human native pixel/interaction acceptance. Production media,
paused position, cue state, workspace and controller connectivity must remain
unchanged; no physical output is exercised by this UI pass.
