# Production deployment and consolidation checkpoint

Owning tracker: [Pealayer issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Observed on 8 October 2026. This is an acceptance ledger, not a claim that all
historical requests or branches are complete.

## Recent requests reconciled

| Request | Source and verification | Remaining acceptance |
| --- | --- | --- |
| Compact custom app icons, Browse, no repeated cards/help, Config file last in Advanced | Shared Preferences implementation; 46 focused tests and inspected native dark/light captures | Native Browse selection still needs interaction testing |
| Semantic row icons; Language below Fullscreen background | Rust-owned icon metadata rendered by egui and React; inspected screenshot | Installed Web/native cross-surface interaction audit |
| Vertically center single-line input text, not multiline/wrapped text | Shared native helper at all 44 single-line call sites; actual galley geometry tested | Broader dialog visual audit |
| Deploy Cafe and verify real playback state | Current host-compatible build installed through its updater; PCController and protected firmware update completed; Play advances time, Pause and remote seek verified | A cue acknowledgement exceeded the timing limit; sustained precise hardware playback remains unaccepted |
| Deploy Erfan-Gaming | Destination runtime independently identified; installed and candidate smoke tests passed; candidate uploaded to canonical staging | KMPlayer is running: user explicitly prohibits launching Pealayer there; installed build remains the previous version |
| Keep David running and synchronized to Cafe | Canonical installation updated, running as a cache-only consumer; no local hardware scheduler; remote seek, mute, pause, preferences and WebSocket snapshot verified | Latest replay fix is staged locally but not installed while the healthy consumer remains open; native close must not be confused with remote Quit |
| PCController startup service | Single interactive coordinator recovered and updated; no competing TUI owner remains | A real Windows service has not been installed; an interactive recovery task is not a service or boot-start proof |
| Consolidate useful work and merge safe PRs | Peer polling PR #82 and Vite PR #83 reviewed and merged; combined Preferences branch includes main plus exact PR #77/#79 heads | Remaining CI faults, experiment integration and old-ref/stash reconciliation |
| Keep D3D11 optional and disabled | Preserved experiment already defines renderer selection with OpenGL default and detached video disabled | Not integrated into main Preferences yet; do not activate or silently deploy the experimental executable |

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
