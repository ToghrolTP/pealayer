# Virtual Board parity

Use **File → Start Virtual Board** with existing media or an empty workspace.
**File → Open Virtual Board demo** also prepares the example clip and cues.
Both use the normal PCController coordinator and native Virtual Board firmware.
The local bundle starts them on private loopback ports, with physical device
mappings and network discovery disabled.

Use the board owner’s paired coordinator and Virtual Board builds from
[PCController #764](https://github.com/atomicdeploy/PCController/pull/764)
and the [LCD follow-up](https://github.com/atomicdeploy/PCController/pull/765).
For a Windows package, give `scripts/package-windows.ps1` a
`-VirtualBoardBundleDirectory` containing both `controller.exe` and
`virtual_board.exe`. The package manifest records their hashes. The two files
can also sit beside a local Pealayer executable. The existing
`PEALAYER_PCCONTROLLER_BINARY` and `PEALAYER_VIRTUAL_BOARD_BINARY` overrides
allow an isolated package to use verified companion builds.

- Hardware Monitor and its floating window render the raw firmware segment
  mask, brightness, LCD availability, and key state. Pointer holds use a
  bounded firmware lease. Releasing the pointer sends a release. A disconnected
  or closed client stops renewing the lease, so firmware releases the key.
- The virtual buzzer uses board-emitted notes and the configured SFX output.
  Audio can be switched off in the local twin.
- Both views consume normalized coordinator position samples for the left and
  right sides of the same seat. In 2D, hover over the far profile or select
  **Right side** to highlight it. In 3D, drag to rotate and Shift-drag or
  right-drag to pan. Use **Reset view** to return to the initial camera.
- Full travel defaults to 3 seconds. The local twin changes it on the
  coordinator and remembers the setting for its next private session. The
  equivalent IPC request is
  `{"command":"set_virtual_seat_travel","full_travel_ms":3000}`.
  JSON-RPC uses `pealayer.hardware.virtual_seat_travel.set` with the same
  `full_travel_ms` parameter.

## Verification

In an isolated Windows fixture using paused synthetic media, a Pealayer key
command produced firmware Down and Up events. A 733 Hz tone and a saved melody
produced board note events and matching buzzer activity in Pealayer. Independent
left and right moves reached Pealayer from the coordinator, and the stopped
side retained its position. Travel changed live from 3000 ms to 1100 ms and
back. Seeking to three seconds produced board-synchronized playback and
matching seven-segment masks `3F BF 3F 4F` in firmware and Pealayer. The same
coordinator acknowledged playback advancing and pausing.

The fixture used a Virtual Board with no physical peripheral mappings.
The LCD fixture pushed both 16-character rows from the firmware at I²C address
`0x27`, then pushed the timed clear without a client display read.
Read-only PCController catalog queries against the physical cinema profile
confirmed `seat.a` as Seat Left and `seat.b` as Seat Right, with raw seat
relays hidden. The private Virtual Board uses those same defaults.
The bundled jellyfish clip was opened from the File menu on the final Windows
build. The native Virtual Board applied both seat cues and energized Relay 5
through the normal timeline executor. The two views showed the independent
left and right positions from those events. In an earlier run with a release
build compiling at the same time, the timing guard paused playback after a
late controller read. The demo replay passed after compilation finished.

The [docked 2D capture](../screenshots/virtual-board-twin-2d.png) shows the
firmware LCD and both seat profiles. The [floating 3D capture](../screenshots/virtual-board-twin-3d.png)
shows those same positions after rotating the view.
