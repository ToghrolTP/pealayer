# Protocol and API

The authoritative board/host and host/host contract belongs to PCController.
Do not copy protocol tables into this Wiki: duplicated tables drift.

- [Protocol and routing authority](https://github.com/atomicdeploy/PCController/issues/366)
- [Generated compatibility baseline](https://github.com/atomicdeploy/PCController/issues/377)
- [JSON/CBOR representations](https://github.com/atomicdeploy/PCController/issues/378)
- [Embeddable host API](https://github.com/atomicdeploy/PCController/issues/372)
- [Native local IPC](https://github.com/atomicdeploy/PCController/issues/373)

Pealayer currently uses PCController JSON-RPC snapshots and peripheral catalogs,
plus exact-target action delivery over the `:8787` WebSocket fallback. It accepts
both `controller.state` and `controller.event`, rejects expired deliveries with
an explicit reason, deduplicates receipts, and reports applied only after the UI
executes the player command.

Pealayer's own local command interfaces converge on typed player commands. New
interfaces must preserve the same validation and completion semantics.
Contract evolution is additive and capability-negotiated; readers ignore fields
they do not use. Numbered schema or protocol identities are intentionally not
used as a compatibility substitute.
