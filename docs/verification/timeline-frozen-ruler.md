# Frozen timeline ruler and Ctrl-wheel vertical scrolling

## Changes

- The egui ruler formerly occupied the first band of scrollable content. It now uses the real ScrollArea viewport's vertical origin, while retaining the content's horizontal origin. Time labels, exact-keyframe diamonds, chapter flags, playhead handle and their click/context-menu targets stay at the top during vertical scrolling and stay time-aligned during horizontal panning/zooming.
- Tracks retain their existing scrolled geometry. A separate body paint/input clip prevents offscreen tracks or analog keyframes from covering the ruler or stealing its gestures. The fixed header column and canvas continue to share the vertical offset; dropping effects or starting a lasso in the ruler band is excluded.
- Ctrl/Command + vertical wheel scrolls tracks vertically by default. Shift + wheel remains horizontal scrolling and plain wheel remains pointer-anchored zoom. Middle-button panning and axis modifiers remain intact.
- Egui consumes Ctrl/Command wheel into its zoom input, so these events are read with their Point/Line/Page units rather than depending on an empty smooth-scroll delta. Track headers also receive the gesture. The shared native/Web Preferences → Input → Timeline navigation option persists as `timeline_ctrl_wheel_vertical_scroll`; old configs acquire `true` via defaults. Disable it to allow the existing Ctrl-wheel zoom option to take effect.
- The Web timeline already places its ruler outside the track scroller. Its native wheel listener now applies the same saved Ctrl/Command scrolling policy without zooming the browser page.

## Verification

- `cargo test --lib --locked --jobs 1 timeline_ -- --test-threads=1`: 96 passed, 0 failed. Includes new frozen-origin/horizontal alignment checks, actual egui ScrollArea viewport and separate painter clips at 0/120/420 px vertical offsets, wheel routing/priority/unit normalization, defaults and persistence, plus existing track alignment, pointer-anchored zoom, pan, keyframe, cue and drag regression checks.
- TypeScript `tsc --noEmit` passed. Web production/PWA build and canonical optimized binary deployment results follow below.
- Tests do not activate hardware outputs. No full suite or native visual acceptance is claimed.

## Local deployment checkpoint

- Optimized `scripts/package-windows.ps1 -SkipTests -NoUpx` succeeded, with existing deprecation/dead-code warnings. Embedded PWA `f0657027fc5304f0` verified 39 precached resources.
- Old canonical Pealayer accepted IPC `quit` and exited before replacement; no forced termination. Canonical `C:\Users\David\AppData\Local\Programs\Pealayer\bin\pealayer.exe` relaunched, observed PID `10960`.
- Live `/healthz`: `ok`; `/api/player/status`: `hardware_connected: true`. No hardware output activation was performed.
- Live `/api/update/manifest`: commit `5340148a9e955953d6704e3f9e3c18e33a670d4f`, `git_dirty: false`, executable SHA-256 `042605bbfe6bc8362c62542fb9a1d4344c0ce3d31908f1980898857405931e0e`.
- Live shared Preferences contract contains the new Input → Timeline navigation checkbox and runtime value `timeline_ctrl_wheel_vertical_scroll: true`. No user preference value was overwritten to obtain this default.
- Cafe-PC `http://cafe-pc:8080/healthz` timed out after 5 seconds; no remote deployment is claimed. Use the peer updater when reachable. Source, embedded assets and this checkpoint are pushed to PR #46, which remains unmerged. This documentation-only checkpoint follows the deployed binary's commit.
