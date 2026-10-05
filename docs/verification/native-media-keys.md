# Configurable native keyboard media controls

## Use

In native or Web **Preferences → Input → Keyboard shortcuts**, use **Respond to keyboard media controls**. It defaults to enabled, including for existing config files without the new field. Changes apply immediately; Save confirms the usual preference preview and persistence workflow. This policy is independent of ordinary in-app shortcuts and global hardware hotkeys.

The authoritative JSON setting is `media_keys_enabled`. The existing config API, IPC/RPC config update, external config watcher and native/Web preference contract all use the same field. For example:

```powershell
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8080/api/config -ContentType application/json -Body '{"media_keys_enabled":true}'
```

Native sessions use the existing [Souvlaki OS integration](https://github.com/Sinono3/souvlaki): Windows System Media Transport Controls, macOS Now Playing/remote commands, and Linux/BSD MPRIS. Windows needs a registered app HWND; macOS needs the app event loop; Linux needs the desktop session bus and its media-key routing. The OS chooses which media session receives keys: Pealayer does not globally hijack keys from another selected player or register duplicate media hotkeys.

| Native command | Shared player action |
| --- | --- |
| Play / Pause / Play-Pause | Play / Pause / TogglePause |
| Stop | Stop (close the current media, matching CLI/API) |
| Next / Previous | Next / Previous playlist item, matching CLI/API |
| Fast forward / Rewind | Relative seek by the configured skip interval |
| Explicit relative / absolute seek | Preserve the OS-supplied time |
| MPRIS volume | Set the player volume and acknowledge it to MPRIS |
| Open URI / Raise / Quit | Open / Activate / graceful Quit |

Windows/macOS system-wide volume/mute keys remain OS-owned. These are not captured as player-local hardware shortcuts.

## Repairs and preservation

- Replaced unconditional OS listener creation with an enabled-by-default configurable lifecycle; disabling detaches the native registration, and enabling registers again without restarting.
- Dedicated OS event queue feeds the existing unified InteropCommand application path, not another playback engine. Disabled policy also discards queued OS events, including after a config update in the same frame.
- Per-registration lifetime guard prevents retained native callbacks from becoming active again after disable/re-enable; no second global-key listener was introduced.
- Corrected old Stop → Pause and Next/Previous → hardcoded ten-second seek mappings. Playlist navigation is now standard and consistent with existing CLI/API actions; fast-forward/rewind still provide timed seeking and use the user's preference.
- Completed previously ignored native seek, URI, volume, raise and quit events. Volume values are validated and normalized.
- Guarded missing Windows HWND to avoid a library panic. Re-enabling republishes the loaded media title and state.
- Live or still-undetermined-duration media no longer advertises Stopped merely because duration is zero. Finite duration is published to the OS timeline. Idle identical playback state is not resent each render pass.

## Verification and deployment

- Seven focused checks passed: five native media-control module tests plus shared preference-policy and persistent/API-config tests. These cover command mapping, exact seek values, volume validation, stale-callback gating, missing HWND, live/paused/stopped state, enabled defaults, JSON roundtrip and rejected invalid config type. No full Rust test suite was run.
- Native Windows release packaging with `scripts/package-windows.ps1 -SkipTests -NoUpx` passed, including the existing Web TypeScript/production/PWA checks.
- Live `/api/preferences` advertised the new Boolean control and both current/default values as true. Config API changed it to false; a later GET confirmed false while ordinary shortcuts and global hardware hotkeys remained true. It was restored to true, confirmed live and in `%APPDATA%\pealayer\config.json`.
- Graceful `/api/ipc` Quit preceded replacement; the process list was empty at that check. Canonical executable relaunched at `%LOCALAPPDATA%\Programs\Pealayer\bin\pealayer.exe`, PID 41112, health `ok`.
- Running source commit `1f1170f17e0579c3870288401483320284aa526b`, `git_dirty: false`, SHA256 `11d5441a4a8b77994e8741bf2deac18f7df7728e578a0c7dc71c54e160133eab`, 36,392,960 bytes.
- Windows physical media-key/session gesture verification is not claimed: a read-only PowerShell WinRT session query could not load the projected runtime type in the available shell. Mapping/lifecycle tests and live config checks are distinct from physical keyboard acceptance. macOS/Linux hosts were not available for runtime verification; their existing native adapters and shared handling are wired, but need host acceptance testing.
- Cafe-PC updater endpoint timed out after five seconds; no remote deployment or manual SSH replacement is claimed.
- Source and this checkpoint are preserved on PR #46. The PR is not merged.
