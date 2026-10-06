# RF controls

Open **Workspace → RF controls…** (also available in Hardware Monitor and the RF
tool in Effects Library). In the Web Hardware page, use **RF controls → Manage**.

Use **Remotes → Learn buttons**, press a remote button, stop learning and refresh.
Choose **Assign** to bind it. Choose **Application → pealayer (all) → Toggle** for
play/pause. **Down** runs immediately on reception; use **Up** for a separate
release action. Multiple ordered actions can be added to an assignment.

The same manager supports advertised board controls, effects, external programs,
scripts, keyboard keys and full RF waveforms (code, bits, protocol, pulse width,
repeats). Actions and button data come from PCController's API. Assignments are
saved there, not in Pealayer. Change a saved assignment with Edit/Save; disable
it with its checkbox or remove it. The code/bits/protocol/gesture identify the
button, not its list position.

Existing board mappings are shown in Remotes. If replacing one with a Pealayer
action, explicitly Unassign board action; leaving it mapped intentionally runs
both paths. Board actions can also be reassigned with the semantic controls in
the Board assignment section.

Application targets are discovered live. The surface target `pealayer` survives
process restarts and applies to every matching live Pealayer; an instance ID
targets only that one process. `pealayer.command` accepts a compact JSON command
from the same validated IPC/API model, for example `{"command":"play"}` or
`{"command":"seek","seconds":10}`. Individual common commands have simpler
argument values, e.g. `pealayer.seek` with value `10`.

Keyboard and program actions run on the displayed **controller hostname**, not
implicitly on the browser/GUI computer. Native keyboard injection is subject to
the controller's allowlist and explicit consent. The existing key executor
supports single keys and exact modifier chords, e.g. `CTRL+SHIFT+S`; each chord
needs its own consent and modifiers are released on cancellation or error.
External program/script tasks
retain the controller's 30-second timeout; **Launch independently** starts an
application without waiting for it to close. Host mappings need PCController online;
existing EEPROM mappings can work independently.

## Unified control API

All Pealayer command transports accept:

```json
{"command":"rf_control","operation":"catalog","params":{"read_board":true}}
```

JSON-RPC uses `pealayer.rf` with `operation` and `params`. `open_rf_manager` (or
JSON-RPC `pealayer.rf.open`) opens the desktop manager. Mutations are asynchronous;
`status.rf.pending`, `.error`, `.catalog` and `.last_result` in `/api/player/status`
and the normal WebSocket state stream report actual controller results.

Allowed operations are catalog, learn.start/status/cancel, map, remove, clear,
transmit, binding.put/remove. This is not an unrestricted controller RPC proxy.
RF hardware execution still honors the existing E-STOP contract.

Receive/release timing is receiver-derived: Up follows the packet-gap timeout,
not an actual electrical release signal. Learning suppresses host assignments
but does not alter existing firmware-side actions. Real RF over-the-air testing
requires the actual receiver/transmitter; VirtualBoard can validate protocol and
application routing but cannot prove radio range or waveform delivery.
