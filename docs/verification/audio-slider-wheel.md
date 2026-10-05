# Audio settings volume slider wheel adjustment

Cause: Audio settings constructed a standard egui slider without a wheel handler. Wheel input therefore reached the dialog's enclosing ScrollArea instead of adjusting volume.

The volume slider now uses the shared `dialog::numeric_slider_wheel` helper and the same `volume` adjustment settings as its numeric context menu. Wheel up increases, wheel down decreases; default is 1 percentage point per event. Ctrl/Command uses the fine step (default 0.1), Shift the coarse step (default 10). Custom adjustment steps remain honored. The range stays 0–130%.

Wheel changes enter the existing libmpv volume update and configuration save path immediately. The helper adjusts from raw events only, consumes the smoothed scroll tail, and does not adjust disabled, non-hovered or popup-covered sliders. At volume bounds it still consumes the gesture so the dialog does not move unexpectedly. Normal scrolling outside the slider is preserved.

Focused checks: the two new pointer/wheel tests pass across Point/Line/Page events, normal/Ctrl/Shift increments, positive/negative directions, clamping, hover and disabled behavior. Twelve trailing frames produce neither repeated value changes nor parent scrolling. Existing shared dialog and audio geometry checks also pass. No full suite or hardware output activation; live native screenshot acceptance is not claimed.

PR #46 remains unmerged. Web UI is unchanged; this repairs the native Audio settings slider.

## Deployment checkpoint

- 14 shared dialog tests and the audio geometry test passed (15 focused checks total).
- Previous canonical instance gracefully accepted IPC quit and exited before replacement. Optimized `package-windows.ps1 -SkipTests -NoUpx` completed, including Web/PWA verification.
- Canonical local executable relaunched as PID 13728. Health `ok`; manifest commit `59a5ab930bd3b3fa0eeab8e918bd944e3b2f3f95`, clean build, SHA-256 `d8cf49e5e21e8d79e877111787b137f8b31b5378a9c52fc5d6e904cdd735a887`.
- Cafe-PC health timed out; no remote deployment is claimed. Native runtime mousewheel acceptance is ready for user inspection; automated pointer/wheel tests are the verified behavior evidence in this pass.
