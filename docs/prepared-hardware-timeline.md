# Prepared hardware timeline

## Why effect starts were late

The previous engine waited for a UI-driven elapsed-time update before asking
PCController to start each effect. Compilation/upload then happened at the cue
boundary. The cue index advanced even when starting the effect failed.

Coordinator connections now prepare the complete timeline while media is paused.
PCController resolves its own effect definitions and freezes compiled commands
and strip frames in a bounded RAM plan. No effect library is moved into Pealayer.
The direct-serial/virtual-console path is preserved, but does not provide this
coordinator timing contract.

## Playback contract

1. Add effects/cues normally. Pealayer submits their stable references and offsets,
   relay edges, and sampled PWM curves to `controller.media.timeline.prepare`.
2. PCController acknowledges the revision, SHA-256 plan hash, board generation,
   and compiled command count. Invalid/oversized plans are rejected, not truncated.
   A temporary output-ownership conflict is returned as a structured,
   retryable `resource_busy` response. Pealayer shows this as **Hardware waiting**
   and retries at the coordinator's bounded interval; it is not reported as a
   timing fault. This keeps standalone strip previews and other intentional
   output sessions from creating a false alarm or a tight RPC retry loop.
3. An independent libmpv observer samples actual playback every 20 ms; the
   telemetry thread sends clock updates about every 40 ms while hardware is armed.
   Repainting, Web UI connection, and GPU callbacks do not own this clock.
   Authority checks compare the engine's already-established client identity;
   they must not load/parse/validate configuration while holding the timeline
   mutex in the sampling or publishing path.
4. Playback waits for both plan preparation and a matching paused-clock/epoch
   arming acknowledgement. PCController schedules commands itself, rather than
   receiving a new play RPC at every cue boundary.
5. Each native command is acknowledged before advancing its ledger. An expired
   clock, changed board session, NACK, or deadline violation faults the plan.
   Pealayer pauses and shows the error. A possibly applied command with a missing
   ACK still causes an output-off attempt.

Pause releases motion/relay/PWM/strip outputs. Resume reconstructs latched values;
historical RF, buzzer, menu and display one-shots are not replayed. A deliberate
seek re-arms a new epoch/revision. Earlier steps are explicitly counted as
`rebased_steps`, rather than being reported as acknowledged. To retry a faulted
cue, seek before its start and prepare again; continuing after the failed cue is
not automatic recovery.

The egui status bar and Web Effects Library expose preparation/timing faults.
Semantic preparation, authority, execution-ledger and fault transitions wake the
existing UI/Web state publisher; ordinary clock echoes and ACK-age changes do
not force idle repaint loops. The wake callback runs outside timeline locks.
`/api/player/status.hardware_sync` exposes plan and clock acknowledgements,
their age, any deferred resource reason, and the PCController ledger. PCController also publishes
`media.timeline` state events and includes the ledger in its playback snapshot.

## Timing limits — not a hard-real-time claim

The current admission limit is 50 ms ACK lateness (RPC plans permit 5–250 ms),
with a 250 ms clock-feedback lease. Lateness is relative to the received,
monotonically extrapolated media clock. It is **not** proof of actual video-frame
presentation or the physical output edge. Network transit and decoder clock
granularity remain uncertainty; a late ACK can arrive after hardware acted.

This version prepares commands in PCController RAM and uses the host executor.
It does not yet implement an absolute video-epoch start/seek/rate/lease contract
inside the firmware's resident timed queue. Standalone MCU effects remain intact.
Arbitrary OS stalls cannot guarantee perfect timing; the implemented guarantee is
explicit faulting instead of silently discarding pending commands. Physical-board
timing and low-end-laptop load acceptance remain required before claiming the
user's complete end-to-end timing requirement.

## Verification

Focused Go wire tests cover offsets/capacity, initial arming, ordered relay edges,
late rejection, failed-ACK cleanup, clock lease, pause/resume and seek re-arming.
Focused Rust tests cover content-driven revisions and the dual ACK playback gate.
Web UI production/PWA build checks pass. These are not physical timing measurements.
