# Native FPS sizing and clipping

The FPS item reserved a fixed 88-point slot, including a 19-point icon gutter,
regardless of the selected font's measured advance. Optical digit-height matching
could enlarge the monospace face, leaving three-digit captions wider than the
remaining number clip. Earlier tests checked slot stability and vertical centering,
not whether the entire number actually fitted its horizontal clip.

The native counter now keeps the user's selected family and regular weight but
uses a secondary text size, at most 11 points, below the surrounding 12-point
status captions. Tooltip typography and other status widget fonts are unchanged.

FPS allocation measures the actual font, its widest digit and glyph overhangs.
It reserves a stable three-digit caption even while showing a shorter value;
larger factual values can grow the slot rather than disappear under its clip.
Painting and both status groups' width budgets use that same calculation.
The existing 88-point minimum remains a floor, not a clipping ceiling.

## Focused verification

All 29 native status-bar tests pass. The added real-paint regression checks
9.99, 99.99, 100.00, 144.12, 999.99, 1000.00 and 9999.99 against their actual
shape clip, with system/bundled faces, proportional/monospace choices, both text
directions and 1/1.25/1.5/2 display scales. It also checks the smaller font size,
visible vertical centering and stationary neighboring slots through three digits.
Existing source switching, regular weight, tooltip stability, menus, drag ordering,
Workspace placement and hardware/status tests remain passing.

Packaging, local runtime capture and canonical installation are the next gates.
Cafe deployment remains explicitly user-deferred/offline. This focused fix does
not claim the broader status-history or production/hardware acceptance complete.
