# Monitoring, publishing and production

Cafe can own hardware playback while David either connects to Cafe with
`pealayer://` or connects directly to the same PCController for monitoring.
A remote Pealayer consumer operates on the server's authority; it is not another
hardware scheduler. A direct client has its own registered host/port identity.

In Hardware Monitor, **Request authority** asks the current publisher for a
handoff. The owner receives a popup and can decline or pause and accept. After
acceptance, the new publisher prepares its own timeline and obtains a fresh clock
acknowledgement. Previously prepared acknowledgements are not reused.

**Release authority** requires paused playback. **Lock production** reserves
publishing and live output control exclusively; the owner uses **Unlock production**
before handing over. Locked observers can inspect status and activate E-STOP.
Unlocked observers can use manual controls while the prepared timeline is idle,
but cannot interfere with an executing timeline.

The same controls and conflict dialog exist in native and Web interfaces, using
one Rust command `pealayer.hardware.authority` and PCController's authority contract.
Production reservation is runtime-resident, survives clock expiry, but does not
survive a PCController restart in this implementation. It is not authentication.

## Unattended handoff and remote alternative

Preferences > Hardware > Publishing authority contains **Allow unattended
publishing handoffs** (`allow_unattended_hardware_takeover`), off by default.
This is the current owner's consent, not a requester's force-takeover permission.
When enabled, the libmpv observer pauses actual playback; only a fresh observed
pause and matching clock acknowledgement allow automatic acceptance. PCController
then completes its existing acknowledged output cleanup before transferring
ownership. Production lock always blocks the handoff. A failed acceptance is not
replayed automatically; playback remains paused and the user sees the real error.
The path is independent of modal rendering and works with the existing headless
playback observer too. Controller authentication/access policy remains required.

**Connect to authority** offers the other approach: use the publisher's Pealayer
session instead of becoming a competing hardware scheduler. Native opens the
existing remote-client connection dialog with the registered origin prefilled
and editable. Web navigates to the owner's Web application. Origins come from
the matching registered Pealayer instance and are validated against credentials,
paths, queries and unsupported schemes; display labels and client IDs are never
parsed into addresses. A loopback-only/disabled Web server advertises no remote
origin. Web then disables the action with an explanation; native allows manual
entry for a separately configured tunnel or route.

A remote consumer's Preferences belong to its server. To opt in a different
machine for its later direct-publisher role, edit that machine's own stored
configuration, not the consumer's forwarded Preferences. Enabling a consumer's
local policy does not grant takeover of its server or bypass production lock.

Connection notices distinguish **Remote Pealayer link interrupted**,
**PCController connection lost**, **Board unavailable**, **Board status update
failed**, and **Hardware command rejected**. They carry the actual transport or
controller error and shared warning icons. A stale peer link pauses only its
local preview; it never forwards Pause to the server. Reconnecting a monitor no
longer issues an implicit effect-stop command.
The updater uses a process-local shutdown command rather than forwarding Quit
to the authority. Session Quit still deliberately targets the remote server.
The consumer's `/api/update/*` endpoints likewise stay local; session APIs,
Preferences and hardware controls still relay to the authority. Inspect the
destination's actual runtime manifest before submitting an update.

Verification and deployment results belong in the dated deployment checkpoint.
Do not infer that a source build, test, or staged package is installed.

## API-first process control

Session control and process lifecycle are deliberately different. `quit` in a
remote consumer operates the authoritative session; `pealayer.process.quit`
gracefully closes only the process receiving the request. Never use session Quit
to restart or update a consumer.

`GET /api/process/status` reports the actual receiving process ID, executable
source/runtime identity, local control port, peer diagnostics, effective local
handoff permission, and any current connection operation. It is not a relayed
copy of the server's application identity. `/api/client/status` remains the
smaller local peer diagnostic snapshot.

For timing diagnostics, use its direct `playback_clock` and `hardware_sync`
objects. `hardware_sync.ack_age_ms` is computed from the executor's monotonic
ACK timestamp **when requested**, not when the UI last refreshed. It includes
the current prepared/clock revision and epoch, reprepare/error state, clock
transport counters and an identity-free subset of controller timeline evidence.
Controller `clock_timing` separates feedback intervals from position corrections;
worker-gap, clock-read and board-read delays distinguish execution starvation
from clock updates. Inspect these together with dispatch/ACK lateness.
Counters and maximum delays are cumulative for this process; compare before and
after a bounded test rather than attributing an old maximum to a new test.
This read uses a nonblocking lock: `state: busy` or `unavailable` does not mean
healthy/empty hardware. Before registration the value is null. This is diagnostic
evidence, not permission to bypass the executor's fresh-ACK or safety checks.
`/api/player/status` is a cached UI presentation; do not use its cached ACK age
as an authoritative live preflight gate. No extra heartbeat repaints are needed.

Send `pealayer.process.connect` through native IPC, `/api/rpc`, `/api/ipc`,
`/api/process/command`, or `/ws`. All paths use the same validated Rust command;
the native connection dialog uses it too. For example:

```powershell
$request = @{
    jsonrpc = '2.0'; id = 1; method = 'pealayer.process.connect'
    params = @{
        operation_id = [guid]::NewGuid().ToString()
        endpoint = 'pealayer://publisher.example:8080'
        client_port = 8080
    }
} | ConvertTo-Json -Depth 4
Invoke-RestMethod http://127.0.0.1:8080/api/process/command `
    -Method Post -ContentType application/json -Body $request
```

The response acknowledges dispatch, not a completed role change. Poll local
process status. The connection operation checks the current peer session and
rejects self-connections before closing anything. An existing healthy connection
to the same origin/port is a no-op. A direct publishing owner must already be
paused and production-unlocked; the controller must acknowledge publication
release. Raw serial hardware must be disconnected first. Observers do not
release another client's authority.

Changing role uses a graceful process restart so the existing decoder, event
loop and hardware engine are not duplicated. A helper waits for the current
process to exit; it never force-kills it or writes replacement files. Failed
initial consumer startup restores the previous command-line mode. Repeating an
operation ID within the receiving process does not restart it again; changing
the payload under the same ID is rejected. After restart, verify the new PID,
peer server, connected/fresh sample and `local_hardware_scheduler: false`, not
just the earlier accepted response. Loss of network connectivity is not proof
that physical hardware was unplugged.

`pealayer.process.status` is the equivalent RPC/native query. Process commands
remain local even when submitted to a consumer's ordinary RPC/IPC/command
endpoint. They cannot be nested in a forwarded session launch. The dedicated
process command endpoint rejects session commands; Web mutations require the
existing Web control permission and origin checks. Consumer Preferences and
session configuration still belong to the server. This is not a new authentication
or production-lock bypass.
