# Status bar widgets

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

Native and Web footers share fourteen stable widget IDs, independent visibility,
and a persisted complete order. Dragging uses the original grab offset and a
before/after insertion marker; context menus also offer Move earlier/later.
An individual widget owns its menu, with a separated Hide action containing its
icon and name. Hiding hardware connection does not hide voltage/current/temperature.
Hiding the footer E-STOP indicator does not remove the main emergency control.
The workspace is pinned at the logical trailing edge, outside the native primary
widget scroller; it is not draggable into the telemetry group. Other widgets keep
their saved order. Updating labels have fixed-width, truncated slots with full
hover text. FPS and hardware numbers use fixed slots and monospaced digits, so
digit-count changes cannot move neighboring widgets. FPS, connection, hardware
readings and alerts use emphasis faces without increasing their font size.

Frame rate stays visible when unavailable (`— fps`). Clicking cycles Media,
Screen and Auto. Screen means measured native rendering cadence, not the
monitor's advertised refresh rate. Auto uses media FPS while loaded and playing,
and screen rendering cadence when paused or unloaded; playback speed and pointer
interaction do not change source media FPS. Existing explicit source selections
and widget visibility remain user preferences.
The visible FPS caption contains only its source icon and number/unit. Mode names
(including Auto) remain in the tooltip/menu, not in the changing footer text.
Reactive idle frames use their actual measured interval (for example 2 FPS at
500 ms) rather than losing the number whenever the active rendering window
resets. This does not force a continuous rendering loop or assume monitor Hz.

Hardware monospace values use compact 12-point native / 11-pixel Web text and
five-point inter-widget spacing. Missing/disconnected readings are absent, not
zero. The Effects Library heading starts at its content edge, vertically centered.

## Logging and progress contract

- The optional chart is hidden by default. It retains at most 240 RAM-only
  samples at no more than two samples per second, with voltage/current/temperature
  selection, range/units, and Clear history. Disconnections break the line.
  This is bounded session history, not persistent logging or an export service.
- `status_operations` exposes actual active app updater, controller artifact and
  initialized download-manager operations to native peers and Web clients.
  Controller `controller.update.status` seeds a mid-operation connection; changed
  events carry subsequent real stage/byte progress. Percent is shown only when
  `progress_known` is true. Preflight/unknown totals remain indeterminate.
- Controller operation timestamps are compared with normalized RFC3339Nano
  fractions. Older events/catalog snapshots cannot resurrect a completed operation.
  Operations from a different coordinator instance are not retained.
- The download footer reads a narrow memory-only progress projection: no engine
  provisioning, download-store initialization, filesystem probing, or duplicate
  per-frame speed-history copies. Completed/failed/cancelled operations disappear.
- HOT uses the controller's status flag and typed HOT alert events (including the
  Go omitted-false clear representation), never a guessed temperature threshold.
  No physical alert is induced for UI verification.

## Verification and delivery boundaries

### Peer RGB cache invalidation

The hardware cache introduced in `fff09372` required each producer to advance
`hardware_revision`. Native peer workers installed changed snapshots and woke
the UI, but omitted that invalidation: a paused status RGB dot could keep its
cached color even after a new authoritative frame arrived. Full peer snapshot
installation now changes the snapshot and revision in one critical section,
before the worker requests repaint. Identical snapshots do not advance it.
The cache remains enabled; no animation timer, synthetic color, disk logging,
or extra controller polling is introduced.

Regressions prime the cache first, install RGB-only peer changes (including off),
render the paused status dot with no input, and verify native/Web projections.
They also cover unchanged snapshots, missing hardware, a restarted source and
the existing direct-controller RGB push path. These fixtures do not command a
physical LED or claim live animation acceptance when the board reports steady RGB.

Focused tests cover real native FPS clicks, menu ownership/hiding and drag/drop,
title placement, unavailable telemetry, bounded charts, source switching, typed
HOT activation/clear, and ordered operation recovery. Web model tests cover the
same order/visibility/FPS/logging rules; TypeScript/PWA builds verify integration.

Local verification: 786 native library tests passed, one existing test ignored;
14 download unit/integration tests passed, three external-engine acceptance
tests remain explicitly ignored. The Web model, parity, TypeScript and packaged
PWA checks passed. Installed runtime captures are recorded in the delivery receipt.
Source/fixtures do not prove physical HOT events, firmware programming progress,
interactive human acceptance or a sustained production soak. Cafe runtime
deployment remains blocked by the previously recorded smoke-launch denial; do
not bypass it with another launch mechanism or build on Cafe.
