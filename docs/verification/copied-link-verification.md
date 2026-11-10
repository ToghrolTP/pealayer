# Copied-link verification delivery

Owner: [PR #122](https://github.com/ToghrolTP/pealayer/pull/122).
Behavior and preferences: [remote locations](../remote-folders.md).

The native clipboard observer and Web clipboard hook identify copied requests;
Rust applies the host's persisted verification policy. Verification before the
dialog is the default. Users can instead show the dialog during verification or
show it without an initial network request. Explicit Browse still opens normally.
Identical route errors have one reason; distinct errors retain typed route
identities and separate Proxy/Direct blocks on native and Web surfaces.

## Verification

- Web TypeScript, interaction guardrails, build and PWA verification passed.
- Ten remote-browser Rust tests passed, including identical/distinct route
  failures, all three preference modes, successful verification, rejected copies,
  preserving a pending dialog and stale verification cancellation.
- Two CLI/JSON-RPC/IPC contract tests and the clipboard URL normalization test
  passed with a parent-provided isolated configuration and the shared Cargo cache.
- The Windows package passed resource/icon checks, local libmpv smoke and the
  standalone downloader smoke. Focused tests ran separately; package-wide tests
  and UPX were explicitly skipped on this incremental delivery.
- Cafe's own libmpv smoke passed before deployment. Its peer updater gracefully
  replaced the application and acknowledged the exact executable.
- `node scripts/verify-copied-link-api.mjs http://127.0.0.1:8080` passed on Cafe:
  unsupported HTML and plain text did not open; a supported directory opened;
  an unsupported copy did not replace it; dismissing a pending verification
  prevented reopening. Playback, cue and hardware state remained unchanged.
- The deployed Preferences contract exposes all three choices and advertises
  `verify_before_dialog`. Production preferences were not changed by acceptance.

An isolated local application launch was rejected by automatic approval review
with `blocked by policy`; it was not retried through another launcher. Native
pixel-level and browser-interaction inspection remain a human acceptance gate.
The shared state behavior was verified through unit tests and Cafe's live APIs.

## Technical receipt · 10 October 2026

- Built source: `3d5f7ae324d1d8fa16752e6cbe2ecbe661becc77`, clean.
- Cafe executable SHA-256:
  `36832a8c21702ae906983ad23d5097b6e24f572cf9fd1b4c499a788e26ef5618`.
- Destination libmpv SHA-256:
  `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
- Updater receipt: `update-20019ba3-8f3a-4268-bc9e-def2f7d0d4e5`, completed.
- Live process: PID 32168, interactive session 1, canonical installed `bin`.
- Hardware connected, no hardware error, paused and non-E-STOP after delivery.

Only the acceptance script and documentation were added after the built source.
No additional Rust/Web implementation changed, so this evidence-only checkpoint
does not trigger another local relink or package copy.
