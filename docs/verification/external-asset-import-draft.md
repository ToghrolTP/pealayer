# External asset imports — preserved work in progress

This branch is a checkpoint, not a production/deployment candidate. The user
prioritized the Hardware Monitor disconnected-state regression before the
remaining import verification. No regular application was replaced by this work.

## Implemented source

- One off-thread native/API import path; browser/remote files use bounded ordered
  uploads to the authority rather than sending client-local paths as server paths.
- Audio uses the existing SFX probe, library and playback engine. Timeline drops
  retain integer-millisecond placement. Import failures carry the audio identity.
- Subtitles reuse mpv's track-loading path; movie files reuse existing playback.
- Reviewed timeline fragments append with remapped identities without replacing
  existing cues, media, viewport or playback position.
- Reviewed scenes become set-and-keep relay/PWM cues, not live scene commands.
  Unknown/motion/unsupported mappings fail closed and board identity is checked.
- Effect definitions open the existing designer and use its Save/Publish path;
  PCController continues owning hardware programs. Web has an import button too.
- Import queues/reviews participate in native authoring/draft inspection.

## Existing owner contract

`src/projects.rs` was recovered from the preserved project-persistence draft
PR #227 (`feat/project-scene-persistence`). Its data types, validation, path
resolution and tests are reused rather than defining another scene format.
The RPC adapter and scene capture (which depended on draft-only telemetry fields)
are deliberately not brought over. This does not finish project restore, scene
actuation, workspace restoration or the outstanding acceptance work in that PR.
Reconcile this shared model when the owning draft continues; do not merge a
second independent format or replace its pending application adapter blindly.

## Verification and remaining gates

- Initial Rust library check and three pure asset-model tests passed.
- Browser ordered-transfer/failure/cancellation checks and TypeScript passed.
- The focused upload test exposed a Windows read-only flush-handle error; the
  finalizer now opens a writable handle. Rerun that regression before acceptance.
- Rerun the native suite after subsequent authoring/lock/error-routing edits.
- Complete rendered native drop/hover/geometry tests, browser phone/tablet/desktop
  interaction tests, effect-import round trips, source-field drops, scene partial
  rejection checks, clean packaging and destination-runtime smoke verification.
- Check media+subtitle asynchronous load ordering and pin authority identity over
  an in-flight native upload. Verify incomplete/cancelled-batch cleanup behavior.
- Improve review presentation and localization; ensure previews display the exact
  imported channels and programs before approval. No physical output test occurred.
- Deploy through the approved graceful updater only after these gates pass.

The connected production application, its private configuration and hardware
outputs were not used as fixtures. No successful Cafe deployment is claimed.
