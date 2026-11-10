# Hardware preparation and recovery

Owner: [Windows readiness #30](https://github.com/ToghrolTP/pealayer/issues/30).
Related physical acceptance: [seat cue timing](seat-cue-timing.md).

## Corrected transitions

- An acknowledged empty plan clears prior hardware outputs but has no executor
  to arm. A pending explicit Play is consumed only after matching plan/clock
  revisions, media epoch, a confirmed zero-step ready plan and a fresh ACK.
  Unloaded, buffering, ended or stale media stays paused.
- A clock ACK for an older plan cannot erase a newer preparation/compilation
  failure or cancel the explicit request to prepare a replacement executor.
- Before preparation, query the coordinator's clock. If our old clock is still
  playing, acknowledge its pause first without changing its position, epoch,
  plan or loaded ownership. A failed old-executor ACK requires exact paused
  read-back; an unconfirmed failure cannot authorize replacement. Never pause
  another publisher's clock.
- Recovery clears only the observer's own stale connection diagnostic, not an
  independent board disconnect error.
- Native/Web status distinguishes preparation blocked, invalid plan, authority
  required, resource waiting, preparing, arming and real timing faults. The
  actual error remains available; captions do not suppress the fault.

## Safety and verification

The 50 ms dispatch/ACK deadline and 250 ms fresh-clock guard are unchanged.
Timing faults remain latched until explicit Play requests a new revision and
successful prepare/arm acknowledgements. No automatic reconnect actuation,
late-cue skipping, fabricated board state or relaxed timing budget was added.

Deterministic native coverage includes empty-clear completion, wrong revisions,
wrong epochs, missing/nonzero step counts, buffering/EOF/unloaded/stale ACKs,
old-plan error preservation, explicit retry, genuine fault latching and
diagnostic ownership. A loopback TCP coordinator fixture exercises the actual
JSON-RPC client and enforces pause-before-prepare ordering; no physical outputs
are involved. Web validation covers TypeScript, parity and installable PWA.

The reported screenshot's original fault was not present in the first live
sample: local Pealayer was paused, connected, prepared at revision 46 with zero
steps, no error and a fresh decoder observation. That does not disprove an
intermittent physical fault. Fixtures and paused runtime checks do not establish
physical movement, sustained deadline acceptance or production readiness.
The initial Cafe GUI smoke-launch denial was not bypassed. The later
[paired deployment receipt](paired-peer-deployment.md) supersedes that historical
rollout blocker; installation still does not establish physical timing acceptance.

## Live recheck, 13 October 2026

Read-only process-local API checks verified Cafe source
`d42cfeb8d08d1c662aea9683f27cca326bdd9b39`, clean, player digest
`26bbc3fc5ffb9af1569f229bc8ff3008af466b89ed8da7d1f0d676f220773630`.
The preparation recovery merge `62773b13` is an ancestor, and the installed
source's `media_timeline.rs` and `media_sync.rs` match the current source. Thus
the known preparation/retry fixes are installed, not merely waiting in a PR.

Three initial paused samples reported a connected board, no latched timing
fault, and `Hardware waiting`: standalone strip streaming owned the conflicting
resource. A subsequent sample reported `Hardware preparation blocked` with a
coordinator pause/read timeout. This is an actionable live blocker, not evidence
that all physical playback faults are resolved.

After the user authorized stopping that strip stream only, one bounded stop RPC
timed out. Its outcome is unknown; it was not replayed. A later inventory found
no controller process or listener on Cafe port 8787; both IPv4/IPv6 loopback
health probes failed. This does not establish why the controller exited.
Pealayer remained paused; no Play, cue, motion, relay or firmware/reset command
was issued. The user separately authorized restarting PCController only. Before
starting the existing `PCControllerVerifiedRecovery` interactive launcher, the
installed executable digest matched the verified host package, no controller
process/listener existed, and Pealayer had neither live nor pending Play intent.

After restart, controller PID 76904 served a healthy IPC endpoint. Read-back
reported strip `running=false`, `id=0`. Three samples at 01:39:04--01:39:09 UTC
reported connected hardware, `Hardware timeline`, desired/prepared/clock ACK
revision **67**, epoch **26**, ready zero-step timeline, no error/deferred reason,
no retry latch and no pending Play. Clock updates advanced 6404 to 6408; failures
remained at the historical count 16, with current round trips 4--10 ms. Idle ACK
age may exceed 250 ms because paused clocks publish once per second; explicit
Play still requires its own fresh matching ACK before resume.

The current live preparation blocker is resolved, and the earlier fixes are
installed. This is paused empty-plan recovery, not sustained output scheduling,
physical cue execution or production timing acceptance. Pealayer was not
restarted/replaced and no authored timeline was overwritten. Genuine timing
faults must remain visible and fail-closed; future standalone streams can still
legitimately block conflicting media preparation.

The [later paired update](issue80-native-delivery.md) installed the newer player
and PCController sender through their owned updaters. After the coordinator
restart, Cafe remained paused and connected at matching desired/prepared/clock
ACK **72**, without an error or retry latch. This is fresh paused recovery
evidence, not permission to actuate outputs or physical deadline acceptance.
