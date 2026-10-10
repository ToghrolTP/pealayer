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

Windows launch and packaging reject a `CARGO_TARGET_DIR` located inside the
source checkout. Repeated publication hashes existing payloads first and
hard-links immutable libmpv assets when the filesystem permits, instead of
rewriting identical large files. `scripts/storage-hygiene.ps1` provides a
read-only inventory by default; its explicit `-PruneRepositoryTarget` mode only
accepts the exact ignored `target` child, refuses unknown evidence, and refuses
to remove an executable used by a running process.

This relocation is not a claim that other historical caches or worktrees have
been deleted. Preserve their owners and follow the older inventory below before
any separate cleanup.

## Routine compiler policy (11 October 2026)

The workspace root explicitly selects `debug = 0` and `incremental = false` for
dev, test and release. Release already used those compiler defaults and strips
symbols. Routine dev/test builds no longer produce full PDB/DWARF symbols or
rustc incremental snapshots. Debug assertions, overflow checks and test panic
behavior are unchanged. Reusable dependency artifacts remain in the one canonical
Cargo cache: disabling rustc incremental snapshots is not `cargo clean`.

Existing shared debug/dependency artifacts are preserved. The next dev/test build
will have a one-time fingerprint transition to the smaller profile; do not trigger
a whole rebuild merely to validate these settings. `cargo metadata --offline
--locked --no-deps`, `node scripts/check-build-policy.mjs` and
`pwsh -NoProfile -File scripts/test-storage-hygiene.ps1` validate configuration and
cleanup guards without compiling the application. CI checks the policy and the
Windows fixture tests.

Only enable symbols for an essential diagnosis, such as a reproducible native
teardown fault. Supply `CARGO_PROFILE_TEST_DEBUG=2` (or
`CARGO_PROFILE_DEV_DEBUG=2`) to that single child build, retain the same canonical
target directory, and remove the override afterwards. Do not persist a global
debug/incremental override or allocate a second diagnostic cache.

`scripts/storage-hygiene.ps1 -PruneRepositoryDebug -WhatIf` previews deletion of
only repository-local `target\debug` and the two known Windows target-specific
debug children. Its explicit destructive invocation omits `-WhatIf`. It verifies
Git ignore state, unknown root evidence, the shared Cargo cache tag, Cargo's actual
effective target directory, every selected path and nested link, and active
compiler/target-executable ownership. Any failed gate refuses deletion. This mode
does not remove release caches, packages, shared dependencies or user state. The
older `-PruneRepositoryTarget` mode remains a separate explicit whole-target
operation; it is **not** appropriate when release output must be retained.

A RAM disk is optional transient storage, not a replacement for the durable shared
dependency cache. Measure available physical RAM and actual temporary-build high
water marks before choosing its size; installed RAM alone is not available RAM.
Do not allocate an 8 GiB RAM disk when only roughly 7–9 GiB is currently free. A
small 1–2 GiB scratch disk may be a starting point only after measuring that the
selected temporary workload fits with system headroom; a larger one needs freed
RAM and measured justification. Keep durable Cargo dependencies on disk and do
not redirect global TEMP, other projects or production machines. No RAM disk or
automatic cache relocation is installed by this change.

## Verified debug-only cleanup (11 October 2026)

The older canonical checkout's `target\debug` (103,750 files, 174.950 GiB
logical) and `target\x86_64-pc-windows-msvc\debug` (5,945 files, 4.107 GiB)
were permanently removed after the dry run and ownership/path gates passed.
Their latest file writes were 7 October and 1 October UTC respectively, before
the canonical-cache migration. They contained retired generated compiler output,
not unique source. These cached artifacts are reconstructible by compiling;
deletion is not an archive or rollback of the old debug binaries.

C: free space measured 33.075 GiB before and 201.063 GiB after this operation,
a net recovery of 167.988 GiB. This is a live filesystem measurement, not the
logical-size sum; hardlinks and unrelated concurrent disk activity can differ.
Protected pre/post file counts and summed byte lengths matched exactly:

- Repository-local release: 21,250 files, 12,281,718,681 bytes.
- Repository-local package output: 3 files, 281,911,808 bytes.
- Canonical shared Cargo cache: 41,178 files, 36,497,684,039 bytes.

Installed binaries, settings, rollback and other owners' work were not selected.
No application compile/link, runtime replacement, `cargo clean`, new worktree or
RAM-disk allocation was performed. The existing shared debug (18.780 GiB), shared
release (8.169 GiB), GNU debug (6.264 GiB) and dependency sets remain available.
The PCController audit found about 2.894 GiB of reusable Go build cache, 0.443 GiB
of stable-path tests and no files in guarded `go-noexec-temp`; those were retained,
including the temporary directory's deny-execute ACL. Its production build already
strips symbols. No PCController build-policy change or rebuild was needed.

RAM was 63.79 GiB total with roughly 7.40 GiB free in a contemporaneous sample.
No new compiler run was made to measure peak compiler RAM or temporary-output
high-water marks. Idle empty temp directories do not prove a 1 GiB RAM disk fits
a build. Refresh headroom and measure a future necessary build before allocating
scratch RAM, rather than repeatedly rebuilding just to obtain that measurement.

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
3. Reuse the dependency cache on David-PC or use CI; routine compiler incremental
   snapshots are disabled. Never compile/link Rust on CAFE-PC.
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
