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
