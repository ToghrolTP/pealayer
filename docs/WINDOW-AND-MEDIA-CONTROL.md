# Window and media automation

Pealayer has one typed command dispatcher for native IPC, HTTP JSON, JSON-RPC,
WebSocket, newline-delimited JSON over TCP, the existing binary UDP transport,
and Windows `WM_COPYDATA`. Discover supported methods and actual native IPC
addresses at `GET /api/player/commands`. No transport bypasses media/hardware
authority, file-access validation or configured network-control permissions.

## Window control is process-local

`GET /api/window`, typed `get_window`, and RPC `pealayer.window.get` return the
addressed process's observed window state, never its remote authority's window:

- `available`, `process_id`, `revision`, `error`, `native_handle` (Windows only).
- `minimized`, `maximized`, `fullscreen`, `focused`.
- `outer_bounds`, `inner_bounds`, `normal_bounds`, `pixels_per_point`, `units`.

Coordinates are **egui logical points**, not raw physical pixels. Position is
the outer-window origin; width/height set the **inner client** size. Negative
positions are allowed for secondary displays. Windows readback samples native
geometry and minimized/maximized flags even while the UI is minimized. Current
bounds are null while minimized; `normal_bounds` retains usable restore geometry,
not Windows' offscreen minimized sentinel. `available: false` identifies a
headless/not-yet-created native window rather than inventing a rectangle.

