# Current Pealayer acceptance and next steps

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Web counterpart: [issue #63](https://github.com/ToghrolTP/pealayer/issues/63)
and [product ledger](../WEB-APPLICATION-PARITY.md).

This is the current index. Older handoffs and installed-build receipts remain
historical evidence; their old open-PR, missing-feature or disconnected-runtime
statements are not the present product state.

## Delivered source and deployment

- PRs #77 and #79 and CPU fixes #78/#81/#82 are merged. Subsequent feature and
  recovery PRs through #108 are integrated; none should be fetched as a stale
  replacement for current main.
- SFX loading, scheduled playback and audio routing are implemented, not merely
  a preference scaffold: [SFX acceptance](SFX-AUDIO-DELIVERY.md).
- Clipboard inspection/source browsing and event-driven external mpv modes are
  implemented: [browsing](../remote-folders.md) and
  [external player acceptance](EXTERNAL-MPV-DELIVERY.md).
- Preferences, live branding, responsive volume, elapsed entry, buzzer catalog,
  Effect Controls and palette-safe widgets have focused delivery records.
  [Latest widget delivery](PALETTE-SAFE-ELEGANCE.md) links the current installed
  package; exact source/runtime identities belong there, not in product summaries.
- The production controller startup service exists and is running. A short
  read-only paused sample found responsive applications, fresh media observation,
  connected hardware and no synchronization error. This is not a CPU soak or
  physical-output timing acceptance test.

## Remaining gates · do not silently mark complete

- [ ] Independently exercise elapsed-entry cancel/blur/Enter, Browse, toolbar
  drag/hide, Add cue backdrop and native Appearance/Audio/Subtitle visuals.
- [ ] Verify phone/tablet/desktop Web gestures, dialogs and interactions against
  the corresponding native features; complete the missing rows in the parity
  ledger, including negotiated low-latency video fallback.
- [ ] Confirm SFX audibility, output routing/hotplug and interactive editor use.
  Real decoder/EOF/seek checks do not prove which physical speaker produced sound.
- [ ] Recheck restart/reconnect and prepared timeline re-arming with the controller
  owner; preserve the paused session and existing cues.
- [ ] Complete safe authorized physical cue testing and sustained timing/load
  checks. VirtualBoard ACK timing is not physical edge measurement. Only the
  currently authorized R5–R7 may be used; no seat or R8 test is implied.
- [ ] Exercise recording, offline editing, RF assignments and strip controls
  end to end with real advertised hardware and retain explicit human acceptance.
- [ ] Recheck idle/playing/taskbar-preview CPU and responsiveness for both
  applications in two separated live samples. Keep #62 open until its gates pass.
- [ ] Resolve remaining GPU/native-surface and exact-navigation gates in
  #64–#66; experimental D3D11 must remain optional/default-off until accepted.
- [ ] Clear applicable platform CI/runtime/package checks and privacy-safe
  screenshots before selecting the next release version and publishing packages.

## Successor instructions

Inspect current main, linked issues and fresh process-local APIs before editing.
Coordinate controller/firmware and shared worktree/cache/updater ownership.
Implement a focused current-contract slice, retain exact test limitations, build
on CI or the faster authorized host, and deploy through the product updater.
Never compile/link Rust on the production host or replace its validated libmpv
with a different host's DLL. Reverify health, actual playback, controller state
and error/CPU evidence after delivery; preserve private settings and original cues.

For genuine human intervention, use an advertised suitable buzzer melody and
a large Persian message through the existing APIs. Attention must be bounded,
acknowledgement-aware and stopped when answered; never use motion/relays as an
alert or leave an unbounded sound loop. No alert is needed for routine progress.

Update human-facing issues with outcomes, remaining gates and useful links;
keep hashes, host identities and measured timestamps in technical evidence only.
Maintain [the changelog](../../CHANGELOG.md), curated version notes and genuine
screenshot evolution. Publish a new milestone only when its stated scope is
stable and verified; do not claim that every parity/backlog item is finished.
