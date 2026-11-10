# Hardware Monitor — precise inputs and observed actions

Tracking: [Hardware workspace](https://github.com/ToghrolTP/pealayer/issues/29),
[acceptance](https://github.com/ToghrolTP/pealayer/issues/80), implementation in PR #225.

## Operator controls

- PWM sliders: arrows and wheel use one percentage point; Ctrl (Command on
  macOS) uses 0.1; Shift uses ten. Ctrl wins when both are pressed. The optional
  adjacent minus/plus buttons follow exactly the same policy and use the same
  bounded, throttled PWM dispatch path. They default to hidden.
- Preferences → Hardware → MOSFET cards controls step-button and individual
  On, Off, Toggle visibility. Power actions appear Off, Toggle, On, followed by
  other advertised actions. No action is invented when the catalog omits it.
- The number field commits on Enter/blur. `.5` is 0.5%, `1..2` becomes 1.2%,
  and trailing dots disappear. View/edit retain one decimal and the percent
  suffix. Merely focusing/blurring never writes rounded telemetry to hardware.
- Up/Down highlight the coordinator's semantic direction, including its
  advertised break-before-make transition. PWM On/Off use observed power, not
  optimistic clicks or a rounded displayed percentage. Unknown stays neutral.
- Default local shortcuts: R then 1–9 toggles the advertised relay; P then
  1–9 toggles the advertised PWM output. Zero means output ten. A sequence
  expires after 1.25 seconds; unrelated input, Escape, editing and focus loss
  cancel it. It cannot create unavailable outputs or bypass locks/E-STOP.
- Channel management → Keyboard bindings records a key/chord. “Record next
  key” adds up to three more steps; “Clear sequence” returns to a single chord.
  Explicit local bindings take priority over the numbered defaults. Native
  global single-key/chord and hold bindings remain available; multi-step
  sequences are local-only, not silent global keyboard hooks. Linux global
  registration still requires X11; OS conflicts appear as registration errors.
- Shared duration editors and the segmented elapsed field reject illegal
  text/paste/IME input before applying it, with three red pulses at 2 Hz.
  Windows uses the system warning ding; other native platforms currently use
  the terminal-bell fallback. Browser timing fields use the existing optional
  audio-feedback system and bounded animation. An OS/browser mute can suppress
  sound; audible acceptance must be checked on the actual host.

## Regression investigation

- `33496e1` introduced `draw_pwm_power_actions` with `selected=false`. It
  separated PWM from the general action renderer but dropped observed-state
  styling. The repaired helper receives the capability snapshot and shares
  selection/styling across compact/full cards and channel properties.
- `967c9f9`'s general-card selection was restricted to relay identity and On/Off.
  Motion Up/Down therefore never became selected. The shared selection helper
  now reads the existing semantic motion contract, not raw relay guesses.
- The existing shortcut Toggle dispatcher used `channel_is_active`, which
  handled relay/motion but not PWM. It could repeatedly choose On for PWM.
  Dispatch now prefers an advertised atomic Toggle; paired On/Off fallback
  requires known observed state and does not turn unknown into Off.
- The wheel path added an unmodified sign directly to the value. It now shares
  the modifier policy with the slider and buttons, preserving the non-primary
  pointer guard, smoothed-tail consumption and final unthrottled commit.

## Verification and remaining gates

Focused native tests cover PWM telemetry/no-write safety, editor isolation,
non-primary input, normalization with actual TextEdit events, sequence order/
expiry/cancellation, advertised IDs and zero-to-ten mapping, semantic motion,
config round-trip and time formatting. React type checking and PWM source/
behavior guards are required as well. These are not physical output proof.

- [ ] Enable step buttons in the user's actual process via configuration API,
  preserving existing assignments; confirm persisted readback.
- [ ] Capture and verify the current native controls, phone/tablet/desktop Web
  layouts, audible rejection feedback and real global hotkey delivery.
- [ ] Destination-compatible Cafe package deployment and live board readback.
- [ ] PCController-owned Auto / Manual / Hybrid capability/state/configuration
  contract and enforcement, then a trailing mode selector and dynamic settings
  menu in both Pealayer surfaces. Auto blocks manual writes, Manual blocks
  autonomous writers, Hybrid allows both. Opening UI must never actuate.

The PCController owner chat was asked to own the last item. Current inspected
source has no generic advertised per-channel autonomy-mode contract; individual
settings and host PWM gates do not establish enforcement against autonomous
firmware. Do not ship a cosmetic selector or invent enclosure/power channel IDs.
The Cafe SSH route refused connections during this pass; it is a deployment
blocker, not evidence that the board or application is unhealthy.
