# Front-panel mirror fidelity

The controller's old WebSocket limiter used one timestamp for RGB, PWM and
seven-segment frames. A just-delivered RGB frame could cause a digit or blink
edge to be discarded. Pealayer's 16 ms subscription requested that decimation.
Its separate peer hardware relay also sampled only every 100 ms, potentially
losing intermediate display states before a remote Pealayer could paint them.

Pealayer now requests immediate changed-state events, preserving the independent
500 ms telemetry sample. PCController additionally exempts changed segment edges
and keeps continuous frame budgets independent. Peer hardware delivery checks
the existing atomic hardware revision in its WebSocket loop and serializes only
when the revision changes. It no longer waits for a 100 ms hardware sample.
Unchanged idle hardware is not repeatedly cloned/serialized; disabled Web sync
and configuration access still prevent transmission, including re-enable refresh.

Native rendering still paints authoritative latest state at the available screen
cadence. No invented intermediate frames, stale-animation replay, extra board
polling or per-frame persistence is added. A screen cannot display every physical
transition occurring faster than its own refresh. The general Web status cadence
remains separately configurable; this pass fixes direct native/peer state paths.

## Source verification and historical packaging gate

Offline cargo check passed with existing warnings. The subscription regression
was updated; tests and CI were not executed at the user's request. Local Pealayer
was not running at inspection. Cafe/Erfan replacement and human visual acceptance
remain separate delivery gates; preserve host-specific libmpv and authoring state
and use the approved updater. Paired Linux short-note recovery is documented in
PCController's `docs/verification/buzzer-mirror-jitter.md`.

The final Linux receiver has now been installed, restarted and hash-verified,
with Cafe connected and native mirroring enabled; this does not install the
Pealayer fix or establish acoustic fidelity. `scripts/package-windows.ps1`
requires a clean source checkout. Its existing generated `web_ui/dist` changes
were preserved rather than overwritten/reset to force packaging. The #80 follow-up
now hash-verified all 49 files in a named preservation stash (see
[workspace hygiene](LOCAL-WORKSPACE-HYGIENE.md)), clearing that source-cleanliness
blocker. Package creation, approved host updates and visual/audio acceptance
remain separate steps; this preservation does not install the fix.

## Current delivery

The subsequent [issue 80 receipt](issue80-native-delivery.md) verifies the new
Pealayer and paired PCController sender on Cafe, with its original libmpv and
authoring state retained. This supersedes the older Cafe packaging/install gate
above. Screen/physical display fidelity and Linux acoustic fidelity still require
live observation; latest local/Erfan activation is not verified.
