# Surface-scoped mouse controls

Owner: [PR #291](https://github.com/ToghrolTP/pealayer/pull/291), continuing
[#80](https://github.com/ToghrolTP/pealayer/issues/80).

## Behavior and configuration

Preferences → Input contains separate Back/Forward groups for Video, Timeline,
Effect Designer and Remote Browser, plus Video modifiers. They use the same
validated `mouse_bindings` configuration on native and Web surfaces; previews
apply immediately through the existing Preferences reconciler.

| Surface | Back / Forward | Ctrl or Command | Alt | Shift |
| --- | --- | --- | --- | --- |
| Video | Seek backward / forward | Previous / next chapter | Previous / next frame | Previous / next file |
| Timeline | Pan left / right | Zoom out / in | Inherit | Inherit |
| Effect Designer | Pan left / right | Zoom out / in | Inherit | Inherit |
| Remote Browser | Previous / next verified folder | Back: parent folder | Inherit | Inherit |

Each action can be disabled or changed independently. Modified actions may
inherit the unmodified action. Combined modifiers resolve Ctrl/Command, then
Alt, then Shift; selection is pinned at press, not recalculated at release.
Only actions supported by that surface are accepted; there are no hardware
actuation bindings in this feature.

- Middle-click on video defaults to Fullscreen. The existing click selector also
  offers Boss key (pause, then minimize), playback, mute, menu and no action.
  Explicit saved user choices remain unchanged; defaults do not overwrite them.
- Right-click opens the menu on release without movement. Right-drag defaults
  to seek; movement cancels its click action even when that drag is disabled.
  Shift+right always requests the menu and suppresses the configured hold/drag.
- Ctrl+left defaults to native window dragging; Alt+middle defaults to seeking.
  Both are configurable separately. Window dragging is a native-only operation:
  a Web page cannot drag its browser window. Select Seek, temporary fast-forward
  or None for a browser-applicable drag instead. No remote OS-window movement is
  dispatched as an implicit browser gesture.
- Seeking reuses the ordered exact scrub/finish path, at 12 logical pixels per
  second, bounded to seekable finite media. It does not issue competing relative
  seeks. Fast-forward hold waits 180 ms unless deliberate movement starts it.
- A gesture belongs to the surface and media where it started. Six logical
  pixels cancel a click, including travel back to the origin. Escape, focus loss,
  capture loss and media changes revoke it; a hidden native video panel still
  restores its temporary hold. Old gestures cannot seek/pause a new movie.

Remote folder history belongs to the host, retains at most 64 successful
directory visits and records their proxy route. Back/Forward re-fetch the target
before changing the history cursor. Failed or stale requests preserve the last
good listing and cursor; a new successful branch removes forward history.
Background prefetch does not append visits. Back without history can use only
the listing's verified parent, never a guessed URL. Visible native/Web navigation
buttons and mouse actions use the same owner command:

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.remote.history","params":{"direction":-1,"parent":false}}
```

The ordinary command API uses `{"command":"remote_history","direction":1}`.
Only -1/+1 are accepted; parent-only requires -1. Connected clients route this
through their existing authority API, not a browser-local folder stack.

Web uses the standard [mouse button indices](https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button)
and suppresses both pointer-down defaults and
[auxiliary-click defaults](https://developer.mozilla.org/en-US/docs/Web/API/Element/auxclick_event)
inside its owned surfaces. Browser/driver-reserved shortcuts can still precede
page dispatch; physical-device/browser acceptance remains a separate gate.

## Verification and delivery limits

The isolated serial native suite passed **1,374 tests, 0 failed, 6 opt-in ignored**.
Real egui input tests exercise both extra buttons, separate-surface ownership,
modifier/settings pinning, click/drag/menu separation, cancellation, default
fullscreen, hold timing, hidden-panel cleanup and media-change revocation.
Pure tests cover scope validation, shared Preferences, history success/failure,
branching, stale requests and its bound. A subsequent RPC parity test adds
round-trip and invalid-direction/type coverage.

TypeScript, the mouse release/policy matrix, 53 Web parity contracts, timeline
wheel routing, media controls and control-bar checks pass. These are source and
synthetic input checks, not actual physical mouse or installed-build screenshots.

A targeted rebuild exposed a cached core-only PortableShell archive lacking the
Pealayer host adapter. The combined core/adapter archive now has the distinct
`pealayer_portable_shell_host` identity; comparison builds must not overwrite it
with a core-only artifact. Its symbol table now includes the host/editor entry
points. A first recheck ran out of compiler memory; the subsequent single-job
build and complete serial suite passed. No shared-cache cleanup or process
termination is used.

The previous packaging helper rejected concurrent authored changes safely. It was
disabled while this batch was edited and is pinned to its committed checkpoint
before use. Keep its source-change guard: bypassing it could package
mixed work. Existing generated Web outputs are not authored source and remain
excluded from this commit. Package completion is separately blocked by the
review-held scratch executable and a tool-policy denial of generated-output
cleanup; the bounded user-run helper is the authorized completion path.

Remaining: current package build, graceful installed update, physical buttons
4/5 and browser-default checks, screenshot/human review, and the explicitly
user-deferred Cafe deployment. No running application or hardware output has
been changed by these tests.