Use `POST /api/window` with the placement object, typed `set_window`, or RPC:

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.window.set","params":{"x":80,"y":80,"width":960,"height":640,"state":"normal"}}
```

Each bound is optional and retains its unspecified normal-restore counterpart.
Bounds without an explicit state restore to normal before placement. State can
be `normal`, `minimized`, `maximized` or `fullscreen`. Width/height must be at
least 420×300, at most 16384 points; coordinates must be finite and within
±131072 points. Empty/misspelled placement objects are rejected. OS work-area,
minimum-size and window-manager constraints still apply: read back what actually
happened rather than assuming an accepted request is an applied rectangle.

Existing `pealayer.window.activate`, `.minimize`, `.maximize`, `.restore`,
`pealayer.fullscreen.set` (`enabled`) and `.toggle` use the same local ownership.
Restore exits fullscreen and minimization; Maximize also exits fullscreen and
restores a minimized window before maximizing. Playback still targets the session
authority when operating as a remote consumer. A window request never becomes a
server-window request merely because it came through that consumer's WebSocket.

## Playback and media

| Operation | JSON-RPC method | Typed command / parameters |
| --- | --- | --- |
| Playback readback | `pealayer.playback.get` | `get_playback` |
| Detailed cached media information | `pealayer.media.get` | `get_media` |
| Chapters and active index | `pealayer.chapters.get` | `get_chapters` |
| Complete shared snapshot | `pealayer.status` | `get_status` |
| Open | `pealayer.open` | `open`, `target` |
| Play / Pause / Toggle / Stop | `pealayer.play`, `.pause`, `.toggle`, `.stop` | `play`, `pause`, `toggle_pause`, `stop` |
| Relative / absolute seconds | `pealayer.seek`, `pealayer.seek_to` | `seek`, `seek_to`, `seconds` |
| Percentage seek | `pealayer.seek_absolute` | `seek_abs`, `percentage` (0–100) |
| Scrub / settle seek | `pealayer.scrub_to`, `.finish_scrub` | `scrub_to`, `finish_scrub`, `seconds` |
| Frame step | `pealayer.frame.step` | `frame_step`, `frames` (nonzero, ±120) |
| Chapter select / previous / next | `pealayer.chapter.set`, `.previous`, `.next` | `set_chapter` with zero-based `index`, `previous_chapter`, `next_chapter` |
| Volume | `pealayer.volume.set` | `set_volume`, `value` (0–130) |
| Mute / Toggle mute | `pealayer.mute.set`, `.toggle` | `set_mute` with `muted`, `toggle_mute` |
| Playback rate | `pealayer.rate.set` | `set_rate`, `rate` |
| Track select / disable | `pealayer.media.track.select`, `.disable` | catalog commands with `kind`, `id` where applicable |

Convenience GETs: `/api/player/playback`, `/api/player/media`,
`/api/player/chapters`, `/api/player/status`. Playback readback includes loaded,
playing, position/duration in seconds, volume/mute/rate, seekability/live state,
buffering and seek settlement. Detailed information reuses the Media Inspector's
cache and includes tracks, chapters, duration and FPS; reads do not probe libmpv
on network or window-message threads. Metadata redaction is cached on media
changes, not retransmitted as a large new object on every status tick. Source URL
credentials are redacted. Querying does not seek, load, reconnect or actuate outputs.

Stop follows the existing application's stop/close and recording policy; it is
not a second transport-specific interpretation. Frame stepping uses the same
timeline seek/decoder path as native navigation. Exact decoded frames remain
subject to source seekability and decoder capabilities.

## Wire entry points

| Transport | Entry point and framing |
| --- | --- |
| HTTP JSON | `POST /api/player/command`, e.g. `{"command":"pause"}` |
| HTTP JSON-RPC | `POST /api/rpc` or `/api/ipc`, JSON-RPC 2.0 object |
| Native IPC | Catalog `native_ipc.endpoint`: Windows per-session named pipe, or Unix-domain socket; UTF-8 JSON/JSON-RPC, newline terminated |
| TCP | Same configured listener/port as HTTP, UTF-8 JSON/JSON-RPC lines; up to 1 MiB/line and 256 requests/connection, bounded idle/read/write timeouts |
| WebSocket | `/ws`; typed JSON or RPC messages; existing live state events remain available |
| UDP | Existing provisioned COBS/CRC binary host RPC; same JSON-RPC values, source/target identity, fragmentation and replay protection. Not unauthenticated text datagrams. See [binary UDP](verification/binary-udp.md). |
| Windows messages | `WM_COPYDATA` to catalog/window `native_handle`, as below |

### WM_COPYDATA

Set `COPYDATASTRUCT.dwData = 0x504c4a51` (`PLJQ`), `cbData` to the UTF-8 byte
count (maximum 1 MiB), and `lpData` to JSON/JSON-RPC. A trailing NUL is optional.
`wParam` supplies a sender/reply HWND in the same Windows session; zero requests
no reply. `LRESULT = 1` means the bytes were copied into the bounded 32-request
queue. Zero means rejected/not queued; do not assume the command ran.

Replies arrive asynchronously as another `WM_COPYDATA` with `dwData = 0x504c4a52`
(`PLJR`) and UTF-8 JSON/JSON-RPC result/error. Keep the sender message loop alive
and correlate JSON-RPC IDs. Response tags never enter the command parser. The
worker validates the reply window owner again and bounds a hung reply to 500 ms.
No JSON parsing, mpv, RPC or egui access happens in the receiving WndProc; it
copies bytes and queues only. Standard Windows UIPI/integrity boundaries remain;
no message filter or security-policy bypass is installed.

All mutations are queued acknowledgements, not synchronous completion promises.
Poll observed window/playback state or consume live events to confirm application.
Do not blindly retry an outcome-uncertain mutation. The passive fixture client
in `scripts/control-transport-client.cs` demonstrates native pipe and bidirectional
WM_COPYDATA without controlling the user's mouse/keyboard.

## Verification

[Measured transport matrix and deployment boundary](verification/window-media-control-results.md)
records the actual passing run, source/package identity and remaining installation gate.

`scripts/test-window-media-control.ps1` launches an identity-fenced fixture with
independent storage, no hardware/global keys, a synthetic silent video and two
chapters. It exercises HTTP, RPC, HTTP IPC, raw TCP, native pipe, WM_COPYDATA,
WebSocket and the production binary UDP client; it checks actual window placement,
normal/maximized/fullscreen/minimized state, minimized playback/pause/seek,
metadata, frame stepping, chapters, volume/mute/rate and Stop. It closes only its
own clean fixture normally and retains results/failure evidence. It never drives
the regular app or the user-retained October 10 comparison.

Source tests additionally protect command validation, pure queries, local window
ownership, permission denial and real multiplexed TCP request/reply behavior.
Measured run results and deployment boundaries belong in the owning PR; a
successful compile is not a claim that every native OS has been runtime-tested.
