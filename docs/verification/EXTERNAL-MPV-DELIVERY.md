# External mpv: delivery and acceptance

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Implementation: [PR #106](https://github.com/ToghrolTP/pealayer/pull/106).
Setup and limits: [External mpv](../external-mpv.md).

## Delivered on 9 October

External-only, external with muted internal preview, and remote control of an
existing mpv use one persistent, full-duplex JSON IPC adapter. Windows named
pipes and Unix sockets observe actual properties and acknowledge outbound
commands. Native, Web, IPC/RPC and Pealayer peers share the adapter; there is no
separate polling controller or mandatory helper script. Internal remains the
default. Configure external playback on the authority, not its remote consumer.

Hardware timing observes the external clock rather than the muted preview.
Only actual seek/start-file transitions arm a playback-restart clock rebase;
ordinary buffering restart must not clear a hardware fault. Existing prepared
plans, authority, acknowledgement and E-STOP gates remain unchanged. No
PCController or firmware contract changes were needed in this pass.

## Verification

- Four focused Rust tests passed, including a real Windows mpv integration
  using generated lavfi video and null outputs. Playback actually advanced;
  external volume changes reached Pealayer, and Pealayer play/pause reached mpv.
- The remote-only internal decoder remained unloaded. Dual preview was muted
  and aligned within 250 ms. Thirty-two commands were acknowledged in 20.56 ms
  in the fixture. This is a measured fixture result, not a universal guarantee.
- Attach/detach ownership, reconnect with fresh authoritative state, rejection
  of disconnected commands, no stale command replay, and managed process
  launch/cleanup passed. Configuration round-trip, shared preference controls,
  JSON-RPC update parsing and actual-seek-only restart regression passed.
- `cargo check --locked`, the Web production build and its contract,
  responsiveness, timeline-input, messaging, clipboard, asset and offline PWA
  guardrails passed. Release linking, seven Windows task icon resources and UPX
  integrity passed. No Rust compilation ran on Cafe.
- Linux and Apple-silicon CI passed at the final code checkpoint inspection.
  One Intel job passed; another Intel job and Windows jobs remained running.
  They were not waited on or claimed complete.

## Installed package

Code commits: `945207bcb75930eb7c5451c59d1baa5bee7c2c2a`, followed by the clock
safeguard at installed source `9d415535b73e6e3a6a014cf668e08d0098e77541`.
Later documentation commits do not change this package's embedded identity.

Executable SHA-256:
`b91b34d7249e02d652e9c754e80d364f8f140fc53fc5533296f2a7d8ced80031`.
Packed size: 11,314,688 bytes; unpacked size: 45,286,400 bytes.

| Host | Retained libmpv SHA-256 | Installed evidence |
| --- | --- | --- |
| David | `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65` | Graceful updater; live local manifest and process API confirm final package. Cafe consumer connected, no local hardware scheduler. |
| Cafe | `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f` | Graceful updater; fresh local manifest and process API confirm final package, responsive GUI and connected board. |
| Erfan | `0a81c004aae0ee7d512b9a26e38f66281f9591e84e1663215cc3a36a4bde6f6a` | Stopped-app atomic replacement; retained-runtime smoke exit 0, final file hash verified, no GUI launched. |

Cafe stayed paused at 660.8 seconds with its original three cues preserved,
hardware and sync errors null, Cafe authority retained and E-STOP unchanged.
David's consumer reported the same final server commit and position. All hosts
retained their own DLLs. Erfan's temporary update/previous-build files were
removed after smoke verification; its pre-feature rollback remains available.

The first Cafe installation of the earlier feature build rolled back. No fresh
panic/WER evidence established the cause; it is not attributed to mpv. One
controlled retry succeeded, and the subsequent final safeguard build also
succeeded. Startup activity settled and the API/window remained responsive.

## Remaining acceptance boundaries

The isolated full-app API fixture launch was blocked by execution policy and
was not run. Library integration, live production APIs and package smoke are
separate evidence, not substitutes for that test. Production settings were not
switched to external mode and no physical cue outputs were actuated this pass.
Human native/Web interaction, audible output, external-driven physical timing,
and Linux/macOS socket runtime acceptance remain explicit next checks.

Dual decoders are drift-corrected, not frame-locked. External/remote-only modes
do not provide decoded taskbar/Web fallback frames. The executable still
initializes embedded libmpv; external-only leaves its decoder idle rather than
removing that dependency. See the setup guide for source bytes and peer paths.
