# Shared-firmware no-hardware development

Pealayer can author, execute and monitor its normal hardware workflow through
PCController's OS-native production firmware, without opening a serial device.
This is not a separate Pealayer board implementation.

Run PCController's `Tools/VirtualBoard/dev-host.mjs` with current native/controller
binaries and select `pccontroller://127.0.0.1:18987` in Pealayer's hardware endpoint.
The launcher provisions only a new isolated cinema authoring profile. It retains
existing profiles/effects and uses RAM-only EEPROM; production hosts on 8787/8788
and saved Pealayer drafts are not changed. The native board uses loopback 18765.
See PCController's [native target instructions](https://github.com/atomicdeploy/PCController/blob/main/Tools/VirtualBoard/README.md)
for build profiles, launcher options and explicit host sensor mappings.

## Closed gaps

- The native HELLO origin bit identifies mapped/fixture sensors. Catalog titles
  use Native defaults or the operator's authored source title, not an implied
  temperature of an absent lighting/audio peripheral.
- A headless coordinator initially knows HELLO/status but may not have settings
  or front-panel data. Pealayer seeds only explicitly missing native caches with
  read-only requests and rereads the authoritative snapshot. Physical UART
  refreshes retain their existing nonblocking front-panel policy; there is no
  per-frame probe or new polling loop.
- The previous integration silently looked for a retired scratch executable and
  wrote persistent EEPROM. Tests now require an explicit binary, use RAM-only
  EEPROM and dynamic loopback ports, and reject a non-native target before output
  commands. An explicitly configured missing binary fails rather than passing.

## Verification

With the existing canonical Windows Cargo cache and host-compatible libmpv,
set these variables in the parent test process:

```powershell
$env:PEALAYER_CONFIG_FILE = '<private isolated test config>'
$env:PEALAYER_VIRTUAL_BOARD_BINARY = '<current native virtual_board.exe>'
$env:PEALAYER_PCCONTROLLER_LIBRARY = '<current pccontroller.dll>'
cargo test --locked --test pccontroller_virtual_board_test -- --test-threads=1
```

The full-stack test starts its own firmware process and packaged C ABI coordinator
with integrations, discovery/autoconnect and system-native listeners disabled.
It verifies the real Pealayer catalog parser, explicit cinema seat controls,
settings/front-panel bootstrap, absent power, Native sensor titles, observed relay
and PWM writes, exact segment cells/TM1637 and all-off/reconciliation. Firmware
startup/door notices retain priority; an ACK alone is not a rendered-display claim.
Without the opt-in paths, these live integrations do not count as acceptance.

PCController's native conformance additionally checks mapped temperature-role
stability, signed power, source corruption/unavailability and automatic recovery.
The standalone launcher was exercised locally and its authoritative snapshot and
catalog exposed both seats, settings and the front-panel without a controller GUI.

## Remaining gates

This pass does not claim physical outputs, interrupt/electrical fidelity,
unavailable LCD/RF/GPIO peripherals, complete autonomous-hardware commissioning,
or installed native GUI/media acceptance. No Cafe production application was
replaced: that deployment requires its approved peer updater, fresh host runtime
and authoring-state preservation gates. The isolated native/controller binaries
and source tests are development evidence, not a Cafe deployment receipt.
