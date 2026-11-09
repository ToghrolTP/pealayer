# External player switching and recovery

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## User-facing behavior

- Program Monitor's panel menu offers **Launch external player** and contextual
  **Player mode** choices in both CraftUI and egui Dock. Launch from Internal
  keeps the internal preview; External-only can be explicitly relaunched.
- Internal, External-only and Both can be switched at runtime. Existing-player
  attachment remains available as **Control existing mpv** in Preferences.
- Closing a managed external player does not become a request to attach to its
  dead endpoint. Both hands playback back to the internal player; External-only
  stays stopped without loading an internal decoder. Neither respawns until an
  explicit launch/mode selection. An attached player's explicit Quit event also
  permits fallback; a temporary IPC disconnect retains reconnect behavior.
- A paused internal preview retains its published frame on handback. Native
  decoder completion wakes the synchronizer if the external player is paused
  and no longer emitting events. This is not a polling/reseek loop.
- Preferences mode entries have icons and separate descriptions. Executable
  Browse/reset labels describe a player, not an image. On leaving the field,
  surrounding whitespace/quotes are normalized and the full resolved path or a
  useful validation error is shown. Resolution does not run the executable.
- Native and Web use the same four-mode contract and executable resolver. A
  remote consumer resolves on its authority, not against its own filesystem.

## Shared control path

HTTP `/api/rpc`, WebSocket and native JSON-RPC accept:

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.player.mode","params":{"mode":"dual"}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"pealayer.player.executable.resolve","params":{"executable":"mpv"}}
```

The resolver returns `input`, `resolved` and `error`. It searches the actual
launch environment without a shell; Windows prefers the GUI `.exe` over mpv's
console wrapper. The launcher uses that same resolved path. The corresponding
typed commands are `set_player_mode` and `resolve_player_executable`.
Mode changes require control/configuration permission; read-only resolution
requires configuration permission. Remote consumers forward session operations
to their authority. Status exposes `closed` and `resolved_executable` alongside
the configured mode/connection/ownership facts. Closed Both means internal
fallback; closed External means no active decoder.

Panel-menu changes are queued until workspace painting has returned its layout,
then use the same application command as IPC. They must not serialize the
adapter's temporary borrowed/empty workspace or close unrelated panels.

## Taskbar artwork regression

The live application reported Modern with no temporary branding override, but
the shell-refresh counter was zero. First HWND property assignment previously
skipped the delayed membership refresh: a new HWND has no old relaunch property,
yet Explorer may still hold Classic artwork for the stable application ID.
First assignment now uses the same coalesced remove/add refresh as a live icon
change. Unchanged properties remain a no-op. Both native icon slots are installed
first; thumbnail-toolbar restoration, timer teardown and the stable ID remain.
Native property readback alone is not acceptance of the displayed taskbar image.

An identity-scoped follow-up found that the canonical app still retained Classic
after this first change, despite four successful isolated-app artwork switches.
Its registered Start Menu shortcut still pointed at executable resource zero;
the old refresh inspected only taskbar pins. Ordinary Start Menu launchers for
the exact executable are now included, preserving their arguments, descriptions
and unrelated targets. The bounded discovery does not follow directory symlinks.
This broader launcher fix still requires canonical displayed-artwork acceptance.

The canonical follow-up isolated the actual remaining stale registration:
Explorer's `User Pinned/ImplicitAppShortcuts` entry for this exact executable
explicitly referenced Classic. Updating the ordinary Start Menu link alone did
not change that entry. The same bounded, target-checked refresh now includes
implicit application shortcuts, not only explicit pins and ordinary launchers.
This is a persistent branding path, not a one-time shortcut repair. Other apps,
custom command arguments and the retained historical build remain excluded.

That repaired initial canonical artwork, but repeated switching still left
Explorer's image-list entry stale after the link metadata had changed. The link
update now captures its old extractor location and system-image-list index and
sends the targeted [SHUpdateImage notification](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shupdateimagew)
in addition to the item notification. It uses the old image identity as required
by Windows, not a global cache flush. Canonical repeated-switch pixel acceptance
remains the deciding gate for this follow-up.

The canonical repeated-switch check rejected that attempt too: notifying the
old immutable icon refreshed the old artwork. The final strategy uses stable,
per-application taskbar and default-launcher ICO resources. New bytes replace
their contents atomically; the targeted notification now names the same old
resource whose contents actually changed. Default launcher and playback-state
artwork retain separate roles. Identical bytes do not rewrite the file, no
partial ICO can be observed, and historical hashed cache pruning cannot remove
these resources. Diagnostics exposes an application revision rather than using
filename churn as proof of change. Canonical pixel checks remain mandatory.

The pixel fixture itself now locates the button by its exact AppUserModelID
through read-only accessibility metadata before capturing pixels. Whole-taskbar
icon matching could select the retained historical Pealayer instead, yielding a
false pass followed by a false failure. It never invokes accessibility patterns,
drives input, changes focus, restarts Explorer or resets the global icon cache.

## Verification boundaries

The focused native suite covers actual mpv JSON IPC, ownership-safe detach,
temporary disconnect/reconnect, managed Dual/External exit, attached Quit,
explicit relaunch, failed launch, paused-frame retention, rich mode metadata,
resolver validation, deferred mode commands, real pointer activation and bounded
phone/tablet/desktop choice geometry in LTR/RTL. Fixtures use finite synthetic
media, null outputs, isolated configuration and no hardware.

TypeScript and the existing control-bar/media-control/responsive source checks
cover the Web changes. Packaging, displayed-shell pixel checks, native captures
and installed-runtime receipts are recorded after verification; source tests do
not establish those outcomes. Cafe deployment remains user-deferred while its
normal connection is unavailable. The historical comparison stays untouched.
