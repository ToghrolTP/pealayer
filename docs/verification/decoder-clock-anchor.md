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

## Paused admission before Play

The installed clock correction dispatched the first strip cue within 2.2 ms,
but further live verification exposed competing serial transactions. A PWM
readback held the request slot across the cue's ACK deadline. Blocking that
readback alone exposed the internal playback-display writer competing with
resume restoration. Priority cannot preempt a transaction already on the wire.

The current coordinator contract therefore adds `prepare_play` to paused media
updates and `play_ready` to timeline feedback. A pending Play first requests a
reservation while libmpv is genuinely paused. The coordinator arms the matching
revision/epoch, drains the prior transaction and reserves timeline/safety
admission before acknowledging readiness, within the existing 250 ms budget.
Only then may the native observer request unpause. It continues advertising a
paused preparation intent until libmpv actually reports playback; it never
fabricates a playing clock to acquire the reservation.

Ordinary paused arming is distinct from readiness to Play. Nonempty plans need
the explicit matching readiness acknowledgement; empty plans retain their
clear/clock acknowledgement path. Missing current-contract readiness is an
error, with no obsolete-coordinator fallback. Play/cancel intent revisions bind
asynchronous replies to the current local request, so a delayed grant cannot
authorize a later Play. A pending cancellation bypasses the idle heartbeat
interval and revokes an unconsumed or not-yet-observed native start. Paused
preparation/start intents publish at 100 ms, keeping the bounded reservation
alive while the ordinary paused clock remains at its one-second idle interval.
Readiness freshness is measured from request send time, so a reply delayed
beyond 250 ms cannot become fresh merely by arriving. A fresh paused heartbeat
may acquire a new acknowledgement while the media remains paused.

PCController services board-display/program-state presentation only in suitable
cue-free gaps through the timeline owner and reports dense-playback deferral
honestly. This preserves output
deadlines; it does not establish continuous front-panel clock acceptance during
dense effects. Pause cleanup, fault and disconnection release the reservation;
safety commands retain admission. The 50 ms cue and 250 ms freshness guards
remain unchanged. Final synchronized replay is required with both updated peers.

## Hosted test fixture correction

The combined candidate's pull-request quality job exposed an existing download
fixture race: its timed response could complete before the test requested Pause,
invalidating the asserted partial-cache precondition. The first fixture response
now waits at a known partial prefix until Pause/Cancel is requested, then releases
the blocked reader. Resume and progressive range requests remain ungated; all
original byte, validator and partial-cache assertions are retained. Fixture Drop
also releases the gate so a failed assertion cannot strand the response thread.
This changes test synchronization only, not production download behavior.
