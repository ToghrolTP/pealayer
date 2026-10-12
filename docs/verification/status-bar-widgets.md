# Status bar widgets

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

Native and Web footers share fourteen stable widget IDs, independent visibility,
and a persisted complete order. Dragging uses the original grab offset and a
before/after insertion marker; context menus also offer Move earlier/later.
An individual widget owns its menu, with a separated Hide action containing its
icon and name. Hiding hardware connection does not hide voltage/current/temperature.
Hiding the footer E-STOP indicator does not remove the main emergency control.

Frame rate stays visible when unavailable (`— fps`). Clicking cycles Media,
Screen and Auto. Screen means measured native rendering cadence, not the
monitor's advertised refresh rate. Auto uses media FPS while loaded and playing,
and screen rendering cadence when paused or unloaded; playback speed and pointer
interaction do not change source media FPS. Existing explicit source selections
and widget visibility remain user preferences.
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

Focused tests cover real native FPS clicks, menu ownership/hiding and drag/drop,
title placement, unavailable telemetry, bounded charts, source switching, typed
HOT activation/clear, and ordered operation recovery. Web model tests cover the
same order/visibility/FPS/logging rules; TypeScript/PWA builds verify integration.

Local verification: 785 native library tests passed, one existing test ignored;
14 download unit/integration tests passed, three external-engine acceptance
tests remain explicitly ignored. The Web model, parity, TypeScript and packaged
PWA checks passed. Installed runtime captures are recorded in the delivery receipt.
Source/fixtures do not prove physical HOT events, firmware programming progress,
interactive human acceptance or a sustained production soak. Cafe runtime
deployment remains blocked by the previously recorded smoke-launch denial; do
not bypass it with another launch mechanism or build on Cafe.
