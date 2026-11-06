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
The user rejected this candidate's actual icon appearance. A read-only native
capture reproduced the missed gate: Classic was in both `WM_GETICON` slots while
the visible taskbar still displayed Modern. The metadata and toolbar tests had
passed, but did not establish shell artwork acceptance.

### Icon notification ordering follow-up

The old order published relaunch properties and re-registered membership before
installing the new owned HICONs. Explorer could retain the preceding artwork.
Installing both window icons first is necessary but, experimentally, not
sufficient: re-registering in the same UI turn also left stale taskbar artwork.
A separate probe that re-registered after Explorer had processed those changes
made the actual displayed Classic icon appear; the reverse transition also
worked, without restarting Explorer or flushing its global cache.

The refresh now installs HWND icons first, publishes relaunch metadata, then uses
a single HWND-owned timer to coalesce changes for 200 ms, remove the shell button
and restore it 100 ms later. A same-turn remove/add was experimentally unreliable
even after the first delay; Explorer could combine the operations and keep the
old artwork on a later repeat. Restoration is scheduled before removal, and a
scheduling/removal failure attempts immediate Add/toolbar recovery while reporting
the failure. The actual window is never hidden or activated. The timer is killed
before handling it and on window destruction; no worker, recurring poll, forced
repaint or focus/size change is added. Diagnostics distinguish pending and
completed refreshes. Existing media-toolbar recovery is retained. This is an
observed asynchronous shell cache
ordering issue, not evidence that a successful property-store call refreshes the
visible button.

`-TestDisplayedBranding` extends the native fixture with a read-only primary
taskbar capture. It matches the first Classic icon and holds that exact slot
through Modern → Classic → Modern, rejecting the alternate artwork at the same
position. Only icon-sized crops are saved; the notification area is excluded.
An unavailable/hidden/vertical taskbar or mismatched image fails rather than
being counted as acceptance. Keep other Pealayer taskbar icons on Modern while
running this isolated Classic-first fixture, to avoid ambiguous initial matches.
The old candidate fails this visual check despite passing its API assertions.

The updated debug fixture passed all four displayed transitions in the same
taskbar slot. Classic matching error was 19.24 versus 53.08 for Modern; Modern
was 25.28 versus 56.65 for Classic (per-channel pixel RMSE). The threshold allows
Explorer's size-tuned ICO frame versus a resampled master but requires clear
separation from the alternate design. These are actual taskbar crops from that
isolated build, not reconstructed icon previews:

| Classic selected | Modern selected |
| --- | --- |
| ![Classic displayed in taskbar](../screenshots/taskbar-classic-live.png) | ![Modern displayed in taskbar](../screenshots/taskbar-modern-live.png) |

The same run retained media-toolbar registration, three normal-size restarts,
full UI composites and cached minimized previews. Installed-build verification
is a separate gate, recorded after packaging and graceful deployment.
The focused native branding, shortcut/cache, HICON ownership/alpha and placement
unit run passed 17 tests. No Web contract or video render path changed in this
slice; the earlier broad-suite evidence remains historical, not a claimed rerun.

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

## Previous installed candidate (dynamic artwork rejected)

The previous canonical update acknowledged clean source
`afb5761b791fb93ee8925538dda0d727089b6da6`, executable SHA-256
`efd474f2fd873174a17511343f5fd6f82fcc18ff75f0fe8d15661c6c4cd53231`.
The existing host libmpv remains
`e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
The packaged executable—not just debug—passed the expanded three-launch
placement, four-change branding, visible/full UI and minimized-cache publication
test. The library run passed 1,123 tests with one existing ignored; the excluded
updater URL fixture passed separately. Packaging's test flag remains honestly
skipped because those checks ran before packaging, not inside it.

The running process reports clean native authoring and the user's selected
Modern preset. Its native full-UI cache is **3110 × 1713**, with non-uniform
pixels and no preview/branding error. A bounded 5.01-second paused sample used
0.94% of one CPU core, with both full-UI and video capture counts unchanged.
That confirms no snapshot loop in this sample, not sustained playing/GPU/soak
acceptance. The app remains running. The user has been asked to verify repeated
visible icon changes, normal Restore size and Windows Task View while restored
and minimized. Those human-visible shell outcomes remain pending, not inferred
from successful API calls. Cafe remains user-deferred while offline.
