# Branding API and timeline-input delivery

Owner: [PR #239](https://github.com/ToghrolTP/pealayer/pull/239), continuing
[issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Contract: [Live application branding API](../BRANDING-API.md).

## Delivered

The canonical local application was updated through its graceful peer updater
after a fresh process-local authoring inspection reported clean/safe. The updater
took 71.1 seconds and acknowledged the exact new executable; no force termination
or manual executable replacement was used. Loaded media returned paused at its
saved position. The application remains running.

| Artifact | Verified identity |
| --- | --- |
| Packaged/installed source | `f83ee45fd2729a96b1691e1a92228c93348834fe`, clean |
| Canonical executable SHA-256 | `dddf875978eceeded39234b470e0b4efa6f1a7fe49c8f3916efed547b20c1fe4` |
| Retained libmpv SHA-256 | `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65` |

Canonical executable: `%LOCALAPPDATA%\Programs\Pealayer\bin\pealayer.exe`.
Source, shared Cargo cache and staging stayed under that program root. No new
worktree, alternate installation or remote-access service was created.

## Checks actually performed

- Native library: 1,130 passed, one existing ignored, one updater fixture excluded
  from the broad run. That updater fixture then passed separately.
- The five elapsed-field tests passed, including actual painted pointer targets
  in display/edit modes, milliseconds shown/hidden and three display scales.
  The first test run exposed a centered-galley origin error; the correction is
  included in this package, not left as an unexplained failing test.
- Measured native ruler-label centering/edge-omission regression passed.
- Web TypeScript, timecode/clock checks, responsive guardrails and all 53 parity
  contracts passed. Packaging also passed Web/PWA integrity, host libmpv smoke,
  seven distinct action resources and standalone-downloader resource/headless
  smoke. Packaging's own test flag is honestly `skipped`: tests ran separately.
- Exact release real-window fixture: temporary branding, unrelated-volume
  autosave isolation, selective persistence, reset, open native Preferences
  event propagation and Web runtime name/description all passed.
- Exact release displayed taskbar artwork: Classic → Modern → Classic → Modern
  passed in the same actual taskbar slot, retaining stable identity, playback
  toolbar registration and a stopped/coalesced refresh timer.
- Exact release normal placement remained 900 × 650 logical points on three
  launches. Full UI composites were nonblank 1350 × 975; both API/native minimize
  paths delivered the last-visible cache without a zero-client recapture.
- Installed canonical application independently passed the same four visible
  artwork transitions. Classic pixel error was 19.24 versus 53.07 for Modern;
  Modern was 25.28 versus 56.65 for Classic. Original Modern selection was retained.
- Installed API reports the clean matching source, no temporary overlay, the
  original name/preset and unchanged runtime. Its full UI cache is nonblank
  3722 × 2016 with no preview error. A bounded 5.01-second paused/minimized sample
  used 0.31% of one CPU core; this is not playing/GPU/soak acceptance.

One debug native-minimize fixture timeout was recorded. An independent debug
three-launch recheck and the exact release's combined run passed; the timeout is
not erased or attributed to a proven cause. Failure diagnostics are retained
when this recurs. A separate combined-fixture assertion initially used a stale
pre-API shell counter; its baseline now waits for the preceding coalesced refresh.
That was a test-phase synchronization issue, not a product fix inferred from it.
The installed artwork check also validates the registered HWND's actual process
owner rather than .NET's potentially different `MainWindowHandle` selection.

Private fixture captures, logs and configuration remain under `verification`
outside Git. The canonical window was minimized during the final read-only
capture attempt; it was not restored/focused just to obtain a screenshot.

## Remaining acceptance

Cafe deployment is explicitly user-deferred while offline. No Cafe deployment,
physical output, audio routing, sustained playback/CPU/GPU or human Win+Tab
acceptance is claimed by these local checks. Installed native/Web pointer
walkthrough and Web visual/touch acceptance remain separate from rendered unit
and responsive-source guards. Browser-installed PWA caches and signed executable
resources are not instant live metadata. Temporary-overlay replication into
separate native peer processes remains outside the current process-scoped API.

Keep #239/#80 and the broader #63/#64/#62 acceptance gates open accordingly.
