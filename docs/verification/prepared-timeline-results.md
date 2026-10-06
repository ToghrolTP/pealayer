# Prepared timeline acceptance checkpoint — 2026-10-06

| Check | Evidence | Scope |
| --- | --- | --- |
| Go media/E-STOP control and IPC admission tests | Project-owned stable-path runner passed | Automated wire fixtures, not hardware |
| Rust revision/clock-ACK playback gate | 2 focused tests passed | No GUI/native-window test |
| Web production/PWA build | TypeScript and existing wheel/time/messaging checks; installable/offline PWA verification passed | Build validation, not browser visual acceptance |
| PCController native package | Host executable and C ABI build/smoke passed | Windows local staging |
| Own updater restart | `op-1a350a5f19e1cafe`, `terminal_verified=true` | Running local PCController |
| Prepared relay sequence | 2 ordered native ACKs, maximum host-clock ACK lateness **2.1618 ms** | Running PCController + explicitly checked VirtualBoard |
| Deliberately overdue cue | 802.1 ms late; 0 successful actions; retained deadline fault | Same VirtualBoard; no late command sent |
| Test cleanup | Empty plan prepared; test identity removed; Pealayer automatic connection restored | Original paused media was not changed |

Reproduce only with a connected updated VirtualBoard, paused media, no cues, and
automatic hardware connection enabled:

```powershell
node docs/verification/verify-prepared-timeline.mjs
```

The script refuses physical-board actuation. Its relay targets are native
zero-based indices (7 corresponds to displayed R8). It temporarily disconnects
the Pealayer consumer through the configuration API, registers an explicit
verification identity, prepares both relay edges before starting the clock,
checks ordered ACKs and late-cue rejection, then restores the connection.

These measurements are relative to the received/extrapolated host clock. They
are not physical output-edge or video-frame measurements. Heavy CPU/GPU load,
physical-board playback, and absolute firmware-epoch queue scheduling remain
unverified acceptance gates. Do not describe this checkpoint as perfect sync.
