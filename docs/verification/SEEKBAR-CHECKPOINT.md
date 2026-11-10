# Seekbar consistency and marker checkpoint

## Changes

- Exact cached previews still move mpv; nearby cached frames no longer
  impersonate the requested time. Hold the exact preview through release until
  decoder settlement, require `seeking=false`, and reject older/two-frame-away
  completion samples.
- Shared `scrub_to` / `finish_scrub` commands across API, RPC, IPC and peers.
  Web uses ordered, coalesced previews and a matching settlement revision/target
  instead of discarding its thumb position on mouse-up.
- Chapter ticks are shorter, grey by default, and contained within the rail.
  Near-marker clicks select exact chapter timestamps. The active chapter has a
  subtle range highlight. Chapter numbers/titles are available on hover.
- Taller red keyframe dividers expose names/times; native keyframe menus now
  support naming. All three marker colors are configurable in shared Playback
  preferences.
- Shared semantic swatches in native/Web color pickers, including effects,
  lighting, channel colors and board controls.
- Reconciles the user-visible ruler-wheel intent of [PR #92](https://github.com/ToghrolTP/pealayer/pull/92):
  one nominal frame per plain vertical wheel event, immediate logical head,
  pause-on-step. Horizontal and modifier gestures remain intact. The arbitrary
  nearest-cache path and unrestricted cached drawing are intentionally not
  retained. Decoder-owned keyboard stepping remains available for VFR media.

## Verification and remaining gates

TypeScript compilation, Vite production build and PWA regeneration passed.
Native BuildOnly passed before final ruler reconciliation; the final incremental
build result is supplied in the PR checkpoint. Regression sources were added
or updated, but tests were not run in this pass as requested by the user.

No running canonical executable was replaced by this worktree. The coordinated
release owner must independently review the exact PR head, package with each
host's actual libmpv runtime, gracefully deploy, and verify on installed Cafe
and David. Particularly check David as Cafe's remote consumer: playing/paused
scrub and release, no frame replacement on release, exact fractional chapter
clicks, keyframe naming/hover, preferences propagation, and phone/tablet/desktop
marker layout. These are pending acceptance gates, not claims of delivery.

Related: [frame-accurate navigation issue #66](https://github.com/ToghrolTP/pealayer/issues/66)
and [Web parity issue #63](https://github.com/ToghrolTP/pealayer/issues/63).
Implementation/usage details: [Seekbar markers and seeking](../SEEKBAR-MARKERS-AND-SEEKING.md).
