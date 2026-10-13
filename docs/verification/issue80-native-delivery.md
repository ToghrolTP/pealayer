# Issue 80: current native delivery and acceptance boundary

Observed 13 October 2026. Owner: [#80](https://github.com/ToghrolTP/pealayer/issues/80).
This receipt supersedes the older Cafe source/package observations in the
individual UI delivery records, not their interactive or physical acceptance gates.

## Delivered on Cafe

The user confirmed that drafts on Cafe, Erfan and David-PC were saved and native
updates were safe. One shared-cache Windows MSVC release build on David-PC used
`package-windows.ps1 -SkipTests -NoUpx`. TypeScript/Vite/PWA integrity, executable
resources and actual process-exit smoke probes passed. Regression tests and CI
were skipped as requested; no compiler/linker ran on a destination machine.

Pealayer package source is `00b12d38830fd2c9aa4ddb6f80335deb16e69132`.
Player SHA-256 is
`1e96916771f0a2ef82d2868f4249b5486139d728de25f76faf632b7c2c6f31ea`.
PR #174 merged normally into `main` at
`a1641d9de9c05c6616034a52be81507bd7a33eba`; its subsequent source differences
from the package are deployment helpers/documentation only, not application code.
The package contains the immediate seven-segment/peer revision delivery, shared
native video GL-state isolation/present order, four-step density, semantic relay
actions and caption-centering changes.

Cafe retained its own libmpv, SHA-256
`872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
The default David package uses David's different runtime; a new host-paired
candidate was prepared without rebuilding or replacing Cafe's runtime.
Retained-runtime smoke and build identity passed before activation. A direct
PowerShell GUI-subsystem invocation initially returned no `$LASTEXITCODE`;
the maintained `Invoke-PackageProbe` waited on the actual process and confirmed
exit 0. The null shell variable was not evidence of a DLL failure.

The approved peer updater completed operation
`update-dc41ceb9-bf58-4bb7-91cb-043b8ea581d3`. Fresh process-local status verified
PID 185152, interactive session 4, package source/player digest above, retained
runtime digest, authority role and paused playback. Before/after equality checks
preserved the effect working draft, workspace session/layout/profiles, active
profile and application branding. Private backups remain in Cafe's program-root
`verification/issue80-native-acceptance`; they are not published in Git.

PCController's primary-owned `update host` transaction then committed operation
`op-929090c43535bedc`, with `terminal_verified=true`, PID 170448/session 4 and
executable SHA-256
`12b38064a761f425af52e116257741b23bf72f2c166c8033dca12b58c0b7d60c`.
Its package source digest is
`146f4c31cd0f91c10fafc20d0bf31f188b7c7c69eeacee697cefe21b77247248`.
The self-update journal is `committed`; the old executable remains in the named
rollback file. PCController configuration is semantically unchanged. Pealayer
reconnected and prepared matching desired/prepared/clock-ACK revision **72**,
without hardware/sync error, pending Play or reprepare latch. The strip stream
was read back stopped before the update. No Play, cue Run, seat/relay/RF action,
tone, board firmware flash or reset was issued.

Release Web assets are embedded in the updated player; an old adjacent Web tree
does not override them unless `PEALAYER_WEB_ROOT` is explicitly configured.
This pass updated the active executables, not every downloader/C ABI/adjunct file.
Do not infer complete adjunct-package reconciliation from executable receipts.

## Preserved work and outstanding gates

Both Pealayer worktrees were clean at audit; no authored local-only branch commit
was found. The generated Web bundles remain in named preservation stashes, not
published as source or reapplied over a newer bundle. PCController's generated
bundle from this host build is preserved as stash
`d17a95f3060e4aa8761934439dd534fd43bc7e3e`, without touching the active Flutter
owner's published work.

- David-PC's direct Cafe LAN probe timed out. The old local consumer launch did
  not remain running; a later combined launch/inventory command was denied by
  tool policy. No alternate activation route was attempted. Local installation
  of this package is **not verified**. A private persisted-config backup exists.
- Erfan's prior updater denial remains unbypassed. Its historical staged package
  is not this new package; the old staged-update command must not be advertised
  as installing the latest build. Refresh/stage/activate through an allowed path
  and verify its own retained runtime and process-local consumer identity.
- The local computer-use inventory had no Pealayer window. Cafe's maintained
  read-only capture utility, run once in its signed-in session, returned a white
  client area. It is not usable UI/screenshot acceptance. The temporary capture
  task was retired; the capture remains private, not committed or reconstructed.
- PCController PR #654 is mergeable but **BLOCKED** by three high CodeQL findings:
  a development ownership-file race and two CLI sensor-path findings. Its active
  Flutter owner was informed. No security gate was bypassed or dismissed, and
  its dependent PR #655 was not merged underneath ongoing owner work.
- Human buzzer/SFX listening and routed-output/hotplug acceptance, external
  physical cue-edge measurement, sustained playing/GPU/taskbar/soak checks,
  Web touch/parity work, RF/strip acceptance and applicable release acceptance
  remain open. Paused ACKs, source builds and static captures cannot prove them.

Issue #80 remains open until its actual acceptance ledger is demonstrated.
The Linux receiver receipt remains separate from acoustic fidelity; see
PCController's `docs/verification/buzzer-mirror-jitter.md`.
