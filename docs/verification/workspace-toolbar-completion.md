# Command toolbar and palette completion

Owner: [delivery #80](https://github.com/ToghrolTP/pealayer/issues/80).
This complements [workspace tools](workspace-tools.md), not a replacement toolbar.

## Causes and correction

- The command palette used an unbounded horizontal header in a centered modal.
  Its inherited remaining height produced a nearly full-height blank column,
  pushing search/results out of view. Explicit header allocation and a stable
  results viewport fix that feedback loop. The results budget includes measured
  wrapped keyboard help, control/font height and modal chrome.
- Workspace toolbar reorder reused shared drag helpers but omitted the overflow
  hide destination entirely. `action_customization::menu_hide_target` now owns
  the dots-to-red-Trash transition for both workspace and timeline toolbars.
  Each owner validates its typed payload; only primary release hides an action.
  Editor-row payloads cannot remove toolbar shortcuts. Workspace panel state,
  command order and unavailable extension IDs are preserved.
- Button text sizing made toolbar icons smaller than the rail. Toolbar and drag
  preview glyphs now use its 19-point baseline, bounded by the current control
  size, with visible-ink centering. No resting outlines were added.
- Studio weak text was not sufficiently different from enabled icon ink.
  Unavailable actions now use explicit 35% icon opacity and no active/hover
  decoration. Invocation stays capability-gated; customization remains possible.
  Cursors distinguish available actions, unavailable actions, reorder grips and
  an ongoing typed drag. Lock/modal restrictions still win.

## Verification

Focused native coverage includes actual pointer reorder/hide, reversible Trash
hover, Escape/lock protection, unavailable Save dragging, editor isolation,
palette keyboard/modal ownership, actual disabled paint/cursors, and stable
palette bounds under narrow/large viewports, large fonts and RTL. The existing
timeline and transport customization regressions exercise the shared target.

Native captures, installed-package identity and final test counts are recorded
with the delivery receipt below. Cafe-PC is user-deferred/offline; local delivery
must not be reported as Cafe deployment or physical hardware acceptance.
