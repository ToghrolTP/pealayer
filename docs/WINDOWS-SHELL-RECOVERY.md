# Windows shell actions and timeline recovery

## Confirmed failure

On David-PC, Preferences from the Jump List did not open and timeline toolbar
actions stopped responding. The background HTTP health/status service continued
to respond, but the GUI window was hung; there was no Preferences helper process.
A non-invasive Windows debugger inspection found the GUI waiting inside egui's
exclusive context lock while rendering the docked timeline.

Two animated-navigation paths (toolbar commands and playhead following) read
`ui.input` while inside `ui.data_mut`. Both use egui's non-reentrant context lock.
Thus a navigation action could deadlock the entire GUI, leaving subsequent shell,
Preferences and other UI actions unprocessed. This was not an icon-resource error.

## Repair and regression checks

- Toolbar navigation, wheel navigation, automatic following and playhead reveal
  share one transition starter. It snapshots the input clock, performs a narrow
  data transaction, then requests repaint outside that transaction.
- A real headless egui transaction test exercises that starter repeatedly and
  checks the resulting clock, zoom and duration. The targeted navigation tests
  also cover interpolation endpoints, toolbar order and shared preferences.
- Default toolbar positions swap Pan left/right and the lock-style follow action
  with Bring playhead into view. Existing custom orders remain user-owned; apply
  the requested swap to a running instance through its configuration API.
- Jump List entries retain the embedded per-action Phosphor icon resources.
  Icon acceptance alone does not prove command dispatch or GUI responsiveness.

## Delivery gate

Use the project Windows host resolver and retain a clean, pushed source checkpoint.
Package separately for the actual David-PC and Cafe-PC libmpv runtime identities.
Preserve Cafe-PC's independent timeline/effect changes before creating a release
worktree; do not overwrite its source checkout or deploy a feature-losing build.

Verify Preferences dispatch by observing a responsive helper/main window, and ask
the user to exercise the actual Jump List and timeline toolbar. Request graceful
IPC quit before replacement; a confirmed hung process may be terminated only after
diagnostics and a failed graceful attempt. Deploy Cafe-PC through the application
update protocol, then verify its live build identity, health and GUI responsiveness.
Do not confuse a successful upload with an installed or working update.
