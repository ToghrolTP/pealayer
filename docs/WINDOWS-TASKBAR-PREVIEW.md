# Windows video-only taskbar preview

## What changed

The previous implementation provided hand-drawn, fixed 16x16 monochrome icons
and asked Explorer to crop the main egui window using `SetThumbnailClip`. It
did not supply an MPV-only iconic bitmap. The reported Windows 11 screenshot
therefore exposed a weakness in the crop-only implementation. It is **not**
evidence by itself of a confirmed Windows 10 versus Windows 11 OS bug; the
Windows 10 host was unreachable during this pass.

The replacement uses the native DWM iconic-thumbnail contract:

- Enable `DWMWA_HAS_ICONIC_BITMAP` and `DWMWA_FORCE_ICONIC_REPRESENTATION` when
  the configured video-only preview is active; disable both to restore the
  ordinary application-window preview. Clear the obsolete shell crop.
- Read only MPV's offscreen framebuffer, before egui/OSD composition, with an
  aspect-preserving, maximum 640x360 GPU downsample. Preserve framebuffer,
  renderbuffer, pixel-pack buffer and pixel-pack settings afterward.
- Seed one frame. Further readback is requested by DWM and capped at 5 Hz;
  it stops after preview demand expires. No continual idle full-screen capture,
  ffmpeg process, network request or disk write is involved.
- Respond to `WM_DWMSENDICONICTHUMBNAIL` and
  `WM_DWMSENDICONICLIVEPREVIEWBITMAP` with aspect-preserving, 32-bit bitmaps.
  Windows may use the last captured video frame while the app is minimized or
  no new video frames are rendered. The message handler never runs OpenGL.
- Toolbar glyphs are bundled Phosphor vectors rasterized at Windows' native
  small-icon size for the window DPI. Each glyph is centered and antialiased,
  then converted into a premultiplied BGRA `HICON`. OS shell light/dark color,
  DPI and theme changes are respected. Bitmap/icon handles are released.
- Existing five actions, IDs, contextual enabled/hidden flags and command
  dispatch remain intact. Only the inaccurate icon masks were replaced.

Microsoft references: [DWM iconic thumbnails](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmseticonicthumbnail),
[iconic preview attributes](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute),
[shell crop semantics](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itaskbarlist3-setthumbnailclip).
The two iconic attributes are setter-only; diagnostics report successful setter
results, not invalid `DwmGetWindowAttribute` readbacks.

## Configuration

Preferences -> Advanced -> Windows shell:

- **Show the video in the taskbar thumbnail**: on selects MPV-only; off selects
  the whole application. Enabled by default and persisted.
- **Show playback actions below the taskbar thumbnail**: independently controls
  the toolbar. Enabled by default.

The shared Preferences contract applies to both native and Web interfaces.
Changing the preview preference applies live, without restarting.

## Verification on 2026-10-06

| Check | Result |
| --- | --- |
| Local OS | Windows 11 Enterprise, build 26100 |
| Compile-only test-target check | Passed; Rust tests were not executed |
| Release build | Passed with existing warnings |
| Final installed source | `78d7d75e956464399a6285e25854eeba126b5036`, clean build |
| Final binary SHA-256 | `36e8591f3510c4adee9e1704db4f5611f918413acfeebb9d592049ad1dd48407` |
| Deployment | Own updater verified upload, gracefully closed, replaced and relaunched |
| Configuration setters | Both DWM setter calls succeeded; no configuration error |
| MPV-only bitmap | Captured 640x360; visually inspected video only, upright, no egui controls |
| Live disable | DWM mode disabled, cached frame removed, image endpoint returns 404 |
| Live enable | DWM mode restored, fresh 640x360 image, stored preference true |
| Icon size | 24x24 at local DPI |
| Icon center measurements | All eight glyph variants: exactly 0 px X/Y center offset |
| Icon antialiasing | All variants contain partial-alpha edge pixels |
| Paused playback/workspace | Preserved at 1256.089 seconds, NLE |
| Actual taskbar screenshot and click verification | Pending: native capture service failed twice with `0x8007041D`; no taskbar screenshot is claimed |
| Windows 10 / Cafe-PC comparison and deployment | Pending: updater endpoint timed out |

The following diagnostic atlas comes from the **installed** app's production
icon rasterizer, at its actual native size. It is not a screenshot of Explorer.
Order: back, play, pause, forward, mute, unmute, fullscreen, restore.

