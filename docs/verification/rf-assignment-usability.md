# RF assignment usability

Owning issue: [#181](https://github.com/ToghrolTP/pealayer/issues/181).
Coordinator actions and RF metadata: [PCController #663](https://github.com/atomicdeploy/PCController/issues/663).

Learned button cards show the actual board assignment descriptor, every matching
host binding (including disabled ones), named action summaries, execution location
and the last genuine reception date. Matching uses code, bits and protocol together.
Dates are formatted in local time and recent receptions sort first. Unknown reception
dates remain unknown; refreshes and transmissions must not invent them. The backend's
bounded observation cache is per controller session, not persistent reception history.

Choose **On board** for supported autonomous mappings. The editor uses advertised
mapping actions, targets and behaviors and preserves the existing map when editing.
Direct board PWM Press and Toggle both toggle 0/full-scale; Momentary releases after
the firmware's packet-gap timeout. They are not fixed On/Off or percent increments.
Choose **On controller** for logical percent steps, gesture bindings, effects and
application actions. Hardware control bindings execute in PCController without
Pealayer, but still require the controller host. Application bindings require their
target application. No host action is represented as board-autonomous.

The native and Web editors select learned signals and configured keyboard choices.
Boolean and common speed values use contextual selectors; names, external programs,
unadvertised command parameters and explicit advanced waveform entry remain editable.
Incomplete or unavailable actions cannot be saved. Native fields share bounded widths
and zero expansion on hover; dismiss uses the shared square close control. Web fields
fill their columns, narrow forms stack and the learned table scrolls horizontally.

Tracked controller errors retain message, kind, resource and retry semantics. The
message is shown once with optional structured details, without displaying a raw RPC
envelope or deriving machine semantics from human error text.

Verification must include native Rust tests in CI (no Rust build on production), Web
behavioral tests and the full Web build, followed by exact-build deployment and live
Computer Use. Check Remotes, both assignment paths, disabled Save, selector keyboard
navigation, hover borders, narrow layouts and structured errors. Reception/storage,
binding persistence and physical actuator acceptance are separate checks. Private
production captures stay local. A capture timeout is not visual acceptance.
