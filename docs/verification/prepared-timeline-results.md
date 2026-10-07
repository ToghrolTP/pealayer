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
| Running Pealayer cue | Plan/paused-clock ACK before play; two ordered R8 native ACKs; maximum **3.4795 ms** host-clock lateness | Canonical Pealayer plus PCController plus updated VirtualBoard |
| Bounded CPU contention | Same real Pealayer cue, two CPU worker threads; two ordered ACKs; maximum **3.5406 ms** | This local machine only, not a low-end laptop/GPU saturation benchmark |
| Paired playback clock | Pause/seek/play/2x rate, front-panel time and 67 subscribed playback events passed | Independent libmpv observer and updated VirtualBoard |
| Final local deployment | Pealayer `2edfd68`, SHA-256 `d52a7289c234e501912b9c9437a0a2297a69338717ef68635d0914338b6e8349`; PCController `27a90ab8`, own update `op-c01669571f89e51f`, terminal verified | Product-owned updaters; canonical running paths, no manual executable replacement |

The first paired playback check after PCController restart failed correctly: its
stored endpoint reconnected to the older VirtualBoard on port 8876, which rejects
the media-clock opcode. Reconnecting through `controller.connect` to the updated
VirtualBoard on port 8896 made the full check pass. Neither process was killed.
This does not prove an old/unsupported physical board is synchronized.

The paired cue check reuses an existing controller-owned effect whose complete
definition is exactly native relay index 7 ON followed by OFF one second later.
It adds a temporary Pealayer cue two seconds after the current paused position,
checks preparation before play and both native ACKs, removes that cue, then
restores the original paused position/rate. It never changes the effect library:

```powershell
node docs/verification/verify-pealayer-prepared-cue.mjs
```

The canonical Pealayer update preserved paused media at 1256.089 seconds, rate 1,
and the host-specific libmpv DLL hash. Cafe-PC's own-update endpoint was
unreachable; no deployment there was verified in this pass. No PR was merged.

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
