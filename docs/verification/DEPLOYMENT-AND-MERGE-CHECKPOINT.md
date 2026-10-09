# Production deployment and consolidation checkpoint

Owning tracker: [Pealayer issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Updated on 9 October 2026. This is an acceptance ledger, not a claim that all
historical requests or branches are complete.

## Timeline toolbar recovery (9 October)

The requested Pan left/right and Follow/Bring-into-view swap was not lost from
source: its original commit remains an ancestor of both installed binaries.
Cafe's saved complete order retained the previous defaults, and David consumes
Cafe's preferences. Only those two pairs were swapped through Cafe's config
API; subsequent API read-back confirmed the same corrected order on both hosts.
Future default orders inherit through an empty override, while genuinely custom
orders remain persistent. Unrelated saves no longer freeze current defaults.

The ruler toolbar now uses the shared workspace sublayer, also used by hardware
cards, rather than an independent Foreground layer. This preserves its position
above the ruler but beneath the Add cue backdrop and other dialogs. New source
regressions cover default-order persistence and actual modal paint/input blocking
in light/dark modes; they are deliberately **not run** at the user's request.
The prior package/test pipeline was stopped, and slow CI waits are deferred.
The live order correction is delivered; the backdrop code still requires an
updated executable and live acceptance. No hardware actuation was performed.

## Cue-clock and startup-service follow-through (9 October)

The cue-timing acceptance remains physical/live, not a compiler-only gate.
The user authorized Relay 5 through 7 for this pass; Relay 8 and seat/motion
outputs are excluded. Preserve paused media and existing timeline state, use
compare-and-swap when removing temporary test cues, and never overwrite a
concurrent user's edits.

The first baseline stopped before creating or actuating a cue: Cafe accepted
an API seek but did not move the paused media. The cause is the native lifecycle:
eframe 0.36 invokes `App::logic`, not `App::ui`, for minimized/occluded windows.
Commands, device results, media event processing, status publication, shell
state and disconnect safety now use that single shared logic callback. Painting
and pointer/keyboard UI gestures stay in the UI callback. This is also required
for Web-only operation; do not work around it by making the window visible.
A logic-only regression covers IPC/Web commands and status without painting.

Service cutover exposed a second recovery issue: a peer RPC error was flattened
without marking the command stream failed. Additionally, failures after the
mid-pass cleanup could be overwritten by the next pass's `transport.is_some()`
assignment. All typed RPC failure paths now share transport-failure recording;
the same cleanup runs before deriving next-pass connectivity and after command
processing. Domain rejections keep the connection. Failed mutations are returned
once, never automatically replayed. Regression coverage includes peer reply,
late wire failure and healthy domain rejection.

The stream-loss checkpoint passed the full native MSVC release suite: 654
library tests and 112 integration tests across 26 targets. Startup now lives in
the library, so the binary no longer duplicates the application's unit tests
or process globals. MSVC resources are linked once through that library; GNU
keeps its direct symbol-less resource object. All seven action icons, embedded
clean-source identity, bundled runtime smoke and Web/PWA packaging passed.
[Exact checkpoint CI](https://github.com/ToghrolTP/pealayer/actions/runs/37860873352)
passed Web, Windows GNU, Linux and both macOS architectures, including the
previously failing PWA icon test without skips. Integration tests own ephemeral
listeners instead of fixed ports that could reach a running app.

PCController's owner completed the delayed-auto-start SCM service in merged
PRs [#610](https://github.com/atomicdeploy/PCController/pull/610) and
[#611](https://github.com/atomicdeploy/PCController/pull/611), including the
owned-data-root correction. Cafe's `PCController`
service runs as `NT SERVICE\\PCController`, survives restart, owns the board,
and has verified flash/EEPROM readback with settings preserved. Dispatch, board
ACK and total cue lateness are separated; dispatch consumes the same 50 ms
safety budget. Do not deploy a competing controller or call an interactive
recovery task a startup service. Independent SCM read-back confirmed Running,
Auto, delayed-auto enabled and the service-owned account. During the later
controller cutover, the same Cafe Pealayer process reconnected and re-armed
without changing its paused position or three original cues. A deliberate
restart acceptance test against the final Pealayer build remains required;
this is not a reboot-test claim.

Two more controller fixes are merged: [fresh authority sequence reset](https://github.com/atomicdeploy/PCController/pull/612)
and [serialized programming ownership](https://github.com/atomicdeploy/PCController/pull/613).
The programmer must close the primary reconnect loop before claiming the serial
port, then reconnect through the bounded current contract. A board HELLO failure
was recovered through guarded flash/EEPROM backup, exact firmware write/readback
and preserved settings; the original HELLO-loss cause is not proven merely by
that recovery. Do not run a competing programmer or controller process.

Linked-worktree Git ref watches and package-time embedded-identity validation
now prevent a manifest from claiming a newer commit than its executable.
`--build-info` prints identity without starting a player; process-local observer
diagnostics expose actual loaded/playing/buffering, epoch and sample age without
media paths. Both canonical apps are running with the preserved paused session:
Cafe publishes, David consumes with cache-only storage and no local hardware
scheduler or clock observer. Their distinct host runtimes are preserved.

Clock recovery is intentionally fail-closed. Every live-stream loss invalidates
its arm and pending Play exactly once; disconnected polls cannot churn revisions.
Semantic safety faults stay latched. On reconnect, only a matching publisher's
retained timeline revision is advanced. A final source guard also reads that
publisher's authoritative playback counter and continues above it, because
registry removal and plan preparation do not reset the clock sequence on a fast
same-ID restart. Never adopt another actor's counter or old epoch, nor release
exclusive authority merely to reset a counter. A newly acknowledged plan and
paused clock arm still precede Play. That last guard needs its own final test,
package and deployment proof; the preceding green checkpoint is not a substitute.

The tool policy blocked an ad-hoc Cafe-runtime smoke launch through SSH localhost
before execution. The user was asked to perform that non-deploying check; no
equivalent bypass was attempted. Final deployment and physical timing acceptance
remain gated. No temporary cue, Play or relay actuation has occurred in this
continuation. The safe test must observe Relay 5 ON for one second and OFF, both
device acknowledgements within the unchanged 50 ms budget, then CAS-restore the
original timeline and paused position. Stop before the original motion cues.

## Recent requests reconciled

| Request | Source and verification | Remaining acceptance |
| --- | --- | --- |
| Compact custom app icons, Browse, no repeated cards/help, Config file last in Advanced | Shared Preferences implementation; 46 focused tests and inspected native dark/light captures | Native Browse selection still needs interaction testing |
| Semantic row icons; Language below Fullscreen background | Rust-owned icon metadata rendered by egui and React; inspected screenshot | Installed Web/native cross-surface interaction audit |
| Vertically center single-line input text, not multiline/wrapped text | Shared native helper at all 44 single-line call sites; actual galley geometry tested | Broader dialog visual audit |
| Deploy Cafe and verify real playback state | Current host-compatible build installed through its updater; PCController and protected firmware update completed; Play advances time, Pause and remote seek verified | A cue acknowledgement exceeded the timing limit; sustained precise hardware playback remains unaccepted |
| Deploy Erfan-Gaming | Destination runtime independently identified; installed and candidate smoke tests passed; candidate uploaded to canonical staging | KMPlayer is running: user explicitly prohibits launching Pealayer there; installed build remains the previous version |
| Keep David running and synchronized to Cafe | Canonical app relaunched interactively as Cafe's cache-only consumer; process-local status verifies no local hardware scheduler or clock observer; paused state and three cues agree | Final counter-recovery package still needs deployment; native/local Quit must not be confused with remote session Quit |
| PCController startup service | Real delayed-auto SCM service installed as `NT SERVICE\PCController`; Running/Auto/delayed-auto read-back and service restart/cutover verified | Final-build same-PID reconnect acceptance remains; no actual reboot test claimed |
| Consolidate useful work and merge safe PRs | Peer polling PR #82 and Vite PR #83 merged; PR #87 preserves main and exact PR #77/#79 heads; preceding recovery checkpoint passes all desktop CI targets | Final counter-guard verification/deployment, experiment integration and old-ref/stash reconciliation |
| Keep D3D11 optional and disabled | Preserved experiment already defines renderer selection with OpenGL default and detached video disabled | Not integrated into main Preferences yet; do not activate or silently deploy the experimental executable |

## API-first remote role change: implemented and running

Current process-local read-back verifies David is an interactive cache-only
consumer of Cafe, not a second publisher. The local updater/process prefix stays
local; ordinary session commands reach Cafe. The detailed observations below
are historical pre-cutover evidence, not current PIDs or deployment identities.

Source `1cdf1fe125d3408d75ebc93b6467c88b43135b0e` on PR #87 implements
`pealayer.process.connect/status/quit` through native IPC, HTTP, JSON-RPC and
WebSocket. Process commands stay local on a consumer's gateway; normal session
commands still reach the authority. The native connection dialog no longer
launches a competing app through a separate implementation. The connection
checks the target's real session and loop identity first, deduplicates operation
IDs, and gracefully restarts the same executable. A direct publisher must be
paused/unlocked and receive publication-release acknowledgement; an observer
never releases somebody else's ownership. Session-launch batches cannot hide
local lifecycle commands. See [API usage](../HARDWARE-AUTHORITY.md#api-first-process-control).

Verification: 7 focused lifecycle/probe/transport tests plus 17 authority,
media-sync, prepared-timeline and consumer-routing regressions passed. React/PWA
build passed with 35 parity and 24 responsive contracts. These are bounded
automated checks, not proof of physical handoff, all historical API coverage, or
installed-host role-transition acceptance. All six existing Pealayer worktrees
were inspected and preserved. No controller checkout/branch was changed and no
build was run on production Cafe.

The clean Windows release build completed on David. Existing canonical staging
was reused for both destination runtimes; both libmpv smoke tests exited 0.
The executable is 43,161,088 bytes, SHA-256
`0e507c7df5e4e70f7bd0ed8aeebd9a5d9c44b503a3f2bd6c5018627b3a3d740e`.
Private host manifests were updated without copying the large DLLs again.

An isolated headless consumer briefly exercised the compiled APIs against Cafe.
REST/RPC/WebSocket returned its own PID 48852 and source `1cdf1fe`, not Cafe's.
`pealayer.process.connect` to its existing origin/port reached `connected`;
`pealayer.process.quit` through IPC closed only that verification process.
Read-back confirmed Cafe retained PID 17292, paused position 1196.24 and both
controller/board connection flags. The verification process exited normally and
no extra Pealayer instance remained. This verifies the live no-op connection and
local shutdown path, not a canonical installation or a full role-change restart.

A fresh read-back now confirms Cafe installed the preceding `ce630d1` package,
not the older package recorded below: actual PID 17292, executable SHA-256
`ac2f5ed4f6b306636997265edd4c0520f527d4e143ec148439ae04df224b7622`,
runtime SHA-256
`872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
Its unattended policy is false; board/controller connected flags were true and
media remained paused. This establishes the earlier package's installation,
not a deployment of the new process APIs.

David's canonical PID 44980 remains a cache-only consumer of Cafe with
`local_hardware_scheduler: false`. Its current peer sample age crosses the
two-second freshness bound intermittently, and diagnostics retain a failed
media-command transport error. A connected snapshot was seen, but the link is
not accepted as continuously healthy. Do not suppress that error or describe a
peer transport lapse as a physical board unplug.

Remaining delivery gate: install the new host-compatible package and read local
process status on both hosts, then use the process connection API on David,
verify the fresh consumer snapshot/no scheduler, and prove Cafe's PID/session
and publisher remain intact. David's older executable forwards update APIs to
Cafe and lacks the safe local lifecycle API; do not send session Quit or trust
its relayed updater identity for a local bootstrap. A prior updater launch was
tool-policy blocked; do not route around that denial. Preserve both running
apps until an approved/bootstrap path is available.

## Earlier unattended handoff and authority connection checkpoint

On 8 October, source `ce630d14e2f47e2060c913564702c8f16f5e481a` on
`release/preferences-organization` / PR #87 implements the owner's opt-in
`allow_unattended_hardware_takeover`, default false. The existing libmpv observer
pauses actual playback; a fresh observed paused/non-buffering sample and matching
clock echo precede the existing controller acceptance/acknowledged cleanup.
Production lock is never bypassed. Failed acceptance is attempted once per claim
and reports the real error; previous-owner arming is invalidated on transfer.

Native/Web use one Preferences control. Both conflict surfaces offer Connect to
authority using the matching registered Pealayer's validated optional origin.
Native allows editing it for a tunnel; Web disables it when not advertised.
Address lookup is only on the observer path, not the active publisher's clock
path. No live physical handoff/output command was issued for acceptance testing.

An additional deployment defect was found: consumers forwarded `/api/update/*`
to their authority. Those routes now stay process-local. Session Preferences,
media and hardware APIs still relay. Do not blindly target the old installed
David consumer's API for an update—it currently reports Cafe's update manifest.

Verification: 17 focused Rust library tests passed (authority, media sync,
prepared timeline and consumer update routing), release binary build passed,
Web build passed with 35 parity / 24 responsive contracts and installable PWA
validation. Seven native quick-action icon resources were verified. Staging
smoke tests exited 0 with each independently checked destination runtime.
This is not a claim that the entire suite or live handoff has passed.

Canonical staging was reused at `staging/preferences-organization` (David) and
`staging/preferences-cafe` (Cafe); source and artifact/runtime identities are in
their private host manifests. No new worktree or Cafe compilation was performed.
David's own persisted policy is true; Cafe's old stored field is absent and the
new default is false. David currently consumes Cafe's session, so its local
opt-in applies only when it later becomes a direct publisher, not to Cafe's
forwarded Preferences.

The Cafe peer-updater invocation was rejected by the tool execution policy.
No alternate route was attempted. Both installed apps remain running; Cafe's
last read-back source is `2821f994e005e2bdeb232cdcb94dc1550c272baf`. Neither new
package is installed yet. The user was asked to run the staged Cafe executable's
`--deploy-to http://127.0.0.1:18088` command. Next: verify the installed source,
live owner origin, Cafe's false policy, board connectivity and retained paused
media, then finish David's process-local consumer update without forwarding Quit
or replacing Cafe with David's runtime.

Controller coordination: no PCController checkout, branch or PR was mutated.
Required deployed contracts are `controller.media.authority.get/change` with
owner-only acceptance and production lock, matching
`controller.media.playback.update` echoes, and exact leased
`controller.app.instance.get/report`. `values.peer_origin` is optional Pealayer
registry metadata, not a firmware change. Wait for the controller consolidation
owner's finished main rather than redeploying an older feature branch.

All six Pealayer worktrees were inspected with command-scoped ownership checks
where necessary and were clean before this documentation checkpoint. Other
branches, runtimes, private settings, caches and experiments were preserved.

## Connection reporting and publishing authority: controller delivered, application pending

On 8 October, the connection path was found to collapse controller-domain
rejections into a disconnected state. Status-stream sampling errors also lacked
authoritative board connection context. These are not evidence of a physical
unplug. The replacement preserves typed RPC errors, distinguishes transport,
board/status and command failures, and publishes the actual reason with shared
warning icons. A stale remote link pauses only the consumer preview. Connecting
a monitor no longer implicitly stops effects.

Native and Web Hardware Monitor now share publishing-authority commands and
conflict/handoff dialogs. The owner must consent to a paused handoff. Unlocked
observers may control idle outputs; an executing prepared timeline keeps its
existing exclusion. Production lock reserves publishing and live output control,
while monitoring and E-STOP activation remain available. Authority is presently
RAM-resident and resets on controller restart; registered IDs are attribution,
not authentication. See [Hardware authority](../HARDWARE-AUTHORITY.md).

PCController PR [#607](https://github.com/atomicdeploy/PCController/pull/607)
is based on the current unified-effects branch. Its executable was installed on
Cafe through the primary-owned host updater, which returned terminal verification
after restart. Installed executable SHA-256:
`5cb198a291f64ef7242e38ad4776262deb4e9fa04dbd128b6d112f36a6d416eb`.
The updated C ABI was built and smoke-tested but has not been installed as a
replacement embedded runtime on Cafe. No firmware flash was required this pass.

Live Cafe RPC checks verified a queued observer request, denial of requester
self-acceptance, owner rejection, production lock, typed locked rejection and
rejection of a foreign empty prepared plan. No output actions were sent. The
temporary observer was removed, the original owner stayed assigned and unlocked,
and the media remained paused at 1196.24 seconds. Cafe's old application later
re-prepared its timeline after the controller restart. Its status still records
a real TCP abort during capability refresh while `hardware_connected` is true;
do not erase that diagnostic or report a physical disconnect from it.

Verification: 84 focused Rust hardware tests, the peer-link transition test,
the process-local updater shutdown test, React build and responsive/parity
contract guards passed. The controller package passed 44 Go packages plus vet,
353 Web tests, C ABI smoke and its packaged executable smoke. Existing native
binary-suite/physical timing gates below remain open. Phone/tablet/desktop live
interaction and the new native popups have not been visually accepted this pass.

The clean Pealayer candidate is source
`eb84f9698d5b989c34b6883bb81b301d478ebe06`, executable SHA-256
`d9b1db070b787a9f55ef416a55cb638b7b6c55de2fbaef1e55dd6f00c54407a3`.
David packaging smoke passed. Cafe staging has that exact executable through
the existing hardlink and preserves Cafe's distinct DLL identity. A stale
staging-file removal was blocked by tool policy; it was not retried. The final
updater launch was also blocked, so no Pealayer installation is claimed. The
user was given the scoped Cafe updater command. No compilation occurred on Cafe.

The new updater sends an internal, process-local Quit rather than forwarding
session Quit to the remote authority. David still runs the old healthy consumer,
whose session Quit is forwarded to Cafe. Do not use its remote Quit to install a
local update or forcibly terminate that healthy process. Finish the scoped
consumer update separately and verify the running manifest on both machines.

## Cafe: real playback recovered; precise cue timing remains open

The latest live manifest identifies source `2821f994e005e2bdeb232cdcb94dc1550c272baf`,
with a clean build and executable SHA-256
`69d21778159ccb8f8d76b053b46cf50230245c22617b90cc18c34552a5e60e8d`.
The package uses Cafe's own previously identified runtime, not David's DLL.
The updater verified the complete upload before replacement.

The old process acknowledged commands without applying them and did not exit.
A private minidump was preserved. After graceful IPC/HTTP Quit attempts failed,
the user's standing hung-process permission was used for that exact process.
Its existing verified update journal/helper was resumed in the signed-in
interactive session; the helper completed successfully. No manual overwrite of
a running executable occurred. The new process is in interactive session 1.

The missing media RPC was traced to the old deployed controller. Its first
host-update acknowledgement did not prove delivery: a competing TUI process
became coordinator during replacement, and the update journal rolled back.
After graceful shutdown of that secondary owner, the primary-owned second
transaction committed the new executable. The updated host then exposed a
separate board fault: the old firmware rejected the media-clock opcode. The
user authorized host and firmware updates; the protected firmware transaction
completed with readback/reconnect verification and restored board settings.
No EEPROM reinitialization was requested.

Cafe now owns the only hardware scheduler. Actual Play advanced media time by
1.48 seconds during a three-second observation, with live board acknowledgements
and no synchronization error. Pause applied. A later cue test reported a
58.1 ms acknowledgement against the 50 ms limit and correctly paused playback;
the guard was not relaxed. Physical output-edge timing and sustained cue
playback are therefore not accepted yet. The original paused position was
restored through David's remote client and matched on both sides exactly.

The replay fix creates a fresh prepared revision after a stopped/faulted
executor, instead of repeatedly accepting the terminal state of an old revision.
Seven focused media-timeline tests passed, including compilation-error
fail-closed behavior and avoiding revision churn on repeated Play. The release
package and Web/PWA guards passed on David; the package intentionally skipped
the full native suite, whose Windows binary-test access violation is still a
merge gate. The new build was delivered to Cafe using its own updater.

Measured three-second CPU samples after coordinator consolidation were 1.5%
for PCController and 7.7% for Pealayer, normalized across four logical CPUs.
These are short observations, not a sustained-playback performance certificate.

Controller evidence and next owner actions are on
[startup/recovery issue #598](https://github.com/atomicdeploy/PCController/issues/598#issuecomment-6065386655)
and [media synchronization issue #554](https://github.com/atomicdeploy/PCController/issues/554#issuecomment-6065387269).

## David: synchronized consumer, not a second controller

The canonical application remains running as a consumer of Cafe. A separately
scoped Chisel reverse link was authorized by the user and binds the server-side
forward only to loopback. Existing remote-access tunnels were preserved; no
Codex execution policy or system-wide security policy was changed.

Live client diagnostics report `local_storage=cache_only`,
`local_hardware_scheduler=false` and no transport/command error. Remote mute,
pause and exact seek reached Cafe. Changing one preference through David
updated both API views while David's local config-file hash stayed unchanged;
the original value was restored. David's local WebSocket returned a complete
authoritative snapshot. Paused preview drift measured zero. Sample round trips
of roughly 0.5–1.1 seconds mean playing-preview synchronization is not certified
frame-exact over this tunnel.

David currently runs the preceding clean canonical build, not the latest replay
fix above. The new candidate is staged. Native window close closes only the
consumer; API/IPC session Quit is forwarded to Cafe. Do not send remote Quit to
install a local consumer update. Reconnect after the scoped local update and
verify the installed manifest before marking David's latest fix delivered.

## Erfan-Gaming: staged, not delivered

The existing canonical installation and logged-in user were discovered through
the established SSH route. The current destination DLL stayed unchanged.
Both the old installed executable and the new candidate exited 0 in runtime
smoke tests. The staged preceding candidate's full executable hash was verified;
the latest Cafe replay fix has not yet been delivered to Erfan.
The old installed application was initially started before the user's KMPlayer
constraint arrived, then exited gracefully. The latest process inventory shows
KMPlayer running and no Pealayer process. Do not launch Pealayer while KMPlayer
is running. Upload and smoke checks do not constitute installation. The last
verified installed manifest remains source
`95be0c4c31d0263e1ec8d2a15c853d595c522aad`.

## Main and remaining merge gates

- PR #82 merged as `f3b613837eb513d3b4027e498e5adbff344fced6`.
- PR #83 merged as `db328d153e19c1713b98c02f6adefdf3d92717f3`.
- `release/preferences-organization` includes those merges and the exact
  current #77/#79 heads; it is a consolidation candidate, not merged main.
- Windows CI run 37808589741 passes the library tests, then its binary test
  process exits with `0xc0000005`. Do not describe this as all-green or merge
  the combined candidate until diagnosed and verified.
- PR #84's sha2 update breaks digest hexadecimal formatting and the old
  `std::io::Write`-based hashing path. Adapt to the new digest API and rerun
  all affected hash/update tests before merging.
- PR #86's latest repository-health job fails on an extra EOF blank line;
  repair it, review actual cache/presentation behavior and verify before merge.
- D3D11 remains on `experiment/directcomposition-preview`. Its feature-gated
  renderer, popup/OSD composition, minimized-preview and frame-stability gates
  must be reconciled with current contracts. OpenGL must remain default, with
  explicit Preferences opt-in and restart semantics. A preserved branch is not
  an integrated or production-accepted feature.
- Historical live-media/unified-port branches and the private preserved
  Windows-readiness stash need content-level disposition, not wholesale stale
  merges or deletion to produce an artificially empty branch list.

## Next owner steps

1. Finish David's scoped consumer update and reconnect without stopping Cafe or
   creating a second hardware scheduler. Verify the running manifest.
2. Instrument/fix the controller cue acknowledgement lateness without weakening
   the safety limit; verify physical outputs and sustained playback. Implement
   and verify the requested real startup service with correct user/config and
   updater ownership. Recovery scheduled tasks alone do not satisfy this.
3. Complete Erfan's own updater only when its KMPlayer constraint permits it;
   verify live source/runtime identity, media decoding and controller behavior.
4. Diagnose Windows binary-test access violation; review/fix #84 and #86.
5. Merge the verified consolidation, then integrate D3D11 opt-in without
   discarding newer Preferences/media/hardware work. Audit old refs and stash
   hunks before closing superseded PRs or retiring branches.

Private dumps, settings, artwork and media references stay off public GitHub.
No Rust compilation/link occurred on Cafe. Existing source/history and useful
incremental caches remain preserved; blocked bulk cleanup was not retried.
