# Seekbar markers and release-frame consistency

## Using the seekbar

- Chapters use muted grey ticks contained inside the rail. Clicking within
  six pixels of a tick selects its exact chapter timestamp, not a rounded
  percentage. The currently playing chapter has a subtle range highlight.
- Hover shows chapter number and title. Web thumbnail captions retain a fixed
  height and ellipsize longer chapter names, with the full name in the tooltip.
- Project keyframes use taller red dividers. Hover shows their name and exact
  time; the native timeline keyframe context menu includes a Name editor.
- Preferences → Playback → Seekbar markers controls chapter, active-chapter
  background and keyframe colors. These are shared configuration, not separate
  browser-only settings.
- Color pickers share `assets/themes/ui-colors.json`: red, orange, amber,
  Pealayer green, blue, cyan, violet, grey, white and black. The catalog is
  available in HEX fields, compact native color buttons, and Web pickers.

## Scrub contract

Preview and release both use absolute exact mpv seeks. A cached preview may
only substitute the exact timestamp and still dispatches a decoder seek. An
arbitrary nearby cached frame is never presented as the requested frame. An
exact cached preview remains visible through release until final decoding
settles, rather than exposing the old mpv texture on mouse-up.

The final command must be dispatched, playback restarted, `seeking` false,
and the actual decoder time within one media-frame interval plus 2 ms of the
requested timestamp. Unavailable decoder time does not fabricate success.
mpv still selects a real source frame for timestamps between frames; paused
logical timeline time retains the requested millisecond value.

Web and peer gestures use the shared commands:

```json
{"command":"scrub_to","seconds":10.125}
{"command":"finish_scrub","seconds":10.125}
```

JSON-RPC methods are `pealayer.scrub_to` and `pealayer.finish_scrub`; text IPC
accepts `scrub_to 10.125` and `finish_scrub 10.125`. Both validate finite,
non-negative seconds. Preview sends are throttled to at most about 30 Hz,
coalesced, and ordered before the final release command. Pointer cancellation
and window blur commit the last gesture target. Media changes fence stale
queued previews and commits.

The status snapshot publishes `seek_pending`, `settled_seek_revision` and
`settled_seek_target`. A Web draft is released only after a newer matching
decoder settlement. Comparing a playing clock with a narrow target tolerance
would miss the acknowledgement after playback has already advanced.

## Ruler-wheel integration from PR #92

Plain vertical wheel events over the ruler advance one nominal media frame per
event, regardless of Windows line-magnitude settings. The logical playhead
updates immediately using the same scrub/commit path and remains paused.
Horizontal scrolling and configured modifier gestures are preserved. Point
jitter below one pixel is ignored; toolbar hover does not trigger ruler steps.

The decoder-owned keyboard `frame-step`/`frame-back-step` paths remain intact,
especially for variable-rate media. Ruler time quantization uses advertised
media FPS; if it is unknown, native mpv stepping is used instead of inventing a
30 FPS rate. PR #92's 20 ms/nearest-cache fallback and unconditional cached
texture drawing are intentionally not retained.

## Acceptance checkpoint

Source/regression cases cover exact-cache dispatch, nearby-frame rejection,
chapter pixel hit range, marker defaults/validation, shared palette and command
contracts. Tests were added/updated but not run in this pass at the user's
request. TypeScript, Vite/PWA generation and native BuildOnly are recorded in
the PR checkpoint.

Cafe-compatible deployment and live verification remain release-owner gates:
paused and playing scrub/release, exact fractional chapter starts, keyframe
name hover, phone/tablet/desktop marker layout, and David consuming Cafe. A
compiled build is not evidence that those installed-host checks passed.
