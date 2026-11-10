# Unified control-port checkpoint

The [original checkpoint](https://github.com/ToghrolTP/pealayer/commit/d2fd6593d9dc325b5f61b422ccfc0d98f0b5c89b) is reachable from `restored/unified-control-port`. The `archive/unified-control-port` tag also points to that exact commit.

[PR #50](https://github.com/ToghrolTP/pealayer/pull/50) unified HTTP, WebSocket and loopback IPC on one port. Its merge commit `6e74d0b` has the same tree as the checkpoint. The unified WebSocket route is covered by `status_is_unknown_until_first_authoritative_snapshot` in `tests/web_media_explorer_test.rs`. The controller engine still keeps a requested catalog refresh pending while disconnected, as introduced by the checkpoint.

The restored branch uses the current server, transport tests and generated Web assets. The original files remain available from the checkpoint. This reconciliation was verified from Git history and current source.
