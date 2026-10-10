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

Twenty-two shared transport tests and four NLE tests passed. Windows packaging
passed Web/PWA guardrails, resources/icons, libmpv smoke and downloader smoke.
Package-wide tests and UPX were skipped for this incremental delivery.

Hosted CI exposed an existing downloader UI fixture race: its refused dummy
connection could fail before Pause. The widget test now exercises real queue
actions without an HTTP scheduler; the separate transfer fixture still owns
network acceptance. All sixteen downloader unit/transfer tests passed; three
external-tool integration tests were explicitly ignored. This test-only
correction does not change deployed code.

## Cafe delivery · 10 October 2026

- Clean built source: `1697895d5e2e699a227ab3e53cef88d71e688541`.
- Executable SHA-256:
  `b8f1503b509583309d3b4ec548784f53f8b0acfdb7db215a32dd8d8a3a96c5e9`.
- Retained destination libmpv SHA-256:
  `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
- Peer updater: `update-6eed9abd-0f30-4a40-8394-9e63bb38a937`, completed.
- Live PID 96104, interactive session 1, canonical installed `bin`.
- NLE workspace, loaded media and paused position preserved. The authoritative
  cue array was empty before and after; this is not nonempty-cue acceptance.
- Controller connected, no hardware error, no E-STOP or new Application/WER
  fault during deployment. No motion or relay actions were invoked.

An isolated local runtime launch was blocked by automatic approval review in
the preceding pass; it was not rerouted. Tests and live deployment health do
not establish human native pixel/interaction acceptance, which remains open.
