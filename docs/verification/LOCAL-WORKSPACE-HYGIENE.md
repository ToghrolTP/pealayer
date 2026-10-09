# Pealayer working locations and low-write workflow

The user requested canonical working locations, fewer duplicate worktrees and
executables, and reduced unnecessary SSD writes on 2026-10-08.

## Working locations

- Canonical source: `%LOCALAPPDATA%\Programs\Pealayer\source\Pealayer`.
- Installed application: `%LOCALAPPDATA%\Programs\Pealayer\bin`.
- Shared Windows Cargo cache: `build-cache\cargo-target` under that program
  root, selected by the shared Windows resolver and generated Cargo settings.
- Reusable host profile/import library: `config\windows-build-host.json` and
  `build-dependencies\libmpv` under the same program root.
- Candidate packages: `staging\preferences-organization`; reuse the directory
  after validating ownership instead of creating another dated executable copy.
- Private screenshots/diagnostics: `verification` under the program root. Do not
  commit private host state, media references, runtime DLLs or user artwork.

`C:\development` is reserved for development tools, not Pealayer-owned source,
runtime files or caches. Do not recreate a checkout or target directory there.

Before adopting a new working location, check the actual Git worktree ownership,
branch, dirty state, remote reachability and active owners. Do not overwrite an
existing canonical checkout's branch or remove another owner's worktree.

## Canonical relocation completed (9 October 2026)

The clean, detached build checkout formerly at `C:\development\Pealayer` was
relocated with Git's worktree move to `source\checkpoints\build-checkpoint`
under the program root. Its Git identity, tracked files, npm dependencies and
effect-guide PDF were preserved; no new checkout or duplicate copy was created.
The existing Cargo target directory was moved separately to the shared canonical
cache using a same-volume directory rename. Its last built executable digest
is unchanged, and the old development directory no longer exists. Git metadata
and user write access were repaired for the relocated directories without
resetting branches or changing global safe-directory exceptions.

Both Windows runner and packager now resolve one shared default cache path;
intentional `CARGO_TARGET_DIR` overrides remain available. The host profile and
generated Cargo settings for canonical source and the current release worktree
were refreshed without a build or test run. No global Cargo cache override was
set, avoiding interference with other Rust projects. Running David/Cafe binaries,
media, controller service and settings were not replaced for this path-only
change. No symlink or compatibility alias was left in `C:\development`.

This relocation is not a claim that other historical caches or worktrees have
been deleted. Preserve their owners and follow the older inventory below before
any separate cleanup.

## Historical cleanup checkpoint (8 October), not cleanup completion

Read-only audit found six registered worktrees, all clean and with no commits
unique from origin. Two current-task worktrees remain under
`Documents\Codex\2026-09-27\ple-2`: `cafe-shell-verification` and
`pealayer-directcomposition`. Their useful source is pushed. Ignored content is
generated Cargo output, resolver-generated `.cargo` settings and npm dependencies.

The canonical checkout also contains older Cargo `target\debug` (about 175 GiB
logical) and `target\release` (about 11 GiB logical). A newer reusable incremental
cache is currently in the Codex experiment worktree. Counts are logical file
sizes, not a promise of physical recovered space: some artifacts are hardlinks.
Preserve useful `target` evidence and the newer incremental cache before deleting
old generated products. Keep compiler dependency caches that avoid recompilation.

The bulk relocation/deletion operation was blocked by execution policy before
any changes. Do not claim it completed or reroute the blocked operation through
another shell/tool. Existing screenshots, EEPROM evidence, source and packages
remain intact. The newly generated screenshots use the canonical verification
directory, and the next candidate package uses canonical staging.

## Reuse instead of rediscovery

1. Read `agent-handoff.md`, this file and the owning issue checkpoint first.
2. Reuse the shared Windows host resolver; do not re-download libmpv each pass.
3. Build incrementally on David-PC or use CI. Never compile/link Rust on CAFE-PC.
4. Validate destination runtime identity, package smoke and actual media separately.
5. Deploy through the peer updater with graceful IPC quit; a local staged build is
   not Cafe deployment. Do not substitute David's DLL to bypass a mismatch.
6. Audit/push useful changes before handoff. No new worktree is needed just to
   render Preferences or run a focused test. Refresh historical hashes/health.

Cafe's first runtime download stalled, then a bounded HTTP range resume completed
it without re-downloading the entire DLL. Its final size/hash matched the live
peer. Partial downloads are invalid until independently checked. The identical
import-library hash allowed the David-built executable to be smoke-tested with
Cafe's actual runtime in separate canonical staging. Hardlinks avoided copying
that executable/DLL again. This does not imply the workstation's default runtime
profile was changed; it was not.

The first peer update stalled at graceful shutdown. The subsequent pass preserved
a private hang dump, used the user's standing permission for the exact hung
process after IPC/HTTP Quit failed, and resumed the already verified native
update helper in the signed-in desktop session. The candidate is now installed;
its live manifest and saved paused position were verified. Subsequent primary-
owned PCController and protected firmware updates restored media-clock support
and actual playback. A fresh-revision replay fix was built incrementally on
David and installed through Cafe's updater. Do not rebuild on Cafe or replace
its validated DLL. See
[deployment and merge checkpoint](DEPLOYMENT-AND-MERGE-CHECKPOINT.md) for the
remaining measured cue-timing failure, David's cache-only consumer and Erfan's
KMPlayer launch constraint. Installation and smoke tests must not be substituted
for successful playback or physical-output acceptance.
