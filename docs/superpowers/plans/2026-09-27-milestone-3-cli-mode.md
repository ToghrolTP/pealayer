# Milestone 3: CLI Mode and Launch Arguments

This milestone is complete. The CLI uses the same control origin as the web UI,
REST API, JSON-RPC API, and WebSocket service.

## Final contract

- `pealayer [OPTIONS] [FILE_OR_URL]` opens local or remote media.
- Essential playback, seek, volume, mute, speed, workspace, and window-management
  switches are translated to the same typed command model as the APIs.
- `--command COMMAND` queues a text or JSON command; `--remote COMMAND` sends one
  directly to the running instance and exits.
- Single-instance mode is persisted in Preferences and enabled by default.
- A second launch forwards its target and ordered command list through a Windows
  named pipe or Unix-domain socket. Loopback `POST /api/ipc` is the bounded
  compatibility fallback, and the process exits only after acknowledgement.
- If no instance answers within the bounded connection timeout, the process
  starts the GUI and loads the requested target itself.

`PEALAYER_PORT` remains the network automation port setting. Native IPC is
scoped to the current Windows session/application identity or the configured
Unix socket path. Every transport validates the same command ranges, while
`GET /api/player/commands` exposes the current living command catalog.

Coverage lives in `src/cli.rs`, `tests/cli_forwarding_test.rs`, and
`tests/interop_tcp_test.rs`.
