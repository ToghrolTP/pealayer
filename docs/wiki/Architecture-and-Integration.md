# Architecture and integration

Pealayer has one player state path. Desktop controls, command-line forwarding,
the Web UI, media controls, and targeted PCController actions all become typed
player commands before application state changes.

PCController is the normal serial owner. Pealayer requests snapshots,
peripheral names, capabilities, commands, and macros from PCController and
renders only what was advertised. Direct serial remains an explicit diagnostic
mode and continuously yields if PCController becomes available.

## Transport order

The target integration order is:

1. linked or bundled PCController host when the supported library is present;
2. an advertised same-machine native endpoint (Windows named pipe or Unix-domain socket);
3. the existing `127.0.0.1:8787` HTTP/WebSocket service;
4. explicit direct-serial diagnostics only.

Native endpoints and library symbols must come from PCController discovery and
living, additive manifests. Pealayer does not guess pipe/socket names or fork protocol
semantics. Follow the active contracts in
[PCController #372](https://github.com/atomicdeploy/PCController/issues/372),
[PCController #373](https://github.com/atomicdeploy/PCController/issues/373), and
[Pealayer #32](https://github.com/ToghrolTP/pealayer/issues/32).

## UI contract

The UI is driven by authoritative state, capability, and permission. It uses
progressive disclosure, stable focus/layout, and explicit pending/degraded/error
states. Permanent UI copy describes actions and current state, not internal
architecture or future specifications.
