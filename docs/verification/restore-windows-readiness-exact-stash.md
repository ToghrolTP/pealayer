# Exact Windows readiness stash

The [original stash commit](https://github.com/ToghrolTP/pealayer/commit/c42d0a67271e6dbe198f2beb5ce406dd0fbed3d8), including its original base, index and empty untracked tree, is reachable from `restored/windows-readiness-exact-stash`. The `archive/windows-readiness-exact-stash` tag points to the same commit.

The stash contains Windows resource and packaging work, isolated screenshot profiles, authoritative startup state, and hardware-capability-driven controls. [Windows packaging PR #25](https://github.com/ToghrolTP/pealayer/pull/25) and [the readiness integration commit](https://github.com/ToghrolTP/pealayer/commit/3b9670cd2dc0d358710a786fa09a75626287ddf7) carry that work in `main`.

The original generated Web bundle remains in the stash. The restored branch uses the current bundle and npm lockfile. Packaged Explorer drag-and-drop and Win32 lifecycle acceptance are tracked in [#30](https://github.com/ToghrolTP/pealayer/issues/30) and [#34](https://github.com/ToghrolTP/pealayer/issues/34).
