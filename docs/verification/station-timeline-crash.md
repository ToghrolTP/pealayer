# Station-1 timeline crash investigation

## Deployment policy

- Cafe-PC remains the normal deployment/interactive verification target.
- Station-1 (`station@st-1.local`) is an optional deployment target when reachable.
  Its executable must remain in `C:\Users\Station\Documents\Share\Pealayer`.
  Do not create another clone, build directory or installed executable there.
- Use Pealayer's peer updater (`--deploy-to` or `/api/update/from-url`) for executable transfers, graceful
  exit, replacement, relaunch and acknowledgement. An SSH API tunnel is transport,
  not a replacement for the updater. Preserve each host's libmpv runtime.
- Do not launch Station-1 on every pass. An already-running instance can acknowledge
  its own update; otherwise leave it closed unless an explicit test requires it.
- Remove only the identified `pealayer_old.exe` after the new executable's commit,
  digest, HTTP health and interactive-session process have been verified. Preserve
  configuration, workspace state, assets and user effects.

## Initial evidence (2026-10-05)

Station-1 was reachable. DAVID-PC's existing public key was appended to the
administrator authorized-keys file without replacing existing keys or changing
sshd policy/ACLs. BatchMode public-key authentication and `sshd -t` succeeded.

The running Station executable reported commit
`08c6eac2115c157eadf3f3b16327322004406c1b` from `/api/update/manifest`.
Its on-disk `host-manifest.json` referred to an older package and is not evidence
of the running build. Both DAVID-PC and Station-1 reported libmpv SHA256
`e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
Station's existing workspace was NLE/compact timeline, with no media loaded at
inspection time and a connected PCController. No physical output was actuated.

No matching Pealayer event was found among Application Error/Windows Error
Reporting/Hang events in the preceding three days or the last 150 Application
events. No Pealayer dump was present in the user's CrashDumps directory.
This does **not** prove that a reported crash did not happen.

## Implemented diagnostics and focused coverage

GUI builds now install a panic hook before initializing mpv or handling CLI
commands. It writes one local, build-tagged report with thread, panic location,
message and forced backtrace. The file is capped at 256 KiB, preserves UTF-8 and
replaces the preceding report. The original panic hook still executes; the
application never silently resumes after an unexpected panic. No report is
automatically uploaded. Native access violations, process termination and hangs
are not captured by this Rust panic hook.

The path follows the configuration location. On Station-1's current system mode:
`C:\Users\Station\AppData\Roaming\pealayer\diagnostics\latest-panic.log`.

Focused validation (not the full test suite):

```text
cargo test --test timeline_pointer_regression_test --locked --jobs 1 -- --test-threads=1
cargo test --lib --locked --jobs 1 diagnostics::tests -- --test-threads=1
```

The native egui timeline test runs left/right/middle clicks and drags at 18
header/canvas coordinates with seat, relay, PWM, cue and keyframe fixtures. It
asserts that real context menus open, rather than merely sending a right-button
event. The engine is isolated from serial/network hardware. A child-process test
also triggers an actual panic and verifies that its build-tagged report is saved.

## Confirmed crash and focused fix (2026-10-05)

The user clarified: no media, no cues/keyframes, empty track space, and any of the
left/middle/right mouse buttons. The first diagnostic build, commit
`8f62792e5caee0f81f6044d5297c5842f48856d2`, subsequently captured a real Station
main-thread panic in `accesskit_consumer-0.35.0/src/tree.rs:71`:

```text
Focused ID #6251793858115588225 is not in the node list
```

The timeline requested keyboard focus for `timeline-keyboard-focus`, a synthetic
memory-only ID, while allocating the canvas with another automatic widget ID.
When Windows accessibility was activated, its consumer rejected the nonexistent
focused node. All mouse buttons took that focus path, explaining the report.

Enabling AccessKit in the regression test reproduced the **same numeric ID** on
the empty timeline before the fix. The canvas now registers the stable keyboard
ID through `ui.interact`; no accessibility feature is disabled or panic ignored.
Every generated accessibility frame asserts that its focused node exists.
Empty and populated timelines now pass left/right/middle click and drag replay
(108 gestures per state), including real context-menu opening assertions.

The source defect is reproduced and fixed. Live Station desktop interaction
still requires user confirmation: desktop capture failed with
`IGraphicsCaptureItemInterop.CreateForMonitor / 0x8007041D`, and the desktop tool
has no remote Station target. Do not treat automated pointer replay as live
OpenGL/libmpv playback verification or merge PR #46 without its other gates.

## Updater findings and visibility policy

The first peer transfer verified 35,503,616 bytes and SHA256
`de7f66c6278095673b02202daf77abb65841c4a271c7b77d5f7614147126ad1c`.
However, the old updater enqueued delayed Quit without waking an idle GUI.
The helper timed out rather than killing it. An explicit JSON IPC Quit shut it
down gracefully, and its existing hash-verified helper/journal completed the
replacement in the interactive session. No executable was manually copied.

The updater now wakes the live egui context after progress/status changes and
**after** enqueuing delayed Quit. A focused test consumes the staging redraws
before enqueueing Quit, validating the formerly idle case. A missing dispatcher
reports failure rather than remaining indefinitely in restarting state.

All updater-owned downloads, extracted candidates, helper executables, journal
temporary/final files, health acknowledgements and rollback files use Windows
`FILE_ATTRIBUTE_HIDDEN`. Newly created files receive the attribute at creation.
They stay Hidden until cleanup; the promoted/restored installed executable has
only Hidden cleared, preserving unrelated attributes. Unix dot-prefixed staging
paths remain unchanged. A real Windows filesystem test covers hidden staging,
journaling, activation, rollback, preserved attributes and deletion while hidden.

Focused validation: 10 updater tests, 3 timeline/diagnostic-process tests, and
1 diagnostic writer test passed. Only focused tests were run, not the full suite.
