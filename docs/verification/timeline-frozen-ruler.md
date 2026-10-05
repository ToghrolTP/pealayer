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
