# External mpv

Pealayer can control a standalone mpv using a persistent, full-duplex JSON IPC
connection. Playback properties come from mpv `observe_property` events, not a
property-polling loop. No Lua helper or injected DLL is required.

## Choose a mode

In **Preferences → Playback → External mpv**, choose:

| Mode | Playback owner | Pealayer video surface |
| --- | --- | --- |
| Internal (default) | Embedded libmpv | Normal player |
| External mpv only | Standalone mpv | Controls, timeline and connection state; internal decoder stays idle |
| External mpv + internal preview | Standalone mpv | Muted, synchronized internal preview |
| Control existing mpv | The mpv at the configured endpoint | Controls and timeline; internal decoder stays idle |

For External or Dual, leave **IPC endpoint** blank to let Pealayer launch mpv.
Set **mpv executable** to the executable on PATH or use Browse for its full path.
Entering an endpoint instead attaches to an existing player; Pealayer never
terminates that player on detach or exit. Managed players close on exit/mode
change. Switching between embedded and managed playback transfers the current
file/position paused, so changing the renderer does not unexpectedly actuate cues.

Managed launch uses Pealayer's audio, speed, subtitle positioning and proxy
preferences. **Load mpv's own configuration and scripts** is optional and off by
default. Attaching adopts the existing player's current session instead of
opening Pealayer's last file over it. Managed startup restores the last media and
saved pause state when the existing playback-history preferences allow it.

### Attach on Windows

Start mpv with a local named pipe:

```powershell
mpv --input-ipc-server=\\.\pipe\pealayer-mpv "C:\Media\movie.mkv"
```

Select **Control existing mpv** and enter `\\.\pipe\pealayer-mpv` as the endpoint.
Select Dual with that same endpoint for an internal preview as well.

### Attach on Linux/macOS

Use a socket inside a private directory, not a public TCP port:

```sh
install -d -m 700 "$HOME/.cache/pealayer-mpv"
mpv --input-ipc-server="$HOME/.cache/pealayer-mpv/ipc.sock" /media/movie.mkv
```

Enter the absolute socket path. Pealayer-generated Unix endpoints use a unique
0700 directory and clean up their own socket when changing modes. mpv JSON IPC
is not an authenticated network protocol; do not expose it directly to a LAN.
For cross-machine control, use Pealayer's existing authenticated peer connection
to the Pealayer instance that owns this local mpv endpoint.

## What synchronizes

File/playlist changes, play/pause, seeking, playback rate, volume, mute, video,
audio/subtitle selection, subtitle text/settings, chapters, buffering and media
metadata flow into Pealayer's existing player state. Native, Web, IPC, JSON-RPC,
media-key and peer controls use the same outbound adapter. External player
actions immediately update the observed state; incoming events never echo back
as another outbound command.

Dual mode uses an independent, latest-snapshot-only preview worker. Only the
external player owns audible main-media output. Preview decoding must not block
IPC acknowledgements or become another hardware timing authority. Synchronization
corrects preview drift; it is not a promise of frame-locked presentation between
two independent decoders. Unsupported internal codecs produce a preview error
without stopping external playback or losing remote control.

PCController's existing prepared plans, playback clock, acknowledgement checks,
authority and E-STOP gates remain in place. The independent media-clock observer
reads the external event cache, not the internal preview decoder. A stale playing
clock is treated as buffering; interpolation is capped at 250 ms. Cues still
belong to the Pealayer timeline: changing a file externally does not manufacture
a matching cue project, so inspect/load the appropriate timeline before arming.

The Web app controls all modes. Browser-playable source bytes can still use the
existing media endpoint. Decoded Web fallback frames and native taskbar video
captures require an internal decoder (Internal or Dual); External/Remote do not
silently start a second decoder to provide those frames.

## API and command line

The shared preference contract exposes `external_mpv` through normal validated
configuration updates. Send this to `POST /api/rpc` (or native IPC / WebSocket):

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.config.update","params":{"external_mpv":{"mode":"remote","executable":"mpv","endpoint":"\\\\.\\pipe\\pealayer-mpv","use_mpv_config":false}}}
```

On Linux/macOS replace the endpoint with the absolute Unix socket path. `external`
or `dual` plus an empty endpoint launches the configured executable. `internal`
detaches and returns to the embedded player.

The CLI already accepts unified JSON commands:

```powershell
pealayer --remote '{"command":"update_config","values":{"external_mpv":{"mode":"remote","executable":"mpv","endpoint":"\\\\.\\pipe\\pealayer-mpv","use_mpv_config":false}}}'
pealayer --pause
pealayer --seek-to 10
pealayer --volume 50
pealayer --rate 1.25
```

Configure first, then open/control media using the normal commands. The current
JSON-RPC equivalents (`pealayer.pause`, `pealayer.seek_to`, `pealayer.volume.set`,
etc.) and existing Web transport need no separate external-player API.
`GET /api/player/status` includes `external_mpv`: connection/mode/ownership,
endpoint, reconnect and command-acknowledgement counters, pending count and
connection/preview errors. Pealayer consumers forward control to their server;
mirrored settings never launch a competing local external player.

## Failure and verification boundaries

Commands are bounded to 128 queued/pending entries with unique request IDs.
Connection generations discard obsolete commands/snapshots. Disconnects and
three-second acknowledgement timeouts fail closed; non-idempotent commands are
never replayed after reconnect. Reconnection installs fresh property observers
and adopts actual state. Only the latest explicit Open request may wait for the
first connection; arbitrary disconnected control commands are rejected.

Windows named-pipe I/O is overlapped and cancellation completes before buffers
are freed. Unix sockets have bounded I/O timeouts. A managed mpv process that
exits is not continuously respawned: switch modes to deliberately relaunch it.

Focused real-Windows-mpv validation uses generated lavfi video, null outputs and
no board commands: duplex property changes, actual advancing playback, muted
dual preview, 32 acknowledged commands, detach ownership, reconnect/no replay
and managed launch/cleanup. Run explicitly with:

```powershell
$env:PEALAYER_TEST_MPV = 'C:\Program Files\mpv\mpv.exe'
cargo test --locked --lib live_duplex_ipc_and_muted_dual_preview -- --ignored --test-threads=1 --nocapture
```

Linux/macOS socket and native visual/audio acceptance remain distinct from
Windows IPC verification. See [delivery evidence](verification/EXTERNAL-MPV-DELIVERY.md)
for installed packages, live API receipts and remaining acceptance checks.
Protocol reference: [mpv JSON IPC](https://mpv.io/manual/stable/#json-ipc).
