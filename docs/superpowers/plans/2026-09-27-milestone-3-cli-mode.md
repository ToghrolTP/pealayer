# Milestone 3: CLI Mode and Launch Arguments

This milestone is complete. The CLI uses the same control origin as the web UI,
REST API, JSON-RPC API, and WebSocket service.

## Final contract

- `pealayer [OPTIONS] [FILE_OR_URL]` opens local or remote media.
- `--fullscreen`/`-f` and `--volume`/`-v` set startup presentation state.
- `--remote COMMAND` sends a command to the running instance.
- A second launch forwards its media target with
  `POST http://127.0.0.1:${PEALAYER_PORT:-8080}/api/ipc` and exits after the
  running instance acknowledges it.
- If no instance answers within the bounded connection timeout, the process
  starts the GUI and loads the requested target itself.

`PEALAYER_PORT` is the only TCP port setting. The client constructs a normal
HTTP request with an exact `Content-Length`, accepts only a successful HTTP
response, and preserves JSON command and JSON-RPC envelopes in the request body.

Coverage lives in `src/cli.rs`, `tests/cli_forwarding_test.rs`, and
`tests/interop_tcp_test.rs`.
