# Native video click timing

In Preferences → Playback → Player controls, **Video double-click interval**
sets the time between nearby clicks that toggle fullscreen. It defaults to
300 ms and applies to the native video surface. The Web Preferences editor
can change the same setting on the host.

**Wait for double-click before fast-forwarding** is on by default. A
stationary fast-forward hold waits for the configured interval. Turning this
option off restores the 180 ms hold threshold. Deliberately moving the mouse
still begins the configured drag immediately. Releasing an active
fast-forward gesture restores normal speed and consumes the release.

Previously the native surface activated a hold at 180 ms while egui counted a
double-click for 300 ms. A second press held slightly longer than 180 ms
could therefore begin fast-forward and consume a valid double-click. The
native video now measures both gestures on the same input clock. Other
controls keep their existing double-click timing.

This ports the input portion of the
[DirectComposition experiment](https://github.com/ToghrolTP/pealayer/commit/416388163640b442e4f00ee617db7bad0426344b)
onto the current per-button and modifier-aware input implementation.

## Verification

The Windows native video test group passed all 18 tests. The shared Preferences
contract test passed for defaults, edits and invalid bounds. The production
Web build passed its tests, TypeScript and offline asset verification.

[Native Playback Preferences capture](../screenshots/native-video-click-timing.png)
from an isolated instance. The capture opened the section and scrolled through
the Preferences helper's navigation channel.
