# Effects Designer interaction pass

Tracks #141. Branch: `feat/effects-designer-gestures`.

## Scope

- Move seat cues across compatible motion tracks without changing other placements of a shared template.
- Ctrl-drag duplication in the main timeline and sequence designer; explicit duplication offsets, selects and reveals the copy.
- Whole-effect finite repetition, replacing per-step repetition, coordinated with PCController.
- Explicit recording channel/opcode selection enforced by PCController, with an empty default selection.
- English/Persian labels, Effects Designer naming, Start/Length/Finish, exact zero as `0s`, rename reconciliation and contextual publishing/running.

## Verification and delivery

Source implementation is checkpointed on PR #142. Do not treat the checkpoint as delivered UI. No Rust compilation/linking, clean builds or test runners on the production machine. A lightweight TypeScript `--noEmit` diagnostic passes. GitHub CI at checkpoint `815fac7f` passed the repository/Web gate and Linux native tests; follow-up changes require their own green checks. Windows/macOS results and installed visual acceptance must be verified separately.

Gesture: hold Ctrl while starting a cue drag to duplicate. Ordinary seat-to-seat dragging retargets the copied/selected placement only, preserving other uses of a shared template. Explicit duplication advances by at least one second (or the cue length if longer), selects and reveals the new cue without seeking media or issuing hardware commands. Pointer cancellation in the Web UI discards the preview.

The native and Web sequence designers use whole-effect repetition, computed full span, Start/Length/Finish, exact `0s`, configured seat names and Publish & Run. Execution waits for the publish acknowledgment. Remote publishing retains an unrelated native working draft. Recording capabilities are fetched when choosing inputs; the explicit empty default selection cannot start recording. Invalid current-contract snapshots retain the last valid catalog rather than silently dropping invalid steps. Scoped English/Persian translation coverage and authoring regressions are maintained in CI, not asserted as a whole-application translation audit.

Controller dependency: [PCController PR #634](https://github.com/atomicdeploy/PCController/pull/634), tracking [#633](https://github.com/atomicdeploy/PCController/issues/633). Its source/tests are independently checkpointed. PCController's own unwired Web/TUI recording start actions are guarded until its full chooser is implemented; this is not a claim that those interfaces are finished.

Deployment remains blocked by the recorded command-tool rejection of runtime smoke validation. Do not bypass that rejection with a different launch route. The existing production player, controller and physical outputs are unchanged during this source pass. Previous physical seat-motion acceptance is separately pending and is not covered by editor tests.

Four production effect definitions contain nontrivial per-step repeats. Before paired deployment, explicitly preserve their complete scheduled actions and timings under the current contract; do not simply remove fields or reinterpret interleaved step repeats as whole-effect repeats. Private production definitions are not copied into this repository. Autonomous command-tail completion (running until a final beep/PWM/display hold has ended) is a separate controller lifecycle gate; computed editor length alone does not establish physical completion.

## Handoff

The owning PR contains the implementation checklist and coordinated controller dependency. Finish the current alpha contract jointly; do not introduce per-build compatibility aliases. Keep private runtime settings, screenshots and media credentials out of Git.
