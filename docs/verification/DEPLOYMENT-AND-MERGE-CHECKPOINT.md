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
| Deploy Cafe and verify real playback state | Exact candidate installed through its own verified updater; saved paused position restored | Playback is blocked by the running PCController's missing media-clock RPC, not verified working |
| Deploy Erfan-Gaming | Destination runtime independently identified; installed and candidate smoke tests passed; candidate uploaded to canonical staging | Updater invocation policy-blocked; installed build is still the previous version |
| Consolidate useful work and merge safe PRs | Peer polling PR #82 and Vite PR #83 reviewed and merged; combined Preferences branch includes main plus exact PR #77/#79 heads | Remaining CI faults, experiment integration and old-ref/stash reconciliation |
| Keep D3D11 optional and disabled | Preserved experiment already defines renderer selection with OpenGL default and detached video disabled | Not integrated into main Preferences yet; do not activate or silently deploy the experimental executable |

## Cafe: installed, but playback acceptance failed

The live manifest identifies source `f49db607e51f5ea3d806f6c80570edc737c7f3ad`,
with a clean build and executable SHA-256
`fcd6b9dded88029d0220040591bfba3de5a241dd260b75c04c7c56107fe2a8bd`.
The package uses Cafe's own previously identified runtime, not David's DLL.
The updater verified the complete upload before replacement.

The old process acknowledged commands without applying them and did not exit.
A private minidump was preserved. After graceful IPC/HTTP Quit attempts failed,
the user's standing hung-process permission was used for that exact process.
Its existing verified update journal/helper was resumed in the signed-in
interactive session; the helper completed successfully. No manual overwrite of
a running executable occurred. The new process is in interactive session 1.

Saved media and its paused position restored. A Play request did **not** advance
the position: the hardware synchronization error is
`Hardware clock acknowledgement failed: method not found`.
An independent read-only call to the running PCController confirmed that
`controller.media.playback.get` is also absent. Current PCController source
implements the media playback/timeline methods. The deployed counterpart must
be reconciled through its bridge-owned update, without bypassing prepared-cue
arming or flashing firmware without a separate need/authorization. The user
was asked for direction on that host update. Paused-state restoration is proven;
successful play/pause/seek and precise board synchronization are not.

## Erfan-Gaming: staged, not delivered

The existing canonical installation and logged-in user were discovered through
the established SSH route. The current destination DLL stayed unchanged.
Both the old installed executable and the new candidate exited 0 in runtime
smoke tests. The candidate's full executable hash matches Cafe's candidate.
The old installed application was started in its signed-in interactive session
so its updater could be used. The candidate updater invocation was rejected by
execution policy before running; do not reroute that blocked invocation or claim
the upload/smoke checks constitute installation. The live manifest remains the
previous source `95be0c4c31d0263e1ec8d2a15c853d595c522aad`.

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

1. Obtain direction for the Cafe PCController bridge-managed replacement;
   verify the advertised current RPC contract, then real Play, advancing media
   time, Pause, stable paused time, seek, graceful reopen/restoration and board
   acknowledgement. Keep the original paused state after verification.
2. Complete Erfan's own updater through an allowed operator action and verify
   the live source/runtime identity, media decoding and controller behavior.
3. Diagnose Windows binary-test access violation; review/fix #84 and #86.
4. Merge the verified consolidation, then integrate D3D11 opt-in without
   discarding newer Preferences/media/hardware work. Audit old refs and stash
   hunks before closing superseded PRs or retiring branches.

Private dumps, settings, artwork and media references stay off public GitHub.
No Rust compilation/link occurred on Cafe. Existing source/history and useful
incremental caches remain preserved; blocked bulk cleanup was not retried.
