# Control-bar action customization

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Interaction contract

Simple player controls and the NLE Program Monitor now use one native action
component. Right-click any action, including an unavailable one, to move it
earlier/later, place it at the logical start/end, or hide its icon-labelled
entry. The menu retains existing transport options where applicable. The
permanent More button exposes Show / hide actions and Reset toolbar.

Primary dragging starts only after real pointer movement, preserves the
mouse-down grab offset and accepts only that bar/panel's typed payload.
Drop before/after another button to reorder; dropping into another side adopts
that side. Empty-side space remains a drop target. Dragging over More changes
it to Trash and fades/strikes the original-render preview; release hides the
action. Escape cancels the drag and cannot restart it during the same press.
Secondary clicks, stationary holds, customization and disabled primary clicks
do not execute transport or board commands.

The action catalog covers Play/Pause/Replay, Stop, contextual Back/Forward,
previous/next chapter, video/audio/subtitle selection, fullscreen, floating
control pinning, audio/subtitle settings, Mute and Preferences. Simple and
Monitor have independent saved layouts; Monitor does not expose floating-window
pinning. Existing defaults are retained. Additional actions can be enabled from
More. Seek/time and volume sliders remain independent widgets, not drag handles.

Geometry follows the shared control density. Icon buttons and More/Trash use
square slots; borders paint inside invariant bounds. Hover/drag feedback and
the retained seek/frame-step caption crossfade honor animation preferences.
Play/Stop/Mute are neutral when idle, with semantic colors for enabled
interaction or authoritative active playback/mute state.

The NLE selects its smallest fitting row count from actual configured group
widths. Its trailing action group precedes volume on the final row. Narrow
action strips scroll horizontally without consuming another vertical row;
More/Trash stays outside the strip and reachable even when all actions are
hidden. Existing Program Monitor no-scroll viewport tests remain applicable.

## State and persistence

`control_bars.simple` and `control_bars.monitor` contain a complete semantic
action order plus indexed hidden/start-end flags. Native and Web tests compare
the exact current action identities and flag indexes. Hidden actions retain
their identity/order; unknown action IDs and malformed fixed arrays fail
deserialization.

Painting queues a coalesced, narrow save outside DockArea. It changes only
`control_bars`, never a temporary dock placeholder, workspace/session, media
properties or unfinished effect draft. Status-bar saves reuse the same bounded
helper. Equal-state saves do not rewrite the file. Peer saves use optimistic
concurrency checks and wait for authority acknowledgement/snapshot confirmation;
errors roll back to authoritative preferences with a visible warning. A live
Preferences preview must finish before toolbar customization is persisted.

Web player/Monitor reuse a shared ControlBar and the same saved layouts, with
context menus, before/after drops, side placement, hide/restore, Escape and a
trash target. Disabled inner buttons/selectors remain independently draggable.
The browser's native drag image uses the original grab offset; the original
slot becomes invisible rather than adding a duplicate framed ghost. The Web
client does not advertise native-only floating-window pin/audio/subtitle panes.
Its existing Simple file/chapter navigation and media selectors are retained.
Browser fullscreen and Preferences open browser-owned surfaces.

The typed `pealayer.transport.nudge` RPC accepts only direction -1 or 1 and
uses the authority's current paused/playing state and configured step. This
avoids sending stale client-side seek intervals or substituting seeks for
paused frame stepping.

## Verification and delivery limits

Native regressions exercise actual disabled controls, cross-side drag/drop,
grab offsets, hide drops, right-click menus, Escape, all-hidden/narrow/RTL/density
geometry, configured monitor row reservation and dock/authoring preservation
through local and peer saves. Web checks cover the shared action schema,
immutable ordering/visibility edits, customization integration and narrow CAS.
These complement existing ruler/status, full native and Web/PWA regressions.

Source tests and packages do not prove human UI, physical/audio timing, or Cafe
runtime acceptance. Local Pealayer is closed and its updater endpoint is
unavailable. The earlier local GUI launch policy rejection and Cafe smoke-launch
denial are not bypassed. Source-only Cafe fast-forward is not an installed
runtime update. Live native screenshots and runtime acceptance remain gated on
an authorized/manual launch and the host-compatible peer updater.
