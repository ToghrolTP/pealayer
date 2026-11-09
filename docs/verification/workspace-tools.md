# Optional command toolbar and terminal

Tracking: [#272](https://github.com/ToghrolTP/pealayer/issues/272),
[plugin architecture #44](https://github.com/ToghrolTP/pealayer/issues/44), and
[delivery #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Using the tools

Both surfaces are hidden in a fresh profile. Enable them from **Workspace** or
**Preferences → Workspace tools**. They use the current palette, fonts and
control density with either CraftUI or egui Dock; enabling or customizing them
does not reset panel membership or the sidebar rail.

The command toolbar reuses existing application commands. Its overflow menu
provides hidden/narrow actions and customization; individual buttons can be
hidden or moved earlier/later. **Ctrl+Shift+P** opens the searchable command
palette even when the toolbar is hidden. Arrow keys select, Enter invokes one
available command, and Escape cancels. Existing dialogs retain keyboard priority.

The resizable bottom terminal has independent session tabs. Opening it never
starts a process or connects. Choose a target and **Connect**:

- **PCController:** the selected host's command catalog and explicit command
  RPC, with history and advertised-name/alias completion. This is not the raw
  board serial terminal. PCController owns authorization, authority and safety.
  Its ordinary client may start the packaged local host on explicit Connect;
  no host is started merely by displaying the panel. A remote-consumer profile
  must choose an explicit controller endpoint, rather than interpreting its
  authority's loopback address on this computer.
- **System shell:** an owned local PTY/Windows ConPTY, using the system shell
  unless overridden in Preferences. Arguments are literal argv: quotes group
  spaces, doubled matching quotes are literal, backslashes are preserved, and
  no expansion/evaluation occurs in Pealayer. The shell itself is not a sandbox
  and may run its own startup profile.
- **PortableShell:** an optionally installed executable, using the same PTY.
  No additional shell binary is bundled or silently downloaded.

Hiding the panel preserves sessions. Disconnect/Close session cancels queued
input and tears down only the owned session; a host command already dispatched
cannot be rolled back. Normal application quit asks before ending sessions and
waits for owned-worker cleanup. Active sessions block unattended updater replacement.

## Terminal processing and boundaries

`vt100` parses fragmented UTF-8 and escape sequences into actual cells, including
cursor movement, erase, screen modes, ANSI/256/truecolor, underline/inverse/bold
and wide cells. The view supports selection/copy, bounded scrollback, application
cursor keys, common control/function keys and bracketed paste. Paste is atomic
and capped at 16 KiB; rejection is visible. Status/cursor queries are answered
only for PTYs at their actual byte-stream position, including ConPTY startup.

OSC output cannot modify the clipboard, open URLs or execute app commands. No
transcript or command history is persisted or published by the status API.
Mouse-report protocol, full-scrollback selection, complex-script shaping and
complete xterm compatibility are not claimed. A fixed-cell Unicode parser is
not proof of bidirectional Persian shaping.

Inputs/outputs are bounded, command execution is serialized, uncertain commands
are never retried, and all process/IO/resize/termination work is off the UI thread.
The explicit active-child waiter uses a bounded 100 ms wait and cancellation wake;
there is no permanent terminal repaint loop when idle or hidden. Controller TCP
responses are bounded before JSON allocation; console results have a separate
1 MiB limit. Shell output streams through bounded 8 KiB chunks.

## Local automation

The existing JSON/JSON-RPC transports expose process-local methods, never relayed
to a playback authority:

```json
{"method":"pealayer.workspace.tools.get","params":{}}
{"method":"pealayer.workspace.tools.control","params":{"action":"set_visible","toolbar":true,"terminal":true,"persist":true}}
{"method":"pealayer.workspace.tools.control","params":{"action":"palette","open":true}}
{"method":"pealayer.workspace.tools.control","params":{"action":"terminal","operation":"connect"}}
{"method":"pealayer.workspace.tools.control","params":{"action":"terminal","operation":"write","data":"help"}}
{"method":"pealayer.workspace.tools.control","params":{"action":"terminal","operation":"disconnect"}}
```

Connect, Write and Resize require **native local IPC**, not browser HTTP/WebSocket
requests. UI/visibility commands use existing Web control permissions. Status
contains only availability, visibility, session IDs, target, dimensions and phase.
An accepted command is queued once, not confirmation that it executed. Poll status
to observe connection transitions. `persist` requests an immediate configuration
save; normal workspace persistence still applies on later saves/application exit.

## Build/package boundary

Default builds include `workspace-toolbar` and `workspace-terminal`, both visually
disabled. The shipped product is one fully featured distribution, not separate
tool flavors. Internal `--no-default-features` checks omit both; either feature can
be checked alone without publishing another edition.
Omitted implementations retain configuration values without exposing broken
controls. [Extraction backlog](../backlog/workspace-tools-plugins.md) defines later
package ownership; this feature does not install a speculative plugin loader.

## Verification receipt

Local evidence:

- Default compilation; no-tools, toolbar-only and terminal-only checks passed.
- 42 focused workspace regressions passed, including seven command toolbar/palette
  tests. A separate terminal-filter pass passed 27 tests with two deliberate
  ignored fixtures; these counts overlap and are not an additive total.
- The opt-in real Windows PTY fixture passed UTF-8 I/O, resize, final-output/exit
  ordering and owned child cancellation. It uses the Rust test executable, not
  a production board or the user's shell profile.
- Mocked loopback PCController catalog/execute/error/disconnect/no-replay passed;
  the bounded-response reader regression passed. No physical output was actuated.
- Web TypeScript, preference-file tests, 53 parity contracts, workflow lint and
  whitespace checks passed. The packaged Web/PWA integrity check passed.
- A clean standard package from `1c8cc78911604100f49159b15fec39079d13bff8`
  passed manifest, identity, quick-action resources and runtime smoke checks.
  An isolated native review profile runs that package, not a reduced flavor.
- Native inspection found the disconnected terminal frame shrinking below its
  configured height. The subsequent source fix has an actual three-frame sizing
  and Preferences-height-change regression, which passed. That fix is not yet
  installed in the review process.

The user is actively reviewing the isolated window and providing feedback; leave
its settings and processes alone until coordinated. Final native screenshots,
feedback reconciliation and canonical deployment remain pending. Cafe is
user-deferred/offline. No physical controller or remote deployment acceptance is
implied by local fixtures.
