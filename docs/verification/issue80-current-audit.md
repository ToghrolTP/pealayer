# Issue 80 current acceptance audit

13 October 2026. Owner: [#80](https://github.com/ToghrolTP/pealayer/issues/80).
Source reviewed at `02a08430902031e13cb504bff1334de1c8e37b77`.
This fresh audit supersedes dated runtime observations, not unfulfilled gates.

## Current delivery

Cafe's live Pealayer is still clean source
`9c9845c91908e83c027c6cddc5e1bb0a49d116f4`, PID 43416/session 4. It is paused,
controller/hardware connected, no hardware/sync error, with matching desired,
prepared and clock ACK revision 88. No update is active. Its latest native
timeout-restoration patch is merged in [#176](https://github.com/ToghrolTP/pealayer/pull/176)
and compiled/smoke-verified in canonical retained-runtime staging, but **not
installed**. Current draft-state confirmation is outstanding. See
[exact package and runtime evidence](controller-response-deadlines.md).

Cafe's PCController timeout fix is already installed through the owned updater;
[paired receipt](https://github.com/atomicdeploy/PCController/blob/fix/serial-response-deadlines/docs/verification/serial-response-deadlines.md)
records its hash, six live TCP readbacks, preserved config and no physical output
actions. Its [#659](https://github.com/atomicdeploy/PCController/pull/659) remains
OPEN/BLOCKED with no status checks and normal auto-merge enabled. Tests/CI remain
skipped under the user's instruction; no protection/security override was used.

## Reproduced CPU gate failure

Two separate five-second process-CPU samples, several minutes apart, used
TotalProcessorTime deltas divided by elapsed monotonic time. Percentages are of
**one core**, not whole-machine utilization.

| Process | Sample 1 | Sample 2 | Working set 1 / 2 |
| --- | ---: | ---: | ---: |
| Pealayer PID 43416 | 89.0% | 85.4% | 1753.4 / 1858.9 MiB |
| PCController PID 107324 | 26.8% | 26.5% | 29.6 / 30.5 MiB |

Playback stayed paused; no recording, audio import/voices, seek or operation was
reported active. The API reported UI cadence 40.33 and later 52.41 fps. The
front-panel snapshot was unchanged since 07:39:11 local; RGB state was steady
and program mode Idle. These observations reproduce high paused CPU and do not
establish its owning call stack or prove no background work exists. Do not call
[#62](https://github.com/ToghrolTP/pealayer/issues/62) fixed, infer a timer-only
cause, or weaken live segment/RGB/event responsiveness to hide this measurement.
Playing/taskbar/GPU/soak checks were not performed.

## Remaining implementation and acceptance

- [#114](https://github.com/ToghrolTP/pealayer/issues/114) still lacks explicit
  placement-level play-once/repeat-window/repeat-N execution modes and their
  scoped exits. Current source has finite source/step repeat counts, not that
  placement contract. Repeat ghosts do not supersede this requirement.
- [#63](https://github.com/ToghrolTP/pealayer/issues/63) still has real feature
  gaps: negotiated video fallback, complete inspector/dialog/timeline/recording
  parity and responsive/keyboard/touch acceptance. Existing parity tests are
  source evidence, not completion of the current product ledger.
- [#64–#66](https://github.com/ToghrolTP/pealayer/issues/64) retain dedicated native
  compositor, adapter/decoder and exact adjacent-frame matrices. The experimental
  DirectComposition branch must not be promoted by an issue-close request alone.
- SFX listening/routed output/hotplug, physical edge/frame timing, RF/strip and
  recording acceptance require an agreed safe physical scope and observation.
  Historical relay/seat receipts are not new authority to actuate outputs.
- Native saved-draft readiness, local/Erfan activation restrictions, current
  privacy-safe UI captures and human walkthrough remain separate delivery gates.
- Applicable release/platform checks and milestone publication remain unpassed.
  Keeping CI/tests skipped cannot be represented as passed required checks.

All eight acceptance areas on #80 remain open. No firmware flash, reset, media
Play, relay/seat/PWM/RF/tone action, deployment restart, screenshot reconstruction
or new test/CI run was issued during this audit. Useful source is already on
GitHub; no code/worktree was reset and no new worktree was created.

## Next actions

Obtain current saved-draft readiness, then use the approved peer updater and
verify exact running source plus restored private draft/workspace state. Diagnose
the observed paused CPU with bounded source/runtime evidence. Resolve the
implementation rows in their owning issues, resume required checks only if the
user changes the skip instruction, and agree a supervised physical/audio/UI
acceptance pass. Keep #80 open until its stated scope is actually demonstrated.
