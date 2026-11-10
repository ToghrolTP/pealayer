# Playback and the board front panel

Connect Pealayer to PCController through the Hardware Monitor. Playback from
the GUI, Web UI or remote commands uses the same MPV state, so no separate
front-panel setup is needed. A new PCController host and matching firmware
(or updated VirtualBoard) are required for the media-clock command.

1. Load media. The board displays its decoded elapsed time.
2. Play: elapsed time advances and the separator blinks.
3. Pause: the display holds its time and the separator stays lit.
4. Seek or change speed: decoded MPV time and actual playback rate update the clock.
5. Close media/disconnect: the clock releases the front panel. A lost process
   expires automatically rather than leaving a falsely advancing clock.

Four physical digits display `mm:ss` through `99:59`, then `hh:mm`.
Board warnings, local editors, learning and programming keep their existing
display priority; ordinary pages return when the media display expires.
The clock does not actuate any relay or replace effect-cue timing.

Pealayer registers its process identity, app name, version, commit, OS and
architecture in PCController's existing leased app-instance registry.
Playback and the reverse-control subscription share one identity and payload;
neither can erase the other's advertised control endpoints or actions.
It publishes playback approximately ten times per second while advancing,
once per second while paused, and immediately upon state/seek/rate changes.
Cache stalls, scrubbing, EOF and E-STOP hold clock advancement. No media
filename or private URL is included in the identity or playback payload.
Network I/O runs on a separate bounded worker, not the render or actuator thread.

To inspect the connection, query PCController JSON-RPC:

```json
{"jsonrpc":"2.0","id":1,"method":"controller.app.instances","params":{}}
{"jsonrpc":"2.0","id":2,"method":"controller.media.playback.get","params":{}}
```

The playback snapshot reports `position_ms`, optional `duration_ms`, `playing`,
`loaded`, `rate`, source `client_id`, ordered `sequence`, receipt time and
separate board acknowledgement fields. `board_synced` is not just a socket
connection indicator; inspect `board_error`, `board_sequence`,
`board_synced_at` and `board_round_trip_ms` when diagnosing firmware mismatch.
The structured event stream publishes `media.playback` events for subscribers.
Subscribe on PCController's `/ipc` WebSocket with the `state` topic (the
`events` topic is the separate activity stream):

```json
{"jsonrpc":"2.0","id":3,"method":"controller.subscribe","params":{"topics":["state"],"interval_ms":100}}
```

Notifications use method `controller.state`, kind `media.playback`, and carry
the playback fields in `params.metadata`.

PCController advances between samples and sends exact segment bytes to the
board. This is bounded best-effort real-time synchronization, not a claim of
frame-perfect timing over arbitrary networks: sampling, transport RTT and
the board's 20 ms display service contribute latency. Clock source and board
presentation each expire after three seconds without refresh.

The authoritative wire/API contract is documented in PCController's
`docs/Playback-Board-Sync.md`; this is one playback presenter, not an effect
catalog or a second board-owned playback engine.

Local acceptance evidence and remaining physical gates:
[Playback verification](verification/playback-board-results.md).
