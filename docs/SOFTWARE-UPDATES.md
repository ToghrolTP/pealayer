# Mutual software updates

Pealayer and PCController keep ownership of their own installation, rollback,
configuration and restart. A gateway transports only explicitly selected software
operations. One reachable port can therefore update both programs without a
generic HTTP proxy, repeated mutations or a second hardware owner.

## Comparison and alignment

| Area | Pealayer | PCController | Integration |
| --- | --- | --- | --- |
| Self replacement | Verified same-directory helper, rollback, host-specific libmpv fence, graceful local Quit | Verified helper, durable transaction/health journals, service/interactive restart ownership | Retain each platform/process owner; never copy a foreign replacement implementation into a relay |
| Remote uploads | Existing HTTP begin/chunk/finish/abort | Immutable artifact store and chunked RPC uploads | Pealayer now exposes the same updater through RPC/native IPC/WebSocket; no second receiver |
| Acknowledgements | Previously advanced after any decoded status | Checks transfer ID, next offset, total and digest | Pealayer and the shared client now enforce exact acknowledgements; no blind mutation retries |
| Completion | Previously matched a running SHA before health commit | Distinguishes staging and healthy terminal replacement | Pealayer publishes `health_state`, stabilizes then requires a fresh UI-thread response before committing. Clients verify the running digest/size; controller installs additionally require the matching durable completed operation |
| Transfer storage | Hidden update scratch and separate runtime compatibility checks | Immutable content-addressed assets, quotas and expiring uploads | Controller update staging/helper/backup/journal paths gain Windows Hidden handling; installed/rolled-back executables are visible |
| Pull/copy | HTTP executable download | HTTP immutable artifacts | Bounded RPC reads pin verified files, avoiding full-file rehashes and local executable copies per chunk |
| Mutual routing | Remote-consumer gateway and connected controller RPC | Registered UI instances and guarded bridges | Explicit software target IDs/revisions; existing controller artifact APIs bootstrap older hosts, existing Pealayer HTTP updates bootstrap older players |
| Bundled candidates | Deployment-owned assets | Deployment-owned assets | Shared `software-bundle.json` format, product-scoped listing/read, confined paths and SHA/size verification; no directory scanning or implicit install |

Literal Go/Rust updater replacement code is not duplicated. Both gateway adapters
call the existing application owner. The dependency-free reference transfer client
is [scripts/software-update.mjs](../scripts/software-update.mjs); both ecosystems
use that one client, not separate upload/copy implementations.

## Routes and targets

Pealayer accepts the following JSON-RPC methods on `/api/rpc`, `/api/ipc`, `/ws`
and native IPC. They remain **process-local**, even in remote-consumer mode:

- `pealayer.update.manifest`, `status`, `artifact.read`
- `pealayer.update.upload.begin`, `chunk`, `finish`, `abort`
- `pealayer.update.from-url`
- `pealayer.update.bundle.list`, `bundle.read`
- `pealayer.update.targets`, `peer.call`, `controller.call`

`upload.begin` uses the existing HTTP request fields: `sha256`, `size`, optional
`version`, `platform`, `arch`, `libmpv_runtime`. RPC chunks carry `operation_id`,
`offset`, and base64 `data`, up to 384 KiB decoded. Finish/abort carry the
`operation_id`. `artifact.read` takes `sha256`, `offset`, `length`; its result
contains the same digest, `offset`, `next_offset`, `size`, base64 `data` and `eof`.

`peer.call` selects an advertised `authority:<instance_id>` and accepts only
direct updater methods. It never accepts another forwarding operation or an
arbitrary URL. `controller.call` selects `target: "controller"`, a permitted
controller software method and its `params`; a remote consumer sends that explicit
request to the controller of its bound authority, not an unused local scheduler.
The existing `/api/peer/controller` path remains available and now applies update
permission checks to software requests too.

PCController exposes `controller.software.targets` and `controller.software.call`
through its normal RPC interfaces. A call supplies:

```json
{
  "target": "app:REGISTERED_PEALAYER_ID",
  "revision": "REVISION_FROM_TARGET_CATALOG",
  "authorized": true,
  "request": {
    "jsonrpc": "2.0",
    "id": "update-status",
    "method": "pealayer.update.status",
    "params": {}
  }
}
```

The target is the primary controller (`self`, when it owns an artifact service)
or a live registered native Pealayer instance. Registered process/endpoint changes
invalidate its revision. Self calls reuse `controller.artifact.*` and
`controller.update.host/status`; app calls map to the existing Pealayer HTTP
updater. Read-only artifact/bundle operations use the new bounded RPC reader.
An embedded library without a primary artifact owner is not advertised as an
independently replaceable controller executable.

Registered software targets do not grant arbitrary third-peer routing. An existing
controller bridge can carry a validated `controller.software.call` for the peer's
own software/registered player, but nested software/bridge/board commands are
rejected. Remote forwarding requires the existing programming **and** bridge-call
capabilities. Pealayer Web transports require application-update permission;
native IPC retains its existing local-user boundary. No new placeholder login or
weakened origin policy is introduced.

