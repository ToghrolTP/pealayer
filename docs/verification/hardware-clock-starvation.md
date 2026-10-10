# Prepared playback clock starvation

The recurring Hardware timing fault reported on 2026-10-11 was a controller
`video clock feedback expired` fault, with an authenticated board and no
dispatched cue in the faulted plan. The earlier explicit-Play re-arming fix was
already installed. This report therefore concerns clock delivery, not a failed
board acknowledgement or the old Play-button recovery defect.

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

Deployment and sustained playback acceptance are pending an exact successful
Windows build. Preserve the destination-validated libmpv aliases, deploy using
the product updater, verify smoke/startup and actual playback, and inspect these
clock counters before claiming the recurring fault has been resolved. No
physical cue acceptance is implied by loopback or read-only diagnostics.
