# Shared MOSFET On policy

Companion to [Pealayer #187](https://github.com/ToghrolTP/pealayer/issues/187)
and [PCController #675](https://github.com/atomicdeploy/PCController/issues/675).
The controller implementation is [PR #677](https://github.com/atomicdeploy/PCController/pull/677).

## Configuration

Open a MOSFET channel's **Manage / Settings** detail page in native or Web UI.
**Shared On behavior** selects **Full brightness (100%)** or **Restore last
brightness**. Press **Apply policy** to save. Merely opening, rendering,
reconciling events or editing the choice sends no command and does not actuate.
The update sends only `channel` and `on_behavior` through
`controller.pwm.channel.configure`, preserving curve, gamma, icon and step.
The policy is persisted by PCController, not copied into each Pealayer config.
Existing indicator Toggle/On/Off and hidden-button preferences remain cosmetic
client preferences; they select the same shared coordinator semantic actions.
Explicit percent setters, including 100%, stay absolute.

The Pealayer API bridge is `hardware.pwm.policy.set` with an explicit channel
and `on_behavior` equal to `full` or `restore`. Queue acceptance is not a
configuration/readback ACK; authoritative descriptor events refresh the UI.
No version-specific aliases, per-client recall or fabricated brightness exist.

## Memory and live updates

Only a real advertised `on_behavior` enables the configuration editor. The
current controller catalog and its `peripherals.changed` event own policy.
Owner/generation/revision-fenced PWM snapshots update confirmed remembered raw
duty and logical percentage for all connected native/Web clients. Incomplete or
malformed fields retain last-good metadata; explicit authoritative unknown
clears it. A new connection's initial model has no remembered value.

Off retains the controller's confirmed nonzero memory. On/Toggle apply its
policy centrally. Memory is RAM scoped to that board connection generation;
restart/reconnect makes it unknown, falling back to full. The UI explains this
limit instead of calling unknown zero. Autonomous firmware RF still uses its
own raw Off/full behavior; select a controller hardware action for host recall.
Existing board RF and motion mappings are never rewritten by this feature.

## Acceptance

Hosted tests cover policy parsing, invalid/missing channels, fenced memory,
malformed/unknown retention, exact partial payload and no writes on rendering.
No application compilation or tests run on the production host. Paired source
checks, exact tested merge/package, installation, native/Web interaction and
two-client safe-output recall must be recorded before calling #187/#675 done.
The RF owner retains exclusive live updater/output ownership and fresh private
learning/settings backups; root source work cannot erase new human RF learning.
