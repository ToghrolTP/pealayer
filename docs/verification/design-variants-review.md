# Native presentation review

Owner: [PR #248](https://github.com/ToghrolTP/pealayer/pull/248).
The PR remains unmerged until the user explicitly requests its merge.

## Preservation and choices

The existing compact Program Monitor and standard timeline remain available.
FilmCraft is an independently selectable presentation for each, alongside the
existing egui Dock / CraftUI workspace renderer choice. Sidebar shortcuts,
saved workspace state, action customization, input behavior and accessibility
remain application features rather than presentation-dependent removals.

## Full review and preservation

The original PR head `267522b30f304300876bea5817b4d47af053a934`
changed 17 files. Its eight commits remain ancestors of the revised branch;
the current main stack was merged into that branch without rebasing or squashing
the author's work. The existing sidebar, SVG loader dependency, workspace
configuration, tab ownership/restoration, menus and regression tests are retained.

| Original shortcoming | Revised behavior |
| --- | --- |
| Sidebar and saved controls removed | Existing configurable rail/drawer and all current shortcut behavior retained |
| Monitor/ruler replacement forced globally | Independent persisted `native_monitor_style` and `native_timeline_style` controls in Appearance → Workspace panels |
| Playback-resolution selector always reports 1/2 and does nothing | Nonfunctional selector omitted; its original source remains in PR history |
| Fit menu overwrites decoder-owned source aspect | Omitted unsafe control; natural source fitting remains owned by the renderer |
| Pointer-only volume slider | Existing accessible native slider and movable volume group retained |
| Hover and click seeking use different coordinate ranges | Shared actual slider, preview, markers and seek mapping in both layouts |
| Chapter colors/visibility and active highlighting replaced | Existing configured chapter/keyframe behavior retained |
| Fixed row sizes and overflowing paint | Measured tier slots, bounded ruler/seek paint, shared density and RTL geometry |
| Demux cache labeled as rendered frames | Existing factual buffering display retained; no invented render-status stripe |
| Native-services test annotation removed | Test restored and included in the native suite |

The default remains **Compact** monitor controls and the **Standard** timeline
ruler. **FilmCraft** can be selected independently for either surface, in either
CraftUI or egui Dock. Switching does not replace the workspace, discard saved
action order/visibility, seek media or alter the hardware timeline. The shared
Preferences API, configuration and live appearance preview expose both choices.
FilmCraft retains the author's frame ladder, pentagonal timeline playhead and
three-tier monitor structure while using existing production control semantics.

The removed inactive stubs are not claimed as implemented resolution/aspect
features. The retained zoom-thumb and marker helpers remain available in the
source; the native seekbar keeps its established interaction and accessibility.

## Reproducible native captures

Use `scripts/review-design-variants.ps1` with the exact executable under review.
It creates a fresh private offline profile, uses Pealayer's HTTP APIs for
configuration and placement, and calls the maintained read-only Win32 capture
helper. It does not copy production settings or send desktop mouse/keyboard
input. Normal shutdown checks the fixture's identity and draft state first.

Example, from the repository, with output under the private verification root:

```powershell
.\scripts\review-design-variants.ps1 -Executable $reviewExecutable -OutputDirectory $captureDirectory -Label before -Baseline -WorkspaceUi egui_dock -SyntheticMedia
.\scripts\review-design-variants.ps1 -Executable $reviewExecutable -OutputDirectory $captureDirectory -Label after-compact -WorkspaceUi egui_dock -MonitorStyle compact -TimelineStyle standard -SyntheticMedia
.\scripts\review-design-variants.ps1 -Executable $reviewExecutable -OutputDirectory $captureDirectory -Label after-filmcraft -WorkspaceUi egui_dock -MonitorStyle film_craft -TimelineStyle film_craft -SyntheticMedia
```

Repeat for `-WorkspaceUi craft_ui`. Use the same palette, density, language,
window dimensions and generated media across both sides. Defaults are Studio,
dark, accent `#6362c7`, Compact density, English and 720/1200/1800 by 800 logical
points. Capture manifests record source/executable/image hashes, observed DPI
and dimensions. Baseline mode never sends new presentation keys to old builds.

PrintWindow can return blank or stale content on some GPU drivers. Inspect
every image, check the visible application identity, and reject failed captures.
Use a user-visible, unobscured window and the maintained foreground capture
helper if necessary. Never replace actual screenshots with rendered mockups.

Only inspected public fixture screenshots and their manifests belong in
`docs/screenshots/`. Keep fixture configuration, generated media and logs under
the private program verification directory. Post pinned raw image links in the
existing PR so comparisons remain visible without downloading an artifact.

## Before / after gallery

The retained baseline `85993976d334eb5c858af3ec4c6d54c797c81f66` has four
inspected native captures: egui Dock and CraftUI at 1200 and 720 by 800 logical
points, 150% display scale. Each shows the same paused synthetic test pattern,
Studio dark palette and compact controls without a toast overlay. Both isolated
processes exited through the process-local API after clean authoring inspection.
Executable SHA-256:
`59a6e37c8390d4d77011f413d7eb5f25b0b082247062c9f807ffeeb969acda98`.
The images and manifests remain private pending the final comparison gallery.

Final reviewed-build captures and publication remain pending. Do not relabel
historical screenshots as a capture of this PR.

| Comparison | egui Dock | CraftUI |
| --- | --- | --- |
| Existing presentation before | Pending | Pending |
| Existing presentation after integration | Pending | Pending |
| Optional FilmCraft monitor and timeline | Pending | Pending |
| Narrow transport and ruler details | Pending | Pending |

## Acceptance ledger

- [ ] Preserve sidebar implementation, configuration and saved workspace behavior.
- [ ] Verify both presentation selections persist independently of workspace renderer.
- [ ] Compare matched native captures and enlarged monitor/ruler details.
- [ ] Check density, font growth, RTL, palette and narrow-width rendering.
- [ ] Exercise seeking, thumbnail time agreement, markers and exact paused position.
- [ ] Exercise keyboard/accessibility volume, action ordering, visibility and overflow.
- [ ] Reconcile removed or weakened tests and run meaningful regression coverage.
- [ ] Record source/package/runtime identity and deployment status separately.
- [ ] Obtain the user's explicit approval before merging PR #248.

Cafe deployment remains a separate acceptance gate while its connection is
user-deferred. No fixture capture establishes physical hardware acceptance.
