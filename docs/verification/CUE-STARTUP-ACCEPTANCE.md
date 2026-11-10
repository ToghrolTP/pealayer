# Cue timing and startup service

## Delivered and checked

PCController is a real delayed-automatic Windows SCM service using its dedicated
service account and owned data root. Restarting only that service kept the same
Pealayer process, exact paused position and all four current cues. The physical
board reconnected and a fresh timeline/paused clock was acknowledged and armed.
Installation and restart recovery passed; this was not a production reboot test.

A CAS-isolated Relay 5 cue held ON for one second, then switched OFF. Both
native board acknowledgements met the unchanged 50 ms total lateness limit:

| Measurement | Observed maximum |
| --- | ---: |
| Host dispatch lateness | 10.01 ms |
| Board request round-trip | 7.00 ms |
| Total host-clock ACK lateness | 17.01 ms |

The original timeline and paused position were restored through existing APIs;
board feedback confirmed Relay 5 OFF. Motion and Relay 8 were excluded. Focused
controller timeline and Windows-service tests passed through the stable-path
runner. A 20-second paused telemetry sample remained fault-free, not a load soak.

## Repeat safely

Confirm the output is safe before testing. Current authorization permits only
Relay 5–7. Run against the authority's API origin:

```powershell
node --test docs/verification/verify-live-cue.test.mjs
node docs/verification/verify-live-cue.mjs http://authority:8080 5 --preflight
node docs/verification/verify-live-cue.mjs http://authority:8080 5
```

The script refuses active/unsampled outputs, unloaded/playing media, unarmed
clocks and authority conflicts. It derives the channel key from the advertised
relay catalog, isolates one ON/OFF cue from the original cues/analog curves and
checks both ACKs. Restoration uses compare-and-swap, seeks back to the original
paused position and waits for the original plan to re-arm. Lost mutation ACKs
are inspected, never replayed. Concurrent edits cause a pause and an explicit
manual-recovery error, not an overwrite. Network loss cannot prove outputs OFF;
attend to the actual board if recovery fails. Interruption requests cleanup.

Service restart uses the existing `controller service restart` command. Require
paused media/inactive outputs first; afterward check the same Pealayer PID,
unchanged timeline/position, real board connectivity and matching prepared and
clock-ACK revisions. Never install another controller to perform this test.

## Remaining limits

This is **real-board device-ACK acceptance**, not external physical output-edge
or video-frame measurement. Host-prepared execution does not supply an absolute
firmware media-epoch queue start/seek/rate/lease contract. Physical instrumentation,
sustained loaded low-end playback and optional actual reboot verification remain
explicit gates. Do not describe ACK or VirtualBoard success as perfect sync.

[Technical checkpoint](https://github.com/ToghrolTP/pealayer/issues/80#issuecomment-6080215490).
