# Milestone 2: Unified Cross-Platform IPC

This milestone is complete. The earlier pre-1.0 design for a dedicated raw TCP
IPC listener has been removed so this record describes the shipped contract.

## Final architecture

Pealayer exposes one TCP control listener, selected by `PEALAYER_PORT` and
defaulting to `127.0.0.1:8080`:

| Capability | Endpoint |
| --- | --- |
| Web UI and REST | `http://127.0.0.1:8080/` |
| JSON-RPC 2.0 | `POST http://127.0.0.1:8080/api/rpc` |
| CLI and single-instance IPC | `POST http://127.0.0.1:8080/api/ipc` |
| WebSocket | `ws://127.0.0.1:8080/ws` |

The shared listener routes HTTP requests and WebSocket upgrades before mapping
control payloads into `InteropCommand`. It binds to loopback unless
`PEALAYER_WEB_BIND` explicitly selects another interface. Unix builds may also
expose a native domain socket; it is not another TCP service.

## Contract

- `PEALAYER_PORT` is the only TCP port setting.
- `/api/ipc` accepts Pealayer command JSON and JSON-RPC envelopes over HTTP.
- `/api/rpc` is the canonical JSON-RPC route for general automation.
- `/ws` carries live state and bidirectional control on the same origin.
- Bind, parse, and client errors are reported without terminating the UI.
- The listener, CLI forwarding, and WebSocket upgrade are covered by integration
  tests that allocate isolated control ports.

See the root README for the current request and response examples.
