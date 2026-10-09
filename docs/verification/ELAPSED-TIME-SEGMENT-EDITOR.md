# Elapsed-time segment editor

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Web counterpart: [parity ledger](../WEB-APPLICATION-PARITY.md).

## Interaction contract

The editable field has a fixed `HH:MM:SS.sss` mask. Hours, minutes, seconds
and milliseconds are selected as complete groups. Text edits never insert or
delete punctuation. Native non-editing display still honors the subsecond
preference; editing always includes milliseconds for exact entry.

- Unchanged blur does not seek. Changed blur seeks once. Enter explicitly seeks
  even if the value is unchanged. Escape restores the original draft, releases
  focus and always suppresses the blur seek, including the Enter/blur event batch.
- Digits replace the selected group; completing it automatically selects the
  next group. An explicitly typed separator after automatic advance is skipped
  without advancing twice. Left/Right selects adjacent groups; Home/End selects
  the first/last. Up/Down adjusts the active group.
- Backspace removes the last entered digit (or clears the selected group when
  no digits were entered); after automatic advance it returns to the prior
  group. Delete clears the selected group. Separators and field length survive.
- Invalid characters and malformed/out-of-range pasted values are rejected.
  Arabic/Persian decimal digits normalize to ASCII. Valid seconds/timecodes
  paste into the fixed mask. Hours cap at 99, minutes/seconds at 59, milliseconds
  at 999; the existing authoritative seek path bounds the final media position.
- Native selection is stored before painting and edit events request repaint,
  fixing stale group highlighting while paused. Only the owning elapsed widget
  edits the draft when more than one monitor is present.
- Web Simple/NLE share one input/model, immutable-mask handling and synchronous
  cancel/commit state. They reuse the correlated seekbar commit path rather than
  introduce a second seek contract. Small monitor controls wrap to keep the
  complete mask, seekbar and volume strip reachable.

## Verification and delivery

Native/Web regression checks cover commit precedence, typed separators,
Backspace, local digits, invalid paste and group navigation. The user deferred
test suites and slow CI waits; checks are added for the next normal run and are
not claimed executed. Build/type-check, host-runtime smoke, actual installed
identity and human input/gesture acceptance are recorded separately below or in
the owning issue checkpoint. Do not replace destination libmpv from another host.

### Installed checkpoint — 9 October 2026

- Source: `b7cefba2043e3665704b523c9001fbb8cec5455b`, clean build;
  [PR #97](https://github.com/ToghrolTP/pealayer/pull/97) is stacked on
  [PR #96](https://github.com/ToghrolTP/pealayer/pull/96), not merged into main.
- Native release build, Web TypeScript/Vite/PWA build, seven Windows quick-action
  icon resources, package integrity and both David/Cafe-runtime libmpv startup
  smokes passed. The focused input regression suites remain unrun as requested.
- Installed executable SHA-256:
  `85ab0b6903da2966a7f81dd1b66877d7966c5b6a08731f889b976e50dc3e3127`
  (11,147,264 bytes). Embedded PWA: `f01f333bf9e39fbc`.
- David and Cafe report that exact source, executable and PWA through their
  process-local APIs. Both canonical processes are running in interactive
  session 1. Cafe retains its own runtime; hardware is connected without an
  error. Media remains paused at 660.800 seconds. David remains a cache-only
  Cafe consumer, with no hardware scheduler, no command/transport error and
  zero paused preview drift in the checked sample.
- Both updates used the existing begin/chunk/finish updater contract and graceful
  exit. David's first activation did not remain running and returned to the
  prior executable while Cafe was restarting. The exact first failure cause
  was not captured; do not invent one. After Cafe was healthy, the prior David
  executable was relaunched and the same API update succeeded. Consumer and
  authority deployments should be sequenced rather than restarted together.
- Erfan's canonical executable and rollback hashes were verified after offline
  atomic replacement; its existing libmpv was unchanged. KMPlayer remains
  running, so Pealayer was deliberately not launched there. Interactive/playback
  acceptance on Erfan remains pending, not passed.
- Human acceptance requested: unchanged blur, explicit Enter, changed blur,
  Escape cancellation, group highlights, separators and Left/Right navigation.
  Source/build/deployment evidence is not a substitute for that input check.
