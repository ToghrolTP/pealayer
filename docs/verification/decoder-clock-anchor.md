# Decoder clock continuity at hardware cues

Owner: [Pealayer #80](https://github.com/ToghrolTP/pealayer/issues/80).
Related: [hardware preparation](hardware-preparation-recovery.md) and
[seat cue timing](seat-cue-timing.md).

## Reproduced failure

A bounded replay of the operator's existing timeline stopped at the lighting cue
near 113.1 seconds. The first strip configure operation was rejected before
execution: dispatch was 60.0 ms late, with a +110.1 ms incoming clock correction.
The executor's worker gap was 2.0 ms and its clock/board reads were negligible.
This reproduced a media-clock failure before strip-frame transmission; pixel
count and serial-frame throughput are separate acceptance checks.

The native media observer polls the decoder every 20 ms. It previously reset the
position's monotonic anchor on every successful read, including repeated equal
decoder positions. Those repeated reads repeatedly moved the projected clock
backwards. When the decoder finally published its next position, the controller
received a forward jump large enough to miss the unchanged 50 ms cue deadline.

## Change and regression boundary

Equal positions during uninterrupted playback retain their original monotonic
anchor. Successful observations still refresh observation freshness. Changed
decoder positions, pauses, buffering, seeks, rate changes and resume establish
the appropriate new anchor. No persisted field or transport contract changes.

Deterministic tests exercise 160 ms decoder updates across a 113100 ms cue,
pause/seek/rate/resume transitions, and the existing 250 ms stall discrepancy.
The observer's discontinuity checks, 50 ms execution deadline, 250 ms freshness
guard, authority and explicit re-arm requirements remain unchanged.

The reproduction left the player paused and both seats reporting Stop. This
does not establish physical output acceptance. Native compilation and tests run
on hosted CI, not the production host. Delivery remains pending the exact-source
Windows package, draft-safe peer update with the destination-validated libmpv,
real-media replay across the cue, and separate human LED/motion observation.
Private runtime snapshots and media references stay outside Git.

## Hosted test fixture correction

The combined candidate's pull-request quality job exposed an existing download
fixture race: its timed response could complete before the test requested Pause,
invalidating the asserted partial-cache precondition. The first fixture response
now waits at a known partial prefix until Pause/Cancel is requested, then releases
the blocked reader. Resume and progressive range requests remain ungated; all
original byte, validator and partial-cache assertions are retained. Fixture Drop
also releases the gate so a failed assertion cannot strand the response thread.
This changes test synchronization only, not production download behavior.
