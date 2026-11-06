# Prevent failed native renders from flashing black

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Report and confirmed defect

The user reports occasional black flashes in the regular application while
watching fullscreen video, with audio continuing normally. The exact trigger
is not established yet. Do not
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

During delivery, both Preferences windows were closed by the user and their
helper processes had exited, but the older apps still reported an unknown helper
because inspection tested only whether a controller handle existed. A hidden
Preferences editor also returned before reaping that exited controller.

The follow-up checks confirmed process exit independently of window visibility.
The existing Windows exit watcher records only a signaled process handle; a
running or uninspectable child remains unknown. Read-only inspection can report
the exited helper as closed without saving/discarding config. Normal UI lifecycle
cleanup reaps it even when preview reconciliation has hidden the editor. The
existing exited-helper regression verifies that unverified and confirmed exit
remain distinct. This does not authorize ignoring other dirty/busy/unknown
editors during deployment.

## Verified candidate checkpoint

- Final native source: `ed6cc9e8962528b460fe1641b4807d204226cd18`.
- Full native suite: 1,144 passed, zero failed, two opt-in tests ignored.
- The real OpenGL fault test was then explicitly run and passed separately on
  the final source, including both same-size and resized failed renders.
- Optimized Windows executable compiled successfully; native runtime smoke
  exited with code 0 using this host's retained runtime.
- Build-cache executable SHA-256:
  `3d1895dc7d82faf94cd52084e677897d26b576d6bf5f818945424a68f5636d7f`.
  This is the raw optimized executable, not a newly published package or an
  installed-build receipt.
- Both Preferences windows were closed by the user; process inspection confirmed
  that only the retained historical comparison's helper remained. Fresh API
  checks of the two newer apps reported only the stale `preferences_helper`
  item as unknown, with all other draft/operation items clean. Permission to
  proceed despite that older-build stale flag was requested, not assumed.

Private final-test, GPU and optimized-build logs remain under the canonical
program root's `verification` directory. The normal apps are still running
their previous executables; no movie playback, board output or historical
comparison was changed to obtain this candidate evidence. Packaging, graceful
installation and sustained fullscreen movie acceptance remain open.
