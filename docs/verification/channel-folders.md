# Hardware channel folders

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Delivered together: [Pealayer PR #128](https://github.com/ToghrolTP/pealayer/pull/128)
and [PCController PR #626](https://github.com/atomicdeploy/PCController/pull/626).
Controller contract: [Channel-Folders.md](https://github.com/atomicdeploy/PCController/blob/main/docs/Channel-Folders.md).

## Delivered behavior

Motion / seat controls, Relay outputs and PWM outputs have section context menus,
even while collapsed. Manage channels, New folder, Manage folders and expand /
collapse actions are available. Native and Web folders can be created empty,
renamed, given an icon and deleted without deleting their channels. Folder deletion
asks for confirmation and moves all members, including hidden members, to Ungrouped.

Channel context menus offer Move to folder. Channel grips can drop onto same-section
folder headings (including empty folders and Ungrouped), with accent feedback.
Dropping on another operator card changes its folder and kind-local rank in one
presentation transaction. A wrong-kind or raw-seat-wiring drop cannot change an
operator folder. Raw relays is a fixed diagnostic heading for controller-declared
R1–R4 seat wiring; R5–R8 are not implicitly members.

The previous renderer put ungrouped cards immediately after a named folder without
another heading. This made unrelated MOSFET channels appear inside Cinema lighting.
A fresh Cafe catalog assigned only `pwm.0` there, so no bulk reassignment was needed:
the repaired rendering gives other channels an explicit Ungrouped heading. Output
type, channel name and relay number alone do not infer folder membership.

Folders persist in the controller's board profile, not a separate player store.
Revision-checked transactions reject stale edits; clients apply the complete
authoritative response before accepting its new revision. Empty folders remain
available after a move and have no empty expansion chevron. Native failed saves
retain draft values; Web completion waits for the corresponding host operation
sequence/acknowledgement rather than assuming queued means saved. Board changes
discard an editor for the previous board.

## Verification and boundaries

- All 44 stable-path Go package suites and Go vet pass. The controller embedded
  Web suite also passed 357 tests across 72 files.
- Isolated shared-cache native `cargo test --lib --locked --jobs 1 hardware_ --
  --test-threads=1`: 48 passed. This includes real collapsed-header context menus,
  folder-drop release/invalid-kind checks, editor failure/success/board-change
  lifecycle, exact membership and hidden/empty-folder partitioning. Existing
  hidden-disclosure geometry and card/manager/drag regressions remain green.
- Full isolated native library suite: 746 passed, none failed, one ignored.
- Web TypeScript build, source parity/responsive guardrails, PWA and asset validation
  pass. Executed pure TypeScript folder tests check exact R1–R4 membership, ordinary
  R1 rejection, lighting membership, hidden channels, section scoping and empty
  Ungrouped targets. Browser gesture evidence is recorded separately below.
- No folder operation energizes hardware or changes EEPROM, firmware, cues or media.
  Human native and touch gesture acceptance, sustained playback and physical timing
  acceptance remain separate gates.

## Live folder verification · 2026-10-10

A real browser connected to Cafe-PC's installed Web application over an SSH
forward. The section context menu opened on PWM and on collapsed Motion. Creating
an empty PWM folder waited for the host acknowledgement, then CH2 was dragged from
its grip onto the empty folder heading. The rendered member count changed to one
and Ungrouped to eleven. Renaming the folder retained its member. On the final Web
build, choosing New folder from a collapsed section left that section collapsed;
canceling the editor did not create a folder.

Metadata-only RPC verification changed the test folder icon, rejected an edit with
the previous revision, and verified that the rejection changed neither revision
nor membership. Deleting only that temporary folder moved CH2 back to Ungrouped.
The final catalog contains Cinema motion and Cinema lighting only. Cinema lighting
contains only `pwm.0`; R5–R8 have no group. R1–R4 are controller-declared seat wiring
and are not exposed as operator controls in this profile (`expose_raw_relays=false`).
Their Raw relays partition, when advertised, is covered by exact-membership tests.

Both private saved board profiles were compared with the pre-update checkpoint.
Wiring, roles and all original presentation records were unchanged. The only new
channel record materialized the existing default `hidden=false` for `pwm.1` after
its test move. No whole configuration was restored over the live store.

## Exact deployment receipt

Packages were built on DAVID-PC in the canonical checkout/shared cache, not on
Cafe-PC. Both candidates passed native resources, runtime-load and smoke probes.
The final evidence/docs commits do not change the packaged runtime source.

| Application | Runtime source commit | Installed executable SHA-256 |
| --- | --- | --- |
| Pealayer | `0e37c0258f60e4ba35ef21efc2315d35d9439ef9` | `ebb6b4d04e271590319dc74f4e131b94f68c59577e04f99bbd50be7ce391825f` |
| PCController | `6537f1f56982408aec2b9395ed40126c87da22cd` | `6a8482a8c2eecee41619a30b091fe7e4339db0446a8b5ffb3fd45fc81f1a043b` |

Pealayer's graceful peer update `update-6acb994f-afd0-4921-8dd4-2c19dcf7c7fd`
completed and the live manifest acknowledged that exact hash, clean source commit
and Cafe's unchanged libmpv runtime
`872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
Its new interactive process was PID 144428, session 1. Hardware reconnected with no
error, playback remained paused, E-stop was false, and media, cues, workspace and
paused position were preserved. No new matching application fault was found.

PCController's primary-owned host updater replaced the interactive Cafe process
without changing its launch policy. At `2026-10-10T17:43:30.8044904Z`, PID 69688
owned port 8787 with the exact executable hash above; the board and player were
connected and healthy, original profiles/channel metadata were preserved, and no
new matching runtime fault was found. SCM remained Stopped/Auto with its existing
service account; automatic service/reboot acceptance is not asserted here.
The C ABI DLL was built and smoke-tested but not replaced on Cafe; Pealayer uses
the verified external controller transport.

The local port-8788 headless IPC coordinator had no normal per-user primary record,
so the host updater safely refused it. Its own RPC graceful exit was acknowledged,
its process exited without a forced kill, and the same arguments/configuration were
restarted with the verified executable and a retained rollback image. At
`2026-10-10T17:42:56.4960383Z`, PID 77784 listened on `127.0.0.1:8788`, its private
configuration hash was unchanged, and the Cafe peer remained enabled at
`ws://127.0.0.1:8787/ipc`. This command/event bridge does not claim a local board.

No firmware, EEPROM, motion, relay, PWM, playback or audio command was used for
folder acceptance. Native headless interaction tests and live Web gestures are
distinct evidence; no native desktop screenshot, physical timing or soak pass is
claimed.