![Production Phosphor taskbar icons](images/windows-taskbar/phosphor-toolbar.png)

Read-only diagnostics (subject to the existing Web file-access permission):

- `GET /api/player/taskbar-preview`: mode, accepted DWM configuration, cached
  frame size/age, native icon size, DWM request/delivery counts and errors.
- `GET /api/player/taskbar-preview.png`: the exact cached MPV-only source image.
- `GET /api/player/taskbar-icons.png`: the production glyph rasterization atlas.

The video image was kept local rather than committing movie frames to GitHub.
At verification time DWM request/delivery counts were zero: the frame and
configuration were verified, but user-hover shell delivery still requires a
real taskbar check. The user was asked to provide that screenshot because
Windows computer-use capture was unavailable. PR 46 remains unmerged.

## Cafe-PC follow-up, 2026-10-06

After the SSH tunnel returned at `asus@localhost:7022`, Cafe-PC was confirmed
as Windows 10 Enterprise, build 19045. Its running canonical installation was
updated from `5ddf22e` to the same clean `78d7d75` executable verified above,
using its own `/api/update/begin`, chunk and finish endpoints over an SSH API
forward. No executable was copied through SSH and no process was forcibly killed.

- Operation: `update-5d4292de-cf13-48f1-8e52-8dfd1e1ac355`.
- Receiver acknowledged all 38,867,456 bytes, verified the SHA-256 and returned
  `restarting` with graceful closure before replacement.
- Relaunched canonical process: PID 13696, interactive Session 1,
  `C:\Users\Asus\AppData\Local\Programs\Pealayer\bin\pealayer.exe`.
- New runtime manifest reports `78d7d75`, `git_dirty: false`, and executable
  SHA-256 `36e8591f3510c4adee9e1704db4f5611f918413acfeebb9d592049ad1dd48407`.
- Cafe's distinct libmpv runtime remains unchanged:
  `0a81c004aae0ee7d512b9a26e38f66281f9591e84e1663215cc3a36a4bde6f6a`,
  version `v0.41.0-1084-ga1bf4b655`. It was not replaced with David-PC's DLL.
  Their import libraries are byte-identical, SHA-256
  `bef1b89f534bc86b33135e1f04fa2d5064b9d48b5de8bc9866665bbf43def793`;
  the existing EXE could therefore be reused with Cafe's runtime profile.
  This was a deployment of the existing build, not a claimed second compilation.
- Cafe's existing host-configuration script persisted its own build profile,
  libmpv path and generated Cargo configuration. Its native Rust toolchain is
  `x86_64-pc-windows-gnu`, unlike the local MSVC toolchain.
- `/healthz`, player status, update manifest, Web index and production icon PNG
  respond. Playback is paused at 517.268 seconds, duration 6204.245, in NLE.
  Both taskbar preferences remain enabled. Web assets expose generation
  `d039b3cafcd11df0`, including the Fuji loader and readable asset names.
- Cafe's production icon atlas was inspected at its native 16x16 size. This
  does not constitute an Explorer screenshot or toolbar-action click test.

Actual taskbar acceptance remains pending on both hosts. At the Cafe checkpoint,
the effective video-only representation was inactive, no frame was cached and
there were zero DWM requests/deliveries. The enabled preference alone is not
evidence that Explorer received a video-only bitmap. A real user-hover check with
the video surface visible is still required; no Windows 10 versus Windows 11
visual-equivalence claim is made.

## Jump List quick-action icons

Jump List tasks use their own bundled Phosphor icons: play/pause, previous and
next chapter, mute, fullscreen, preferences and exit. The application entry
retains the branded app icon. These multi-resolution resources (16–64 pixels)
are generated from the existing icon font at build time, not shipped as extra
loose assets or borrowed from OS DLLs. Each shell link selects its explicit
negative resource ID, so branding changes and resource ordering cannot turn
all actions back into the app logo. The action titles, CLI switches, glyphs and
resource IDs are shared between the build and runtime publishers.

Windows packaging verifies that the shell can extract the app icon and every
task resource and rejects empty or duplicate task images. Run
`scripts/verify-windows-quick-action-icons.ps1 -Executable <exe>` to repeat the
check; optional `-OutputDirectory <directory>` writes a dark/light contact sheet
of the icons actually extracted by Windows, rather than a mockup.
