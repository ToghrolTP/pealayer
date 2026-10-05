# Audio settings volume slider wheel adjustment

Cause: Audio settings constructed a standard egui slider without a wheel handler. Wheel input therefore reached the dialog's enclosing ScrollArea instead of adjusting volume.

The volume slider now uses the shared `dialog::numeric_slider_wheel` helper and the same `volume` adjustment settings as its numeric context menu. Wheel up increases, wheel down decreases; default is 1 percentage point per event. Ctrl/Command uses the fine step (default 0.1), Shift the coarse step (default 10). Custom adjustment steps remain honored. The range stays 0–130%.

Wheel changes enter the existing libmpv volume update and configuration save path immediately. The helper adjusts from raw events only, consumes the smoothed scroll tail, and does not adjust disabled, non-hovered or popup-covered sliders. At volume bounds it still consumes the gesture so the dialog does not move unexpectedly. Normal scrolling outside the slider is preserved.

Focused checks: the two new pointer/wheel tests pass across Point/Line/Page events, normal/Ctrl/Shift increments, positive/negative directions, clamping, hover and disabled behavior. Twelve trailing frames produce neither repeated value changes nor parent scrolling. Existing shared dialog and audio geometry checks also pass. No full suite or hardware output activation; live native screenshot acceptance is not claimed.

PR #46 remains unmerged. Web UI is unchanged; this repairs the native Audio settings slider.
