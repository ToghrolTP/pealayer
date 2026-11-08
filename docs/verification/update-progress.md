# Honest update progress

## Cause and correction

The updater retained completed transfer byte counts while moving to verification.
The native status bar and Web information panel reused those counts as current
progress, so verification appeared stuck at 100%. Separately, the native taskbar
only followed media position; the updater never took ownership of that indicator.

The coordinator now publishes `active` and nullable `progress_percent` with every
update status (HTTP, WebSocket and command responses). Byte counts remain transfer
diagnostics, not an estimate of verification work. Native status operations and
Web presentation use the coordinator's current-phase progress.

| Phase | Application presentation | Windows taskbar |
| --- | --- | --- |
| Receiving / downloading, known size | Transfer percentage | Normal proportional progress |
| Resolving / transferring, unknown size | Activity indicator; no percentage | Indeterminate |
| Verifying / staging / restarting | Phase label and activity indicator; no percentage | Indeterminate |
| Failed | Explicit error | Error state |
| Complete / already current / aborted | No active update operation | Restore playback progress, or clear without media |

Verification starts before download flushing, digest checking and archive
materialization. The local updater takes precedence over paused/playing media and
remains visible when this process is consuming a remote peer. That peer does not
become the owner of this process's taskbar update state.

Windows receives no `SetProgressValue` while indeterminate: that call would cancel
indeterminate mode, as described in Microsoft's
[taskbar progress contract](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itaskbarlist3-setprogressstate).
The app caches a shell state only after the Windows calls succeed, and balances
successful COM initialization calls.

## Verification

- Rust phase/serialization tests retain byte diagnostics while removing obsolete
  percentages; cover unknown/zero size and terminal states.
- Taskbar tests cover paused, playing and absent media; 100%-transfer → verifying;
  restart, failure, completion, cancellation and playback restoration. Indeterminate
  states must have no native progress-value write.
- Native paint regression checks that verification renders no `100%` text.
- TypeScript checks and `npm run test:update` protect current-contract consumption
  and the indeterminate Web indicator. No estimated verification percentage exists.
- Passed locally: 12 updater tests, two existing taskbar-state tests, the focused
  progress/paint/serialization tests, TypeScript, Web update/status/responsive
  guards, and host-compatible release packaging with libmpv smoke/PWA/resource checks.
- The packaged isolated review process also passed an actual HTTP transfer/abort
  check: two bytes of a declared four yielded `receiving`, `active: true`, 50%;
  abort yielded `active: false`, null progress and retained transfer diagnostics.
  No executable was submitted or installed; the test staging file was removed by
  the abort API and the review process exited through its local Quit API.
- Live Windows taskbar observation during the next installed-build update remains
  a separate acceptance gate. An older running executable cannot acquire this fix
  merely because it is downloading the corrected replacement.

The installer helper runs after the main process exits; it cannot keep the exited
window's taskbar button alive. These indicators cover the application's live
transfer, verification and graceful-restart phases, not a fictitious helper window.
