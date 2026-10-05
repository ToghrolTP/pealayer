# Shared icon and color picker refinement

## Changes

- Grid results use bordered 54 px tiles, with an 18 px Phosphor icon above an 11 px caption. Two-line layout replaces single-line truncation; search matching remains highlighted.
- Grid columns are measured inside the scrolling viewport after its gutter is reserved.
- Search-row width accounts for its leading icon as well as the view buttons and gaps.
- Combobox popups follow their requested field width (subject to the screen bound); compact icon-button popups retain their 280–320 px bound.
- Channel indicator/direction colors and custom accent colors share one RGB field. A small round swatch sits inside the leading field margin and opens the real color palette. The remaining area edits HEX directly.
- The accent dropdown stays open during editing; selecting a preset still closes it. No global theme/button/slider sizing was changed.

## Verification

13 focused tests pass: 11 shared-icon tests and 2 color-field tests. They include light/dark two-line rendering and tile borders, popup width rules, search/filter/select/reopen, remembered grid mode, compact-popup stability, swatch geometry, actual HEX editing, and clicking the swatch to open its palette.

Windows live screenshot capture was attempted twice with fresh window selection and failed with `IGraphicsCaptureItemInterop.CreateForMonitor` / `0x8007041D`. Automated egui rendering/input evidence is not live desktop screenshot evidence. Visual acceptance remains outstanding.

Source remains on draft PR #46; no merge is authorized by this change.
