# Hardware caption alignment regression

## Exact cause

Commit `277647be49ae2c87b1107aad73360a3512e41dd6` (designer presentation
consistency, merged in PR #147) changed the shared clickable caption painter
from a left-edge anchor to an RTL decision based on
`ui.layout().main_dir == Direction::RightToLeft`.

Normal Hardware Monitor cards deliberately use `Layout::right_to_left` in the
header so Rename and contextual Stop occupy the trailing edge. This is widget
allocation order, not the application's language direction. Feeding it to
`input_alignment` made even English Seat Left/Right resolve to `Align::Max`.
The title then painted at `rect.right() - 4 - text_width`, leaving all remaining
space to its left. Compact headers use a different allocation direction, so
the regression depended on the card mode. Inline editors already used
`app.rtl` and logical names, explaining their inconsistent alignment.

## Correction and verification

The shared painter is now `aligned_click_label` and requires an explicit text
alignment. Both hardware-card modes resolve it from `app.rtl` and the original
logical channel name, matching the editor. Trailing actions cannot change it.
The Effects Library heading also passes application/text alignment explicitly.
The four-pixel edit/display inset and vertical text centering are retained.
Persian/Arabic and actual RTL-UI alignment are preserved rather than disabling
RTL support to fix English cards. Board names, keys, actions and capabilities
are unchanged.

Native rendering regressions inspect painted caption positions in both local
layout directions with English, RTL UI, Persian and mixed text. Real Seat
Left/Right card rendering is checked in normal/compact mode at narrow/wide
widths. Existing Stop-removal/caption hit-target tests remain in the full suite.
The 816 active native library tests pass; one existing test is ignored.

Windows Graphics Capture returned `0x8007041D` twice. The maintained read-only
PrintWindow utility successfully captured the current native window, but its
Effect Controls tab initially hid the affected cards. That capture is not
represented as visual acceptance of Hardware Monitor. Private before/after
captures and installed-build verification belong in the delivery receipt;
no screenshots, private settings or user media are committed here.
