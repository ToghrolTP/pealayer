# v0.5.0-alpha.1 egui Dock comparison

Version identity: [v0.5.0-alpha.1](../releases/v0.5.0-alpha.1.md), source
`ad4b466fbe40e4556edf4bd9594d3f4eb468329c`. The October 10 date below is
historical context, not the build's product version. Original captures, filenames,
hashes and launchers are preserved.

An actual full application build, retained for the user's ongoing UI review.
This is not today's application restyled to impersonate an older release.
Owner: [acceptance tracker #80](https://github.com/ToghrolTP/pealayer/issues/80).
Related: [Dock restoration](egui-dock-restoration.md) and the earlier
[status-bar history](status-bar-history-audit.md).

## Checkpoint selection

PakarController's initial commit is `5512b6c`, **10 October 2026, 00:44 +03:30**.
The requested Pealayer comparison therefore comes from October 10, not the
immediate parent of the later CraftUI PR.

| Pealayer checkpoint | Date | Relevance |
| --- | --- | --- |
| `78563ff` | October 9, 22:58 +03:30 | Last mainline change immediately before PakarController's creation; native Persian shaping. |
| `cc74787` | October 10 | Contextual NLE transport merged. |
| `fa062a9` | October 10 | Effects Library alignment and hover-action refinement. |
| `759dafe` / merge `28de6ba` | October 10 | Unified dropdown navigation and color-picker rows. |
| **`ad4b466fbe40e4556edf4bd9594d3f4eb468329c`** | **October 10, 22:52:59 +03:30** | Selected last mainline checkpoint of that day; its final merge only adds deployment documentation. |
| `37c8acb` | October 26 | Craft workspace renderer integration; not an October 10 baseline. |
| merge `aeb0a2f` / PR #216 | November 5 | CraftUI migration merged into main. |

The historical source uses its own Cargo lockfile, egui **0.36.2** and egui Dock
**0.21.1**. No Rust/UI source changes, backported widgets or theme patches were
made. An existing clean comparison worktree and the shared Cargo cache were
reused; no second dependency cache or new worktree was created.

## Matching appearance and genuine captures

Both isolated profiles report **Dark, Studio, custom accent `#6362c7`** through
their own APIs. The palette JSON is byte-identical between the selected source
and today's source. Different widget chrome/typography can therefore be studied
without confusing it with different palette preferences. Native font selection
and widgets remain those actually implemented by each build.

The main-window captures are both **2560 × 1392**. Preferences were also captured
at matching sizes. Occluded capture initially returned the other foreground
application; that image was rejected. Each final capture was inspected with its
correct title visible. The separate tiny icon image returned by Windows capture
is not used as a full-window screenshot. These are real empty/offline interfaces,
not simulated connected boards, effects, playback or telemetry.

| Surface | October 10 | Current egui Dock |
| --- | --- | --- |
| Full workspace | [Full screenshot](../screenshots/ui-history-october10-full.png) | [Full screenshot](../screenshots/ui-history-current-dock-full.png) |
| Preferences | [Screenshot](../screenshots/ui-history-october10-preferences.png) | [Screenshot](../screenshots/ui-history-current-dock-preferences.png) |

### Initial findings, not an exhaustive regression verdict

| Area | Observed difference | Investigation boundary |
| --- | --- | --- |
| Dock chrome | Compact disclosure and close controls exist in both. The current view additionally reserves space for a trailing activity rail. | The rail was introduced in the CraftUI work and is drawn outside the renderer selection; it is not evidence that October 10 had that rail. Preserve CraftUI while reviewing the desired Dock-only behavior. |
| Transport | The original compact controls contrast with today's larger rounded controls and customizable action groups. Both have a volume slider. | The customizable bar changed after October 10; do not attribute every difference to the Dock adapter itself. Narrow layouts deserve an additional comparison. |
| Effects Library | Original centered heading/Manage effects versus today's leading heading/Effects Designer action. Search/action sizes also differ. | Distinguish intentional naming/layout changes from accidental alignment or hit-target regressions. Empty-state screenshots cannot establish populated-card parity. |
| Status bar | Original compact normal-weight hardware caption versus current emphasized semantic caption, icon and workspace selector. | Reuse the existing status history and typography tests; these are different original designs, not two screenshots of one late revision. |
| Preferences | Historical ungrouped navigation and thin selectors versus current grouped navigation, palette swatch, density and typography controls. | Expected newer features remain available; compactness and alignment can be compared without removing them. Both Preferences are separate native windows. |
| Window placement | The comparison windows required explicit maximization for equal-size capture; a parent returned to normal bounds after the Preferences walkthrough. | Observed behavior only, not a proven root cause or a fix. Check helper/main geometry ownership separately before changing production code. |

## Preserved installation and safe launch

The complete historical package lives under
`%LOCALAPPDATA%\Programs\Pealayer\investigation\pre-craftui-2026-10-10`.
It contains its own executable, runtime, Web assets, companion downloader,
manifest and a separate `profile` directory. A retention notice explicitly
excludes it from stale-artifact cleanup **until the user asks for removal**.
Do not deploy this obsolete comparison onto the ordinary installation or Cafe.

`scripts/start-ui-history-review.ps1` opens Historical, Current or Both. Ports
18092/18093, independent configuration/workspace storage, instance IDs and titles
separate them from the regular app on 8080. It preserves existing review edits,
restores its parent environment and refuses automatic hardware/global-key use.
The review endpoints are quarantined to unused loopback port 18999; merely
opening the apps never connects to the production coordinator. Clipboard URL
detection and update access are also disabled in these profiles. The launcher
was exercised with both running and reused them without creating duplicates.

## Build and runtime evidence

- Historical source: `ad4b466fbe40e4556edf4bd9594d3f4eb468329c`, clean worktree.
- Historical executable SHA-256:
  `e76b61a657fe4ba604c42724ac164632ec1e51689cf7be22005cabdbaf32c345`.
- Current native source: `050fd1c61990982ba272efb494aab28f3406f8be`, verified via
  the running comparison's authoring API.
- Current executable SHA-256:
  `b33a9e8cfca36b1022c5ddb00e69b48d3ddcedbb140a84ad3a5cd2a9c17b438e`.
- Both use the same host-compatible libmpv runtime; its SHA-256 is
  `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
- Historical full release packaging passed Web compilation/parity/timecode/
  preferences/wheel/messaging/PWA/asset checks, Windows resources and all seven
  quick-action icon checks, libmpv smoke and companion downloader smoke.
  Native unit-test reruns and UPX were explicitly skipped for the historical
  package; this is UI investigation, not historical production certification.
- Both live APIs confirm no loaded media, no hardware connection, disabled
  auto-connect and matching appearance. No hardware output was actuated.
- The ordinary current application remains open and unmodified. Both comparison
  mains are left open for user feedback; their inspection-only Preferences were
  closed normally without edits.

Continue from these retained binaries and screenshots. Record user-reported
inconsistencies against this exact baseline instead of rebuilding an arbitrary
late pre-CraftUI commit. Connected, populated, playing, floating and narrow-layout
states remain to be reviewed; this initial comparison does not declare them passed.
