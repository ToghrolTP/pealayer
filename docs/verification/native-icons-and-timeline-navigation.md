# Native icons and timeline navigation

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Causes and source fixes

- Effect Designer used a horizontal-only ScrollArea without the main timeline's
  configurable wheel routing. It now shares raw wheel units/axes, modifier
  precedence, trackpad pinch and touch pan handling. Ctrl/Alt zoom anchors the
  time under the pointer; Shift/physical horizontal wheel pans time. Plain wheel
  scrolls overflowing rows, or pans time when there are no hidden rows. None
  consumes the gesture without changing navigation. Only the visible timeline
  viewport claims input; toolbar/fields and open popup menus retain theirs.
- Direct relay cues were checked against the library effect's fixed relay
  target, rejecting moves between channels. Native destination highlighting and
  movement now share same-kind direct-cue validation and authoritative row
  availability/locks. Relay/PWM retargeting isolates the dragged placement and
  preserves start time, duration, values, end behavior and action edges. It does
  not convert Off into On or rewrite a published multi-channel effect. Drag HUD
  uses the actual target row's renamed caption, not a literal `Track`.
- PCController presentation state was subscribed at 250 ms (4 Hz). The native
  display/PWM state feed now requests 16 ms, while status sampling remains
  500 ms and discrete events stay immediate. This renders actual received
  state, not interpolated or invented segment frames. Peer snapshot transport
  and physical/audio timing remain separate acceptance work.
- Winit's normal window-icon path updated only the small icon slot and built a
  legacy XOR/AND icon. This was not evidence of an 8-bit source asset: presets
  are already multi-resolution ICO. The new Windows path owns DPI-sized 32-bit
  alpha icons for both small/titlebar and large/taskbar slots. Tray refresh uses
  the small native icon; later legacy viewport updates cannot replace it. DPI
  changes rebuild owned sizes; window destruction releases only owned handles.
- Independently hosted Preferences omitted density and animation mode from its
  appearance-cache key, leaving those previews stale until reopened. Both now
  invalidate that cache and Discard restores the saved density.

## Verification and boundaries

Cached Windows MSVC library suite: 828 tests, 827 passed, one pre-existing
ignored test. Coverage includes real egui wheel events for plain/Ctrl/Shift/Alt,
custom None/horizontal policies, vertical overflow, pointer-time invariance and
toolbar exclusion; shared-template relay/PWM preservation; a private invisible
Windows fixture for both icon slots, DPI-sized replacement, borrowed-handle
survival and owned cleanup; true-color icon bitmap sizes; Preferences preview.

These tests do not operate the production window or board. The running local
application has not been replaced while unsaved-edit readiness is unanswered.
The earlier staged density/buzzer package does not contain this follow-on.
Cafe's GUI/smoke-launch denial remains a deployment blocker; no alternate launch
route was used. Contextual Stop also requires the matching PCController runtime
with bounded-tone activity before installing the paired UI.

Linux mirror's native backend was observed selected without playing a test tone.
A bounded read-only source stream sample received no segment/note changes; it
cannot establish acoustic fidelity or transport jitter under front-panel use.
