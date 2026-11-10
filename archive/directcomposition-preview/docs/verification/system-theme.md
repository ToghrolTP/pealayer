# System Light/Dark scheme

System remains the default for new installations and configurations without a
`theme` field. The stored value stays `system`; the current Light/Dark scheme is
runtime state, not a replacement for that preference. Existing explicit Light
or Dark selections and the documented `APP_THEME` override are preserved.

## Native event path

- Windows: winit 0.30.13 handles `WM_SETTINGCHANGE` and emits `ThemeChanged`.
- macOS: winit observes effective appearance changes and emits `ThemeChanged`.
- egui-winit 0.36.2 updates `RawInput.system_theme` and requests repaint.
- egui's System preference selects the corresponding installed palette.
- Main and native Preferences titlebars follow `Context::theme()`, even when
  the saved preferences have not changed. The cached DWM setter avoids calls
  on unchanged frames.
- The host publishes the resolved scheme in `appearance.resolved_theme` over
  the existing status/WebSocket path. Connected Web UI clients follow that
  scheme; offline Web UI uses `prefers-color-scheme` and its change event.

No registry writes, theme polling thread, or new dependency was introduced.
Platforms without native scheme reporting retain the framework fallback.

References: [Windows native event adapter](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/windows/event_loop.rs),
[macOS appearance observer](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/macos/window_delegate.rs),
[egui-winit theme/repaint adapter](https://github.com/emilk/egui/blob/0.36.2/crates/egui-winit/src/lib.rs).

## Corrections

Theme-independent font and static-caption interaction settings now apply to
both styles, preventing a live scheme change from resetting them. Native
Preferences no longer skips titlebar synchronization when its preference tuple
is unchanged. Startup and runtime preference application share the same
appearance setup, including the independently hosted Preferences window.
Status publication uses the context's resolved theme rather than a previously
constructed UI's style.

## Focused verification

```text
cargo test --lib --locked --jobs 1 system_theme -- --test-threads=1
cargo test --lib --locked --jobs 1 ui::tests -- --test-threads=1
cargo test --lib --locked --jobs 1 open_preferences_follows_system_events -- --test-threads=1
node --test web_ui/scripts/appearance.test.mjs
```

Results: 4, 9, 1, and 4 passing checks respectively (the groups overlap).
Tests inject the same RawInput theme updates supplied by egui-winit, verify
Light/Dark/Light transitions with both palettes, explicit overrides, return to
System, unsupported-platform fallback, stable fonts and caption interaction,
clean Preferences drafts, and connected Web UI host-state synchronization.
The full test suite was not run.

Read-only DAVID-PC inspection before deployment showed Windows application
scheme Dark (`AppsUseLightTheme=0`), preference System, and host status Dark.
Actual OS preference switching was not performed or claimed. Native screenshot
capture failed with Windows Graphics Capture service error `0x8007041D`; visual
verification of a real OS change remains a manual acceptance step.

## Manual acceptance

1. Keep Preferences > Appearance > Theme set to System, and keep Preferences
   open while viewing the main window and connected Web UI.
2. Change the operating system's application scheme to Light, then Dark. The
   app, dialogs, native titlebars, and connected Web UI should follow without
   restart; Save should not become enabled just because the OS changed.
3. Select explicit Light or Dark in Pealayer and repeat the OS change. The
   explicit selection should remain in effect.
4. Return to System. The current OS scheme should take effect immediately and
   remain stored as `system` after restarting.
