# Sidebar rail: shortcuts, context menus and panel drops

## Behavior

- The rail uses Pealayer's Phosphor icons and palette, with fixed-size targets,
  visible keyboard focus and Up/Down/Home/End navigation. RTL mirrors its edge.
- Right-click an icon to open its destination, dock a panel in the workspace,
  move the shortcut up/down, hide it, customize other shortcuts, reset, or open
  the exact Sidebar rail Preferences page. Empty rail space has its own menu.
  A permanent three-dot footer keeps configuration reachable even with no icons.
- Drag between icons to reorder. Drag a panel shortcut into a workspace pane to
  join its tabs or split at the indicated edge. The same typed workspace actions
  handle hidden and already-visible panels. Commit is atomic, preserves unrelated
  panels and removes the sidebar copy only after a successful drop. An empty
  workspace can also accept a panel. No panel is opened during hover.
- Dialog shortcuts open existing dialogs; dragging them only customizes their
  position. They never become fake dock panels, publish effects, connect hardware
  or run RF/output actions. Opening RF uses the existing manager/catalog command
  and requires live advertised RF capability, as Hardware Monitor does.
- Escape, secondary movement, modal ownership and workspace locking cannot commit
  a layout change. Invalid drops do not fall back to a click.

## Destinations and ownership

The original Hardware Monitor, Media Inspector, Effect Controls and Effects
Library shortcuts remain the only default-visible entries. Optional destinations
include Program Monitor, Timeline, RF management, Manage effects, Board information,
Downloads, Open Location / URL, Preferences, Keyboard Shortcuts, Workspace profiles,
Audio settings, Subtitle settings and About.

Preferences → Sidebar rail manages visibility, sidebar pane width, individual
shortcut visibility, order and reset. The existing Preferences schema advertises
the same typed action-list control to React; native and Web editors use its
options and current configuration instead of duplicating a live catalog.

Persistence remains in `ui.sidebar_rail` and `ui.sidebar_shortcuts`. Open sidebar
placement remains part of workspace/session snapshots; shortcut customization is
application configuration. The current config API can update these fields; the
existing process-local Preferences navigation target is `sidebar_rail` with
`scroll_y: 0`. No second settings store or hardware protocol was introduced.

## Verification and delivery

- 29 sidebar tests passed, including real pointer context menus and primary drag,
  phone/desktop widths, RTL, no hover actuation, hidden-panel placement, cancellation,
  modal blocking, locks, keyboard navigation and existing sidebar regressions.
- 68 focused Preferences tests passed, including the new page at 390/700/1200
  logical points without edits caused by rendering. The suites overlap.
- TypeScript passed. The initial packaging gate was the older isolated review's
  unknown Preferences helper. The user closed that helper, a fresh clean read
  allowed normal review shutdown, and packaging is now complete.
- The feature is included in the locally installed NLE refinement checkpoint in
  [PR #257](https://github.com/ToghrolTP/pealayer/pull/257). See the
  [native gallery and runtime receipt](nle-transport-layout.md#native-gallery-and-delivered-checkpoint).
  The rail is visible in the actual native review, and its shared Preferences
  editor was inspected using the isolated embedded-dialog fixture. Real pointer
  menus/reordering/docking are covered by the focused renderer tests; these are
  not a claim of human acceptance of every optional destination.
- Cafe deployment remains explicitly deferred until the user restores its normal
  connection. Do not change remote-access services to bypass that gate.
