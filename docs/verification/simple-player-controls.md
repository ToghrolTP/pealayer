# Simple playback controls and source location

Simple playback has its own menu, command-toolbar and status-bar visibility.
All three default to hidden; NLE visibility and customization are unchanged.
Preferences → Playback → Simple workspace and the video's context menu expose
these choices, so hidden chrome can always be restored.

The floating transport uses the video canvas bounds, not the whole viewport.
Its default bottom placement clears an enabled status bar. Drag its grip or
unused background to move it; normalized travel coordinates keep the position
inside resized windows. Reset position restores bottom-center. Placement saves
through the existing narrow, coalesced owner/CAS path, without loading a dock
snapshot, seeking, resetting audio or sending an entire workspace.

Visibility uses the shared motion policy: hold while active, then ease over
250 ms; animate only during that transition, sleep until the next deadline,
and honor reduced motion. Opacity is applied once to controls, not again through
their style. Open menus and elapsed-time editing retain visible controls.

## Backdrop

Automatic GPU blur uses nine Gaussian-weighted samples of the already-published
video texture through the existing egui GPU shader. It does not read a framebuffer,
copy video pixels to the CPU, allocate GL objects, decode another video, or change
libmpv's rendering order/state. Rounded geometry and UVs are bounded to the video
and control surface. Letterboxing/no media/scrub previews retain a solid fallback.

Software or unavailable renderers use the fallback. Automatic mode also latches
off after three consecutive slow video renders (>20 ms), rather than flickering
quality every frame. Explicit GPU blur bypasses that timing heuristic, not the
hardware/valid-texture requirements. Solid mode is always available. This is a
small local video-backdrop blur, not desktop/window-compositor blur; source/mesh
checks do not prove smoothness on every GPU.

## Media Inspector

The header now wraps its actions and includes Copy location, or Copy link for a
remote source. Local paths become absolute without filesystem/network probing;
URLs retain their original escaping/query bytes, not a cache/proxy path or
diagnostic-redacted value. This explicit clipboard operation never puts the
source in a toast/log/general status feed. The action is disabled without media.
It is local to the viewing frontend, not forwarded to another peer's clipboard.

HTTP/JSON/IPC commands use the existing dispatcher:

```json
{"command":"copy_media_location"}
```

JSON-RPC:

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.media.copy_location","params":{}}
```

The normal configuration API accepts and persists:

```json
{"simple_workspace":{"show_menu_bar":false,"show_toolbar":false,"show_status_bar":false,"controls_x":0.5,"controls_y":1.0,"backdrop":"auto"}}
```

`controls_x`/`controls_y` are bounded to 0–1. `backdrop` is `auto`, `blur`, or
`solid`. Shared native/Web Preferences use the same schema and validation.

## Verification gates

58 focused tests pass: actual pointer dragging without media changes, canvas and
status reservation, normalized placement, intermediate/instant motion, valid GPU
meshes, workspace-specific visibility, source byte identity, existing controls,
customizable transport, workspace toolbar and workspace-tool contracts. The
workspace-selector pointer test explicitly enables Simple's optional status bar
before switching into that workspace and back. Its previous implicit assumption
that the bar was always visible failed in hosted Linux/Windows tests; the
corrected test passes locally without changing the requested hidden default.

The clean native release and standalone downloader compiled from
`dac58859f9df058a3da0c4f314872b8c903b6a6f`. Packaging then stopped with Windows
access denied while replacing the shared staging executable. That is not a
completed package or deployment. Only this pass's generated Web output was
restored/removed afterward; authored source and running applications were kept.

Packaging, installed runtime identity, current-instance preference readback and
actual native/GPU captures remain separate acceptance gates. Simple defaults have
not yet been applied to the older regular runtime: it lacks this configuration
contract. Terminal Review is preserved separately and its final package identity
was verified over HTTP after the user launched it, but the runner cannot access
its native pipe and its occluded window does not support PrintWindow capture.
Other reviews/historical instances must not be overwritten. Cafe remains
explicitly user-deferred/offline.
