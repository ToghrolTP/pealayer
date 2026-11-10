# Native dropdown navigation and color rows

## Causes and repair

The native ComboBox did not seed a choice focus when opened, and ordinary egui
directional focus was not a deterministic dropdown selection policy. The accent
menu also laid out the circle, label and custom editor as separate widgets with
different heights. Its nested picker competed with the enclosing menu's close
event; the selected control omitted its color indicator entirely.

All native ComboBoxes now share explicit choice navigation, preserving egui's
exact trigger IDs and existing application commands. Up/Down wrap and skip
disabled choices, Home/End move within choices, Enter commits and Escape cancels.
Search fields keep typing/caret keys until an arrow enters the results. Both
full and compact icon pickers use the same ownership policy; list/grid switches
stay open. Focused offscreen choices scroll into view. Multiselect checkbox
choices are registered without dismissing their popup on every toggle.

Choice rows span the popup width at a fixed 32-point height. Painting uses fixed
text/swatch lanes, inside strokes and zero hover expansion instead of changing
padding or text geometry. Accent and recording color controls retain their
selected swatches while closed.

Custom is one clickable row, including its inset HEX preview. It opens a palette
anchored outside the closed selector, with an editable HEX field, HSV picker,
shared Pealayer swatches (including Orange) and deduplicated recent colors.
Recents are bounded to 12 and session-only: picking colors adds no configuration
file writes. Partial HEX edits remain in the palette draft; invalid values do
not replace the valid preference or become invented recent colors.

## Verification

The isolated native library suite passes **753 tests**, with zero failures and
one intentionally ignored test. The parent supplies isolated
`PEALAYER_CONFIG_FILE`, validated host libmpv and the existing shared Cargo target
before startup. This includes existing group creation/search/clear, icon filter,
list/grid switching, recording color and menu interaction regressions.

New native widget tests cover exact trigger identity, initial focus, disabled
choices, arrow navigation, Home/End, Enter/Escape and editable search ownership.
Accent popup and row geometry is exercised in light/dark themes at 100%, 125%,
150% and 200% scaling: equal full-width rows, stable hover positions, unclipped
inset strokes, closed swatches, and clicks on both Custom and its HEX preview.
The opened palette paints app Orange and recent swatches. Session recents are
bounded, deduplicated and not carried into a fresh context.

These are real headless egui interaction/geometry tests, not a Cafe desktop
screenshot or human native visual acceptance. No controller/firmware/Web API
contract changes or physical-output actions are part of this pass. Release gates
in the current acceptance index remain open.

## Delivery

The destination was audited before packaging: clean main, paused NLE, original
cues, controller connected, no hardware error/EStop and the destination's own
validated libmpv. Delivery completed through the product's peer updater:

- Clean packaged source: `759dafebc836e75dac3678fe4722a4bd7c1cb846`,
  [PR #130](https://github.com/ToghrolTP/pealayer/pull/130).
- Executable SHA-256:
  `67c9b721bc00935592e1ca3e7c2dc4f49d8871b966b62949d31059453a96af5a`;
  46,269,952 bytes. Canonical `build.cmd -SkipTests -NoUpx` reused the existing
  target and staging directory. Release compile/link took 1m35s; Windows native
  resources, executable/downloader smoke and Web/PWA guardrails passed. The full
  native suite ran separately, not redundantly during packaging.
- Both destination libmpv aliases retained SHA-256
  `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
  Only the executable was transferred. Candidate identity and smoke tests used
  the destination's own runtime; no Rust build or workstation DLL replacement
  occurred on Cafe-PC.
- Update `update-c641aa40-45b6-42ba-a1a3-46838041a944` completed. The live manifest
  advertised the exact candidate commit/SHA with `git_dirty=false`.
- Interactive PID 64876, session 1: paused NLE, original loaded private media,
  paused position and zero original cues retained; controller connected, no
  hardware error/EStop, zero reconnect wait samples and no new matching
  Application Error/WER events. Cafe free space was 16.01 GiB.

This receipt confirms installed identity and continuity, not human native visual
acceptance or a playback/CPU/physical-output soak. The later receipt-only commit
does not require rebuilding identical application code.

## Hosted CI boundary

Repository health and CodeQL passed for the packaged source. Its hosted platform
jobs were still running at delivery. Refresh their final results rather than
assuming all-green CI from the local MSVC suite.

Separately, the preceding main run
[38075675309](https://github.com/ToghrolTP/pealayer/actions/runs/38075675309)
passed Linux and both macOS builds but its Windows GNU unit-test process exited
with `STATUS_ACCESS_VIOLATION (0xc0000005)` at the EStop confirmation fixture.
This occurred on main before the dropdown changes. It is additional evidence
for the existing [native lifecycle investigation #116](https://github.com/ToghrolTP/pealayer/issues/116),
not proof of its root cause or a fault caused by this fix. Keep that investigation
and cross-platform release gates open.
