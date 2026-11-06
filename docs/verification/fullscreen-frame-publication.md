# Prevent failed native renders from flashing black

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Report and confirmed defect

The user reports occasional black flashes while watching fullscreen video.
The exact trigger and whether audio continued are not established yet. Do not
attribute every flash to a decoder, driver, display or timing fault without
reproduction evidence.

The shared native renderer contained a concrete black-frame exposure path:

1. It cleared the texture currently displayed by egui to black.
2. It called libmpv to replace its contents.
3. A failed render left the cleared/partially drawn texture visible. Marking its
   metadata invalid did not hide or restore that texture.

Resizing also discarded the displayed texture's storage before checking/rendering
the replacement. These paths are shared by Simple, NLE and fullscreen, not
specific to either workspace adapter. The [earlier GL isolation repair](video-ghosting.md)
remains intact; this change corrects publication on failure, not its blend/clip
boundary. The [mpv OpenGL interop contract](https://github.com/mpv-player/mpv/blob/master/include/mpv/render_gl.h)
still governs the decoder boundary.

## Transactional GPU publication

- Render/clear privately in one lazily allocated GPU texture/FBO pair. No
  per-frame allocation of GL objects or CPU video readback is introduced.
- Only after success, resize the presentation storage if necessary and copy the
  completed frame to the stable texture registered with egui using a GPU blit.
- On decoder failure keep the previously published pixels, dimensions and valid
  state. Keep decoder work pending, bypass idle reuse for the failed pass, and
  schedule at most three 50 ms retry wakeups for paused playback. Further work
  remains driven by real decoder/UI events; no unbounded retry repaint loop.
- Keep callback-before-image ordering, scoped GL restoration, unchanged-frame
  reuse, OSD ordering and taskbar capture of the committed video frame.
- Existing local performance diagnostics now expose `mpv_failed_count`,
  `mpv_update_failed_count` and `mpv_target_resize_count`. These are bounded
  counters, not media logs, frame readbacks or extra decoder polling.

## Actual GPU regression

The explicit Windows OpenGL test
`mpv::render::tests::gpu_failed_render_and_resize_preserve_displayed_pixels`
ran on NVIDIA GeForce GTX 1080 / OpenGL 3.3.0 NVIDIA 576.02 and passed:

1. Publish a green frame and read its real framebuffer pixel.
2. Clear the private target to black, inject a render failure, and verify that
   the displayed pixel remains green.
3. Repeat with a differently sized target; the displayed pixel still remains
   green.
4. Successfully publish the resized red frame and verify it becomes visible.
5. Verify no OpenGL error was left behind.

The probe creates an isolated hidden GL context, exits during app creation,
does not start a player/controller, and never controls another window. Its
persistence path is derived from a parent-provided isolated config path.
It is explicitly opt-in for environments without a local GPU/display.

This proves GPU publication survives injected failures; it does not prove the
intermittent movie symptom is fully resolved. Long fullscreen playback,
pause/resume, controls/OSD fade, seeking and taskbar-preview coexistence still
require installed-build observation. If flashes continue, correlate the new
counters with the event before changing decoding or operating-system settings.

## Delivery safety

Do not replace the regular app or comparison while native Preferences reports
an unknown draft. Preserve those edits and the retained October 10 executable.
Cafe deployment remains user-deferred until its normal connection returns.
