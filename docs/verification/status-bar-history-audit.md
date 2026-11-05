# Complete status-bar source history

Companion to the [chronological screenshot gallery](status-bar-evolution.md)
and [issue #228](https://github.com/ToghrolTP/pealayer/issues/228).

The initial gallery incorrectly selected only late revisions of one design.
This audit starts with the first component and its first actual integration.
It includes all 46 non-merge changes to `src/ui/status_bar.rs` reachable from
the locally fetched refs through the typography repair, including the recovered
work branch. Changes outside that file are listed separately. A source-history
entry is not a claim that its connected/playing/error states were captured.

Reproduce the component inventory:

```sh
git log --all --full-history --no-merges --reverse --format='%h %s' -- src/ui/status_bar.rs
git log --all --reverse -G 'status_bar|System Ready|Memory: 42' -- src/app.rs src/main.rs src/ui/mod.rs
git log --all --reverse --grep='status bar' --grep='statusbar' --grep='FPS' --grep='footer'
```

## Component changes, in order

**Layout** changes affect arrangement/typography; **state** changes concern
conditional content or behavior; **maintenance** entries do not create a new
idle layout. These distinctions prevent relabeling identical idle screenshots
as separate designs. Source identifiers belong here, not in the gallery titles.

| Source | Class | What actually changed |
| --- | --- | --- |
| `dc49b29` | Layout | Initial FPS, placeholder memory, pulsing System Ready and trailing workspace component. Not yet registered or called. |
| `3bc0e08` | Layout/state | Hardware dot/caption and conditional E-STOP added to the running dock layout. |
| `cb36bc7` | State | Coordinator connection distinguished from direct diagnostic serial connection. |
| `bab90c8` | Layout/state | Removed invented memory/System Ready; added amber connecting state. |
| `c0d5769` | Layout | Removed unreliable emoji from E-STOP text. |
| `537237d` | State | Distinguish coordinator availability from actual board availability and name the advertised board. |
| `583a7ff` | Layout | Persian labels and RTL-aware outer layout. |
| `f0e2866` | State/maintenance | Shape board names for display; format and reconcile capability/Persian work. |
| `d0c3680` | State | Show selected transport suffix from the embedded/coordinator connection. |
| `5a83786` | Layout/state | Replace UI FPS with real media FPS; add physical RGB, voltage, current, temperature and warning details. |
| `4f9ec05` | Maintenance | Preserved recovered branch: status-bar blob matches the capability/Persian work in f0e2866, not a new design. |
| `a810fd8` | Maintenance | Explicit float type for LED outline. |
| `eae3538` | Layout/state | Configurable status visibility, context menu, friendly board identity, clickable RGB/warnings. |
| `f023ab7` | Maintenance | Translation borrow repair in status context menu. |
| `90216c4` | Maintenance | Explicit float type in bar; shared icon system changed elsewhere in this commit. |
| `fb3e851` | State | Prefer live board name over profile-name fallback. |
| `8643b10` | Layout/state | Two-decimal media FPS; stabilized source sampling in app code. |
| `56d3946` | Layout/state | Separate RGB/warning visibility; correct authoritative RGB without double brightness scaling; subtle glow. |
| `0ec9ffc` | Interaction | Per-item Hide context menus and correctly owned bar context menu. |
| `1645bc6` | State | Remove unconditional RGB repaint timer; no new idle layout. |
| `390ad55` | Layout/state | Transient OSD messages also appear in the bar with shared lifetime. |
| `d42fd45` | State | Display actual saved workspace profile name. |
| `aa17a65` | State | Active updater message/progress takes priority in status text. |
| `7974460` | State | Honor per-message OSD timeout override. |
| `d255315` | Layout/state | Hardware timeline preparation/fault/readiness caption and acknowledgement details. |
| `3d33e41` | State | Separate temporary Hardware waiting from permanent faults. |
| `0912d45` | Interaction | RGB/warning actions use the corrected Preferences opening lifecycle. |
| `ce85904` | Layout/state | Unified FPS display with UI fallback; hide hardware plan when disconnected. |
| `c1b15be` | Layout/interaction | Media/UI FPS icons, click toggle and explicit menu; reactive hardware refresh and deferred settings save. |
| `759dafe` | Interaction | Use shared keyboard-aware dropdown choice for FPS mode. |
| `a242919` | Layout | Reorderable widgets, chart history, actual operation progress and HOT alerts; workspace initially part of the same scrolling group. |
| `d4acfc7` | State/tests | Measured screen FPS retained through reactive idle; no continuous repaint requirement. |
| `3fb294e` | Layout | Fixed metric slots, strong status fonts, workspace restored to trailing edge. |
| `634abe3` | Layout | Paint FPS icon and numeric text independently, keeping digit positions stable. |
| `3161a6a` | State/tests | Invalidate peer snapshot cache before repaint; test paused RGB frame changes. |
| `8e9a90e` | State | Use shared hardware preparation presentation and retain explicit Play/fault visibility for empty plans. |
| `8c0b445` | Layout/interaction | Dedicated trailing workspace selector with keyboard navigation and management entry. |
| `48fd59c` | Layout/interaction | Compact two-sided slots, per-widget monospace, semantic limits and menus. Connection width reduced from 200 to 148 points: origin of the reported clipping. |
| `51dc532` | State | Programming latch is not misreported as disconnection. |
| `2bf892e` | Layout/interaction | Wider labeled workspace selector, connection icon/transport choices, copy/actions and chart controls. |
| `71b4bef` | Maintenance | Attribute render timing to status surface; no layout change. |
| `7e9b131` | State | Host/board/transfer operation labels and semantic progress colors from real operations. |
| `23eefb1` | State | Do not show coordinator preparation for demo/direct endpoints. |
| `9610177` | Layout | CraftUI well background and separator. |
| `1fbed60` | Layout | Native/custom typography and selectable workspace chrome. |
| `7d9e8f9` | Layout | Measured connection width and visible-glyph vertical centering; modern native monospace and variable-font weights. |

## Integration and changes outside the component

| Source | Relevance |
| --- | --- |
| `87c2668` | Registers the new status-bar module. The prior dc49b29 component alone is not evidence of a visible footer. |
| `e8e6b96` | First call from the dock application's frame; earliest running bar captured in the gallery. Parent app had no status-bar draw call. |
| `3b9670c` | Removes other invented runtime state and adds explicit configuration-path isolation used by later review builds. |
| `eb2d41a` | Clears stale media FPS when changing files; avoids attributing the previous source's frame rate to the new one. |
| `eb53fcd` | Active playback repaint pacing follows VSync/display refresh instead of a fixed cap; changes the observed UI rate, not the bar layout. |
| `f138354` | Removes the corresponding Web footer connection-width ceiling. |

Merges that touched the component (`6a888af`, `386076b`, `8bfa51a`,
`57ae890`) reconcile the above changes; they are not independent new designs.
The preserved `4f9ec05` work is reachable from main and the current UI branch.
No historical source is reset, deleted or rewritten for this comparison.

## Capture limits

The historical full applications are built with each revision's own lockfile.
No modern status widget is restyled to impersonate a historical one. Source
and executable digests, startup isolation, dimensions and capture paths are
recorded in the gallery. Initial June builds have no config persistence, no
network control server and only a console-output hardware engine. Later builds
must be checked for explicit/portable isolation and auto-connect behavior before
launch. Unrelated production settings and physical outputs are never a fixture.

Disconnected captures cannot demonstrate connected-only RGB, telemetry, HOT,
firmware progress or timing faults. Those changes remain explicitly documented
above rather than faked into screenshots. The original memory text really was
a hard-coded placeholder in that historical application, not a measurement.
