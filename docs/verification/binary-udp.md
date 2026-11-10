# Binary UDP host transport

Pealayer and PCController now share a COBS/CRC-8/ATM binary host datagram
contract. This is not raw UART access and firmware is unchanged. Both apps
retain their typed RPC methods, authority, permission and hardware safety gates.

The canonical wire/provisioning specification is
[PCController Binary UDP](https://github.com/atomicdeploy/PCController/blob/feat/mobile-live-controls/docs/BINARY-UDP.md).
Pealayer sources are `server/udp_codec.rs`, `server/udp_client.rs`, and
`server/binary_udp.rs`; native controller RPC and push bridges select them for
`udp://IP:port`. Discovery never auto-provisions a command target.

## Configure

Shared native/Web Preferences > Web UI and network APIs > Binary UDP supplies
enabled, listening address, discover/announce, and event-broadcast settings.
UDP is disabled by default. Configure named peers through local IPC:

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.udp.config.set","params":{"enabled":true,"listen":"127.0.0.1:8080","discovery":false,"broadcast_events":false,"peers":{"controller":"127.0.0.1:8787"}}}
```

`pealayer.udp.config.get`, `status`, `call`, `subscribe`, `unsubscribe` and
`events.get` are available through native IPC, `/api/rpc`, `/ws`, and `/api/udp`.
These operations are process-local even when this player is a consumer; they
must not configure/update the authority instead. Outbound calls name a
provisioned peer, carry its current process identity and are never automatically
retried after an uncertain acknowledgement. Existing authorized API clients can
route to configured peers; UDP ingress cannot chain a further route or persist
peer changes. Provisioning remains local. Discovery/event broadcasts contain no
actuating request.

Configuration acceptance is queued to the normal application dispatcher;
confirm its actual result through config/status. Bind failures are exposed and
retried with bounded backoff; no per-frame network/persistence/probe work is
added. State publishes reuse the current status publisher. Native controller
pushes use the existing source-epoch validation, snapshot installation,
hardware-revision invalidation and acknowledged action-delivery path.

## Evidence, not production acceptance

- Offline `cargo check` compiles the native backend and bridge integration.
- A pure Rust transport client queried the actual isolated Go Host dispatcher,
  received its fragmented 13,536-byte binary catalog and a pushed `udp_probe`
  event. No GUI, player, engine or physical board was started by the probe.
- The standalone fixture is `tools/udp-probe`; it imports the production codec,
client and firmware framing functions rather than implementing a second wire.
- Both encoders produce the identical 109-byte golden fixture for exact signed/
  unsigned 64-bit values, float, Unicode, null, boolean and collection values.
- Full tests/CI waits were skipped as requested. Source, loopback and compile
  evidence do not establish physical timing, sustained packet-loss recovery,
  installed UI or Android acceptance.
- Cafe/Erfan UDP activation has not occurred. Preserve host-specific libmpv,
  authoring drafts and the approved updater path; the Erfan updater denial in
  [paired deployment](paired-peer-deployment.md) remains unresolved.

Discovered UDP controllers appear in the native hardware endpoint menu for
explicit selection. They are not injected into the autoconnect candidate list;
an announcement never silently changes the board owner or selected coordinator.

## Standard LAN discovery alongside binary UDP

The shared Flutter design rejection is recorded in root `AGENTS.md`. Native
Pealayer now advertises its actual LAN HTTP listeners using `_pealayer._tcp.local.`
DNS-SD/mDNS, SSDP and MNDP, and browses both desktop apps and controller-mobile.
Loopback-only development/tests do not start discovery. Source-pinned bounded
health verification, a four-worker queue, throttled probes/replies and 120-second
peer expiry keep advertisements separate from verified peers. Own endpoints are
excluded; no discovery callback claims hardware authority or actuates it.

Read-only `/api/discovery`, `/api/discovery/peers` and
`pealayer.discovery.peers` (native IPC, HTTP and WS) describe this process, never
the consumer's authority proxy. Configured protocols, active local adapters and
real adapter errors are distinct. mDNS activity follows actual daemon announce
events. This is not proof of LAN round trips or physical hardware.

MNDP's Digitalogic extensions are `0xff01` product and `0xff02` a big-endian API
port. Its virtual MAC is application-scoped, not a fake board/NIC identity.
Generic RouterOS neighbours without an API contract are not assumed to expose
HTTP port80. Standard discovery currently uses LAN IPv4 (mDNS browsers can
resolve other families separately); routed MNDP and every-NIC/IPv6 acceptance
are not claimed. Firewall/multicast filtering can still prevent discovery.

`cargo check --offline -q` passed with existing deprecation/dead-code warnings.
The new mdns-sd 0.21.5 adapter requires six lockfile changes, including libc
0.2.190 through socket-pktinfo's minimum version. No SDK or parallel source
checkout was created. Installed Cafe/Erfan packages have not been replaced by
this source-only discovery change, and the Erfan updater denial remains binding.
