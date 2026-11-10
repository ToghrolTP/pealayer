# Renamable seat titles

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

Seat identity and display titles are separate. `seat.a` / `seat.b`, timeline
track keys and action IDs do not change when a seat is renamed. Hardware cards,
NLE tracks and Effects Designer lanes/selectors use the controller's current
profile-resolved title. Host-wide `peripheral_names` are a fallback, never an
override of an explicit resolved board-profile title.

This pass fixes three inconsistent paths:

- Peripheral output descriptions previously ignored their resolved `name`.
- Raw-motion seat controls synthesized before a profile was configured ignored
  saved seat titles. They now use the same catalog title resolution, while
  retaining an independent `default_name` for an explicit Reset.
- Native and Web effect lanes invented `Seat A` / `Seat B` when a catalog name
  was absent. They now show the stable key when the title is unknown. Legitimate
  catalog defaults remain valid when no custom name exists.

Blank control titles fall back to the named peripheral descriptor. User-authored
effect/cue names are preserved; they are not rewritten by matching title text.
No configuration migration, board action or motion command is involved.

## Verification

- The existing controller route advertised `seat.a = Seat Left` and
  `seat.b = Seat Right` during the 2026-10-11 read-only check.
- `node scripts/check-build-policy.mjs` passed without compilation.
- Web cue-authoring regression checks passed: current names, a subsequent
  rename, a Persian title, blank/default handling and no-catalog key fallback.
- `npm run build` passed TypeScript, authoring/folder/layout/parity/timecode/
  preferences/messaging/clipboard checks and PWA/offline asset verification.
  The tracked embedded Web bundle was regenerated from current source once;
  it also removes stale chunk references from the previously merged designer.
- Native tests cover profile-title priority over host-wide names, raw-motion
  rename preservation, immediate rename/reset of both seats, stable actions and
  track identities, and painted renamed lanes while retargeting a motion cue.
  **82 focused regressions passed** (39 controller, 24 Effects Designer, 17
  Hardware Monitor and two timeline tests). Windows MSVC compile/link passed;
  existing deprecation/dead-code warnings remain outside this title fix.
  Test setup selected this host's libmpv, isolated `PEALAYER_CONFIG_FILE` before
  startup, and the existing canonical cache with incremental snapshots disabled.
  After cached test-target compilation, regression groups ran directly from the
  same library-test executable; no clean build or repeated dependency build.

## Deployment boundary

Cafe's recorded smoke-launch tool denial in [current acceptance](CURRENT-ACCEPTANCE.md)
still blocks installed-build replacement and live native acceptance. This source
fix and rebuilt Web bundle are not proof that Cafe's installed application has
changed. No alternate launch mechanism was used. The next authorized deployment
must use the peer updater with Cafe's validated libmpv and confirm the renamed
titles in Hardware Monitor, NLE tracks and Effects Designer without moving seats.
