# Ctrl + arrow frame stepping

## Fix and use

- Ctrl + Left steps backward; Ctrl + Right steps forward through mpv's decoded frames. Command + arrows are accepted too.
- Uses the existing Frames per frame-step action preference, not an approximate time seek. No media loaded means no action.
- Works with the player or focused timeline canvas. Ctrl + arrows retain word navigation in text inputs, and open popups do not trigger this transport action. The existing keyboard-shortcuts enable preference is respected.
- Timeline cue/keyframe-neighbor navigation is retained on Ctrl/Command + Shift + arrows. Bare arrows retain their existing behavior. Frame-button tooltips advertise the new shortcuts alongside bracket shortcuts.

## Cause

The app's player arrow handler did not check modifiers, so Ctrl + arrows performed a normal timed seek. A focused timeline instead intercepted the same chord to jump between cue/keyframe times. One application-level handler now consumes the frame chord before either arrow-navigation path can reuse it, including repeated key presses.

## Verification

- Seven focused application-shortcut tests passed: both arrow directions, Ctrl/Command, key repeat, consumption exactly once, unchanged non-frame chords, disabled shortcuts, key release, text-edit word navigation, and non-text canvas focus; existing application-shortcut persistence and modifier tests also passed.
- `scripts/package-windows.ps1 -SkipTests -NoUpx` succeeded. No full Rust test suite was run. The packaging Web production/PWA checks passed.
- Native screenshot/gesture verification was attempted with the computer-use skill. Capture failed twice, including a refreshed window binding, with `IGraphicsCaptureItemInterop.CreateForMonitor` / `0x8007041D` (Windows service timeout). No blind keyboard input was sent; visual/manual verification remains pending.

## Delivery

- Implementation commit: `3d403bcf1a3530dc60f824224aa8c64705df3e0a`, pushed to PR #46. No PR merged.
- Graceful `POST /api/ipc` quit completed before replacement; no Pealayer process remained at that check.
- Relaunched `%LOCALAPPDATA%\Programs\Pealayer\bin\pealayer.exe`, observed PID 35116, `/healthz` = `ok`.
- Running manifest: implementation commit above, `git_dirty: false`, SHA256 `f0f77140fe06e6bb021e46515692aea471fb506be2ffd9d47ed7db784bf1df3d`, 36,390,912 bytes.
- Cafe-PC updater endpoint timed out after five seconds. Remote delivery is not claimed, and no manual SSH copy was substituted.
