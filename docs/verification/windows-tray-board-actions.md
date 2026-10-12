# Windows tray Hardware submenu

Pealayer's native Windows tray menu includes a localized Hardware submenu while its
selected coordinator and physical/virtual board are connected with an
authoritative host identity. Direct-UART diagnostics and disconnected/unknown
boards do not expose it. The header preserves the advertised board name.

Actions:

- Hardware Monitor restores, reveals and focuses its dock tab and main window,
  including leaving fullscreen and entering the authoring layout as needed.
- Emergency stop engages the existing shared controller latch immediately.
  An active latch is shown disabled, with a separate Release E-STOP action.
  Release follows the existing GUI confirmation preference and brings that
  confirmation into view; it does not implicitly confirm or resume playback.
- Stop buzzer is present when buzzer capabilities or a configured melody catalog are advertised. It
  uses the existing immediate buzzer-stop operation, not a new tone or melody.
- Stop board effects uses the existing tracked effect-stop operation, enabled
  only for a running macro or strip program, not merely a nonempty catalog.
- Stop strip animation / Clear LED strip require advertised strip control.
  Animation stop is disabled when not running; Clear remains idempotently usable.
- Restore automatic status LED releases the host override through its existing
  controller operation, only when status-LED controls are advertised and the
  coordinator reports a preview/effect override, not automatic lifecycle ownership.

Local pending commands and advertised operations from other clients disable
ordinary actions; E-STOP is not
blocked by them. Unsupported actions are omitted. Board actions are translated
into Persian with application copy; peer board names are not translated. The
native menu owns and frees its child menus and theme-colored bitmap icons.

## Safety and freshness

The menu is a memory-only projection. It does not fetch, poll, provision tools,
write configuration, or command hardware just to open. A selection carries its
frozen coordinator/board/profile/port identity and is revalidated against the
live projection on the UI thread. Disconnected, switched, unsupported, busy or
already-changed latch actions are rejected, not replayed after reconnect.
Completed playback and released LED ownership also invalidate old enabled
selections. Macro lifecycle edges request one coalesced authoritative refresh;
step/note events do not cause catalog polling. The native popup is a snapshot
at opening, not an in-place reentrant update during `TrackPopupMenu`; dispatch
always checks the latest state before issuing a command.
Failures publish the existing warning/toast path. Physical operations reuse
Pealayer's existing tracked RPC/authority/error handling, not another board owner.

The submenu intentionally excludes motion, raw relays, reset, firmware upload,
and unsolicited test tones. Adding a tray entry is not permission to actuate
those domains during verification.

## Verification boundaries

Pure model tests cover connection/identity gates, omitted capabilities,
contextual busy/latch/animation/macro/LED ownership state, semantic IDs, source switching and
translations. App tests cover stale/busy/disconnected/direct-UART rejection,
non-destructive release confirmation and restoration of a closed monitor tab.
These app fixtures use a memory-only engine handle with no transport worker and
a closed command channel. Manually setting connection flags on a live engine is
a race: its worker can overwrite them before the UI assertion, as Mac CI caught.
A Windows-only test constructs the actual Win32 submenu and checks its command
IDs, child ownership and enabled/disabled flags without opening a popup or
commanding a board. The runtime shell diagnostics expose the projected labels
and enabled states for a read-only installed check.

Native menu/model tests and diagnostics do not prove a human tray click or a
physical output change. Cafe runtime delivery remains blocked by its recorded
smoke-launch denial; safe source alignment does not replace that acceptance.

The contextual Hardware refinement passed 839 native library tests (one ignored),
including the actual Win32 submenu fixture, live-to-idle stale-selection guards,
remote-operation gates and snapshot/event parsing. Web type checking and the
51-contract application parity, channel-folder and status-widget guards passed.
The existing authoring process was not restarted while draft-save readiness is
unconfirmed. No hardware was actuated to verify menu availability.
