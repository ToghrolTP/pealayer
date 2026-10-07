# Recovered Windows-readiness work checkpoint

Tracking: [Pealayer handoff issue #80](https://github.com/ToghrolTP/pealayer/issues/80) and [Windows readiness issue #30](https://github.com/ToghrolTP/pealayer/issues/30).

This draft pull request preserves useful, previously stashed source changes for review. It is not ready to merge or deploy. The original stash `c42d0a67271e6dbe198f2beb5ce406dd0fbed3d8` remains untouched in the source repository. A private Git bundle copy is mirrored on Erfan-Gaming, independently verified at SHA-256 `AABEC063C4F7EA315D5A784134061293F782CC82198EA2D88AD5651198C55781`.

The recovered diff came from base `6a5b79cde8315ebde0a51680af4befaa522acace` (2026-09-28), which is an ancestor of current `main`. The checkpoint commits the stash's tracked source, tests, scripts, lockfile and documentation changes on that original base, while intentionally excluding the generated `web_ui/dist/**` output. This keeps the recovered code reviewable without re-publishing generated bundles.

The stash spans Windows build/package scripts, configuration and server interfaces, player/timeline and Web UI changes, tests, and dependency-lock changes. Because it is old and broad, compare each change with current `main`, the merged Windows-readiness PR #25, and the owner issues before selecting or porting it. In particular, review the removed `RemoteControl.tsx` module against today's Web UI before considering that deletion.

Only repository whitespace/diff checks have been run on this checkpoint. No Rust build or tests were run on CAFE-PC. CI results, compatibility with current `main`, and feature behavior remain to be evaluated on the successor's current branch/build environment. The intended next action is to review the source hunks, port any still-useful pieces onto current `main`, and close this preservation PR after that work is coordinated. Do not merge this stale-base checkpoint directly.
