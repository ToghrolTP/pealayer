# Effects Designer interaction pass

Tracks #141. Branch: `feat/effects-designer-gestures`.

## Scope

- Move seat cues across compatible motion tracks without changing other placements of a shared template.
- Ctrl-drag duplication in the main timeline and sequence designer; explicit duplication offsets, selects and reveals the copy.
- Designer multi-selection: Ctrl-click toggles, Shift-click selects a range, Ctrl+A selects all, and scoped Delete/Ctrl+D perform atomic bulk draft edits. Explicit Select All/Clear/Duplicate/Delete actions and selection count are provided on native/Web surfaces. Text-field shortcuts remain text editing, not cue deletion.
- Whole-effect finite repetition, replacing per-step repetition, coordinated with PCController.
- Explicit recording channel/opcode selection enforced by PCController, with an empty default selection.
- English/Persian labels, Effects Designer naming, Start/Length/Finish, exact zero as `0s`, rename reconciliation and contextual publishing/running.

## Verification and delivery

PR #142 is the source integration checkpoint, not installed UI acceptance. Its
reviewed implementation passed both hosted platform matrices: Windows native
tests/build/package, Linux and both macOS architectures, repository/Web guards
and CodeQL. [Pull-request matrix](https://github.com/ToghrolTP/pealayer/actions/runs/38102463578)
and [push matrix](https://github.com/ToghrolTP/pealayer/actions/runs/38102462080)
cover the reviewed implementation. Current main was subsequently merged normally
without conflicts; the integrated head must pass its own final checks before
merge. Local cue-authoring, application-parity and responsive-layout guards and
TypeScript `--noEmit` also pass. No Rust compile/link or test runner was invoked
on the production machine.

Gesture: hold Ctrl while starting a cue drag to duplicate. Ordinary seat-to-seat dragging retargets the copied/selected placement only, preserving other uses of a shared template. Explicit duplication advances by at least one second (or the cue length if longer), selects and reveals the new cue without seeking media or issuing hardware commands. Pointer cancellation in the Web UI discards the preview.

For multiple selected cues, explicit Duplicate offsets the whole group by its complete start-to-finish span (at least one second), retaining every relative offset and selecting the resulting copies. Delete removes only selected draft steps and leaves unselected steps unchanged, including their timing. Web selections reset safely on an external draft replacement/reorder instead of applying stale indices to a destructive edit. This is local authoring, not a command to stop/delete published hardware effects.

The earlier Windows GNU access violation remains historical diagnostic evidence,
not a failed result hidden by this checkpoint. Both reviewed-head Windows jobs
subsequently passed, but that does not establish a native lifecycle root-cause
repair. Keep [the lifecycle investigation](https://github.com/ToghrolTP/pealayer/issues/116)
open and installed/native acceptance distinct from harness verification.

The integrated-head push matrix later stalled in `windows_shell_suite_test` on
Apple Silicon, after its earlier native suites had completed. Normal workflow
concurrency canceled that run when the documentation correction was pushed;
the canceled job is not counted as green. The Windows app-lifecycle fixture now
runs on Windows only, while Linux/macOS directly exercise all no-op shell shims
without constructing an unrelated mpv-backed app. Windows lifecycle assertions
remain intact, and media/native-client coverage remains in the other suites.
This is test-boundary stabilization, not a diagnosed native root-cause fix.

The subsequent Intel macOS push run passed those direct shell assertions, then
stalled in `workspace_dock_test` before its title assertions reported completion.
That redundant run was explicitly canceled to retrieve diagnostics, not counted
as passed. Dock titles now use a language-only accessor shared by production
rendering and pure English/Persian label tests; menu translation tests also avoid
constructing a native player just to check strings. Actual app-owned dock toggle
and persistence integration assertions remain. They require the parent's isolated
configuration path instead of mutating process environment under native threads.
Fresh final-head CI is required; the general native lifecycle investigation stays
open in #116.

The native and Web sequence designers use whole-effect repetition, computed full span, Start/Length/Finish, exact `0s`, configured seat names and Publish & Run. Execution waits for the publish acknowledgment. Remote publishing retains an unrelated native working draft. Recording capabilities are fetched when choosing inputs; the explicit empty default selection cannot start recording. Invalid current-contract snapshots retain the last valid catalog rather than silently dropping invalid steps. Scoped English/Persian translation coverage and authoring regressions are maintained in CI, not asserted as a whole-application translation audit.

Controller dependency: [PCController PR #634](https://github.com/atomicdeploy/PCController/pull/634), tracking [#633](https://github.com/atomicdeploy/PCController/issues/633), is merged after its full hosted matrix passed. PCController's own unwired Web/TUI recording start actions are guarded until its full chooser is implemented; this is not a claim that those interfaces are finished. The paired source contract is ready for integration independently of those remaining controller front-end gates.

## Source acceptance versus delivery

- [x] Compatible seat moves isolate a shared placement; Ctrl-drag and explicit
  duplication select the new authoring cue without seeking or live actuation.
- [x] Native/Web bulk selection and text-focus isolation have regression coverage.
- [x] Whole-effect repeat span, interval and invalid-value guards match the merged
  controller contract; retired per-step fields are rejected, not silently dropped.
- [x] Capture starts with no selected channels/opcodes, refreshes advertised
  choices, and sends explicit selections to the controller-owned recorder.
- [x] Publish & Run waits for the definition ACK; rename reconciliation preserves
  unrelated unsaved drafts. Changed labels have scoped English/Persian checks.
- [ ] Paired installed native/Web walkthrough and destination runtime validation.
- [ ] Exact persisted-definition preservation, controller Web/TUI chooser and
  autonomous command-tail acceptance in the controller tracker.
- [ ] Human/physical recording, output and sustained timing acceptance.

The PR owner also reported Linux native tests and interactive Web duplication,
selection and seat-lane moves in
[the review checkpoint](https://github.com/ToghrolTP/pealayer/pull/142#issuecomment-6106409596).
That report is not Windows installed-window or physical-output proof.

Deployment remains blocked by the recorded command-tool rejection of runtime smoke validation. Do not bypass that rejection with a different launch route. The existing production player, controller and physical outputs are unchanged during this source pass. Previous physical seat-motion acceptance is separately pending and is not covered by editor tests.

Existing production definitions include nontrivial per-step repeats. Before paired deployment, explicitly preserve their complete scheduled actions and timings under the current contract; do not simply remove fields or reinterpret interleaved step repeats as whole-effect repeats. Private production definitions are not copied into this repository. Autonomous command-tail completion (running until a final beep/PWM/display hold has ended) is a separate controller lifecycle gate; computed editor length alone does not establish physical completion.

## Handoff

The owning PR contains the implementation checklist and coordinated controller dependency. Finish the current alpha contract jointly; do not introduce per-build compatibility aliases. Keep private runtime settings, screenshots and media credentials out of Git.
