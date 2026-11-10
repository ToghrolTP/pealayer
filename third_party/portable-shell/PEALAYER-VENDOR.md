# PortableShell source receipt

Upstream: https://github.com/atomicdeploy/portable-shell

Resolved canonical main: `3e83e8b717c58048e73507fbba8da006bef16260` (merged PR #87,
including PRs #85 and #86), including the verified
choice popup, optional human-label/key presentation, F6 decoding, explicit
semantic accent protocol, prompt-safe asynchronous blocks/reports and the
existing terminal-profile import, shared Unicode frames and functional framed
dialogs. The private `src/portable_message.inc` is
included from the same owner checkpoint.
`include/`, `src/`, `host/terminal_profiles.c`, `LICENSE` and
`docs/Attribution.md` are pristine Git blobs
from that commit, not copies of the upstream working tree. See LICENSE for
copyright and MIT terms; attribution is retained without bundling preview assets.

This is a reproducible source receipt, not an indefinite dependency pin. During
development follow canonical upstream main and refresh at integration checkpoints.
Review the owner's changes, preserve any local changes, apply the complete Git
delta to the consumed file set above, and compare every resulting Git blob with
the resolved upstream commit (including newly added private includes). Re-run
Pealayer's real editor/dispatch/VT/transport tests and authorized package/runtime
acceptance before installing. Do not update sources automatically during a build
or substitute an upstream worktree's uncommitted files. Release receipts retain
their exact tested identity; they do not block the next reviewed refresh. See
the owner's [integration policy](https://github.com/atomicdeploy/portable-shell/wiki/Integration).

Pealayer compiles `src/portable_shell.c` with `PS_WORKBENCH=0` when
`workspace-terminal` is enabled. Separately, it compiles the portable profile
parser and its bounded read-only native discovery adapter. No workbench, demo commands, OS process host,
CRT replacement, or external PortableShell executable is linked. Its Rust
adapter lives separately in `src/ui/portable_shell_host.rs` and routes typed
actions to Pealayer's existing dispatch outside C callbacks. Disconnect scrubs
the owned session; it never executes an unterminated command with `ps_finish`.
