# Coherent PWM power presentation

Owner: [#187](https://github.com/ToghrolTP/pealayer/issues/187).

The runtime owner observed a nonzero Web PWM percentage with an Off indicator
and selected Off action. Source tracing found Pealayer's web projection derived
active only for relays/seats; ordinary PWM projected null. Web coerced it to false
while deriving indicator actions from percent, splitting status and action truth.
This is not a claim of incorrect PCController data or a native visual defect.

The projection now uses the same owner/generation/revision-fenced PWM model.
Connected, available and valid raw duty gives known zero/Off or nonzero/On;
missing, malformed or unavailable observations stay unknown, never fabricated Off.
Raw nonzero remains On even when logical percentage rounds to zero. Status RGB
roles retain their actual component observation. Retained percentage may still
describe the last sample when unavailable, but is not labelled current power.

Web card/manager indicator, accessibility caption, On/Off selection and configured
indicator action share the nullable authoritative state. Unknown is a neutral
dashed indicator with neither action selected. PWM requests no longer overwrite
status optimistically; no send, dispatch acceptance or remembered brightness is
treated as confirmed output. Explicit advertised actions, preferences, semantic
IDs, locks, E-STOP and safety192 secondary-click isolation are preserved.

Hosted regressions cover zero/nonzero, a raw quantum with rounded logical zero,
unknown/malformed/disconnected/provider-unavailable samples, status RGB and
native-to-Web projection plus shared card/manager selection wiring. Reconciliation
is pure: no disk, engine command, policy or output write. No local builds/tests or
runtime changes in this lane. Runtime owner alone verifies the exact package's
actual positive/zero/unknown UI using read-only observations, without actuating
other people's outputs. Native current appearance and physical success remain
separate gates. Preserve independently owned RF learning/activity PR194/PC685;
this change does not alter their RF state, mappings or preference contracts.
