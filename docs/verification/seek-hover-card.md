# One seekbar hover card

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Cause and current contract

Seek hover had several independent owners: native chapter/keyframe tooltips,
Simple's generic seek hint, the decoded-thumbnail popup, and Web slider/marker
titles. Disabling thumbnails removed the entire card and exposed competing
fallbacks. Marker hover could substitute a marker/chapter boundary for cursor
time; Web also displayed the floored thumbnail-cache second. The native image
card could be positioned outside the viewport when the seekbar was near its top.

Simple and the NLE Program Monitor now share one card in native and Web UI:

- Time is mapped from the actual slider value range/rail at the pointer, never
  chapter snapping or current playback position. The milliseconds preference
  is honored; integer-second thumbnail caching does not change the caption.
- The containing chapter number/name and a nearby named keyframe live inside
  that same card. Their labels do not replace its cursor timestamp.
- Existing per-workspace thumbnail flags control only the decoded-image section.
  They retain their saved values and default-off behavior. Preferences wording
  explains this; image-disabled hover still has one position/chapter card.
- Loading and unavailable images stay inside the same reserved image slot.
  No native marker/generic hover tooltip, browser title or Ant slider tooltip
  competes with the card. Accessible chapter labels and chapter snap clicks remain.
- The card follows pointer X, fits horizontal viewport edges and opens below a
  top-edge seekbar when there is insufficient space above. Web measures wrapped
  card content before positioning and clears stale hover on scroll/resize/blur.
- Active scrubbing suppresses hover; ordered preview/final seek and correlated
  decoder settlement are unchanged. No new playback/board command is sent on hover.
- A non-hovered native surface cannot clear another surface's pending thumbnail.
  Existing bounded single-worker debounce and cache/proxy behavior are retained.

## Verified and remaining boundaries

Native headless fixtures exercise the production slider/marker/card renderer for
both surface identities with no image, loading, injected ready image and injected
failure. They require exactly one cursor timestamp, chapter label and keyframe
label, reject the chapter-start timestamp and generic hint, and never decode
media or actuate hardware. Pure tests cover rail mapping, clipping, top-edge
placement and cross-surface request ownership. The native suite passed 858 tests
with one existing ignored test.

Web helper regressions cover exact time versus decode bucket, RTL/rail mapping,
viewport edges, one portal, removal of competing tooltips and both workspaces'
shared integration. TypeScript and the full responsive/parity/PWA build passed.
These are source/render/build checks, not a live installed screenshot or a real
decoder-thumbnail acceptance test.

Local Pealayer was closed at verification. Previous GUI-launch and generated-file
cleanup denials remain in force; no alternative launcher/deletion path was used.
Untracked generated Web outputs are preserved privately and not committed.
They block the clean-source production packager. Cafe's prior interactive
smoke-launch denial also remains a runtime-delivery gate. A source fast-forward
does not install this fix. Use a clean approved package and the host-validated
runtime via the peer updater once these gates are cleared; verify hover on real
media in Simple and Program Monitor with thumbnails enabled and disabled, edges,
named chapters/keyframes, drag suppression and a genuinely current screenshot.
