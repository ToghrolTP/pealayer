# PortableShell source receipt

Upstream: https://github.com/atomicdeploy/portable-shell

Snapshot: `5993a67a27862addf891d1fd7aac97f92cee6323` (merged PR #86;
verified feature checkpoint `087587986eb78047154784e0c7d6bc5ce83cb989`;
includes merged PR #85 at `4a9fced1f4d460c0fd36343a572c701455e73c86`), including the verified
choice popup, optional human-label/key presentation, F6 decoding, explicit
semantic accent protocol, prompt-safe asynchronous blocks/reports and the
existing terminal-profile import. The private `src/portable_message.inc` is
included from the same owner checkpoint.
`include/`, `src/`, `host/terminal_profiles.c`, `LICENSE` and
`docs/Attribution.md` are pristine Git blobs
from that commit, not copies of the upstream working tree. See LICENSE for
copyright and MIT terms; attribution is retained without bundling preview assets.

The owner reports 61 passing full tests, feature-disabled builds and MSVC ASan.
Those upstream receipts do not replace Pealayer's consumer and runtime checks.

Pealayer compiles `src/portable_shell.c` with `PS_WORKBENCH=0` when
`workspace-terminal` is enabled. Separately, it compiles the portable profile
parser and its bounded read-only native discovery adapter. No workbench, demo commands, OS process host,
CRT replacement, or external PortableShell executable is linked. Its Rust
adapter lives separately in `src/ui/portable_shell_host.rs` and routes typed
actions to Pealayer's existing dispatch outside C callbacks. Disconnect scrubs
the owned session; it never executes an unterminated command with `ps_finish`.
