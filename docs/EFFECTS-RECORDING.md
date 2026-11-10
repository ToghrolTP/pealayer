# Effects recording and authoring

Pealayer and PCController expose one effects system. PCController owns the
catalog and runs the hardware; Pealayer is the media-aware recorder, editor,
and timeline coordinator. This is a living alpha contract without a versioned
or legacy naming layer.

## Start here

1. Start PCController and connect the board. In Pealayer, confirm the hardware
   indicator is green and open **Effects Library**.
2. Select **Record effect**, name the take, and choose **Automatic · all live
   sources**.
3. Select **Start recording**. Operate relays, seat controls, PWM, lighting,
   displays, RF, or front-panel controls from Pealayer, PCController Web/TUI,
   or the physical board. They may be used at the same time.
4. Select **Finish and edit**. PCController saves the take in its effects
   catalog and Pealayer opens the sequence editor.
5. Move or resize steps, edit exact values and time, add repetitions or exit
   actions, and publish the result to PCController.
6. Drag its card from **Effects Library** onto the appropriate timeline lane.
   The placed item is a cue. Move its body or resize either edge.
7. Play the movie. At the cue time Pealayer sends the stable effect reference;
   PCController chooses the best available executor and applies the actions.

The same recorder is available in the native **Hardware Monitor**, the native
effect editor, the Web **Timeline**, and the Web **Effects Library**. All four
surfaces call the same typed commands and display the same PCController
recording state; none owns a separate take.

The native and Web editors also expose PCController's named buzzer melodies.
Opening **Add melody** forces a fresh `controller.melodies.list` request, while
the `melodies.changed` state event refreshes already-open clients. Choosing a
melody expands its validated notes and silent gaps into ordinary editable
buzzer steps; the saved effect therefore stays portable across host-clock and
device-clock execution without copying a private melody catalog into Pealayer.

The native **Hardware Monitor** has a dedicated **Buzzer & melodies** section
for live operation. It shows the active melody and physical-board mute state,
refreshes the PCController-owned catalog whenever its picker opens, supports
bounded repeats or explicit until-stopped looping, and provides a validated
20–20,000 Hz tone tester. **Stop buzzer** cancels a streamed melody and sends
the board's immediate stop opcode; it remains available during an emergency
stop because it can only de-energize the output.

## Terms

| Term | Meaning |
|---|---|
| Effect | A reusable, named PCController-owned definition. It can control one or many peripheral types. |
| Sequence | The timed steps inside an effect. |
| Recording / take | Capturing applied hardware actions to create an editable sequence. |
| Cue | One placement of an effect on the media timeline. It has a start and duration but references the catalog definition. |
| Step | One exact action inside a sequence, such as relay on, PWM 42%, seat down, or display text. |
| Automatic mode | PCController selects device-clock execution when the attached board can run every step; otherwise it uses the acknowledged host scheduler. |
| Device-clock mode | Forces the compatible sequence into the board's volatile timed queue. |
| Board-retained take | A bounded temporary capture in board RAM that can be imported into the same catalog. It is not a second library. |

“Macro” remains an internal queue/implementation term. User interfaces use
**effect**, **sequence**, **recording**, and **cue** consistently.

## What recording captures

PCController records applied command evidence, not mouse clicks. Rejected
actions are omitted. Relay changes originating from Pealayer, PCController,
RF, or physical controls use the board event clock. The host captures the
other acknowledged output commands, including motion, PWM, status RGB,
addressable lighting, display, buzzer, RF transmit, and front-panel/menu
actions. Automatic status animation and housekeeping are not recorded.

The recorder status reports the active name, capture mode, current step count,
retained-board capacity/overwrites, and the most recent error. Both Pealayer
interfaces consume that same live status.

## Editing behavior

- Boolean outputs support one-shot on/off, sustain for a duration, and a
  deterministic exit value.
- Motion steps use semantic up/down/stop actions from the attached profile.
- PWM and color steps expose numeric values and can use duration, fade, and
  easing metadata.
- Repetition fields create blink or pulse behavior without duplicating every
  step by hand.
- **Remove delay** shifts all steps so the first begins at zero.
- Quantization, snapping, exact time entry, keyboard nudging, duplicate, and
  delete refine a recording without losing its source evidence.
- A resized timeline cue receives a placement-specific duration; another cue
  using the same reusable definition is not changed.

Pealayer can retain one unsynchronized working draft while PCController is
offline. That draft is deliberately not a second effect library. Publish it
after reconnecting; all durable effects remain discoverable through
PCController import/export and every other client.

## Relay 8 pulse example

Create a sequence named **Relay 8 — one second** with two steps:

| Time | Action |
|---:|---|
| 0 ms | Relay 8 on |
| 1 s | Relay 8 off |

Publish it, drag it to the relay lane, and place its cue at `00:00:10.000`.
The effect definition stores the two actions; the cue stores the ten-second
media placement. Moving the cue changes when it runs. Editing the effect
changes what every placement runs.

## Addressable-light examples

The first-install PCController data provides editable examples named **Police**,
**White thunder**, and **Converging red**. They are ordinary user effects, not
hardcoded Pealayer choices or renderer branches. `effect restore-examples`
adds only examples missing from the current catalog and never overwrites a
user-edited definition. Users may rename, edit, export, or delete them.

## Hardware-free verification

VirtualBoard emits observable output transitions unless started with
`--quiet`:

```text
ON:8
OFF:8
PWM12:2048
```

This confirms protocol dispatch and makes automated recordings inspectable.
It does not replace physical verification of a loaded relay, cinema seat,
or LED strip. Before testing real motion, isolate or supervise the load and
confirm the current board profile advertises the intended semantic actions.

## Storage and transport

PCController persists the authoritative living catalog and supports
`effects.json` import/export. Pealayer projects store stable effect references
and cue placement data. During a run, PCController may stage compiled steps in
host RAM, board RAM, or both; this is an execution optimization, not a change
of ownership. Pealayer's native and Web interfaces use the same typed IPC/RPC
commands, live status, catalog, and timeline model.

The most recently opened media also keeps a bounded restart-safe cue session in
Pealayer's native configuration. It contains only stable PCController IDs,
placement, duration/lane, and cached display metadata—never the sequence steps.
Consequently a cue remains visible and editable while offline, survives a
graceful restart, and resumes against the authoritative definition when the
same media and PCController reconnect. A matching project sidecar intentionally
takes precedence when one exists.

## Verified delivery evidence

| Check | Result |
|---|---|
| Native recorder | Visible and enabled against an attached raw/virtual board; a configured semantic profile is no longer required just to capture valid advertised controls. |
| Web recorder | Shared component verified in Timeline and Effects Library. |
| Mixed-source capture | One automatic take captured Pealayer/API relay commands and board-origin relay events together. Automatic LCD/status housekeeping was excluded. |
| Virtual output | Playing the Relay 8 example printed `ON:8` then `OFF:8`. |
| Catalog | Relay 8 plus the three editable lighting examples were rediscovered through PCController. |
| Cue | Relay 8's one-second effect was placed at `00:00:10.000` and reported by the Pealayer API. |

Screenshots used for acceptance are stored under
`docs/screenshots/effects-recording/`. The illustrated operator guide is
generated as `output/pdf/pealayer-pccontroller-effects-guide.pdf`.

## What still requires the physical board

VirtualBoard proves protocol dispatch, applied-event recording, library
round-trips, and timeline coordination. It cannot prove loaded seat direction,
relay contact state, WS2811 electrical timing, or RF range. Connect and
supervise the real controller before final physical acceptance. Record the
exact host commit, board session/profile, effect ID, timing evidence, and
observed output for that run.
