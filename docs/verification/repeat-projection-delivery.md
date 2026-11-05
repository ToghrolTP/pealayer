# Repeat projection interaction checkpoint

Owner: [PR #235](https://github.com/ToghrolTP/pealayer/pull/235), following
[PR #232](https://github.com/ToghrolTP/pealayer/pull/232).
The [interaction guide](step-repeat-ghosts.md) covers the requested authoring behavior.

## Verified source and package

- Packaged source: `3bcbc21191762d5189390500c6b839c3add35821`, clean tree.
- Windows executable SHA-256:
  `9b917b07eb7e77c32bb2c9430a65154e5c2da28a60828fd7371ef684012cf9a7`.
- Native tests: 1,104 passed, one ignored. The repeat pointer test exercises
  real source selection, step/whole-effect spacing changes and Escape rollback
  in both themes.
- TypeScript, complete Web prebuild and canonical controller-wire checks passed.
- Installed Edge exercised real controls at 390/768/1440 pixels, light/dark:
  default/saved visibility, source-to-proxy and proxy-to-source selection,
  single-source deletion, spacing dragging/keyboard, shared-length resizing,
  count menus, dotted/dashed outlines, Escape and no page overflow.
- Packaging separately skipped native/Web regression reruns because those
  checks had already passed. Production Web/PWA bundle integrity, Windows
  resources, all seven quick-action icons, libmpv smoke and the standalone
  downloader resource/headless smoke passed. UPX was disabled.

These are disconnected editor checks, not physical timing/output acceptance.
Existing controller definitions, repeat contracts and firmware are unchanged.

## Installation boundary

The running canonical application was started paused and reported clean local
authoring state. The agent's updater launch was policy-denied; the user ran the
approved updater command. That upload reported aborted after 19,660,800 of
58,593,280 bytes, without replacing the executable. A retry with captured output
was requested; installed acceptance remains pending until its actual manifest
matches the packaged executable. No manual executable replacement or policy
workaround was used.

Cafe deployment is deferred at the user's explicit request while unavailable.
It must use that host's compatible runtime and the normal graceful peer updater.
