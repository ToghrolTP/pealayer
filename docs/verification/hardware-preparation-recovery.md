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
Cafe source alignment and Cafe runtime rollout remain separate; the existing
Cafe GUI smoke-launch denial is not bypassed.
