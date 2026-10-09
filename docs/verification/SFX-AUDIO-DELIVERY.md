# Sound effects: delivery and acceptance

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Implementation: [PR #104](https://github.com/ToghrolTP/pealayer/pull/104).
User guide: [Sound effects and audio outputs](../SOUND-EFFECTS.md).

## Delivered on 9 October

Native and Web editors share the Rust audio-effect library and commands. Sounds
can be imported from files or HTTP(S), previewed independently of the movie,
edited, deleted and placed as fixed-duration, move-only Audio-effects cues.
The native Effect Controls and Web timeline editor expose audio-specific fields
instead of lighting controls. Files are referenced, not duplicated.

Audio outputs come from actual libmpv discovery. Media defaults to `auto`;
SFX defaults to following media, with a per-sound override available. IDs include
the backend, so separate selectors cannot create conflicting backend/device
combinations. Remote consumers edit and play sounds on the authority computer.

## Build and package evidence

- Installed runtime source: `43b9323fa62b6f9941d554e908d8074492da24a5`,
  with clean embedded identity. Later documentation-only commits do not change
  this package identity.
- Installed EXE SHA-256:
  `d665a054ecc0157e4274b66b496d5568333853a4b4d3d6d2eb3bde73be9225f8`
  (11,234,816 bytes).
- Rust library check, three focused SFX tests, and compilation of affected
  timeline/hardware integration test targets passed. The Web production build
  passed its contract, responsive, timeline-input, messaging and PWA guardrails.
  Native release, seven taskbar icon resources and UPX integrity passed.
- Package smoke passed with the independently retained David and Cafe runtimes.
  Each host's libmpv DLL remained unchanged; no Rust build ran on Cafe.
- David and Cafe updated through the chunked API and graceful restart; fresh
  local updater manifests confirm the installed source and executable hash.
  Erfan received an offline atomic replacement with its own DLL retained and a
  recoverable prior EXE. KMPlayer was left running; no Pealayer GUI was launched.
- Linux, both macOS jobs, repository health and CodeQL passed at the final code
  checkpoint inspection. Windows CI was still running; it is not claimed passed
  or used as a reason to wait on a slow suite.

## Live acceptance on Cafe

A private, quiet four-second generated sound was imported through the shared
API. Its decoded duration and `audio` catalog/lane were authoritative:

1. Preview created an actual WASAPI voice while the paused movie remained at
   660.8 seconds. Explicit Stop removed the voice. Decoder EOF removed the
   preview ID and voice without requiring another user action.
2. A scheduled audio cue played with the movie. At movie time 661 seconds its
   voice position was approximately 0.201 seconds for a 660.8-second cue start.
   Pausing paused the voice; an in-cue seek produced approximately 2 seconds of
   audio offset. Accepted commands were followed by actual state checks rather
   than treated as execution acknowledgements.
3. Import and preview commands sent through David's consumer API reached Cafe.
   A per-sound override to Cafe's discovered Speakers/WASAPI ID was preserved
   and decoded on the authority with the movie still paused.
4. The temporary sound definition, audio cue and generated WAV copies were
   removed. Cafe returned to paused 660.8 seconds, with its original three
   hardware cue IDs unchanged and no remaining audio voices. No seat or relay
   outputs were intentionally actuated. PCController's service and Pealayer
   remained running, responsive and connected.

## Honest remaining acceptance boundaries

- Decoder/backend feedback is not a human listening test. Speaker audibility,
  subjective quality, and physical routing still need user confirmation.
- Static responsive guards are not interactive phone/tablet browser screenshots
  or native dialog visual acceptance.
- Synchronization is software-clock based, not sample-accurate. Device latency,
  decoding and buffering still affect audible onset; see the guide for limits.
- During final deployment verification, David's peer transport was healthy but
  its local video decoder reported a stale position far from Cafe's position.
  This existing remote-preview issue was explicitly handed to the coordinating
  owner in issue #80; it must not be hidden by declaring all peer playback healthy.

The focused feature is delivered; this checkpoint does not close the overall
Web/native parity backlog or certify unrelated remote-video behavior.
