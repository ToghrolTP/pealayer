# Workspace tools: optional modules and later plugin extraction

Tracking: [workspace tools extraction #272](https://github.com/ToghrolTP/pealayer/issues/272),
under [plugin architecture #44](https://github.com/ToghrolTP/pealayer/issues/44).

Status: implementation and verification in progress. The authorized office proxy
route retrieved the pinned dependencies after the original connection failed.
Default compilation, native headless tests and the owned Windows PTY fixture
have passed. No deployment, physical hardware or screenshot acceptance is implied
until recorded in [the verification receipt](../verification/workspace-tools.md).

Parent tracking: [Pealayer continuation #80](https://github.com/ToghrolTP/pealayer/issues/80)
and [current acceptance index](../verification/CURRENT-ACCEPTANCE.md).

## Outcome

Provide a configurable top command toolbar and bottom terminal without changing
the default workspace. Both are hidden by default. They must work with egui Dock
and CraftUI, preserve their independently owned chrome, and eventually be
extractable into optional packages without copying application business logic.

The initial boundary is compile-time, not a dynamic plugin loader:

Product decision: publish **one standard distribution with all tools included**,
not separate flavors. Tool visibility is a runtime preference. The internal
feature checks below do not introduce user-facing editions. Future broader
build gating also includes [the downloader #110](https://github.com/ToghrolTP/pealayer/issues/110):
its queue/service, native/Web UI contributions, engine adapters and standalone
utility must be gated coherently without duplicated logic or broken routes.

| Cargo feature | Purpose | Default build | Default visibility |
| --- | --- | --- | --- |
| `workspace-toolbar` | Application command toolbar and command palette | Included | Hidden |
| `workspace-terminal` | Native terminal UI, VT parser and PTY transport | Included | Hidden |

`--no-default-features` excludes both implementations. Each feature must also
build independently. Persisted preferences should retain user choices when an
optional implementation is absent; unavailable features must not expose broken
actions. Enabling visibility does not launch a process, open a connection or
send a controller command. A live terminal continues to have an explicit
session owner; hiding its UI must not silently masquerade as disconnecting it.

## Existing source boundaries to preserve

- `src/ui/workspace_toolbar.rs`: visibility, ordering and command presentation;
  application commands remain owned by their existing dispatch paths.
- `src/ui/terminal.rs`: preferences, tabs, explicit connection intent, controller
  line editor/history/completion, session state and application integration.
- `src/ui/terminal_backend.rs`: bounded session queues, worker-only process/IO
  lifecycle, controller command RPC and local PTY transport.
- `src/ui/terminal_screen.rs`: parser-backed native cell grid, selection/copy,
  keyboard encoding and viewport dimensions; it cannot launch a process or
  dispatch a command by itself.

Use these boundaries for extraction, rather than making an alternate controller
client, duplicating command ownership, or adding a second configuration writer.
Define a narrow application service interface for command discovery, validated
dispatch, preferences, palette/fonts, focus/modal ownership, notifications and
capabilities. The host owns permission and authoring gates. Packages must be
unloadable only after their sessions are closed; no detached process, timer,
event subscription, repaint loop or input hook may survive teardown.

## Terminal targets and authority

**PCController host console:** use the existing authenticated/current-contract
controller client. Discover `controller.command.catalog` (a direct JSON array
of command descriptors); submit explicit single-line commands with
`controller.command.execute {"command":"..."}` and render its `output` string.
Host errors, authority locks, confirmation requirements and safety gates remain
PCController-owned. A terminal tab is not a second hardware scheduler. Never
automatically send a command, replay an uncertain mutation, acquire raw device
terminal ownership, or run commands when opening/refreshing the UI.

This is a host command console, **not** the NativeDesktop `/ui/terminal` raw-device
WebSocket bridge. It is also not a remote operating-system shell. The initial
host command path returns complete command results; it does not claim streaming
host-shell output, VT editing or full-screen programs. An explicit console
Connect must have clear behavior for attaching to an existing host versus
starting the bundled host; source review identified `connect_preferred` as a
possible bundled-host startup path requiring that decision.

**System shell:** spawn only the user-selected local executable through an owned
PTY/ConPTY, with literal argv rather than concatenated shell commands. Resolve
paths off the UI thread. Do not elevate, inject into another process, start a
remote shell or fabricate user input. Closing that session only closes its
owned child and transport; unrelated applications remain untouched. Process
startup may itself run the chosen shell's user profile, which must not be
misrepresented as a sandbox.

**PortableShell:** optional user-selected executable using the same PTY backend.
Do not bundle or silently choose a stale development binary. Redirected ordinary
stdio can select PortableShell batch mode and is not an interactive substitute.
A later embedded callback-core adapter can be considered as a distinct package;
it must not duplicate the terminal renderer or grant additional command rights.

Output, clipboard contents, shell history, private paths and controller commands
can be sensitive. Keep history in session memory and do not publish transcripts
or runtime configuration. OSC output cannot modify the system clipboard, launch
URLs, change application branding or execute actions. Only explicit user copy
changes the clipboard. Existing transport authentication and endpoint policy
must remain authoritative; do not invent token fallbacks.

## Initial VT scope and limits

The native view uses `vt100` 0.15.2 with `portable-pty` 0.9.0 behind the terminal
feature. It renders real cells rather than stripping ANSI into a text log:

- Theme-aware default/ANSI colors, 256-color and truecolor foreground/background,
  bold, underline, inverse, wide-character continuations and a steady cursor.
- Current app monospace selection, bounded 4–100 rows and 20–320 columns; resize
  only on an actual geometry change. Backend PTY bounds are independently checked.
- Wheel and Shift+Page scrollback; bounded configured history (100–10,000 lines).
  Selection and Select All currently operate on the visible viewport, not all
  historical output. Soft-wrapped rows are copied as visible rows.
- Cursor/application-cursor sequences, common control/function keys and UTF-8
  input. Paste is one atomic event, at most 16 KiB; bracketed-paste delimiters
  follow the parser's mode. An oversized paste is rejected visibly, never split
  or silently truncated into executable partial commands.
- Focus/modal-aware event claiming and application shortcut suppression. Text
  typed into the terminal must not also control playback or timeline zoom.
- No mouse-report protocol or cursor-blink timer in the initial renderer. Do not
  advertise complete xterm compatibility. Complex script shaping and full-screen
  program compatibility require real visual/PTY verification, not a Unicode
  parser unit test alone.

Device-status replies, if enabled, must be parsed at the correct point in the
byte stream and only emitted for a shell target. Searching raw output for a
query substring can mistake OSC payload text for a command or report the cursor
after later text. Unsupported queries must not become fake input or hardware
commands.

## Acceptance checklist

### Build and automated fixtures

- [ ] Fetch and pin dependencies; build default, no-default-features, toolbar-only
  and terminal-only configurations on supported desktop targets.
- [ ] Run native renderer tests: fragmented ANSI/UTF-8, combining/wide cells,
  color mapping, geometry, raw keys, application-cursor mode, bracketed paste and
  atomic rejection. Record actual results rather than test presence.
- [ ] Exercise real egui input: Space does not play media, +/- does not zoom,
  modal/popup ownership blocks terminal input, and read-only controller output
  does not steal keyboard events from its line editor.
- [ ] Run the explicit isolated PTY test executable with
  `PEALAYER_TEST_PTY_FIXTURE=1` supplied by its parent before startup. Verify UTF-8
  echo, resize, exit ordering, tail-output drain and session cancellation. Do not
  use a production controller or the user's shell profile as the fixture.
- [ ] Cover bounded queues, full-queue visible errors, stale-generation rejection,
  child spawn/read/write/wait failures, close-during-blocked-output and repeated
  tab creation/closure. Verify all owned workers/children eventually exit.
- [ ] A capped per-frame event drain must schedule another repaint while queued
  events remain; it must not stall after a burst or add permanent idle polling.
- [ ] Bound controller response allocation, not merely post-parse catalog size
  or output queue chunk size; malformed/oversized responses fail visibly.
- [ ] Use an offline controller fixture for catalog and harmless command results;
  verify no command retry, no automatic hardware connection/ownership, meaningful
  timeout/authority errors and explicit host-start behavior.

### Native UI and operational delivery

- [ ] Capture real before/after native UI at matching theme, palette, density,
  scale and viewport in both workspace renderers; inspect narrow, RTL and large
  fonts. Keep source-fixture evidence separate from production captures.
- [ ] Confirm both tools are hidden on a fresh configuration, Preferences/API
  changes persist correctly, and unavailable build features degrade clearly.
- [ ] Verify toolbar commands reuse existing dialogs/dispatch; hiding/reordering
  commands must not close panels or run hardware actions.
- [ ] Verify live shell editing, copy/paste, Ctrl-C, scrolling, resize and exit on
  Windows and a supported Unix platform, including one full-screen TUI. Record
  actual unsupported terminal features instead of claiming universal support.
- [ ] Verify the chosen PortableShell executable separately if configured; an
  unconfigured optional target must provide a useful non-invasive empty state.
- [ ] Sample idle and burst-output responsiveness; no idle repaint/poll loop,
  unbounded memory growth, blocked UI IO or orphan processes.
- [ ] Package from a clean checkpoint, inspect authoring state, use the graceful
  updater and verify the actual installed executable and relevant UI. Complete
  Cafe deployment/runtime checks or state the concrete deployment blocker.

### Later package extraction

- [ ] Link this backlog to the live plugin architecture owner before designing
  an ABI; do not ship a speculative dynamic loader as part of the current UI fix.
- [ ] Extract optional packages behind the narrow host services above, preserving
  stable action IDs and user preference values without compatibility aliases.
- [ ] Test feature exclusion and package absence/unload without lost settings,
  duplicate commands, worker leaks or orphaned focus/event hooks.
- [ ] Keep controller authority and executable-launch permissions host-owned;
  packages cannot bypass them. Document dependency licenses and distribution.
- [ ] Extend planned gating to the downloader and other optional feature families,
  preserving the full standard distribution and independently selectable runtime
  visibility; do not create reduced app flavors in this initial PR.

## Evidence still required

Keep current evidence in [the verification receipt](../verification/workspace-tools.md).
Formatting is not compilation; compilation and isolated fixtures are not physical
controller acceptance. #272 retains the package/plugin work after this initial
native implementation is delivered; #44 remains the architecture owner.
