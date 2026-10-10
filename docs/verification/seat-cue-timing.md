# Bounded seat-cue timing acceptance

Owner: [Pealayer #80](https://github.com/ToghrolTP/pealayer/issues/80), coordinated
with [PCController #554](https://github.com/atomicdeploy/PCController/issues/554).

## Live receipt · 2026-10-11

The user explicitly authorized an existing Seat Left Down cue and confirmed
physical readiness. They clarified that the door is the control-panel enclosure,
not a guard around the seat movement area. Runtime host configuration reported
`safety.motion_door_policy="always"`; persisted board settings flags were zero,
also Always. No policy, firmware, emergency-stop or interlock was disabled.

- PCController #631 is merged at `d3c1aa43b182e712e185fdf634eefe464e0b3276`.
  Successful Windows build 38091556515 is head
  `fe1cfefb49218f3aa98a203bab6a8243c84e43e1`, verified as merged ancestry.
- Installed through the product updater: CI `0.0.0-ci.1456`, source SHA-256
  `8f3133d090d55a90606e8d2aea810baad0e44a0133d238c443006cc1d265a960`;
  executable `dab88e9cf904215487d5c8b4cbdd642660f91de4bfc0cf534ff45157af46b8b2`.
  Update operation `op-a75f83ee79779a7a` completed; interactive window responds.
- Pealayer remains source `f414b89f06c9fc5a054ab7110e85606826baeea0` with its
  unchanged destination-validated libmpv runtime. No local Rust compilation.
- Authority recovered; the existing early cue starts at 404 ms, holds Down for
  500 ms and ends with STOP at 904 ms. The requested 154 ms pre-cue seek decoded
  to 160 ms. Four placed effects and no analog tracks were verified; later
  relay/lighting cues were not crossed and the user's project was not edited.
- Current plan revision 178, epoch 5, generation 1, 814 steps was armed paused.
  Explicit Play began; local observation advanced from 160 to 280 then 440 ms.
- **Acceptance failed** before the first motion write: controller clock sequence
  1274 at 458 ms, due 404 ms, `motion-0/0`, dispatch lateness 54.2465 ms,
  acknowledged 0, ACK round-trip 0. Exact error: `cue motion-0/0 missed its
  dispatch deadline by 54.2 ms; not executed`.
- The fault was captured before final Pause. Afterwards playback was paused,
  both sides applied Stop, all relays zero, and board reset count unchanged.
  No seat movement occurred; there is no physical-motion acceptance.
- Full current native-window captures were inspected and retained privately.
  A post-Pause reprepare can clear the displayed fault; it does not supersede
  the captured failed-test receipt or establish recovery.

## Remaining action

The controller owner is separating executor starvation from an incoming clock
jump. Keep dispatch/ACK/clock safety budgets unchanged; a guard increase alone
is not a fix. Do not repeat motion until a reviewed source fix, matching merged
Windows package, valid live ownership/arm and renewed announced bounded test.
Later persistent relay cues remain outside the authorized test window.

An external preflight also exposed a cached diagnostic: `/api/player/status`
ACK age stayed nearly constant across heartbeat updates because it is a UI
snapshot. New `/api/process/status.hardware_sync` reads the executor directly
without extra UI repaints. Its source is added here but installed acceptance
still requires a successful matching CI build and deployment. Physical motion
and stop must then be separately confirmed by the human observer.
