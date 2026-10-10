# Native dropdown navigation and color rows

## Causes and repair

The native ComboBox did not seed a choice focus when opened, and ordinary egui
directional focus was not a deterministic dropdown selection policy. The accent
menu also laid out the circle, label and custom editor as separate widgets with
different heights. Its nested picker competed with the enclosing menu's close
event; the selected control omitted its color indicator entirely.

All native ComboBoxes now share explicit choice navigation, preserving egui's
exact trigger IDs and existing application commands. Up/Down wrap and skip
disabled choices, Home/End move within choices, Enter commits and Escape cancels.
Search fields keep typing/caret keys until an arrow enters the results. Both
full and compact icon pickers use the same ownership policy; list/grid switches
stay open. Focused offscreen choices scroll into view. Multiselect checkbox
choices are registered without dismissing their popup on every toggle.

Choice rows span the popup width at a fixed 32-point height. Painting uses fixed
text/swatch lanes, inside strokes and zero hover expansion instead of changing
padding or text geometry. Accent and recording color controls retain their
selected swatches while closed.

Custom is one clickable row, including its inset HEX preview. It opens a palette
anchored outside the closed selector, with an editable HEX field, HSV picker,
shared Pealayer swatches (including Orange) and deduplicated recent colors.
Recents are bounded to 12 and session-only: picking colors adds no configuration
file writes. Partial HEX edits remain in the palette draft; invalid values do
not replace the valid preference or become invented recent colors.

## Verification

The isolated native library suite passes **753 tests**, with zero failures and
one intentionally ignored test. The parent supplies isolated
`PEALAYER_CONFIG_FILE`, validated host libmpv and the existing shared Cargo target
before startup. This includes existing group creation/search/clear, icon filter,
list/grid switching, recording color and menu interaction regressions.

New native widget tests cover exact trigger identity, initial focus, disabled
choices, arrow navigation, Home/End, Enter/Escape and editable search ownership.
Accent popup and row geometry is exercised in light/dark themes at 100%, 125%,
150% and 200% scaling: equal full-width rows, stable hover positions, unclipped
inset strokes, closed swatches, and clicks on both Custom and its HEX preview.
The opened palette paints app Orange and recent swatches. Session recents are
bounded, deduplicated and not carried into a fresh context.

These are real headless egui interaction/geometry tests, not a Cafe desktop
screenshot or human native visual acceptance. No controller/firmware/Web API
contract changes or physical-output actions are part of this pass. Release gates
in the current acceptance index remain open.

## Delivery

The destination was audited before packaging: clean main, paused NLE, original
cues, controller connected, no hardware error/EStop and the destination's own
validated libmpv. Package and peer-updater receipts are appended after delivery.
