# Station-1 timeline crash investigation

## Deployment policy

- Cafe-PC remains the normal deployment/interactive verification target.
- Station-1 (`station@st-1.local`) is an optional deployment target when reachable.
  Its executable must remain in `C:\Users\Station\Documents\Share\Pealayer`.
  Do not create another clone, build directory or installed executable there.
- Use Pealayer's peer updater (`--deploy-to`) for executable transfers, graceful
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

## Unresolved acceptance gate

The reported Station-1 crash has **not** been reproduced or attributed to a
confirmed source defect. Passing pointer replay is not proof of live OpenGL,
libmpv playback or Windows event-loop behavior on Station-1. Desktop capture
failed with `IGraphicsCaptureItemInterop.CreateForMonitor / 0x8007041D`, and the
desktop tool has no remote Station-1 target. Do not label this crash fixed or
merge PR #46 based on this evidence.

Next: obtain the exact mouse gesture, timeline element, media/playback state and
the report (if present) from the failing build. Match its commit and stack to
source, reproduce that path, implement a focused fix and verify it on Station-1.
