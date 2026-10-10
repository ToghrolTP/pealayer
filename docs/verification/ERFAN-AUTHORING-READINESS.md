# Erfan authoring deployment · 2026-10-10

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
This supersedes the earlier **staged/not launched** Erfan receipts, not their
historical evidence. The user's new request explicitly authorized opening the
authoring application; KMPlayer was absent in the fresh process inventory.

## Delivered runtime and workspace

- Canonical source was clean and advanced to main
  `28de6bafbb67aa6f6fa32f3550a8d4d82f04a78d`. The installed source identity is
  `759dafebc836e75dac3678fe4722a4bd7c1cb846`, dirty=false. Their only tree
  difference is the dropdown verification receipt; application code is identical.
- Running executable SHA-256:
  `67c9b721bc00935592e1ca3e7c2dc4f49d8871b966b62949d31059453a96af5a`;
  46,269,952 bytes. The standalone downloader is also installed, SHA-256
  `aaa29e6fa86ee363b2846b94d84fcc9178c1d7907408d8fd3bc4ec6e4487af2c`.
- Erfan retained its independently identified libmpv runtime
  `0a81c004aae0ee7d512b9a26e38f66281f9591e84e1663215cc3a36a4bde6f6a`.
  This runtime is **not** suitable for Cafe; no DLL was transferred between hosts.
  Candidate build identity and retained-runtime smoke checks both exited 0.
- Replacement occurred while Pealayer was stopped. The previous executable
  `5b74194ad0f18b314ff813822ffa820cd25d38d65550e7987d92f19ec13a57ca`
  and its installation manifest remain in canonical rollback storage.
- The existing `PealayerInteractive` task targets the canonical executable with
  the signed-in user's Interactive principal. Final live PID was 2420, session 1;
  no SSH-session GUI or remote-access service modification was used.
- Product RPC selected NLE, maximized and activated the window. The live painted
  dock geometry reports non-collapsed Timeline, Effect Controls, Program Monitor
  and Effects Library, with Hardware Monitor available beside Effect Controls.
- Packaged subtitle font was missing from the old installation. The verified
  tracked Vazirmatn font was hard-linked into `bin/assets/fonts`, then Pealayer
  exited through process-local RPC and relaunched interactively while paused.
  Runtime aliases likewise use hard links, not duplicate DLL downloads/copies.
- The served PWA inventory reports `8a04f393d16d1ba5`; service worker returned 200.
  Release assets are embedded in the executable, not taken from stale adjacent
  `bin/web_ui` files.

## Board and authoring checks

Erfan uses `pccontroller://cafe-pc.local:8787`, external TCP transport. Its old
local controller is not the board coordinator for this session. Cafe's current
controller remains the sole COM3 driver.

Publishing was handed off using an Erfan request and acceptance by the paused,
unlocked Cafe owner, not by forcing ownership or enabling unattended takeover.
The font restart released the session, so that same guarded handoff was repeated.
Final owner is `pealayer:ERFAN-GAMING:8080`; Cafe stayed running and paused.

Fresh observations after the final restart/handoff:

- Connected=true, hardware_error=null, sync error=null, E-STOP=false, playing=false.
- Prepared revision=3, clock ACK revision=3, epoch=3; ten-step plan is paused.
- Two original cues, two local templates and six advertised controller effects
  are present. Original cue IDs, effect references and placement times match the
  private pre-deployment backup. The working draft and media identity survived.
- One cached macro duration refreshed from 1381 to the controller's authoritative
  1380 ms; settings are normalized by the current application contract, not
  claimed byte-identical to the old installation.
- Paused position remained 28.321 seconds. Board identity schema=3, build hash
  1091576795, build timestamp `261010045554`; no active relays, both seats stopped,
  no hardware warnings, no new Application Error/WER entries for Pealayer/libmpv.
- Startup briefly exposed the existing media-clock-discontinuity safety pause
  during restore. It cleared on subsequent authoritative samples; the final
  prepared/ACK state above is healthy. No guard was disabled or error hidden.

No Play, effect Run, recording or physical-output test was issued. Readiness for
editing and controller acknowledgement does not certify physical cue timing,
SFX audibility, or a motion/relay soak test.

## Preservation and source disposition

Private settings were hash-verified off-machine before launch; pre/post copies
remain private. No media URL, settings file, cache, runtime DLL or binary was
published to GitHub. No new worktree, cache or Rust build was created on Erfan.
Free C: space was about 8.31 GiB after installation; this is not whole-disk hygiene
or wipe readiness.

The old canonical branch at `3bb5479` was an ancestor of current main, had no
unique commits or dirty work, and was retired after switching to main. The clean
detached recording worktree at `067d053bea1ceb95b59105fbd5918108fba19c98` is retained.
Its exact commit was published as
[`archive/live-recording-checkpoint`](https://github.com/ToghrolTP/pealayer/tree/archive/live-recording-checkpoint)
through the authorized workstation: Erfan's credential store rejected a direct
push, so a verified 454,050-byte Git bundle preserved the commit without changing
authentication settings.

That early snapshot is not a new production merge candidate: its app, controller,
engine and IPC files match the accepted head of merged [PR #58](https://github.com/ToghrolTP/pealayer/pull/58).
Remaining UI/formatting/generated-asset differences are archived for final semantic
disposition, not wholesale-applied over newer main.

## Explicit remaining limits

- Current main Build/Test/Release [run 38078051896](https://github.com/ToghrolTP/pealayer/actions/runs/38078051896)
  passed repository health and Linux, Windows GNU, macOS Intel and Apple Silicon.
  CodeQL [run 38078051924](https://github.com/ToghrolTP/pealayer/actions/runs/38078051924)
  also passed. A green run does not close intermittent lifecycle issue #116.
- Computer Use screenshot capture failed twice with Windows Graphics Capture
  `0x8007041D`. No unsupported native-input/screenshot workaround was used.
  Native visual/interaction acceptance remains unverified despite the live
  workspace geometry, RPC and board checks.
- [Current acceptance](CURRENT-ACCEPTANCE.md) and its human/physical/Web/soak gates
  remain open. This receipt does not claim the entire product is problem-free.
