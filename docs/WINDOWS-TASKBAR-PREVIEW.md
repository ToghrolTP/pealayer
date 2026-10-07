# Windows video-only taskbar preview

## Layered presentation strategy (experimental D3D11 branch)

Pealayer now chooses the native representation from the actual video host:

1. **Detached native video HWND:** the zero-copy DirectComposition swapchain
   moves to a resizable top-level video window. That HWND supplies its own DWM
   surface and taskbar representation; the main window is not falsely cropped.
2. **Visible embedded video rectangle in the redirected main surface:**
   `ITaskbarList3::SetThumbnailClip`
   receives the intersection of the current physical-pixel video rectangle and
   the main window client area. Iconic-bitmap forcing is disabled.
3. **Video loaded but embedded rectangle unavailable:** the clip is cleared,
   `DWMWA_HAS_ICONIC_BITMAP` and `DWMWA_FORCE_ICONIC_REPRESENTATION` are enabled,
   and the last MPV-only `HBITMAP` is supplied for both
   `DwmSetIconicThumbnail` and `DwmSetIconicLivePreviewBitmap`.
4. **No video or preference disabled:** both custom paths and any stale clip
   are disabled.

The D3D11 startup contract explicitly enables mpv's flip model, requests three
swapchain buffers and keeps D3D11VA zero-copy decoding enabled. Pealayer reads
the exported swapchain's real `DXGI_SWAP_CHAIN_DESC`; diagnostics distinguish
`flip-discard`, `flip-sequential`, and legacy bitblt fallbacks instead of
claiming a swap effect from configuration alone. mpv remains the swapchain
owner, so driver fallback is respected rather than replacing its live chain.

Taskbar capture also follows the active flip chain rather than assuming DXGI
buffer 0 is visible. Pealayer queries `IDXGISwapChain3` for the current render
index and copies the immediately preceding buffer (the last presented frame).
This prevents Pause from making Explorer jump back to an older buffer in a
triple-buffered chain. A brief post-Pause refresh settles the cached frame, and
the native DWM message callback can continue publishing that cache while the
main window is minimized.

On Windows 11, mpv's embedded DirectComposition visual is not part of the main
HWND's redirected egui bitmap. `SetThumbnailClip` on that HWND consequently
crops the UI behind the video rather than the separately composed video. The
strategy therefore treats embedded D3D11 as case 3, while OpenGL uses case 2
and detached D3D11 uses case 1.

Because that topmost DirectComposition visual also sits above egui's redirected
surface, an egui popup would otherwise be hidden below the video. While a popup
is open, Pealayer takes one bounded snapshot from the same swapchain, temporarily
hides the native visual, and paints the snapshot inside egui. Closing the popup
immediately restores the zero-copy visual. Pealayer OSD messages are additionally
sent to mpv's native OSD so they remain above both embedded and detached D3D11
video without forcing the normal playback path through a CPU copy.

The Program Monitor tab context menu exposes **Detach video panel** when the
D3D11 / DirectComposition renderer is active. The same shared Preferences
contract exposes the persisted option for native and Web settings surfaces.

## Earlier bitmap-fallback groundwork

The previous implementation provided hand-drawn, fixed 16x16 monochrome icons
and asked Explorer to crop the main egui window using `SetThumbnailClip`. It
did not supply an MPV-only iconic bitmap. The reported Windows 11 screenshot
therefore exposed a weakness in the crop-only implementation. It is **not**
evidence by itself of a confirmed Windows 10 versus Windows 11 OS bug; the
Windows 10 host was unreachable during this pass.

That pass established the native DWM iconic-thumbnail fallback now used by
step 3 of the strategy above:

- Enable `DWMWA_HAS_ICONIC_BITMAP` and `DWMWA_FORCE_ICONIC_REPRESENTATION` only
  when the video is loaded but neither a visible embedded rectangle nor a
  detached native host can represent it. Disable both for the other paths.
- Read only MPV's offscreen framebuffer, before egui/OSD composition, with an
  aspect-preserving, maximum 640x360 GPU downsample. Preserve framebuffer,
  renderbuffer, pixel-pack buffer and pixel-pack settings afterward.
- Refresh the seed cache for two seconds after a media change so an initial
  undecoded/black render cannot become the persistent thumbnail. Further
  readback is requested by DWM and capped at 30 Hz;
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

## D3D11 / DirectComposition experiment — 2026-10-07

- Experimental binary: `experimental/d3d11-composition/Pealayer-D3D11-Composition.exe`
- SHA-256: `2234ee8922bb41e50770db7c2911d5e30ecea55b4783ae5ace844ad74776454d`
- Runtime strategy: `IconicBitmap`; cached frame 640x360; DWM iconic attributes
  accepted; video-only PNG visually inspected before and after Pause and while
  the main window was minimized.
- Shell toolbar diagnostics: add and state-update calls succeeded. A real
  Explorer hover/click remains the user-visible acceptance test; synthetic DWM
  request messages are deliberately not counted as proof.
- Actual mpv composition swap effect on David-PC: `flip-sequential`. Pealayer
  requests and recognizes `flip-discard`, but upstream mpv explicitly chooses
  `DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL` when `d3d11-output-mode=composition`.
- Checks: feature and ordinary release checks passed; four focused thumbnail
  tests passed.

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
