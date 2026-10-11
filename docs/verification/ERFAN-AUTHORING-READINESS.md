# Erfan authoring deployment

## Current-source delivery · 2026-10-11

This dated update supersedes the installed identity, publishing owner and visual
limits of the 2026-10-10 receipt below; it does not erase its historical evidence.

- Installed application and downloader source is clean commit
  `9b72fac169238636faa1a7c5f92fb006b383e800`. Canonical checkout was clean main
  `1676ee8662d21dcfd5c7308a15cd27e809a484c3`; its only difference from the packaged
  source was two documentation files. No rebuild was made for that docs-only delta.
- Installed player SHA-256 is
  `6a26992d77cead8b2d0ae3c9f8232292b453be7ea8ced27784b9209ceeb9574f`,
  11,584,512 compressed bytes. Downloader SHA-256 is
  `01e6c9af92fc90ff843103ec0c161960cba7a9c1e7674f595e7ff58c6a01f1a4`.
  Embedded identity and destination-runtime player/downloader smoke probes passed.
- One necessary cached release build on David-PC compiled the application and
  downloader, not a clean dependency rebuild. Rust tests were skipped; the package
  Web guardrails, native resource checks, smoke and compression checks ran once.
  No Rust build ran on Erfan or Cafe. Sampled aggregate compiler working set reached
  2,022,809,600 bytes; this is not a continuous peak or scratch-temp high-water mark.
- Erfan's independently validated `0a81c004aae0ee7d512b9a26e38f66281f9591e84e1663215cc3a36a4bde6f6a`
  runtime and matching DLL aliases were retained. Never send this runtime to Cafe.
  Private settings/manifests and previous executables remain in rollback storage;
  immutable rollback/runtime helpers use hard links rather than duplicate copies.
- The previous application had stopped before replacement. Atomic replacement was
  repaired after a null-backup-path failure without deleting either executable.
  The existing signed-in Interactive task opened the new player: PID 8992,
  session 1. Application RPC selected NLE, maximized and activated its window.
- At 00:47 UTC, a read-only capture through the maintained repository utility in
  the signed-in session produced a genuine, nonblank current native window. It
  visibly contains Timeline, Effect Controls, Program Monitor and Effects Library,
  the green Cafe board connection and paused transport. The capture remains private
  under the program root's verification directory, not in Git. Its temporary capture
  task was retired. This supersedes the prior capture-tool failure as a permanent
  visual blocker, but is not dropdown/drag/dialog interaction acceptance.
- `pccontroller://cafe-pc.local:8787` remains the real board endpoint. A fresh direct
  controller query reported owner `pealayer:ERFAN-GAMING:8080`, revision 30, unlocked,
  no pending requests. This pass did not force a takeover. Cafe remained running,
  connected and paused. Recheck ownership immediately before a preview.
- At 00:46 and 00:47 UTC the process-local API reported no current sync error,
  prepared revision = clock ACK revision = 8, epoch 3, ten-step plan paused,
  play_requested=false. Paused clock updates advanced from 535 to 587; failed updates
  stayed at 2. A startup authority-refresh warning recovered. Recorded paused-clock
  maxima include a 523 ms round trip and 501 ms authority request; a healthy latest
  sample does not erase those faults or establish playing/motion timing acceptance.
- Original cue identities/placements and private draft/media state were preserved.
  No Play, effect Run, board reset, firmware upload or physical-output test was issued.
- The next check found that PID 8992 had exited at approximately 00:49 UTC. Task
  Scheduler records a normal return code 0, with no matching Application Error/WER;
  the task's 72-hour limit was not reached. This does not prove whether a human,
  control command or application lifecycle closed it. After asking the human,
  this pass reopened it once using the existing Interactive task: PID 7464,
  session 1. At 00:51 UTC it was NLE, paused, board-connected, two cues retained and
  no current hardware/sync error, but prepared/clock ACK revisions were zero.
  Treat that as connected editing only, not an armed preview. The unexplained
  repeated clean exit remains a readiness gate; do not hide it with a restart loop.
- At 00:52 UTC, the direct controller authority query reported Cafe as owner,
  revision 40, unlocked, no pending requests. Erfan remained paused and connected
  but its process-local sync reported a JSON-RPC read timeout, with prepare/clock
  ACK zero. A successful catalog connection is not proof that the separate clock
  metadata stream is healthy. Resolve that live warning and obtain the owner's
  consent before describing Erfan as ready for hardware previews.

### Current remaining gates

Erfan is reopened for board-connected editing, not certified problem-free production.
The new coordinated [Pealayer PR #142](https://github.com/ToghrolTP/pealayer/pull/142)
and [PCController PR #634](https://github.com/atomicdeploy/PCController/pull/634)
remain separate unfinished source work, not features delivered by this package.
Their owner must finish CI/native/Web acceptance and resolve persisted definitions
containing nontrivial per-step repeats before deploying the rejecting whole-effect
contract. Do not delete fields, guess equivalent repetitions or merge/deploy only
half the contract. Earlier human physical-seat confirmation, lifecycle issue #116,
audio/Web/interaction and sustained timing gates also remain open.

Cafe source was clean main at the same documentation checkpoint; its running
player remained `f414b89f06c9fc5a054ab7110e85606826baeea0`, paused and connected.
The newer package is staged only. Cafe's recorded command-tool smoke-launch denial
is an explicit deployment blocker: do not bypass it using another launch mechanism.
Cafe controller CI1459 retains its separately verified protocol-success receipt.
Pealayer final-main run 38096648510 was cancelled; do not claim all-green final CI.

## Historical delivery · 2026-10-10

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
