# Reveal-playhead availability

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Continues [truthful timeline state](paused-playback-and-track-state.md).

The existing current-frame visibility check already disables the toolbar action
and ruler-menu entry when the complete playhead is in view. It accounts for the
active scrub target and ruler handle. A dimmed action cannot emit a reveal
request; its drag customization remains available.

What was missing was the requested slight fade. Shared customizable action ink
now transitions between normal and muted colors using the existing 120 ms
motion policy. System reduced motion and the explicit animation-off preference
apply immediately. Invocation remains guarded immediately, independently of
the presentation animation. Re-enabling also permits invocation immediately.
No new preference, timer, media seek, viewport movement or geometry change.

An initial transition regression exposed why disabled customization controls
need special care: their drag-capable response still receives hover, which
could brighten the target color mid-fade. Unavailable ink now uses its steady
inactive source rather than hovered/pressed ink. Normal enabled interactions,
selected backgrounds and drag/hide ownership are preserved.

## Verification and delivery

Rendered tests exercise real toolbar pointer events throughout the fade, verify
monotonic intermediate colors, unchanged glyph position, muted endpoints,
immediate reduced-motion behavior, and immediate availability after leaving
the viewport. Existing visibility/handle/scrub tests and dimmed-action drag
customization checks remain in place.

The isolated native UI suite passed **555 tests, zero failures, one opt-in test
ignored**. Parent-provided private config/instance identities keep persistence
fixtures away from normal settings. `git diff --check` passed.

Installed checkpoint pending. Cafe deployment remains deferred
at the user's request while its normal connection is unavailable. No hardware
actuation or production media playback is needed for this presentation change.
