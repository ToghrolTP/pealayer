# FilmCraft preservation audit and restoration

Owner: [PR #248](https://github.com/ToghrolTP/pealayer/pull/248).
**Draft and unmerged: explicit user approval is still required.**

## What went wrong

The integration merge `dbfc34a64d15040c0eb1252e1ce725f2328cae70`
retained the original commit ancestry but replaced the custom Program Monitor
implementation with shared control groups. Its subsequent review treated source
remaining in history or unused helpers as preservation. That was insufficient:
the original mini-timeline/playhead/marker presentation and volume art no longer
ran, the Simple seekbar/timecode additions were omitted, and the Fit/resolution
selectors were dropped instead of repaired.

The reasons recorded then were real problems—pointer/keyboard accessibility,
inconsistent seek coordinates, metadata mutation and an inert resolution menu—
but reducing the feature was not the required remedy. This restoration keeps the
design and repairs its integration. A small final PR diff is also partly due to
merging main: unrelated main changes disappear from the comparison. That does
not explain away the real removals above.

The eight original author commits remain ancestors. No history was rewritten,
no source was force-reset and the user's existing workspace/runtime is untouched.

## Original additions: disposition and live ownership

| Original addition | Restoration / retained behavior |
| --- | --- |
| `dcd40a7`: remove rail/drawer | Existing configurable rail remains available; users can hide it. Do not erase another requested workspace feature to simplify FilmCraft. |
| `88dcbc2`: token-aware CraftUI IconButton helpers/call sites | Helpers and the existing dialog/color/RF/status call sites retained. Restored monitor settings uses the shared token-aware helper too. |
| `657a313`: frame ruler and pentagonal playhead | Retained as optional FilmCraft timeline style, using actual fractional media FPS, bounded paint and unchanged timeline interaction. |
| Same commit: zoom-thumb algorithms | Original helper implementation and tests retained. Neither original integration nor the reduced review wired a live zoom scrollbar; this is not claimed as an already-delivered interactive feature. |
| `bcb46c8`: seek ladder and blue timecode | Restored for the Simple transport as an independent saved Compact/FilmCraft choice, and retained for FilmCraft Program Monitor. |
| `81c235d`: two-point ruler overview | Restored in FilmCraft. Its actual source is **buffered media**, not rendered frames; absent buffer data never invents progress. |
| `864384b`: mini-timeline and volume design | Live pentagonal playhead/stem, terminal rings, chapter/keyframe pentagons and ring-thumb volume restored. They use the actual accessible slider's value range; marker colors and exact chapter seeking stay configured. |
| `d941b97`: three-tier monitor and selectors | Retained measured clock/seek/transport tiers, with live Fit/aspect and Full/half/quarter/eighth preview selectors plus the settings menu. Narrow reflow, RTL, volume movement, overflow and Fullscreen ownership remain. |
| `267522b`: merge main | Preserved normally in ancestry; no wholesale file checkout or reversion of unrelated fixes. |

The resolution selector now changes the actual private preview FBO dimensions.
It does not claim decoder downscaling or change external mpv's output. Aspect
selection is separate from decoder-owned source aspect; Fit restores that source.
Changing either does not seek, resume playback or rewrite media metadata.

## Configuration and safety

- Existing Compact monitor, Compact Simple transport and Standard ruler remain
  defaults. FilmCraft options work with egui Dock and CraftUI.
- Shared Preferences/API settings: `native_monitor_style`,
  `native_transport_style`, `native_timeline_style`,
  `monitor_preview.aspect` (`source/wide/classic/cinema`) and
  `monitor_preview.resolution` (`full/half/quarter/eighth`).
- All use the normal saved configuration and live preview/apply path. Presentation
  choices do not replace workspace state, hide user actions or change channels.
- Cosmetic slider paint wraps real egui slider input/focus/accessibility.
  It does not install a second pointer mapping. Simple's existing controls and
  default presentation remain available; volume stays with its saved Mute group.
- The original multicolored chapter styling is reconciled with user-configured
  marker colors rather than hardcoding a replacement palette.
- Original inert selectors were completed, not reinstated as misleading stubs.
  The zoom-helper-only addition is explicitly distinguished from live UI.

## Verification

The restoration adds persistent-choice/API validation, actual preview render-size
checks, pointer selection of both real dropdown menus without source-aspect or
playhead mutation, live pentagon/ring paint checks, and volume drag/reorder tests
in both presentations and both directions. Existing seek/keyboard, marker,
hover-thumbnail, responsive footer, workspace/sidebar and palette regressions remain.

Final source-test results, clean package identity and runtime/capture receipts are
recorded in the PR after verification. A parallel repeat exposed an existing global
asset-review snapshot fixture race; the full serial run is recorded separately,
not silently presented as a clean parallel pass.

## Updated native screenshots

The superseded gallery is removed from the current tree at the user's request;
Git history retains it. Replacement captures use the maintained
`scripts/review-design-variants.ps1` fixture with synthetic media and three named
chapters, matching Studio dark palette, Compact density and viewport/DPI.

The fixture explicitly pauses and seeks to **5.25 seconds**, waits for settled
seek readback and records playback state in every capture manifest. Zero-position
or unsettled captures are not accepted. Compact/Standard baseline and restored
FilmCraft are captured from the same review executable in both workspace renderers;
“before” means the preserved presentation, not a fabricated historical build.

Updated image embeds and manifests are published only after inspecting actual
native captures. This is an isolated PR review build, not a replacement of the
regular application or another review session. Production/Cafe deployment,
physical hardware, GPU soak and the user's merge decision remain separate gates.
