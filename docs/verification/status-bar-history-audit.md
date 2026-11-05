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

## Divider history: two different boundaries

The vertical **item dividers** and the horizontal **panel boundary** changed
independently. The original native item dividers really were full-height. The
short strokes in recent screenshots are not the original design and were not
caused by the monospace font or display scaling.

| Revision | Exact change | Evidence and reason |
| --- | --- | --- |
| [Original component](https://github.com/ToghrolTP/pealayer/commit/dc49b29) through [selectable FPS](https://github.com/ToghrolTP/pealayer/commit/c1b15be) | Stock `ui.separator()` between applicable status items. In a horizontal layout, egui extends the vertical line to the available row height; no five-point inset. | Actual original/dual-FPS captures show the full divider. Conditional items disappearing can remove their adjacent separator, but does not shorten it. |
| [Reorderable rewrite](https://github.com/ToghrolTP/pealayer/commit/a242919) | The new widget loop omitted the old separator calls. | PR #149 introduced reorderable widgets and a scrolling group; its implementation did not carry over dividers. The separators still found by text search are inside menus, not the bar. The fixed-slot follow-up also lacks item dividers. |
| [Compact two-sided rewrite](https://github.com/ToghrolTP/pealayer/commit/48fd59c) | Restores configurable, default-on dividers, but replaces stock separators with a 1-point allocated slot and endpoints inset **5 points at both ends** of a 22-point row: **12 points of visible line**. | PR #166 explicitly describes optional dividers and compact stable slots. Neither its body nor discussion explains the particular five-point inset or records a user request to shorten the old full-height dividers. Compact styling is the surrounding context, not proof that this specific change was requested. The same commit gives Web widgets full-height `border-inline-start`, creating a native/Web mismatch. |
| [CraftUI panel styling](https://github.com/ToghrolTP/pealayer/commit/9610177) | Adds `well_bg` and a 1-point frame outline on all four edges. Leaves `Panel::show_separator_line` enabled. | The pinned egui revision's `resolve_frame` reserves an extra point outside the frame; `show_inside` paints another horizontal separator there. This creates two adjacent top lines, not a font-rendering artifact. The original test checked only frame tokens and never drew the actual Panel, so it missed the duplicate. |
| [Native typography](https://github.com/ToghrolTP/pealayer/commit/1fbed60) and [glyph/width repair](https://github.com/ToghrolTP/pealayer/commit/7d9e8f9) | No change to item-divider endpoints or the double panel boundary. | Font changes alter nearby text weight and alignment, not the divider's 12-point geometry. |

The boundary repair disables egui's additional panel separator while retaining
the themed frame. Regression coverage inspects the actual painted shapes rather
than only the frame configuration. Web separators must be independent of the
widget's text box, so borders cannot consume caption width or interfere with
the drag-insertion pseudo-elements. The existing Show dividers preference and
rendered-item filtering remain authoritative.

Painted-geometry verification also exposed a panel-sizing problem: the panel
inherits a density-dependent initial height and a resizable default, but its
widgets all occupy fixed 22-point slots. Centered layout feeds progressively
smaller measured heights into subsequent frames, moving the row while it settles.
Deriving the panel's exact height from those slots plus the existing frame
margins/stroke and disabling resizing makes the first and hovered frames stable.

Follow-up requested during this comparison: keep compact dividers for now, but
allow full-height or larger dividers later. Track a shared native/Web divider
size preference (compact/full-row/custom height and, if requested, thickness)
without tying it to font choice or silently changing existing settings. This is
backlog, not a shipped setting. Any larger size must stay within the bar's own
layout/clip area; actual hit targets, text slots and drag indicators need geometry
coverage. PR #166 receives the real before/after captures and these findings.

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
