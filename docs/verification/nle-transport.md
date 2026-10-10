# Contextual NLE transport

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

The native Program Monitor uses the smallest number of rows that fits its
measured controls and a usable seekbar. A wide monitor has one row. Narrower
layouts keep track selection immediately before volume at the trailing end of
the final row; exceptionally narrow docks place volume on its own final row.
The video reserves the exact footer height rather than a fixed compact cutoff.

Play/Replay and Stop are neutral at rest, with intent color on enabled
hover/focus/press. Pause indicates active playback. Without loaded media,
transport uses normal disabled visuals and cannot activate. Stop is a plain
square and retains the session-authority Stop/recording punch-out behavior;
paused loaded media can still be stopped/closed.

Verification covers exact-fit row selection, real egui group geometry across
eight widths, track-before-volume ordering, actual track/volume widget bounds,
and enabled/disabled/active styling in both palettes and light/dark themes.
The shared elapsed, seek, chapter and nudge behavior is unchanged.

Deployment evidence will be recorded after the destination runtime and live
application identity have been verified. Tests are not human pixel acceptance.
