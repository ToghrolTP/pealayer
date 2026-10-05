# Numeric value menus and adjustment steps

## Delivered

Subtitle position and subtitle/audio delay share one compact stepper with a value context menu: Reset, Copy, Paste, Increase, Decrease, and Adjustment steps. The same menu is attached to subtitle font size, audio volume, and all numeric controls rendered by the native Preferences contract. Web Preferences uses a reusable numeric control with the same actions, explicit adjacent minus/plus buttons, and a compact steps dialog.

Hold Ctrl (or Command on macOS) while clicking an adjacent adjustment button for fine steps; hold Shift for coarse steps. Ctrl wins if both modifiers are held. Defaults are normal, normal/10, and normal*10; integer settings never use a step smaller than one. For example subtitle delay starts with 0.1s, 0.01s fine, and 1s coarse. Each field's menu lets the user change all three sizes or reset the sizes. Value Reset is separate and restores the actual setting default, not the minimum.

## Contract and persistence

`numeric_input_steps` is a map of stable config field keys to `{normal, fine, coarse}`. It is included in the typed JSON config, runtime snapshot, native reload/preview paths and existing `/api/config` contract. Step edits in media settings save immediately; Preferences retains its existing preview/Save/Discard workflow. Native and Web Preferences address the same field keys. `/api/preferences` now supplies actual defaults and integer metadata instead of making the web client guess reset values or types.

Paste requests use the OS clipboard event and only replace the requested numeric field. Negative and suffixed values are accepted when applicable, nonfinite/invalid values are rejected, and values are clamped to the field's range. Web clipboard access uses browser permission/security checks and reports denied/unavailable access rather than silently failing. Existing keyboard editing and sliders remain; fine values are not rounded back to the drag step on repaint. These changes cover numeric media-settings/Preferences controls, not every specialized numeric editor elsewhere in the application or free-text input.

## Checks

- `cargo check --lib --locked --jobs 1`: passed (existing warnings only).
- `cargo test --lib --locked --jobs 1 numeric_input -- --test-threads=1`: 6 passed. Includes real egui right-click/reset menu gestures, actual normal/Ctrl/Shift button press/release, bounded adjustment, paste routing/event consumption, rejection of invalid/nonfinite paste, and config serialization/validation.
- `cargo test --lib --locked --jobs 1 preferences_contract -- --test-threads=1`: 19 passed.
- `web_ui/node_modules/.bin/tsc.cmd --noEmit`: passed.
- Full test suite not run; canonical optimized packaging and live deployment evidence follow in the deployment checkpoint.

## Manual acceptance

Open Subtitle Settings; right-click the delay value and verify all six actions. Reset delay, use + normally, Ctrl+ and Shift+, expecting 0.1s, 0.01s and 1s increments. Paste a negative numeric delay. Edit step sizes, close/reopen the application, and verify they persist. Repeat with position, audio delay, font size and Web Preferences. On an integer preference, a fine adjustment must still change by at least one. No hardware output is required for these checks.
