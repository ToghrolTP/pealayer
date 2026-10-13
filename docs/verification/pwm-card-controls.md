# MOSFET card controls and shared power policy

## Requested behavior

- Distinct minus/plus controls flank the PWM slider by default, in native normal
  and compact cards, channel detail and Web/mobile. Relative actions use the
  advertised stable coordinator action IDs, never a guessed client-side delta.
- Preferences > Hardware > MOSFET cards independently hides On, Off, Increase,
  Decrease and other action buttons. Defaults keep them visible. Hiding a button
  does not disable the channel or remove its contextual action menu.
- Indicator click chooses Toggle, On or Off. It dispatches the corresponding
  advertised semantic action, never writes a fabricated 0/100 value. If Toggle
  is absent, known authoritative state can select On/Off; unknown state cannot.
- The percent editor retains its observed-state and explicit-dirty-commit guards:
  rendering, focus, unchanged blur and settings changes never send PWM output.
  Emergency stop, channel lock and programming availability still guard controls.
- Native/Web cards and channel detail expose **Assign RF code…**. Opening this
  creates an unsaved draft scoped to that channel. The user explicitly chooses
  a learned signal and action. Opening neither saves nor actuates, and existing
  motion/board/controller assignments remain unchanged.

## Shared brightness memory is a paired controller feature

PCController [#675](https://github.com/atomicdeploy/PCController/issues/675)
owns configurable per-channel full-versus-restore On behavior and authoritative
remembered logical brightness shared by all clients. Pealayer does not maintain
its own remembered brightness or change absolute `controller.pwm.set` semantics.
The paired configuration UI and installed acceptance remain pending until the
controller advertises and implements the exact current contract. Autonomous
firmware RF behavior is not claimed to recall brightness without that support.

## Verification and delivery boundary

Source coverage includes distinct icons, independent visibility and persistence,
semantic indicator identity/unknown-state behavior, and scoped RF draft/catalog
non-mutation. Web equivalents run in the normal hosted build regression pipeline.
No Rust/Go/TypeScript compilation or tests run on the production host for this
pass. Hosted results and the exact installed runtime must be recorded separately.
Do not claim a deployment, paired recall, RF learning or physical MOSFET success
from source tests. The RF owner exclusively holds live output/updater ownership
while tracing the independently reproduced post-Off output overwrite in #181.
