# PortableShell source receipt

Upstream: https://github.com/atomicdeploy/portable-shell

Snapshot: `512c1c6ea24865ff9fcdcbbd594a5f5d80653e46`, which retains the core
from `76685c29dc56842ea86e0a0e8dbf2cdb71b50c88` and adds terminal-profile import.
`include/`, `src/`, `host/terminal_profiles.c`, `LICENSE` and
`docs/Attribution.md` are pristine Git blobs
from that commit, not copies of the upstream working tree. See LICENSE for
copyright and MIT terms; attribution is retained without bundling preview assets.

Pealayer compiles `src/portable_shell.c` with `PS_WORKBENCH=0` when
`workspace-terminal` is enabled. Separately, it compiles the portable profile
parser and its bounded read-only native discovery adapter. No workbench, demo commands, OS process host,
CRT replacement, or external PortableShell executable is linked. Its Rust
adapter lives separately in `src/ui/portable_shell_host.rs` and routes typed
actions to Pealayer's existing dispatch outside C callbacks. Disconnect scrubs
the owned session; it never executes an unterminated command with `ps_finish`.
