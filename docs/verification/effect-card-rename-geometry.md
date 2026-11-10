# Effects Library rename geometry

## Cause and correction

Display cards had a 20 × 24 icon button, while rename used a 32-point combobox
with its own arrow/padding and minimum height. The name started farther right,
the row could grow taller, and a different width budget moved the input and
metadata. Narrow titles could also fall below TextEdit's minimum and widen
their card when editing.

Both modes now use matching grip/icon slots, a shared name-width/action-gutter
budget and the same font, line-height, text alignment and four-point inset.
The edit trigger opens the existing searchable icon popup from a fixed slot;
it does not introduce a second picker implementation or a wider combobox.
Save/Cancel replace the action contents without shifting the name slot. Both
modes reserve the editor's minimum width, shrinking the action gutter on
narrow cards rather than widening the card. Caption glyph alignment is
resolved explicitly from application/logical text, not button layout order.
Matching aligned galleys avoid the fractional-pixel RTL shift as well.

Draft icon/name preview, Save, Cancel, Enter/Escape, stable effect references
and catalog-revision validation remain on their existing paths. No effects,
board outputs or persistent authored metadata are changed by rendering tests.

## Verification boundaries

Native renderer tests toggle display/rename repeatedly at narrow, medium and
wide widths, in light/dark and LTR/RTL modes. They compare full card bounds,
name rectangles, metadata positions and actual glyph baselines, not merely
the galley origin (RTL rows can have negative internal offsets). A pointer
test opens the fixed-slot icon's shared popup and verifies the trigger does
not resize. Full local native library suite: 817 passed, zero failed, one
pre-existing ignored test. Shared inside-outline/font/scaling and icon-search
tests remain in the suite. Package, exact-head CI and host runtime receipts
are recorded separately; renderer fixtures are not physical-board acceptance.

Cafe source alignment is separate from its GUI/runtime rollout, which remains
blocked by the existing launch restriction. No alternate Cafe launch or
host-incompatible libmpv replacement is authorized by this UI fix.
