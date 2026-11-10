# Native video ghosting across workspaces

## Cause

Simple, Program Monitor/NLE and fullscreen all call `ui::video::draw` and retain
the same `RttState` texture, framebuffer and libmpv render context. Changing
workspace does not recreate those resources.

The callback inherited egui_glow's enabled premultiplied `BLEND` and enabled
`SCISSOR_TEST`, with a clip rectangle in window coordinates, then changed the
viewport to an offscreen texture without resetting that state. That violates
[libmpv's OpenGL interop contract](https://github.com/mpv-player/mpv/blob/master/include/mpv/render_gl.h):
blending and scissor testing must be at their disabled GL defaults. Clipping
an offscreen pass with window coordinates can retain pixels from a previous
frame; blending can mix writes with old contents. Workspace changes expose
different clip coordinates and sizes but keep the contaminated shared target.
This is a concrete integration defect, not evidence of a decoder defect or
intentional interpolation. Its causal fit to the reported live symptom still
requires the installed moving-video check below.

There was also a confirmed ordering error: the texture image was queued before
the callback that updated it. The screen sampled the previous paint's frame.
Render errors were discarded and the target was marked valid regardless, so
unchanged-frame reuse could retain an unsuccessful render.

## Fix

- Reserve the callback's paint slot before the image; keep pseudo-frame scrub
  previews and OSD/drop overlays above it. The current render is sampled during
  the same paint, including after texture resizing.
- Enter a scoped GL boundary before `update`, resize and `render`. Disable UI
  compositing/clipping and unbind UI drawing resources before entering libmpv.
- Clear the whole video target to opaque black for actual render passes only.
  Preserve event-driven unchanged-frame reuse and the intentional window-move
  throttling policy. No continuous repaint, decoder polling or disk cache is added.
- Restore independent read/draw framebuffer bindings, viewport, scissor box,
  enable flags and clear color on success, skip and failure. egui_glow restores
  its own shader/VAO/buffers/blend function after the callback.
- Mark the target valid and capture taskbar preview only after successful render.
  Failed passes remain invalid, eligible for retry on the next paint. Repeated
  unchanged errors do not add a warning per frame.

The browser's HTML video renderer does not enter this native GL callback; no
Web API, controller protocol, firmware or timing-safety contract changes.

## Evidence and remaining acceptance

Native `cargo check --offline -q` passed with existing warnings. Added source
guards cover paint order, the disabled blend/scissor boundary, separate FBO
restoration, unchanged-frame skip and failure validity. Tests/CI were not run at
the user's request; these guards do not exercise an actual GPU/driver.

Read-only live inventory found no local `pealayer` process and a Cafe process
in its canonical program-root installation. It was not stopped or replaced;
fresh authoring readiness is required before any replacement. Generated Web
distribution changes remain preserved outside this source commit, so the
clean-source packaging gate is unresolved. No installed-build fix or visual
acceptance is claimed.

After the approved updater delivers a host-compatible package, verify moving
high-contrast video in Simple, resize/letterbox/controls fade, switch repeatedly
to NLE and back, pause/resume, seek/scrub, fullscreen, and taskbar preview. Watch
for retained pixels, one-paint lag, black regions, GL errors and excessive idle
work. A static screenshot alone cannot prove temporal ghosting is gone. Do not
replace Cafe's libmpv with another host's runtime to perform this check.

The later [issue 80 native delivery](issue80-native-delivery.md) cleared the
generated-bundle gate and installed this fix on Cafe through the approved updater
with Cafe's retained runtime. The moving-video matrix above is still open;
neither a smoke probe nor the unusable white-area capture establishes it.
