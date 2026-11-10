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
