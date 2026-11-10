# Effects Library card header regression

## Requested behavior

The first-line Run, Place at playhead and More buttons belong at the card's
right-hand content edge, independent of caption length. The drag grip must be
visible and usable without breaking rename, icon selection, context menus or
the original pointer grab offset.

## Cause and fix

The caption used `allocate_ui_with_layout`, but did not give its child a minimum
width. egui sizes that allocation to the actual text: a short caption collapses
the slot and moves the following action container left. The drag surface was
still present, but the production card header no longer rendered a drag grip.

The production header now uses a single reusable renderer. Caption and action
slots hold their reserved widths; the caption stays left-aligned and truncates,
and the three 24 px actions remain anchored to the right. A Phosphor six-dot
grip occupies a stable 14 px slot and is muted at rest, emphasized on hover.
It shares the existing whole-card primary drag gesture rather than creating a
competing drag ID. Inline editing retains its existing icon/name/confirm/cancel
controls and disables the surrounding drag surface.

Only narrow card headers tighten their local spacing. The minimum card width is
160 px so the new grip, icon and actions do not overlap; no global button size,
slider style, app theme, effect data or PCController storage was changed.

## Focused verification

Twelve focused tests passed (not the full suite):

- Real production header: right-edge alignment for short/long captions at
  160, 180, 320 and 520 px in dark and light themes.
- Real production grip: primary drag payload exists and press offset survives
  promotion to the floating preview.
- Real production Run, Place and More buttons: press/release remains clickable
  through the surrounding drag surface, without starting a drag.
- Existing card geometry and group/card width consistency, metadata alignment,
  pointer-offset preservation, real drag/drop including scrolling competition,
  and secondary-button context-menu-only behavior.

Commands use `cargo test --lib --locked --jobs 1 <filter> -- --test-threads=1`,
with filters `effect_library_`, `effect_card_`, `effect_cards_`,
`effect_group_headers_`, and `production_effect_action_row_`.

The Windows computer-use capture was attempted twice with fresh target
selection, but failed with `IGraphicsCaptureItemInterop.CreateForMonitor /
0x8007041D`. Automated egui geometry/input coverage is not claimed as a live
desktop screenshot or visual inspection. Build/deployment evidence is recorded
on draft PR #46; do not merge it based solely on this regression fix.
