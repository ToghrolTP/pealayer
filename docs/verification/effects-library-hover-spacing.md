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
Delivery completed through the product's peer updater, not a manual running-file
replacement or production-host build:

- Clean packaged source: `fa062a91d4afadc6c757db9b7ed8a0dfae0b00ec`,
  [PR #129](https://github.com/ToghrolTP/pealayer/pull/129).
- Executable SHA-256:
  `23aba6871ba45af3992f6a9b4d34f7703f87b98348536246519b554f6da57c64`.
- Canonical `build.cmd -SkipTests -NoUpx` reused the existing shared Cargo target
  and staging directory. Release compile/link, native Windows resource checks,
  executable/downloader smoke tests and Web/PWA guardrails passed. The full
  native suite ran separately; packaging did not redundantly rerun it.
- Both Cafe runtime aliases retained SHA-256
  `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
  No workstation DLL was copied to production; destination candidate identity
  and smoke tests passed against the destination's own runtime before updating.
- Update `update-570ea143-3aca-4c39-909d-8102b19b99a6` completed. Its live manifest
  advertised the exact executable SHA/commit with `git_dirty=false`.
- The new process was PID 66840, interactive session 1. Loaded private media,
  original zero cues, paused position and NLE workspace were preserved. Hardware
  remained connected with no error/EStop, no reconnect wait samples and no new
  matching Application Error/WER events. Cafe free space remained 15.91 GiB.

This receipt proves package and installed-runtime identity/continuity. Production
hover appearance and scrollbar use still need human/native visual acceptance;
no effect preview, playback or physical output was triggered for this layout fix.
