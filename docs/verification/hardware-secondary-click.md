# Secondary-click hardware safety

Owner: [#187](https://github.com/ToghrolTP/pealayer/issues/187).

The runtime owner reproduced a right click over a Web PWM slider changing its
value and coordinator recall memory, while also opening the card menu. Returning
the output to zero does not undo the altered remembered nonzero level.

Source tracing confirms the bundled @rc-component/slider rail mousedown handler
calculates a new percentage and starts a change without checking event.button.
Pealayer isolates non-primary mouse AND pointer starts in capture on its shared
card/detail PWM editor, before rail/handle handlers. It stops propagation, not
default handling: the independent contextmenu event still opens the card menu.
Primary mouse/touch edits and keyboard input are not intercepted. No dependency
source is patched, no version-specific branch is added and no PWM setter semantics
or RF/motion mappings change.

The new actual native pointer regression exposed the same class of bug in egui:
a non-primary drag could produce a changed PWM response. Native rail, numeric and
step editors now reject secondary/middle/extra-button press, held and release
frames without consuming the parent's event. They retain dirty numeric intent
without turning a secondary-triggered blur into Apply. There is no context-lock
recursion, fabricated telemetry normalization or output retry.

Hosted regressions cover all non-primary button IDs, primary pass-through, capture
wiring and preserved contextmenu events. Native egui regressions send actual
secondary/middle press, movement and release frames through the editor/dispatch
path and require unchanged percent and zero engine commands. These are source
tests, not proof of live hardware safety; runtime owner must recheck rail, handle,
detail, flanking actions and menu actions, plus unchanged authoritative brightness
AND remembered brightness. Keep private real waveform/output receipts local.

No source-lane local compilation/tests, runtime or output commands. The accidental
runtime output and its scoped restore are the owner's observations, not an action
or physical acceptance performed by the source lane. Parsed timeout presentation
is a separate remaining follow-up; do not expand this urgent safety fix into an
unverified transport timeout increase.
