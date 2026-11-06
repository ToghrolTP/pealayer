# Taskbar artwork, live preview and density capsule

## What failed

The titlebar/tray path set the window's HICON, but Explorer also owns application
group and pinned-launcher artwork. That separate shell metadata retained the
packaged icon after selecting Classic. Both built-in icon assets remain intact.

The custom video thumbnail captured a seed frame, then only refreshed during a
one-second lease renewed by independent DWM requests. Requests caused by our own
invalidation deliberately did not renew it. Windows does not provide a reliable
"thumbnail is still hovered" renewal signal: the lease expired while the preview
was still visible. The user's reproduction delivered 16 valid bitmaps without a
DWM error, then aged into a stale frame while media continued advancing.

The native density value capsule explicitly used hover-only sensing; the Web
counterpart was a span. Neither implemented the requested next-value action.

## Changes

- Publish the current ICO through the main HWND's relaunch icon/name/command
  properties, followed by its stable AppUserModel ID. Use that same ID for the
  Jump List. Clear HWND properties on destruction.
- Update only ordinary pinned launchers targeting this exact executable with
  no arguments or `--activate`. Preserve captions, arguments and unrelated pins.
  Isolated review instances never change production pins. Reuse content-addressed
  multi-resolution ICOs; unrelated config changes do not decode or write artwork.
  Keep a bounded cache, preserving unowned files and active/default artwork.
- Refresh the bitmap from actual new native video frames, capped at 30 Hz. Reuse
  the video-only FBO; never decode another video, seek, capture egui, or render an
  extra MPV frame merely because the thumbnail was requested. Preserve read/draw
  FBO, pixel packing, renderbuffer and scissor state around downsample/readback.
- After a rate-limited dirty frame, schedule its eligible refresh even if no next
  decoder callback arrives. Unchanged paused frames do not invalidate DWM again;
  delivering a cached bitmap does not schedule another UI repaint. Full-size Peek
  remains DWM-owned, not a low-resolution bitmap over the application's UI.
- Native/Web capsule clicks advance through all five semantic density choices
  and wrap. Native pointer activation retains the slider's keyboard focus. The
  Web control disables input during persistence and rolls back a rejected save.
  Reserve every caption's width, keeping border and layout geometry stable.

The diagnostic endpoint `/api/player/taskbar-preview` now includes captured-frame
count, dirty-frame state and taskbar branding application/error details. It does
not expose private media paths or icon source paths.

## Verification and remaining gates

Native tests cover real hidden-HWND shell property readback and cleanup, real
temporary shortcut persistence/idempotence/target scoping, bounded icon-cache
reuse, frame-refresh scheduling beyond the old lease, paused-idle behavior,
capsule cycle/wrap/disabled behavior and existing keyboard/geometry/GL guards.
The isolated Web browser fixture exercises actual React controls at 390, 768 and
1440 pixels in both themes, including pending/failed saves and stable geometry.

The final native library run passed 1,120 tests, with one existing ignored test
and the updater URL fixture filtered out. That fixture passed separately
(one test); an earlier all-in-one run had stalled there and was stopped. This
is complete executed library coverage in two runs, not a claim that the original
single-process run passed. TypeScript, the 53 application parity contracts and
responsive layout guards also passed. The package workflow verifies the bundled
Web assets and host runtime separately.

This remains the OpenGL bitmap fallback, **not zero-copy GPU composition**. It
requires rendered video frames. A minimized window or hidden video panel whose
backend does not render cannot provide new frames through this path. Dedicated
surface/offscreen rendering and sustained CPU/GPU acceptance remain in
[#64](https://github.com/ToghrolTP/pealayer/issues/64) and
[#62](https://github.com/ToghrolTP/pealayer/issues/62); no synthetic frame or
extra decoder was added to disguise that limit. Long-hover installed acceptance
and the actual Windows taskbar icon must be checked after deployment.

## Installed checkpoint

The canonical local executable acknowledged the graceful update to source
`97761d7d2bfac30a236857984b73f5e639daae95`, clean, executable SHA-256
`235a33f79e4abb665ac8fd084fa42323cc4a3c06addcab809608642abfefc802`.
The existing host libmpv SHA-256 remains
`e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
Packaging verified the Web/PWA bundle, native resources, seven distinct quick
action icons, the downloader identity/smoke check and libmpv loading. Package
test flags are honestly `skipped`: library/focused Web tests ran separately as
recorded above, not silently inside packaging.

Live process readback reports clean native authoring, selected Classic icon,
Compact density and the video-thumbnail preference retained. Shell branding
application succeeded with no error; no matching production pin needed a write.
The media remains paused at the saved position. The video panel had not rendered
after restart at the first sample (no captured frame), so long-hover and visible
taskbar-icon acceptance await the requested user interaction. This is not a claim
of moving-video acceptance. Cafe deployment remains user-deferred while offline.

## Platform references

- [RelaunchIconResource](https://learn.microsoft.com/en-us/windows/win32/properties/props-system-appusermodel-relaunchiconresource)
  documents group/pinned artwork and the explicit ID requirement.
- [AppUserModel ID](https://learn.microsoft.com/en-us/windows/win32/properties/props-system-appusermodel-id)
  documents refreshing the group and setting the ID after relaunch properties.
- [RelaunchCommand](https://learn.microsoft.com/en-us/windows/win32/properties/props-system-appusermodel-relaunchcommand)
  documents paired command/display-name metadata.
