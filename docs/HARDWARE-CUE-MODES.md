# Hardware cue modes

A direct cue controls one advertised relay or PWM output. A recorded effect is
a separate reusable program: its intrinsic duration remains move-only.

| Mode | Timeline appearance | Output behavior |
| --- | --- | --- |
| Set and keep | Compact rounded marker, value and `→ ∞`, move grip, no resize edges | Sends the chosen value at its start. No automatic exit. The next command or a safety stop changes it. |
| Timed hold | Duration block with two resize edges | Sends the start value, then the explicit **On exit** value at the end. Default exit is Off/0%. |
| PWM ramp | Duration block with a gradient and two resize edges | Transitions linearly from the start percentage to **Ramp to** percentage; retains that endpoint afterward. |

## Author and edit

1. Select a linked relay/PWM track. Double-click its lane, or use Add cue.
2. Choose Set and keep, Timed hold, or (PWM only) PWM ramp.
3. Set the exact start and value. Timed modes also expose duration and exit value.
4. Drag the cue to move it. Only timed modes have edge handles. A persistent
   marker stays the same screen width when zoom changes; its right edge is not
   a second timestamp and is not a magnetic snap target.
5. Native double-click/Manage reveals Effect Controls. Web double-click or the
   cue context menu opens its direct-cue editor. Jump to cue start is explicit;
   clicking/editing a Web cue does not seek the media.

For example, choose Timed hold on the appropriate advertised relay, start **10s**,
duration **1s**, start **On**, exit **Off**. Choose Set and keep instead if it must
remain On after 11s. Hardware execution still obeys authority, mute/solo/link,
E-STOP and the prepared-plan acknowledgement gates. Keep is not permission to
bypass an emergency stop or guarantee a physical state after disconnection.

## One contract across surfaces

The persisted `direct_control` model contains `control_key`, `value_basis_points`,
`behavior` (`set-keep`, `hold`, `ramp`) and `end_value_basis_points`. Percentages
are integer basis points, 0..10000. Relay values use Off/On; ramps require PWM.
The duration field is retained as an editing value when switching modes, but is
ignored for Set and keep execution, hit testing, resize and snapping.

The same fields are available in `direct_cue.add` and `direct_cue.value` through
the existing API/IPC/RPC command router; update also accepts `instance_id`.
`direct_cue.value` preserves mode/end value when those fields are omitted.
`pealayer.timeline.effect.update` moves the placement and changes duration only
if its authoritative model permits resizing. Web state supplies mode and
`resizable`; the browser does not infer behavior from the caption.

```json
{"jsonrpc":"2.0","id":1,"method":"direct_cue.add","params":{"control_key":"relay.5","value_basis_points":10000,"behavior":"hold","end_value_basis_points":0,"start_time_ms":10000,"duration_ms":1000}}
```

PCController receives the existing acknowledged prepared relay/PWM action plan,
not a second timer driven by the UI. Persistent commands contribute one start
edge, timed holds contribute start/exit edges, and linear ramps contribute
bounded 34ms samples plus exact endpoints. No synthetic zero precedes an
otherwise standalone direct PWM command. Long idle gaps do not allocate samples.
Commands on a channel are chronological: a later start/exit command changes its
state; an active PWM ramp continues producing its sampled values. An authored
timed exit can therefore supersede a keep command placed before that exit.
Avoid overlapping commands unless this behavior is intentional. Direct commands
override that channel's underlying analog curve from their first edge onward.

## Acceptance checkpoint

Source includes fixtures for persistent markers, relay persistence/mute/unlink,
standalone Off, late single-edge PWM, exact timed exits and ramp endpoints.
Tests are deliberately not executed in this pass per the user's pause on tests.
TypeScript, Vite/PWA generation and native release compilation are the build
checks; they are not physical-hardware or installed-UI acceptance.
The consolidation owner must review the focused PR, build final main, deploy
using the host-specific runtime through the graceful updater, and verify the
native/Web gestures and safe output timing on Cafe. No motion outputs were
actuated in this pass.
