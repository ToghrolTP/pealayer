# Live application branding API

Update the serving application's name, description/tagline and artwork without
restarting. REST, JSON-RPC, WebSocket commands and native IPC share the typed
branding command and existing configuration keys.

## Temporary or saved

Send `POST /api/branding` with:

```json
{
  "values": {
    "app_name": "Cinema",
    "app_description": "Screening room",
    "app_icon_preset": "classic",
    "app_icon": null
  },
  "persist": false
}
```

`persist` defaults to `false`. Temporary values live in a separate session
overlay: changing volume or saving a workspace does not save them accidentally.
Set `persist: true` to save only the supplied branding fields to `config.json`.
Unrelated unsaved Preferences previews are not committed by that operation.
Saving a field also clears its temporary override. Existing `POST /api/config`
remains the persistent configuration alternative.

`GET /api/branding` returns `effective`, `configured`, `temporary` and `revision`.
`configured` is the underlying live configuration, which can include a current
Preferences preview; it is not an independent read of the saved file.
`DELETE /api/branding` clears temporary overrides only and restores configured
branding. It does not erase saved choices.

An accepted write returns HTTP 202 with `{"accepted":true}`. This means queued,
not applied, saved or visibly rendered by the OS. Read the branding snapshot to
confirm application; validation/save failures use the application's error path.

## Fields and artwork

| Key | Value |
| --- | --- |
| `app_name` | Optional name, at most 128 characters; no control characters |
| `app_description` | Optional description/tagline, at most 512 characters; no control characters |
| `app_icon_preset` | `current` or `classic` |
| `app_icon` | Optional absolute artwork path on the serving host |
| `app_icon_playing`, `app_icon_paused`, `app_icon_stopped` | Optional state-specific host paths |

Unknown keys and malformed values are rejected. Null optional values remove the
explicit choice and allow startup/environment/brand defaults to resolve again.
Explicit configured values take precedence over those startup defaults.
State-specific artwork takes precedence over the base icon; clear those fields
too when replacing all playback artwork. Built-in presets need no host file
access. Custom artwork must be a decodable image file, at most 16 MiB and
4096 × 4096 pixels. Use multi-resolution ICO files for native Windows artwork.

Configuration permissions are required for reads; writes additionally require
control permission. Custom file paths additionally require host-file access.
Never send a client's local path expecting it to exist on the server.

## Other transports

JSON-RPC methods are `pealayer.branding.get`, `pealayer.branding.update` and
`pealayer.branding.clear`. The update method's `params` is the same object as
the REST POST body. For native command JSON use:

```json
{"command":"update_branding","update":{"values":{"app_name":"Cinema"},"persist":false}}
```

Native queries use `{"command":"get_branding"}` and temporary reset uses
`{"command":"clear_branding"}`. These commands are listed in the existing
command catalog, not a separate branding-only transport.

## Visible surfaces and limits

Revision-driven updates refresh the serving application's title, viewport and
taskbar artwork, tray tooltip/artwork, About, separate Preferences helper, native
media identity, and connected browser title/header/description/icon metadata.
The helper receives bounded event messages over its existing pipe. Browser
updates do not reconnect its control socket. Explorer artwork refresh is
coalesced across separate notification turns, rather than polled every frame.

The description is shown where a subtitle/tooltip fits; it is not appended to
every window title. An empty description preserves the ordinary interface.
Temporary branding is not serialized into user configuration, although normal
OS artwork derivatives/caches can still be written.

Remote-consumer API requests follow the existing authority routing. This does
not yet promise that a process-local temporary overlay is independently
replicated into every separately running native peer. Target the owning
application when choosing its branding. Browser-installed PWA caches, pinned
shortcut labels and signed executable resources are OS/package-managed, not
instant runtime metadata; the API does not mutate or re-sign a running binary.

## Verification

Focused native tests cover the shared request schema, transport dispatch,
permissions, runtime metadata and preservation of unrelated Preferences drafts.
The isolated real-window fixture's `-TestBrandingApi` checks temporary updates,
unrelated autosaves, selective persistence and reset. Its optional
`-TestDisplayedBranding` checks actual taskbar artwork pixels, not merely HICON
or API counters. See the current acceptance ledger for package/deployment gates.
