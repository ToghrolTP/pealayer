# Timeline toolbar selection and drag feedback

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

Selected and pressed native timeline actions use the configured accent for
their outline, not the selection text-contrast color. Stroke width and hit-box
geometry stay invariant.

Toolbar dragging shares the effect cards' mouse-down grab-offset capture and
translation. The source geometry and offset are frozen in the drag payload;
the preview no longer jumps above the pointer or drifts after source reflow.
Its painter-only overlay remains visible outside the toolbar without blocking
drop-target hit testing.

Hovering a toolbar drag over Trash transitions the preview to faded,
red-outlined, struck-through removal styling over 150 ms. Leaving Trash
reverses it. Hover alone changes no configuration; a primary-button drop
hides the control, which can be restored from the toolbar menu. Escape cancels
without republishing the same held drag. Secondary gestures cannot start it.

## Verification

Focused headless egui tests cover exact noncentral mouse-down offset, source
reflow, actual Trash hover/leave/drop, Escape cancellation, secondary gestures,
accent outlines in both palettes and light/dark themes, removal preview paint,
stable geometry, reordering and held-pan bounds. Existing effect-card drag,
grip, ScrollArea and same-frame payload tests are rerun as regressions.

All 21 focused tests passed: ten toolbar tests, eight effect-card regressions,
one effect-library grip test and two same-frame/translation tests. Windows
incremental packaging passed TypeScript/Web/PWA guardrails, native resources
and icons, libmpv smoke and standalone downloader smoke. Package-wide tests and
UPX were skipped; the focused tests ran separately with isolated configuration.

## Cafe delivery · 10 October 2026

- Clean built source: `d71c88d6dfa670339dbfc808c9724c52ac12c821`.
- Executable SHA-256:
  `0326841851571633113264b8d815f936a57d4aca14c5822e813042aaf1091105`.
- Retained destination libmpv SHA-256:
  `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
- Graceful peer updater: `update-8935a03f-7096-4310-80ac-d8cfe1b0ccdd`,
  completed and acknowledged the verified executable.
- Live PID 119772, interactive session 1, canonical installed `bin` path.
- The immediate post-update verification reported a publishing-authority
  timeout. Normal controller reconnect cleared it without another deployment
  or application/board reset. Two subsequent samples, separated by 12 seconds,
  confirmed connected hardware, no hardware error, paused playback and no E-STOP.
- Loaded media, the empty authoritative cue array, NLE workspace and paused
  position at 657.84 seconds were stable across those post-reconnect samples.
  The initial verification stopped at its hardware-error assertion, before
  exact pre/post media and position comparisons; do not call this a proven
  nonempty-cue or exact pre-update media-identity preservation test.
- No new Pealayer/libmpv Application Error or WER event in the bounded
  deployment window. No motion, relay or other physical output was exercised.

Tests and live deployment health do not establish human native pixel/interaction
acceptance, which remains open in the current acceptance index. Hosted platform
CI is tracked separately; this receipt is not an all-platform green claim.
