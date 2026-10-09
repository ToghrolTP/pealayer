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
