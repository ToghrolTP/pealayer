# Terminal: one host, multiple views

Continuation: [#80](https://github.com/ToghrolTP/pealayer/issues/80).

## Native design and PortableShell

- Session tabs have their own padded body, close target, restrained active
  surface/outline, hover feedback and motion-governed insertion/removal. Closing
  another tab preserves the selected session's identity. Closing the final tab
  leaves an empty state rather than silently recreating it.
- Terminal has a dedicated Preferences page. Its menu and screen context menu
  navigate there directly. Cursor shape, blinking, command-accent coloring,
  font size, scrollback, default target and security are configurable.
- The existing pinned PortableShell framework is embedded, not launched as an
  executable. Its multiline prompt, categorized help/usage, command and setting
  completion, F7 history, bracketed paste and editable settings registry are used.
  `configure` opens the upstream in-place table; `get [KEY]`, `set KEY VALUE`,
  `help set`, `status` and the explicit playback commands operate real Pealayer
  state. A queued setting is pending, not falsely reported as applied.
- Shell configuration uses the application's validation, persistence and runtime
  side-effect owner. Inline writes compare their observed value before applying;
  a stale edit refreshes instead of overwriting another surface. The curated
  scalar registry excludes secrets, executable paths and security grants. Native
  Preferences and the existing configuration API remain available for other keys.
- Real semantic colors are retained. The optional command/accent role follows
  the live app palette without recoloring warnings, errors or numeric values.
  Generic success notices expire; command history and errors are not deleted.
- Alternate-screen mouse reporting is forwarded only when advertised, with Shift
  retaining text selection. Prompt-mode wheel scrolling remains scrollback.
  Control-code/oversized paste is rejected atomically. Multiline paste requires
  bracketed-paste support, and never becomes an implicit submission.
- Transitions and caret blinking respect reduced motion. Hidden/unfocused
  terminals do not install a continuous blink timer. The core's coalesced table
  paint gets one bounded settle tick, not a permanent polling loop.

The PortableShell conversation's user requests were read through its oldest
message, and the actual pinned gallery/settings-editor implementation inspected.
Relevant defaults include semantic colors, multiline prompts, inline editing,
completion rollover/history, distinct prompt/table mouse ownership and no static
introductory narration. OS-specific demos, firmware controls and sample data are
not imported as fake Pealayer capabilities. Vendored upstream files are unchanged;
the small application adapter supplies required panel metadata and callbacks.

## Shared sessions and remote security

Desktop, Web and the native remote viewer use the same host session IDs, active
selection, profile discovery results and VT parser. Browsers are views, never a
second PTY owner. A remote Pealayer viewer uses the same host API and existing
authority origin. General workspace/status/WebSocket data contains metadata only,
never terminal screens, history, environment variables or profile arguments.

All execution grants default off:

1. Local system shells require `ui.terminal.allow_system_shell`.
2. Remote viewing/control requires `ui.terminal.allow_remote`, Web control
   permission, and a native-issued access key. It expires after eight hours,
   lives only in memory and is revoked on restart, replacement, explicit revoke
   or disabling access. A newly paired key replaces the old grant.
3. Remote system shells additionally require
   `ui.terminal.allow_remote_system_shell`. Otherwise their sessions/screens are
   omitted and all access to their IDs is denied. Imported profiles resolve on
   the host, with the existing elevation/unresolved-profile guards.

Remote configuration cannot grant itself execution rights. Native provenance is
transport-owned and cannot be deserialized from JSON. Revalidation occurs again
on the UI owner, closing the queued-config race. Requests have bounded sizes,
queues, deadlines and viewer counts. Input is serialized and never retried after
a lost acknowledgment. Disconnect cancels unsent viewer input, not host sessions.

Use HTTPS or a trusted encrypted tunnel outside localhost: a bearer key does not
encrypt HTTP. No unauthenticated Telnet/OS-shell listener is opened. Existing
same-origin/Host/DNS-rebinding checks remain in force; keys are not URL parameters,
browser storage or general status properties.

## Current API

Native JSON-RPC over the existing IPC path (`--remote` accepts the JSON):

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.terminal.control","params":{"operation":"snapshot"}}
```

Native-only `pealayer.terminal.pair` returns the private access key;
`pealayer.terminal.revoke` revokes it. Pairing is also in the native Terminal menu.
Never publish these responses or put keys in URLs.

Authenticated HTTP uses `Authorization: Bearer ACCESS_KEY`:

- `POST /api/terminal/control`: `snapshot`, `create` (target), `create_profile`
  (host profile ID), `select`, `close`, `clear`, `input` or `scroll` (session ID),
  and `refresh_profiles`.
- `GET /api/terminal/events?after=REVISION`: bounded, event-woken long-poll with
  revision and the authoritative private screen/profile snapshot. Idle timeouts
  are heartbeats, not synthetic state changes. Revocation is checked while waiting.
- The normal Pealayer peer gateway preserves these paths and the authorization
  header. A client cannot substitute its local filesystem/profile for the host's.
- Web UI: `#terminal`; its navigation entry remains hidden until the Terminal or
  remote access is enabled. Disconnecting that view does not close host sessions.

## Evidence and remaining gates

Focused native/core/type checks and actual native/Web captures belong to the PR
receipt. A before image was captured from the installed build, cropped to the
Terminal only (no private media). Production captures/transcripts/access keys stay
outside Git. Hosted platform checks, user acceptance and production deployment
are separate from source verification. Cafe remains unavailable at the user's
request; no tunnel/service or production hardware is changed by this pass.
