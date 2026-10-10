# Effects Library spacing and hover actions

## Cause and repair

The native library deducted the theme's scrollbar allocation and a separate
10-point right gutter. The floating scrollbar could still expand over content;
the extra deduction did not establish a solid scrollbar/content boundary.

Effects Library and Hardware Monitor now use one shared bounded scroll helper:
a 6-point solid scrollbar and 3-point gap, with no extra content margin. The
lane is reserved once, including show/hide animation, so library toolbar, search,
folders and cards have the same trailing edge. Narrow content is not forcibly
widened to a minimum card width.

Library rename, preview, placement and menu actions use stable frameless icon
slots revealed by whole-card hover. Folder New uses the same button style and
hover policy. Keyboard-focused actions remain visible and traversable; an open
card menu keeps its actions visible. Grips follow the Hardware Monitor hover
affordance. Essential Hardware Monitor output controls are unchanged.

## Verification

On 2026-10-10, the isolated native library suite passed **748 tests**, with zero
failures and one intentionally ignored test. The parent process supplied
`PEALAYER_CONFIG_FILE`, the validated host libmpv and the shared Cargo cache
before test startup. No production settings or outputs were used by fixtures.

New headless widget regressions cover:

- One 9-point lane, no card/viewport overlap, no global style leakage, at
  180/309/520-point widths with fitting and scrolling content over four frames.
- Unpainted resting icons; whole-card hover, folder hover and keyboard focus
  reveal their intended actions in light and dark themes without shifting slots.

Existing folder-create clicks, context menus, grip/whole-card dragging, original
grab offsets, scroll gesture competition, and Hardware Monitor checks also pass.
These are native widget/geometry tests, not a production native screenshot or
human interaction acceptance. Remaining release gates in the current index stay
open. This pass does not change firmware, controller contracts or Web behavior.

## Deployment

Before packaging, Cafe source was clean main and the running application was
paused, connected, without hardware error, and in NLE with its original cues.
Build and destination-updater receipts are recorded after delivery below.
