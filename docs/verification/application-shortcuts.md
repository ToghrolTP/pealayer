# Application accelerators

| Action | Windows / Linux default | macOS default |
| --- | --- | --- |
| Toggle fullscreen | F11 | F11 |
| Reveal and focus Media Inspector | Shift+F10 | Shift+F10 |
| Open media containing folder | Ctrl+Shift+F10 | Ctrl+Shift+F10 |
| Preferences | Ctrl+, | Cmd+, |
| Edit config in OS-registered external handler | Ctrl+Shift+, | Cmd+Shift+, |

All five bindings are editable under Preferences → Input → Application shortcuts, exposed through the same preferences schema to Web UI, saved in `application_shortcuts`, and reapplied by the existing config watcher/live settings pipeline. Old config files receive defaults. Empty strings disable bindings; unsupported keys, invalid syntax and duplicate assignments are rejected. The keyboard-shortcut master switch still applies. Existing F fullscreen alias remains available.

Bindings are local accelerators for the focused main window, not OS-global registrations. Exact modifiers are required, repeated key-down/release events do not retrigger, and custom bare-letter bindings do not steal text editing. Hardware key recording suppresses application accelerators. Consumed events are removed before transport/hardware processing, preventing Preferences' comma key from also stepping frames.

The implementation dispatches the same typed commands as menus and IPC/API/CLI. New commands are advertised by the command catalog and accepted by text and JSON-RPC parsers. CLI adds `--media-info`, `--media-folder`, and `--edit-config`. The Media Inspector reuses the existing full libmpv metadata panel, restoring/focusing it rather than opening a duplicate window. Folder opening accepts local paths, relative paths and file URLs; remote streams/no media produce specific errors. Config editing reuses the existing footer Edit action.

Focused tests cover binding matching, repeat/release suppression, customization and serde round trip, validation and typing protection, OS-specific preference modifiers, actual application fullscreen/Preferences dispatch and event consumption, shared preference editing, CLI/text/JSON-RPC parity, local-only media folder resolution, and resolution of every preferences key against real configuration. Full test suite is not required by this pass.
