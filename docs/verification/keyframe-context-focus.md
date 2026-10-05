# Keyframe context-menu priority and deselection

## Cause

The native timeline already had exact-marker and PWM-keyframe menus, but their hit rectangles overlapped the ruler and full canvas context menus. The broad menus could own the right click instead. Keyframe selection also outlived keyboard focus: outside clicks surrendered canvas focus without clearing keyframe selection, and blank analog-lane clicks left its selected points highlighted. Entering a nearby point while the primary button was already held could start a new point drag unintentionally.

## Repair

- One nearest keyframe owns the forgiving 16-pixel-radius hit area, for both ruler markers and analog/PWM points.
- Dedicated, target-scoped popup IDs open from that hit test; ruler/canvas menus are suppressed while the keyframe owns the context. Moving the pointer into the menu does not replace its target.
- Right-click selects the target without seeking. Existing exact-time editing, jump/delete and interpolation actions are retained. Analog menus show keyframe time/value, and both types offer Deselect keyframe.
- Ordinary primary click away or Escape clears exact and analog keyframe highlights. Popup interactions and Ctrl/Command additive gestures retain selection. Text-editor focus and active point drags are not intercepted by the new blur handler.
- Analog point dragging starts only on the initial primary press, not by wandering into another point with the button already held. The actual registered timeline widget remains the keyboard focus target, preserving the prior Windows accessibility crash repair.

## Verification

`cargo check --lib --locked --jobs 1` succeeded.

`cargo test --lib --locked --jobs 1 keyframe_ -- --test-threads=1`: six focused checks, including persistence/deduplication, separate playhead/marker geometry, nearest-point priority, real secondary-button press/release opening the dedicated popup over overlapping canvas/ruler widgets for both marker types, and primary click-away/Escape clearing selection without deleting model data. Menu interaction preserves its target.

Manual acceptance: right-click on a diamond or up to 16 pixels from it; verify its dedicated menu. Move into the menu and use a contextual action. Select a keyframe, click another panel or blank lane, and verify the highlight clears. Select again and press Escape. Right-click/deselect must not move the playhead or send any hardware output.
