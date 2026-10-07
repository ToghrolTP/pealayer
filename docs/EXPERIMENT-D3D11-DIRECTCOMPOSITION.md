# D3D11 / DirectComposition zero-copy experiment

This branch contains an intentionally isolated Windows renderer experiment.
The normal Pealayer build remains on the existing libmpv OpenGL render API.
This experimental build exposes **Preferences → Advanced → Windows graphics
and composition → Video renderer**, where OpenGL or D3D11 / DirectComposition
can be selected. A renderer change takes effect after restarting Pealayer.

## What this path changes

When built with `d3d11-composition-experiment`, Pealayer asks mpv for:

- `gpu-next` with the native D3D11 context;
- DirectComposition output mode;
- D3D11VA hardware decoding with mpv's zero-copy mode; and
- the `display-swapchain` pointer exported by mpv.

Pealayer attaches that swapchain to an `IDCompositionVisual`, positions the
visual over the physical egui video viewport, and lets mpv present frames
directly. This removes the normal on-screen chain of mpv OpenGL render API ->
Pealayer FBO -> egui texture -> window surface. Resizing updates mpv's
composition swapchain size and commits the visual's physical-pixel offset.

The experiment is visibly identified by `D3D11 Composition Preview` in the
window title. Its Web listener is deliberately disabled so the differently
named executable does not request a second Windows Firewall identity merely
to test local presentation.

## Build

```powershell
$mpvLibDir = Join-Path $env:LOCALAPPDATA 'Programs\Pealayer\build-dependencies\libmpv'
$env:LIB = "$mpvLibDir;$env:LIB"
cargo build --release --features d3d11-composition-experiment
```

The feature is effective only on Windows. A normal build does not compile or
activate the DirectComposition module.

## Verified on David-PC

The release build launched with the machine's packaged libmpv, restored a real
1080p H.264/DTS movie, and logged all three required runtime checkpoints:

1. DirectComposition target created;
2. mpv exposed a D3D11 composition swapchain; and
3. the swapchain attached to Pealayer's video viewport.

A window-only capture showed the movie inside the expected Program Monitor
rectangle with the surrounding egui workspace intact.

## Windows taskbar thumbnails

This is zero-copy for Pealayer's on-screen video surface. It cannot make the
custom video-only taskbar thumbnail zero-copy: the DWM iconic-thumbnail
callback requires Pealayer to submit an `HBITMAP`. The D3D11 implementation
therefore copies the swapchain buffer into a reusable CPU-readable staging
texture only when DWM is requesting thumbnails, converts/downscales it to a
maximum 640×360 RGBA frame, and feeds the same 30 Hz publisher used by OpenGL.
This bounded shell-preview readback does not alter the zero-copy on-screen
presentation path.

## Experimental limitations

- The composition visual sits above egui's opaque OpenGL surface. egui-drawn
  overlays inside the video rectangle (notably the OSD) require a later shared
  DirectComposition overlay visual or migration of those overlays into mpv.
- Hardware/driver coverage is limited to the current Windows 11 validation;
  adapter selection, HDR, device-loss recovery, Windows 10, and Cafe-PC still
  require explicit acceptance testing before this could replace the default.
- The build shares Pealayer's normal persisted session/configuration, but its
  network listener is disabled by design.
