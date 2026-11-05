# External asset imports — implementation and verification

The resumed feature includes the Hardware Monitor disconnected-state fix, rather
than losing that intervening work. Source checks and real-browser verification
are complete; packaging/runtime and deployment receipts are recorded separately
below. Cafe deployment is deferred at the user's request until it returns.

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

- Full native library suite: 1,098 passed, one ignored. Final focused asset tests
  additionally cover rendered review/values, typed RPC and exact panned geometry.
- Browser ordered transfer, failure, command rejection and cancellation checks,
  TypeScript and complete Web prebuild checks passed.
- The Windows flush regression is fixed and its actual upload test passes.
- A real installed-browser fixture exercises the actual drop/source/review
  controls at 390/768/1440 widths in both themes. No overflow, automatic commands,
  duplicate source import or page exceptions occurred. These are real controls
  with isolated memory-only transport, not physical-device proof.
- Batch cancellation removes registered partial and completed unretained files.
  Inputs are retained before final command submission: an acknowledgement lost
  after queuing must not delete files the importer may already be using.
- Transfer owner receipts pin the immutable destination. Reordered chunks,
  path traversal, reserved device names, overruns and incomplete finalization
  fail closed. Native consumers transfer files rather than forwarding local paths.
- Movie/subtitle drops wait for FileLoaded for the requested media. An unrelated
  movie cannot receive the pending subtitle, and failed loads expire the queue.
- The first real runtime probe found that no-media mpv reports unpaused. Offline
  placement now checks for actual loaded media before requiring Pause. The
  previous candidate saved its SFX but rejected placement; it is not accepted as
  the final runtime build.
- The next real probe confirmed SFX placement and timeline append, then reached
  the unsaved-media guard. Mixed subtitles now remain queued for their requested
  movie while Save/Discard/Cancel is pending; Cancel removes them without touching
  cues or playhead. Native, consumer, Web and RPC choices reuse one resolver.
  The dialog and its transient state are not restored from browser storage.
- Current PCController EffectDescriptor/effects.json exports adapt into the
  existing typed designer payload, retaining steps, engine, repeats and properties.
  The RPC name validator now allows the same printable punctuation as native
  Save (including slashes in real catalog names), while retaining length/control
  character bounds. Full group-style/empty-group catalog replacement remains
  PCController's existing import workflow, not a silent side effect of a drop.
- Scene review displays real sampled/unsampled values. Missing/incompatible
  channels or board identity reject the whole scene without partial placement.
  Imported motion is not silently converted into output. Cached channel catalogs
  support offline authoring; unknown mappings require a valid catalog first.
- Existing curves keep their channel policies; colliding times and locked
  channels reject the transaction. External effect definitions open as new
  identities and the Save path refuses to overwrite an occupied controller ID.

## Explicit boundaries

The original project-persistence draft remains separate. This importer does not
restore projects, run scenes or publish/execute imported hardware programs.
Saved/published effect cards use the existing timeline drag path after review;
opening a JSON definition is not automatically executing or publishing it.
Relative audio dependencies must accompany browser/consumer uploads in the same
batch. A process crash can leave unretained upload files on disk; ordinary cancel
and failure cleanup is verified, not cross-crash disk garbage collection.
No human listening, physical board output or production CPU soak is claimed.

## Packaging/runtime checkpoint

Pending the clean, source-identified package and isolated live API/decoder pass.
No regular application was replaced during source/browser testing.

The connected production application, its private configuration and hardware
outputs were not used as fixtures. No successful Cafe deployment is claimed.
