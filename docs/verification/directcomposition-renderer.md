# Windows DirectComposition video renderer

The Windows build offers **Preferences → Advanced → Windows graphics and
composition → Video renderer**. OpenGL remains the default. Choose **D3D11 /
DirectComposition** and restart to try native presentation. The Web-only
process continues to use libmpv's OpenGL render API.

mpv owns the D3D11 composition swapchain and presents it to a DirectComposition
visual attached to the current Program Monitor. Pealayer retains one player,
one media clock and the existing native and remote control paths. If
DirectComposition initialization or the required libmpv options fail at
startup, the process starts with OpenGL and records the fallback.

When egui must paint above the video, the renderer temporarily hides the
native visual and displays a bounded readback from the same player. The
on-screen OSD uses the existing layout in the egui fallback and an ASS card
when the native visual is active. DWM small thumbnails use the existing
iconic-bitmap publisher. Task View continues to request a separate composite
of the whole application.

`GET /api/player/taskbar-preview` reports initialization, swapchain
availability, current visibility and the observed DXGI swap effect. The
experiment's zero-copy path describes direct video presentation. Windows'
custom iconic thumbnail still requires an HBITMAP.

The restored branch includes the [original experiment history](https://github.com/ToghrolTP/pealayer/tree/1b6d89d80af69ffa6d9f53d9f2d16f4d4ad9ed91)
as ancestry. See [#64](https://github.com/ToghrolTP/pealayer/issues/64) and
[#65](https://github.com/ToghrolTP/pealayer/issues/65) for renderer acceptance.

## Windows verification

The normal Windows debug build loaded a generated 18-second H.264 clip in an
isolated native instance. Its diagnostics reported an attached DXGI
flip-sequential swapchain. Playback advanced to 5.067 seconds and settled at
5.200 seconds after the HTTP Pause command. The small thumbnail published
11 frames at 640 × 360. A fresh Windows full-preview request published a
separate nonblank 1470 × 1110 bitmap. The active visual suspended when the
About command ran. Two unit tests cover padded staging rows, BGRA and RGBA
color order, 10-bit conversion, and rejection of an invalid stride.

[Program Monitor capture](../screenshots/directcomposition-program-monitor.png)
shows the synthetic video in the actual native window. There was no hardware
connection or deployment.

Small thumbnails retain the last clean video frame while an application OSD
card is visible and during swapchain buffer rotation after it clears. A paused
picture may therefore keep the last captured bitmap. The restored path still
needs minimized and paused thumbnail coverage across drivers, visible Explorer
Task View acceptance, device-loss recovery testing, adapter and decoder
selection, HDR, and sustained GPU and CPU measurements. These are tracked in
[#64](https://github.com/ToghrolTP/pealayer/issues/64),
[#65](https://github.com/ToghrolTP/pealayer/issues/65), and
[#66](https://github.com/ToghrolTP/pealayer/issues/66).

The archived detachable video-window experiment, original subtitle color and
placement work, and direct PWM envelope editor have separate application and
controller integration requirements. The original commits retain their source.
