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
