# Idle video work and CPU verification

Owners: [CPU acceptance](https://github.com/ToghrolTP/pealayer/issues/62) and
[integrated acceptance](https://github.com/ToghrolTP/pealayer/issues/80).

Cafe's paused visible player used roughly 89-91 percent of one CPU core during
bounded samples. Minimizing through its native command reduced a sample to
1.9 percent while commands and controller feedback remained active. This
narrows the cost to visible work; it does not identify the renderer by itself.

The video callback entered libmpv, queried/reset/restored OpenGL state, and only
then found that the paused texture could be reused. Every board/hover repaint
therefore entered that boundary despite no decoder update. The decoder's
existing update callback now marks a per-context atomic signal, even when native
window-move pacing suppresses its immediate repaint. A valid, unchanged-size
texture with no pending decoder work returns before any MPV/GL API call. Initial,
resized, invalid, failed and decoder-updated frames still enter the complete GL
state guard. All actual MPV API calls remain guarded; no assumption that
`mpv_render_context_update` leaves GL state untouched is made. This preserves
the shared Simple/NLE/fullscreen renderer and the ghost-frame fix.

The signal coalesces repeated callbacks and retains a callback arriving after
consumption. Deferred/locked paint exits occur before consuming it. Geometry,
clear-before-render, failure invalidation, independent framebuffer restoration
and paint-before-texture-sampling regressions remain required.

`GET /api/diagnostics/performance` is loopback-only and reports fixed-size
cumulative native logic/UI/video-callback timing, existing render/reuse counts
and the actual bounded OpenGL identity. It records no history, paths, media,
files or board commands and issues no per-frame probes. Timing is wall duration
of those phases, not a claim of processor CPU time; compare counter deltas with
separate process CPU samples. Renderer identity is queried once on the paint
thread, not in a network handler. Source checks and this optimization alone do
not close sustained playback, GPU, physical timing or CPU acceptance.

Live Cafe timing subsequently showed no actual paused MPV render passes and
roughly 1 microsecond per cached video callback, but about 18-20 milliseconds
for UI layout. Panel timing attributed approximately 10 ms to Hardware Monitor,
3 ms to Effects Library and 2.5 ms to Timeline. Thus video reuse is verified,
not a claim that the CPU problem was solved. The actual renderer is Intel HD
Graphics 4000 rather than a guessed software renderer.

Hardware button safety checks repeatedly called the owned snapshot accessor,
deep-copying the full hardware/effect catalog for a boolean programming gate.
Read-only checks now share an immutable `Arc` at the same hardware revision;
the existing owned accessor remains available to mutation/serialization callers.
Tests retain cached/paused RGB and revision invalidation and verify that repeated
reads share a pointer, while a new revision cannot modify an earlier view.
No live-update cadence, latch, ownership, E-STOP or action validation is weakened.
