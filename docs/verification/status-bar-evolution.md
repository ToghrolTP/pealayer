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

The React footer had the same 148/252-pixel ceiling. Its connection widget now
uses intrinsic content width and may shrink within the scrollable group. Only
the caption ellipsizes under real space pressure; its icon cannot shrink.

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

The following genuine captures are published. The two historical rebuilds used
unmodified tracked source and their locked dependencies; the process-local API
confirmed the expected commit before capture. All windows were 2002 × 1346
physical pixels; the public images are the bottom 56 pixels, unretouched.

| Milestone | Source | Capture | Screenshot-only discussion |
| --- | --- | --- | --- |
| Compact two-sided widgets | `48fd59ce65022b039d28570288ff967bf52bca8e` | [Status bar](../screenshots/status-bar-compact.png) | PR #166 |
| CraftUI surfaces | `9610177510c02f393c56bd024625c70b21442c99` | [Status bar](../screenshots/status-bar-craft.png) | [PR #216](https://github.com/ToghrolTP/pealayer/pull/216#issuecomment-6647068767) |
| Native typography before repair | `d6feabf8c0956523aa5050bf978975cee8b6c431` | [Status bar](../screenshots/status-bar-native-before.png) | [PR #225](https://github.com/ToghrolTP/pealayer/pull/225#issuecomment-6647048566) |
| Repaired native layout | `af070c1cd3161514ea035dc2da2ed0a46485f7f5` | [Status bar](../screenshots/status-bar-fixed.png) | PR #225 |

The unused-space ellipsis is present in all three: the fixed slot predates the
native font work. FPS typeface/baseline and the trailing workspace presentation
did change. No historical playback-rate or CPU claim is inferred from the idle
FPS readings in these screenshots.

Executable SHA-256 provenance for the fresh historical builds:

- Compact: `406fe80e7d890faf8464606471bfca8ce43a3ba3d4593450d7d73684750b0847`.
- CraftUI: `59b265ac0fddac383e38c082392bb593283b5469f8a4a906c64f950e32ed8ee0`.

The repaired native package passed host-libmpv, utility, icon-resource and Web
bundle/PWA checks, and reported the clean source revision through the API. The
actual capture shows the full hardware caption and centered FPS using Cascadia
Mono on this installation. The follow-up Web width repair is separately covered
by the status-widget guardrail and TypeScript/bundle checks.

Compilation and screenshots are not production deployment proof; the Cafe SSH
endpoint refused connection during this pass.
