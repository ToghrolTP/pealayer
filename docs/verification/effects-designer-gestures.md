# Effects Designer interaction pass

Tracks #141. Branch: `feat/effects-designer-gestures`.

## Scope

- Move seat cues across compatible motion tracks without changing other placements of a shared template.
- Ctrl-drag duplication in the main timeline and sequence designer; explicit duplication offsets, selects and reveals the copy.
- Whole-effect finite repetition, replacing per-step repetition, coordinated with PCController.
- Explicit recording channel/opcode selection enforced by PCController, with an empty default selection.
- English/Persian labels, Effects Designer naming, Start/Length/Finish, exact zero as `0s`, rename reconciliation and contextual publishing/running.

## Verification and delivery

Source work in progress. Do not treat this checkpoint as delivered UI. No Rust compilation/linking or clean builds on the production machine. CI must validate the native model, widgets and Web UI before deployment.

Deployment remains blocked by the recorded command-tool rejection of runtime smoke validation. Do not bypass that rejection with a different launch route. The existing production player, controller and physical outputs are unchanged during this source pass. Previous physical seat-motion acceptance is separately pending and is not covered by editor tests.

## Handoff

The owning PR contains the implementation checklist and coordinated controller dependency. Finish the current alpha contract jointly; do not introduce per-build compatibility aliases. Keep private runtime settings, screenshots and media credentials out of Git.
