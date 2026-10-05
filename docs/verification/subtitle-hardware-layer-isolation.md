# Subtitle settings / Hardware Monitor isolation

## Cause and change

Hardware Monitor's compact and expanded cards had independent `Order::Middle` paint layers. Subtitle settings also used the default Middle order. The workspace card layers could therefore paint above the floating dialog despite the dialog's opaque frame.

Before changing production code, `hardware_cards_paint_below_subtitle_settings_and_keep_panel_clip` reproduced the bug: `hardware card paints over Subtitle settings; dark=false, compact=false`.

Both card renderers now share `hardware_card_layer`: its order comes from the parent panel, its ID is parent-scoped, and egui registers it as a sublayer of that parent. Individual layers remain available for the existing drag translation, but no longer act as independent workspace windows. Subtitle settings explicitly uses Foreground, retaining its existing opaque theme-matched frame, bounded geometry and scrolling body.

## Regression checks

- `cargo test --lib --locked --jobs 1 hardware_ -- --test-threads=1`: 34 passed, including monitor scrolling, card width, drag/drop, manager interactions and light-theme surfaces.
- The strengthened overlap test passes all four light/dark × compact/expanded combinations: hardware captions paint below subtitle contents, hardware clip stays within the panel, subtitle clip stays within its window, and the overlapping dialog owns pointer hit-testing in the Foreground order.
- Existing `floating_dialog_frame_discards_theme_transparency` passes, protecting the opaque dialog background even when the application's theme uses alpha.
- `git diff --check` passed. No full test suite or hardware output activation; no changes to subtitle values or hardware commands.
- PR #46 remains unmerged. This is a native egui-layer repair; the Web UI is unchanged.