Controller RPC downloads use `controller.artifact.download.begin` with
`kind: "host-executable"` and `sha256`; `download.chunk` takes `transfer_id`,
`offset`, `length`; `download.close` takes `transfer_id`. Verified file handles
are quota/TTL bounded, closed by the existing cleanup/Close lifecycle, and never
rehash the whole executable per chunk. Downloading never programs the board.

## Reference client

Run from the source checkout with Node 24 or newer. Examples use normal default
ports, not a particular installation. A reachable gateway must be able to reach
its selected registered peer; it does not establish a tunnel or bypass a closed
listener.

```powershell
node scripts/software-update.mjs targets --via http://localhost:8080 --gateway pealayer
node scripts/software-update.mjs targets --via http://localhost:8787 --gateway pccontroller
```

The list includes `self`, configured `authority:...`, controller-routed
`controller:self` / `controller:app:...`, registered `app:...`, and available
`bundle:...` / `controller:bundle:...` sources. Optional catalog failures remain
in the `errors` result; a disconnected controller does not hide local candidates.

Supply an updated executable and a matching manifest:

```json
{
  "service": "pealayer",
  "sha256": "REPLACE_WITH_THE_FILE_SHA256",
  "size": 123456,
  "platform": "windows",
  "arch": "x86_64",
  "version": "your-build-label",
  "libmpv_runtime": {
    "file_name": "libmpv-2.dll",
    "sha256": "REPLACE_WITH_THE_TARGET_RUNTIME_SHA256"
  }
}
```

These are placeholders, not valid artifact metadata. Use `service: "pccontroller"`
for a controller executable. Its platform may be `windows/amd64`, or separate
`platform` / `arch` fields; the client translates only architecture vocabulary.
Do not substitute a different host's libmpv merely to pass admission.

```powershell
node scripts/software-update.mjs push --via http://localhost:8787 --gateway pccontroller --target app:REGISTERED_PEALAYER_ID --file C:\Updates\pealayer.exe --manifest C:\Updates\pealayer-manifest.json
node scripts/software-update.mjs push --via http://localhost:8080 --gateway pealayer --target controller:self --file C:\Updates\controller.exe --manifest C:\Updates\controller-manifest.json
node scripts/software-update.mjs pull --via http://localhost:8787 --gateway pccontroller --target self --output C:\Updates\verified-controller.exe
node scripts/software-update.mjs copy --via http://localhost:8787 --gateway pccontroller --target app:REGISTERED_PEALAYER_ID --source-via http://localhost:8787 --source-gateway pccontroller --source-target bundle:player-candidate
```

Pull refuses an existing output rather than overwriting it. Copy streams between
the two selected owners without a local executable staging copy. Different
products cannot be copied onto each other. No automatic reapply is attempted when
the final acknowledgement is lost. The result remains accepted/outcome-unknown
until the exact healthy replacement is verified. Read-only verification may
refresh a restarted process's revision but never replays install commands.

### Bootstrap an older reachable application

Update the reachable program through its existing self-update API first if it
predates these explicit gateway methods. An older controller still exposes its
existing artifact upload/host-update APIs; an older Pealayer still exposes the
HTTP updater. Neither requires a second publicly reachable port.

For an older connected controller behind an updated Pealayer, explicitly select
`--target connected-controller`. This uses the established artifact RPC directly,
without silently trying a different schema. With an older controller directly
reachable, `--gateway pccontroller --target self` uses those same established
methods. After updating it, the registered-player gateway is available. A gateway
maps player updates onto the existing HTTP contract, so an older player can be
updated before it has these RPC methods. Unsupported operations fail clearly;
there are no build-number branches or unbounded general proxy fallbacks.

## Deployment bundles

Put this catalog beside the gateway executable, with explicitly packaged files
inside that same directory tree:

```json
{
  "format": "digitalogic-software-bundle",
  "assets": [{
    "id": "player-candidate",
    "path": "updates/pealayer.exe",
    "manifest": {
      "service": "pealayer",
      "sha256": "REPLACE_WITH_THE_FILE_SHA256",
      "size": 123456,
      "platform": "windows",
      "arch": "x86_64"
    }
  }]
}
```

The schema allows both products in one deployment, at most 16 assets and a
64 KiB catalog. Paths must be relative, confined, existing regular files.
Public catalogs omit filesystem paths. Reading a candidate verifies its size
and digest before exposing bounded bytes. Assets are offered for explicit choice;
timestamps or a different hash are never interpreted as proof of a newer version.
Bundled DLL replacement remains owned by the containing product package, not a
running-library unload or a controller executable update.

## Verification boundary

Focused native/Go/reference-client checks cover both owners, established bootstrap
transports, exact chunks, source/destination product fences, no mutation replay,
target revision/expiry, redirects, bounded pinned reads and confined bundles.
Current package, running-instance, cross-product restart and remote deployment
receipts belong in the linked verification ledger; source checks alone do not
establish those outcomes. Cafe remains user-deferred while offline.
