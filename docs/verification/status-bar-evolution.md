# Status bar: readable captions and balanced typography

Tracker: [status-bar evolution](https://github.com/ToghrolTP/pealayer/issues/228).
Implementation: [workspace and typography PR](https://github.com/ToghrolTP/pealayer/pull/225).

## Regressions and repairs

The connection widget had a fixed 148-point slot (252 with transport shown).
Changing the UI typeface to the native semibold face made “Hardware:
Disconnected” wider, but the slot never used the otherwise empty bar. The slot
now measures its actual localized caption, icon and selected font. Both anchored
groups reserve that width, bounded by the remaining space. Narrow bars retain
truncation/tooltips and horizontal group scrolling; numeric telemetry slots and
the trailing workspace anchor remain stable.

FPS used vertical line-box centering. Different ascenders, descenders and line
gaps in native monospace faces put the painted glyphs off-center even when the
layout box was centered. Status captions and FPS now center their visible glyph
bounds; no font-specific pixel offset is used.

Windows does not expose a desktop monospace preference in NONCLIENTMETRICS.
The generic DirectWrite monospace match can resolve to Courier New. System mode
now chooses an installed Segoe UI Mono / Segoe Monospace, Cascadia Mono, or
Consolas in that order, with the generic fallback only if none exists. Custom
overrides remain available. Linux desktop and macOS fixed-pitch preferences are
unchanged. Fontique's requested weight is explicitly carried into egui's `wght`
axis, clamped to the actual face's range; this also repairs installed YekanBakh
VF falling back to its thin default. No private font is bundled.

## Verification

- 20 status-bar tests pass, including new native/bundled visible-glyph centering
  and unelided hardware-caption checks at 100%, 125%, 150% and 200% scale; LTR,
  RTL, start/end placements and narrow bars are covered.
- 4 font tests pass, including installed variable weight coordinates, modern
  Windows family selection, monospace advances and fallback preservation.
- Test processes use an isolated configuration supplied before startup. UI
  fixtures use the memory-only engine, never a live hardware worker.

## Genuine visual history

Representative source milestones: compact two-sided widgets (`48fd59c`, PR
#166), CraftUI surface styling (`9610177`, PR #216), and native typography
(`d6feabf`, PR #225; status implementation introduced in `1fbed60`).

Historical screenshots come only from real compiled applications, never a
current widget with a reconstructed theme. Use a separate explicit profile with
hardware auto-connect and last-media restoration disabled, the same dark theme
and 1320 × 860 logical window geometry. Idle screen-FPS readings are real and
will differ between captures. No board values or media are fabricated.

`scripts/start-status-review.ps1` launches an already built executable with that
profile and a loopback-only listener. `capture-visible-window-windows.ps1`
captures the real window through read-only Win32 PrintWindow. Crop only the
status bar for public comparison; inspect for private data before publishing.
Use the existing detached comparison worktree and shared build cache, with its
already-generated host configuration (do not add a duplicate Cargo config).
Do not copy another machine's libmpv or publish executables/private profiles.

The current before-fix capture reproduces the short hardware label despite a
large empty area. Historical rebuild and repaired-runtime evidence are tracked
in the issue. Compilation and screenshots are not production deployment proof.
