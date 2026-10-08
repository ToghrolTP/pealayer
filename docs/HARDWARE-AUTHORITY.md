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

Connection notices distinguish **Remote Pealayer link interrupted**,
**PCController connection lost**, **Board unavailable**, **Board status update
failed**, and **Hardware command rejected**. They carry the actual transport or
controller error and shared warning icons. A stale peer link pauses only its
local preview; it never forwards Pause to the server. Reconnecting a monitor no
longer issues an implicit effect-stop command.
The updater uses a process-local shutdown command rather than forwarding Quit
to the authority. Session Quit still deliberately targets the remote server.

Verification and deployment results belong in the dated deployment checkpoint.
Do not infer that a source build, test, or staged package is installed.
