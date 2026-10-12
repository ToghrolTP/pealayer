# Background media-button preference

Preferences > Input > Keyboard shortcuts exposes **Allow media controls while
Pealayer is in the background** (`media_keys_background_enabled`).
It defaults to enabled, preserving existing background support, and requires
the independent **Respond to keyboard media controls** master switch.

- Enabled: OS-delivered Play/Pause/Stop/Next/Previous/seek/volume commands can
  operate while another window has focus or Pealayer is minimized.
- Disabled: transport commands require Pealayer foreground focus. On Windows,
  this includes Pealayer-owned native dialogs, not just its root HWND. On other
  native platforms, focus comes from egui's viewport state.
- In-app keyboard shortcuts and global hardware bindings remain independent.
  Tray/taskbar clicks remain explicitly targeted actions; their identical
  numeric command IDs do not accidentally inherit keyboard focus restrictions.
- OS-session Raise/Quit/Open URI remain explicit window/file requests under the
  master switch. In particular, Raise can bring a foreground-only player back.

Native media handling retains one Souvlaki SMTC/MPRIS/Now Playing registration.
Changing focus never registers a competing global hotkey or repeatedly
recreates that listener. Dispatch checks current permission/focus, including
queued commands following a same-frame configuration update. Disabling the
master still drops the session and invalidates old native callback tokens.
Windows WM_APPCOMMAND selections retain media-key provenance through the queue;
dispatch distinguishes them from tray/taskbar clicks.

The policy does not pause playback on focus loss, synthesize focused state,
capture system volume keys, or force Windows to select Pealayer over another
player. Windows delivers hardware-media actions through its
[System Media Transport Controls](https://learn.microsoft.com/en-us/windows/apps/develop/media-playback/integrate-with-systemmediatransportcontrols).
OS routing and browser/driver capability remain acceptance boundaries.

The shared Preferences/API config and live Web state carry both permissions.
Web MediaSession callbacks use current advertised flags and current document
visibility/focus, not their installation-time state. Cleanup invalidates
retained callbacks; the master switch removes Pealayer's custom handlers.
Web Stop/Previous/Next now use the same commands as native OS transport rather
than substituting Pause or chapter navigation.

Verification includes the full permission/focus matrix, config/API/round-trip,
an actual hidden/headless logic callback, same-frame disable handling, explicit
window-request and shell-provenance regressions, and Web policy/handler checks.
Fixtures use memory-only media managers and do not register native listeners,
send real media keys or actuate hardware.

Local Pealayer is closed and its updater endpoint refuses connection. Prior
local GUI/Cafe smoke denials and the generated Web cleanup restriction remain
respected. Source/CI validation and Cafe source alignment do not establish
installation or physical keyboard acceptance. After an authorized/manual
launch and host-compatible peer update, verify Play/Pause, Stop, Previous/Next,
minimize/restore, focus another application, toggle both settings, and verify
no double dispatch or interception of unrelated players/system volume keys.
