# Centered timeline timestamps

The native timeline positioned each timestamp at its tick plus four points,
then painted from the text's left edge. Longer clock labels made that offset
particularly conspicuous. The Effects Designer used the same left-offset pattern.

Both native rulers now use measured label bounds centered exactly on the tick.
Partially visible edge labels are omitted rather than shifted away from their
time or allowed to bleed into adjacent headers. Tick timing, zoom spacing,
scrolling, ruler height and existing colors are unchanged.

The Web ruler formerly spread variable-width captions using flex space-between,
so their centers did not match the quarter-duration positions. Each caption is
now anchored to its actual percentage within the track area and translated by
half its width, with a tick at that same center. Existing label/end gutters and
the sticky ruler remain intact.

## Checks and remaining gates

- Compact clock-formatting checks, responsive guards, 53 application-contract
  guards and TypeScript `--noEmit` pass.
- Added a native painted-label regression for short, minute and hour timestamps,
  fractional tick positions and 1×–2× scales, including edge omission without
  reanchoring. It is not yet compiled/run: the user requested batching fixes
  without waiting on builds.
- Native/Web live visual verification and guarded deployment remain queued for
  the next combined build, along with the elapsed-segment selection correction.
  No running application was replaced or restarted. Cafe deployment remains
  user-deferred while offline.
