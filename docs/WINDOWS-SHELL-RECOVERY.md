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

The first replacement removed the timeline hang, and direct CLI Preferences
dispatch opened a responsive helper. The user then verified that Preferences
still failed from the actual Jump List, including on a second attempt, while
the in-application Preferences action worked. The shell issue therefore remains
an independent acceptance gate, not a completed fix.

The follow-up patch gives shell links an explicit executable working directory,
wakes the inactive native owner after IPC/HTTP command receipt, and shares one
Preferences open path which reaps a closed helper before reopening it. A process
exit waiter repaints the parent without idle polling. Bounded local JSONL evidence
in the OS cache's `diagnostics/shell-actions.jsonl` distinguishes launch request,
forward acceptance, GUI receipt and helper readiness; it contains no media URLs,
command payloads or credentials. Use it to locate failures instead of treating
IPC acceptance as proof that Explorer's action worked.

Timeline toolbar buttons own their pointer area: the ruler cannot override their
hand cursor or seek while a toolbar button is being pressed. A separate geometry
regression test covers the toolbar, ruler background and continued ruler drags.

A local deployment attempt also produced a real panic report: the GUI-subsystem
CLI process aborted while printing upload progress after its inherited pipe was
closed. CLI stdout/stderr are now best-effort writes rather than panic-on-error
printing. A disconnected-writer regression test covers this failure. Keep an
updater launcher attached with `Start-Process -Wait` and redirected output when
collecting deployment evidence. An interrupted partial upload is not a deployed
build and requires explicit cancellation before another upload can begin.

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
