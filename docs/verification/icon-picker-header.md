# Native icon picker header

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Cause and correction

The shared picker allocated the magnifier as a separate label, guessed the
remaining input width with extra subtraction, and used independent buttons.
Its input's font-aware minimum could exceed the button height. These independent
allocations left unused trailing space and inconsistent vertical alignment.

The shared header now allocates one density/font-aware row. The search input
contains its magnifier with reserved leading padding; the remaining width ends
exactly eight points before a trailing List/Grid segmented control. RTL mirrors
the slots. Both selector and compact-button entry points use this same header.

The mode switch has one quiet enclosing surface, an accent-filled selected
segment, muted inactive ink, hover/pressed feedback and an inside keyboard-focus
outline. Glyphs are centered by their visible ink bounds. Borders and interaction
states never change layout. Each segment exposes its label and selection through
accessibility metadata, and retains the shared picker's existing search focus,
filtering, initial-focus and remembered view-selection ownership.

## Verification

- Rendered header geometry: 720 combinations of five densities, two font sizes,
  three fractional display scales, English/Persian, both palettes, light/dark and
  three widths. Prefix containment, identical vertical centers, eight-point gap,
  non-overlap and exact trailing anchoring are asserted after sizing settles.
- Actual pointer/paint checks exercise both picker forms: selected/hover state,
  prefix-click focus, typing, List/Grid switching and unchanged selected value.
  Exactly one segment is painted selected even on the switch-release frame.
- Existing popup search/filter/selection/reopen, remembered view and bounded-width
  regressions remain part of the focused native icon-picker suite.
- The complete native UI suite passed: 563 checks, no failures, one intentionally
  ignored check. The isolated fixture supplies its own configuration path and
  instance identity before startup; production settings are not test input.

## Before

The user-provided native crop records the reported alignment and spacing defects:

![Before: detached search icon and independently sized buttons](../screenshots/icon-picker-header-before.png)

## Installed-build receipt

The canonical executable is installed through the graceful peer updater and is
running from source `f3ca393ec91f95fbda358fa8cc1fc01b92a2db55`. The package was
built from a clean source tree; Web bundle integrity, native resources, all seven
quick-action icon resources, downloader smoke and libmpv smoke passed. The
packager skipped duplicate regression runs after the separate UI suite above.

The executable's acknowledged SHA-256 is
`a5fc8a55350aec0bd6436e3d7d79ba6f5139a8fb831e3d9b4bf11e74197ef894`.
Its destination libmpv remains
`e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
The post-restart process API independently reports the matching source and
executable. Playback remains paused at the exact pre-update position; the full
workspace/profile/session and selected appearance fields compared unchanged.
No hardware command, layout reset or window-placement change was requested.

The final live native picker capture is **pending** the user opening its popup.
The source/render tests and installed-process acknowledgement are not a claim of
that screenshot check or human acceptance. The PR remains draft for that gate.

Cafe deployment remains deferred at the user's request while its normal route
is offline; this local change is not a production-host delivery claim.
