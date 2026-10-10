# Prepared playback clock starvation

The recurring Hardware timing fault reported on 2026-10-11 was a controller
`video clock feedback expired` fault, with an authenticated board and no
dispatched cue in the faulted plan. The earlier explicit-Play re-arming fix was
already installed. Initial evidence concerns clock delivery; later bounded
playback also exposed restore-ACK deadlines and a paused-arm race in the
controller. These are separate from the old Play-button recovery defect.

## Clock-path corrections

- Every JSON-RPC header previously recomputed its publisher ID via
  `config::control_port()`, which loaded, parsed and validated the complete
  persisted application configuration. The active clock stream sends at 25 Hz.
  It now retains its successfully registered publisher ID for that stream.
- Port reads use the small live configuration field after initialization.
  Periodic instance reports resolve their port once from the configuration they
  already hold rather than loading it again for each advertised URL.
- Explicit registration is retained on TCP and embedded clients; the first
  telemetry request no longer performs a redundant registration.
- A stale observer previously caused an immediate `continue` with no wait,
  producing a tight loop while the media observer was already starved. That
  branch now waits 10 ms and still sends no extrapolated stale clock.

The 250 ms freshness guard and cue deadline remain unchanged. Routine transport
recovery cannot clear a semantic timing fault or replay missed cues.

## Isolate metadata from the clock

The first correction was built, smoke-tested, deployed and merged as PR #132.
With the controller corrections installed, live use still recorded a 489 ms
active send gap; a nearby authority query took 215 ms and the observer copy was
218 ms old. Authority snapshots are inexpensive on the server, but network,
scheduling and client-side metadata work can still exceed a clock's budget.

Authority refresh, instance reporting and remote-owner endpoint lookup now use
a separate bounded RPC worker. Capacity-one request/reply channels prevent
backlog; endpoint and coordinator-session generations reject obsolete replies.
Only a successfully refreshed current authority allows preparation. A failed,
exited or expired metadata worker invalidates the old arm and refreshes it;
clock updates continue to validate publisher ownership on the controller.
Authority mutations remain serialized on the control stream, never silently
retried. The newest decoder sample is read after registration/reconciliation
rather than sending the loop's pre-query observer copy.

This does not extrapolate a stale decoder, increase the 250 ms freshness guard,
increase cue deadlines, or convert a failed physical ACK into success.

## Evidence and diagnostics

Read-only raw JSON-RPC sampling, after warm-up, found authority reads below
7 ms and playback reads below 3 ms in 30 samples each while paused. This does
not prove bounded latency while playing, nor rule out occasional contention.
The controller owner reviewed the guard and shared locks in
[PCController #554](https://github.com/atomicdeploy/PCController/issues/554).

`/api/player/status` now includes `hardware_sync.clock_transport`: update and
failure counts, stale-observer skips, last send gap, maximum playing send gap,
last/maximum RPC round trip, observer age, authority query timing, and instance
report timing. Paused one-second heartbeat gaps do not inflate the playing-gap
maximum. The counters are bounded and contain no media paths or private logs;
they do not request repaint for every telemetry update.

## Acceptance

Regression coverage uses an isolated loopback RPC peer to verify one explicit
registration and a stable publisher ID on subsequent clock requests. Pure
diagnostic coverage verifies that idle gaps are excluded and active starvation
and failed updates remain visible. No local Rust compilation or linking is
permitted on the production host.

The isolated-metadata regression holds an authority response behind a fixture
barrier and proves a separate playback stream completes before that barrier is
released. Both clients use explicit fixture identities, without consulting
user configuration or actuating hardware.

First installed Pealayer source: `86e6ae341c4d838676f3aa1b643f311521769c20`.
Its Windows smoke/health/RPC/media restoration passed; native screenshot capture
timed out, so visual acceptance was not claimed. Controller PR #627 introduced
queued UART priority, restore-ACK timing and exact paused-worker arming; the
candidate at `b1d7d4b5f169e7334b097f22484846c4c0d3a5b0` is installed, but the
later stale-clock trace means joint timing acceptance has not yet passed.

The isolated metadata follow-up still needs its exact Windows build/deployment
and sustained playback check. Preserve destination-validated libmpv aliases,
use the product updater, verify actual media and both identities, and inspect
clock/restore counters before calling the recurring fault resolved. No physical
edge-timing acceptance is implied by loopback or read-only diagnostics.
