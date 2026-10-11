# Bounded seat-cue timing acceptance

Owner: [Pealayer #80](https://github.com/ToghrolTP/pealayer/issues/80), coordinated
with [PCController #554](https://github.com/atomicdeploy/PCController/issues/554).

## Latest bounded receipt · 2026-10-11

After renewed explicit human readiness, one existing Seat Left Down cue was
exercised on the installed, media-loaded Pealayer runtime. No project editing,
broader motion authorization or later relay/lighting execution was involved.

- PCController #632 merged as `c26cc561d2bf353a948312f89f59d90dbda8e38d`, all
  required gates successful. Windows build 38095464084 tested final source
  `01cb5ecb0f9a68179546ba51e9c88786d3709b90`, verified as merged ancestry.
- Product updater installed CI `0.0.0-ci.1459`; live version and source hash
  `f69e5c467072921f8dd848c51e053ec4504be26c2d5f2fe0258033f7d93ef412` matched.
  Installed executable SHA-256:
  `3fa0cb311e6d16106c2ebcb82dd99b4d25c9d3c7990322afa2b637ff9aac878c`.
  Matching public sidecars updated; no board flash/reset.
- Pealayer remained `f414b89f06c9fc5a054ab7110e85606826baeea0`, unchanged
  destination-validated libmpv; no local Rust compilation or alternate smoke
  launch. The diagnostics candidate `316d6681f66fb66636943b7cd5e3df382c10b9bb`
  is staged, NOT deployed: the command tool rejected smoke-launch-containing
  calls before execution with `CreateProcess ... rejected: blocked by policy`.
  No rationale/rule ID was supplied; this is not an observed application crash.
  Do not bypass that rejection using another runner or deployment mechanism.
- Requested pre-cue seek 154 ms decoded to 160 ms. Four placed instances and
  zero analog tracks were verified. Only the cue starting at 404 ms was crossed:
  duration 500 ms, Seat Left Down, STOP at 904 ms. Later persistent relay cues
  were outside the bounded interval. Prepared revision 186, epoch 8 and board
  generation 1 matched before Play; controller ownership and board ACK were valid.
- **Protocol-level test passed:** playback advanced, controller remained Playing
  without a fault, and both `motion-0/0` (Down) and `motion-1/0` (STOP) were ACKed.

| Step | Due (ms) | Dispatch lateness (ms) | ACK RTT (ms) | ACK lateness (ms) |
| --- | ---: | ---: | ---: | ---: |
| Seat Left Down | 404 | 1.4586 | 16.5170 | 17.9756 |
| Seat Left STOP | 904 | 0.9978 | 3.9949 | 4.9927 |

- Playing-interval clock feedback maximum was 94.6428 ms; largest forward
  position correction was 35.6337 ms. Plan-scoped worker/clock-read maxima
  194.4791/106.0208 ms already existed while paused before Play: they are not
  measurements of this playing interval. Final acknowledged step count was two.
- Pause was requested in `finally` after the bounded samples (last sample wall
  1314 ms, player position 1240 ms). Final position 1360 ms was paused; both
  sides applied Stop, all relays zero, E-stop inactive and reset count unchanged.
- Full current native-window capture was inspected and retained privately;
  interactive window responds, real media remains loaded and controller connected.
- **Physical movement/STOP confirmation is pending.** Board ACKs and the GUI do
  not prove mechanical movement, physical output-edge timing, sustained operation
  or successful motion with an open enclosure. The enclosure was reported closed.
  Existing motion-door policy was already Always; no safety policy/interlock changed.

## Earlier failed receipt · CI1456

The preceding test used PCController #631 merged at
`d3c1aa43b182e712e185fdf634eefe464e0b3276`, Windows build 38091556515 from
`fe1cfefb49218f3aa98a203bab6a8243c84e43e1`, CI `0.0.0-ci.1456`, with the same
installed Pealayer. Prepared revision 178, epoch 5, generation 1 and 814 steps
were armed paused at 160 ms. Playback reached 440 ms locally before a fault.

**That test failed closed before the first motion write:** sequence 1274 at
458 ms, due 404 ms, dispatch lateness 54.2465 ms, acknowledged zero. Error:
`cue motion-0/0 missed its dispatch deadline by 54.2 ms; not executed`.
Pause left both sides Stop/all relays zero and reset count unchanged. A later
Pause/reprepare clearing fault presentation did not turn that failure into success.

## Remaining acceptance and continuation

Obtain the human's observation of the latest movement and STOP without silently
converting ACKs into physical acceptance. Keep runtime paused; do not repeat or
expand motion without renewed scope/readiness. Sustained timing/load checks and
open-enclosure physical acceptance remain separate gates. Preserve the unchanged
50 ms dispatch/ACK and 250 ms clock-freshness guards.

Resolve the smoke-validation tool denial through an authorized path before
installing the staged Pealayer diagnostics build. `/api/player/status` ACK age
is a cached UI snapshot, not a reliable live preflight. The new
`/api/process/status.hardware_sync` source reads executor diagnostics directly
without new polling/repaints, but that API is not installed on this runtime.
Use current controller process-local diagnostics and the actual player's
fresh-ACK preparation guard; never compensate by weakening its freshness budget.
