# NLE volume and responsive monitor transport

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80), with Web
parity tracked in [issue #63](https://github.com/ToghrolTP/pealayer/issues/63).

## Cause and correction

The native Program Monitor placed every transport action, volume, media-track
selector, timecode and seekbar in one unwrapped row. The video reserved only
35 pixels for that footer. Also, `add_sized` did not override egui's slider-track
width. A control existing in source was not proof of a reachable deployed slider.

- Native NLE now reserves two complete footer rows on wide monitor panels and
  three below 600 logical pixels. Seeking has its own row; narrow panels get a
  full-width horizontal volume strip with mute and a stable percentage column.
- Native volume and mute use the existing session command engine, including
  remote-consumer forwarding. Changing the slider no longer modifies only the
  consumer's private decoder. Wheel adjustment consumes the slider's wheel event.
- Web Simple and NLE share a single accessible VolumeControl. NLE has a compact
  right-aligned strip on wide monitors and a full-width row below 620 pixels.
  Unknown volume or unloaded media disables control instead of inventing state.
- Muting preserves the displayed stored level; its button shows the muted state.
  The Web monitor explicitly reserves the media-selector row as well.

## Acceptance gates

Build/type-check and destination runtime smoke are distinct from live playback
and UI acceptance. The user deferred test suites and slow CI waits; added parity
guards are retained for the next normal suite run, not represented as executed.
Record actual deployment hashes, live authority/consumer volume round-trip and
phone/tablet/desktop layout checks in the linked issue checkpoint.

Erfan-Gaming must not launch Pealayer while KMPlayer is running. Installation-only
is not playback acceptance. Cafe's libmpv must remain Cafe's own validated runtime.

## Installed checkpoint, 9 October

- [PR #96](https://github.com/ToghrolTP/pealayer/pull/96) runtime source:
  `735a7c22d01c137a8fee065474f5c33283a31c81`, clean embedded identity.
- Release build passed in 1 minute 26 seconds, with 14 existing warnings;
  TypeScript and Vite/PWA build passed. Native icon resources, UPX integrity,
  David-runtime smoke and retained Cafe-runtime smoke passed. Test suites and
  slow CI waits remained deferred.
- Installed executable SHA-256:
  `582457e3632df4da61670440195e5bd1756975663c15062e5d2f7fd8867da98d`
  (11,128,832 bytes), verified on David, Cafe and Erfan.
- David and Cafe updated through the existing chunked upload API and graceful
  restart. Cafe remains connected to hardware with no hardware error; David
  remains its cache-only consumer with no local hardware scheduler or transport
  error. A command through David changed both reported volumes to 17%; muting
  then showed 17% plus muted=true on both. Original 0% and unmuted were restored.
  Media remained paused and no outputs were actuated for this check.
- A full-resolution, DPI-aware Win32 capture inspected the running David NLE
  monitor at roughly 414 logical pixels wide: mute, horizontal slider and
  percentage are visible in the third footer row. The capture remains private;
  do not publish user media/artwork. PrintWindow does not prove GPU video rendering.
- Cafe reported zero new Pealayer Application Error/WER events since restart.
  Erfan's destination DLL remained unchanged; its prior executable is retained
  as the verified rollback. KMPlayer remains running and Pealayer was not launched.
- Web's deployed PWA identity is `eeda9cb8f7aa49ea`; phone/tablet browser geometry
  and direct dragging remain human-acceptance checks, not completed test claims.
