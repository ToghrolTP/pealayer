# Subtitle alignment independent of text direction

## Cause and fix

`overlay_ass_event` formerly selected ASS anchor/position from text direction: LTR used `an1/32`, RTL used `an3/1248`, and Automatic used `an2/640`. Forcing Unicode paragraph direction therefore also moved centered subtitles to an edge.

`SubtitleAlignment` now independently selects Left (`an1/32`), Center (`an2/640`) or Right (`an3/1248`). Direction only controls Unicode embeddings. Center is the default, including when loading an older configuration containing only a forced direction. Font, replacements, vertical position and multiline content remain intact.

Explicit alignment consistently uses the existing processed text overlay, even when direction is Automatic and replacements are disabled, so embedded ASS placement cannot silently ignore it. `Subtitle style` preserves the native fallback when direction is Automatic and replacements are empty; if text processing is requested, this option uses Center. This retains a native-style escape hatch rather than removing it.

Native fallback/startup also apply `sub-align-x` and `sub-justify` independently. Bitmap subtitles remain in the native renderer: text alignment is not applicable to image subtitles, as documented by [mpv](https://mpv.io/manual/stable/#options-sub-align-x).

## Configuration and use

- Subtitle Settings → Appearance and placement → Text alignment.
- Preferences (native or Web) → Playback → Subtitles → Text alignment.
- Select Center and then force LTR or RTL; the text remains centered. Choose Left/Right to place either paragraph direction at that edge.
- `subtitle_alignment` uses `left`, `center`, `right`, or `subtitle_style`. It is included in the typed JSON config, startup loading, runtime snapshot, save/reload/preview paths and shared preferences/API contract. Changes apply immediately; Preferences retains its normal Save/Discard behavior.
- English and Persian labels are provided for the new control/options.

## Verification

- `cargo test --lib --locked --jobs 1 subtitle -- --test-threads=1`: 13 passed, covering all nine explicit direction/alignment combinations, processed-overlay enforcement, old-config migration, round-trip persistence, native/Web contract options/defaults, malformed enum rejection, multiline/position/font preservation, and real headless libmpv `sub-align-x`/`sub-justify` with runtime config snapshot.
- TypeScript `tsc --noEmit`: passed. Binary compile/optimized packaging and live deployment results follow in the checkpoint.
- No full test suite, physical output activation or native subtitle screenshot acceptance is claimed by these checks.

## Deployment checkpoint

- Source commit `9493e8a` and embedded Web UI commit `146ec8abc0e47f8e5491bbcdf593f6215b342e24` are pushed to PR #46. The PR remains unmerged.
- `cargo check --bin pealayer --locked --jobs 1` and optimized `scripts/package-windows.ps1 -SkipTests -NoUpx` succeeded. The embedded PWA build is `0aab44ed8b09192e`, with 39 precached resources verified. Existing deprecation/dead-code warnings remain.
- The old canonical process accepted `POST /api/ipc` with `quit` and exited before replacement. No force-kill was used. The new canonical `C:\Users\David\AppData\Local\Programs\Pealayer\bin\pealayer.exe` is running, observed PID `42024`; `/healthz` returned `ok` and `/api/player/status` reported `hardware_connected: true`. No hardware output was activated.
- Live `/api/update/manifest`: commit `146ec8abc0e47f8e5491bbcdf593f6215b342e24`, `git_dirty: false`, executable SHA-256 `17dc9b52abd38a3a1ff8b321102d5e16eeaaff1fef8671756c4dc7e214c374e4`. Host-specific libmpv runtime SHA-256 remains `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
- Live `/api/preferences` reports `subtitle_alignment: center` and the Left/Center/Right/Subtitle style dropdown. An isolated browser tab verified the visible options and default without altering preferences or the user's existing tab/draft. Screenshot: [Web alignment dropdown](subtitle-alignment-web.png).
- Native Computer Use capture failed on both initial and fresh window bindings with `IGraphicsCaptureItemInterop.CreateForMonitor ... 0x8007041D`. No blind desktop inputs were attempted. Native screenshot acceptance therefore remains pending; the nine direction/alignment combinations and real libmpv properties are verified by the focused tests above.
- Cafe-PC `http://cafe-pc:8080/healthz` timed out after 5 seconds. No Cafe deployment is claimed; use the peer updater when reachable. This documentation/image checkpoint follows the built code commit.
