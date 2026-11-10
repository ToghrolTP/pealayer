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

## Deployment checkpoint

- Code commit: `0223a8b248c26874ed5539c8c4d8a472080750d4`, pushed to `fix/hardware-monitor-layout` for PR #46; PR remains unmerged.
- Built locally with `scripts/package-windows.ps1 -SkipTests -NoUpx`; optimized Rust and Web/PWA packaging succeeded. The full test suite was not run; the six focused checks above passed after the final changes.
- Graceful shutdown through `POST /api/ipc` with `{"command":"quit"}` completed before replacement. Relaunched the canonical `C:\Users\David\AppData\Local\Programs\Pealayer\bin\pealayer.exe` (observed PID 23028).
- Live `/healthz` returned `status: ok`; `/api/update/manifest` reported the exact code commit above and `git_dirty: false`. Executable SHA-256: `c2c5b3ce1175eb972db6c256f73b0f403f14533993fe882698e0127fbba76e4a` (35,682,304 bytes). `/api/player/status` reported `hardware_connected: true`; no hardware output was activated for this verification.
- Native screenshot acceptance remains unverified: both the first capture and one fresh-window recovery retry failed with `IGraphicsCaptureItemInterop.CreateForMonitor failed: The service did not respond to the start or control request in a timely fashion. (0x8007041D)`. The freshly inventoried canonical window was titled `Pealayer — Hardware connected`; no screenshot or manual gesture success is claimed.
- Cafe-PC `/healthz` timed out on this pass; no Cafe deployment is claimed. A reachable peer update and real native-window acceptance remain pending.
- This documentation checkpoint follows the built code commit; the running binary correctly identifies the code commit, not the subsequent documentation-only commit.
