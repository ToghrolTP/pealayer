# Window restoration, live taskbar branding and Task View

## The reproducible tiny-window cause

The first fixes recovered a tiny window by maximizing it. That did not correct
the defect. An isolated real-window test now reproduces the exact sequence:

| Step | Old build's client size in logical points |
| --- | --- |
| First launch with valid saved geometry | 900 × 650 |
| Minimize, quit through the API, reopen | 64 × 64 |
| Repeat | 64 × 64 |

Pealayer retained valid 900 × 650 application geometry. Separately, the pinned
eframe `WindowSettings::from_window` saved the minimized native client as zero
size at physical position (-32000, -32000). Its startup path then overwrote
Pealayer's validated `ViewportBuilder` using that second snapshot and clamped it
to the framework's 64-point minimum. Native decorations make the outer window
look approximately 118 × 152 physical pixels at this display scale.

Pealayer now owns placement, not eframe's independent window-state writer. It
retains widget/egui memory without deleting the workspace file. The final builder
hook overrides stale framework placement, and the creation callback reconciles
the framework's additional post-creation placement step before the first app
frame. It enforces a usable minimum, fits the live monitor, clamps unreachable
titlebars and uses current DPI plus UI zoom. Invalid session geometry falls back
to valid application geometry. Remote consumers use independent default placement
rather than restoring the authority's geometry. Maximized/fullscreen observation
does not replace normal Restore bounds.

The real fixture reopens at **900 × 650 on all three launches** after the fix,
including the same minimize/quit sequence. It has no hardware connection, user
media, production configuration or mouse automation.

## Task View is not the small video preview

The user confirmed that frame-driven taskbar-hover refresh is fixed. However,
forced iconic mode also makes Windows request `WM_DWMSENDICONICLIVEPREVIEWBITMAP`.
Returning unhandled did not restore ordinary compositor behavior; it left a
black full preview. This was a regression in our explicit custom-thumbnail path.

The full-preview request now schedules a screenshot of the completed native
egui+video framebuffer through the backend's callback API. This happens outside
the WndProc and only on demand, coalesced and limited to one full capture per
second. The resulting full-resolution application composite is sent to
`DwmSetIconicLivePreviewBitmap`. It never replaces the application's real
swapchain, hides controls, or reuses the small video-only bitmap. Small taskbar
cards keep their separate, already-confirmed frame-driven path. All pixels stay
in memory; no desktop screenshot or private UI image is written or uploaded.

The native fixture loads a generated test-pattern clip and sends the documented
DWM request to its own isolated HWND. Across three launches, the completed UI
capture was **1350 × 975 physical pixels**, with accepted DWM publication and no
error and non-uniform image samples (not a uniformly blank capture). API
diagnostics expose full-preview request/capture/delivery counts and image
dimensions. These prove the native capture/publication path, not the
human-observed appearance of Windows Task View or minimized live playback.

## Repeated branding changes

The first candidate updated the relaunch resource correctly, but setting the
same AppUserModel ID again left Explorer's running group icon cached. The user
observed Classic remaining after switching back to Modern, despite correct
configuration and resource diagnostics. That candidate was not complete dynamic
branding acceptance.

On an actual artwork change, clear/reapply only this HWND's association and
refresh its taskbar membership using the same stable identity. Reinstall/update
the media toolbar immediately using its existing recovery path. No Explorer
restart, global cache flush, synthetic new identity or pin/unpin operation is
performed. Unrelated settings do not rebuild membership. The fixture exercises
Classic → Modern → Classic → Modern through the configuration API and checks
resource changes, stable identity, refresh counts and retained toolbar state.
Actual icon appearance still requires the requested Windows-shell check.

## Minimized Task View

Windows reports a zero client extent while minimized. Full-preview publication
now reuses the cached normal-size composite rather than discarding it or trying
to read a zero-sized GL surface. The first visible frame seeds this cache once.
API and native `SC_MINIMIZE` actions refresh it immediately before minimizing;
the action is bounded by a 500 ms fallback and Restore/Maximize cancels a pending
minimize. No window is restored or focused merely to make a preview, and no
continuous full-UI capture or second decoder is added.

The native fixture exercises both API and system-command minimization. DWM
accepts the last-visible full UI while minimized, with no new zero-size capture;
the subsequent normal-size restart remains correct. This gives Task View its
normal last-visible-window snapshot. It does not assert continuously advancing
video while minimized: that offscreen-rendering gate remains separate.

## Repeatable verification

`scripts/test-window-placement-windows.ps1 -Executable <verified-exe>
-OutputDirectory <private-verification-directory> -TestFullPreview -TestBranding`
uses an isolated port/profile, synthetic video and the normal API. It refuses an
occupied port, revalidates the exact process and clean authoring before Quit,
and retains fixture evidence. `-ObserveOnly` records old behavior without
pretending that an expected failure is a passing regression test.

The library regression run passed 1,123 tests with one existing ignored and one
updater fixture filtered; that fixture passed independently. The final focused
platform run additionally checks the empty-surface guard. Package and installed
shell acceptance are recorded separately after delivery. Cafe remains
user-deferred while offline. Minimized/offscreen GPU rendering and sustained
CPU/GPU acceptance remain separate gates in #64/#62.

References: [live-preview bitmap contract](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmseticoniclivepreviewbitmap),
[taskbar membership](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itaskbarlist-addtab),
[relaunch artwork](https://learn.microsoft.com/en-us/windows/win32/properties/props-system-appusermodel-relaunchiconresource).
