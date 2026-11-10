# Sidebar rail: shortcuts, context menus and panel drops

## Behavior

- Panel shortcuts now **toggle in their last location**. A visible panel hides;
  clicking again restores its docked/floating location or sidebar ownership.
  An inactive or collapsed dock tab is revealed first rather than closed. Explicit
  Open in sidebar and Dock to Workspace actions remain available in the menu.
- Individual sidebar widths and normal split sizes survive closing, tab grouping,
  serialization and drag/drop. Moving an existing sidebar into a horizontal dock
  carries its current width; otherwise docking retains the remembered split size.
  A rejected or cancelled drop changes neither layout nor placement memory.
- Active rail actions use a restrained palette-tinted surface, thin outline and
  inset edge marker, with contrasting neutral icon ink. Hover, press, focus and
  disabled states retain the exact same hit/paint bounds; never paint accent ink
  over an accent-filled tile. Every visible dock/sidebar panel is reflected.
- Sidebar headers now use application-owned centered icon/title, three-dot menu
  and configurable ×. The optional custom chevron returns the panel to its rail;
  it follows the existing hidden-by-default header preference. The stock donor
  title/menu chevron is not rendered. Context menus omit empty action separators.
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

## Process-local API

The existing `pealayer.workspace.tools.control` JSON-RPC method and
`workspace_tools` command now accept typed panel operations. The same command
uses existing HTTP/native IPC, WM_COPYDATA and socket dispatch rather than
simulated pointer input. For example, POST `/api/player/command`:

```json
{"command":"workspace_tools","action":"panel","panel":"EffectsLibrary","operation":"toggle"}
```

Other operations: `sidebar`, `dock` (optional `anchor` and `zone`),
`sidebar_width` (`width`, bounded to 240–600 points) and `sidebar_menu`.
Width/menu require the named panel to be in the sidebar; unsupported sidebar
destinations fail. Layout changes respect workspace locking; a locked sidebar
menu can still be viewed with its unavailable actions disabled. The tools
snapshot includes panel location/ownership and sidebar placement/size memory.
These commands are process-local even when Pealayer consumes a remote authority;
they do not execute shell input or hardware actions.

## Current repair verification

The focused sidebar, CraftUI header, egui Dock header and workspace-control
suites pass: 64 test executions, 63 distinct tests. Coverage includes native
pointer toggles, active-icon ink/geometry through both palettes/themes, RTL and
fractional scaling, exact dock-tree restoration, per-panel widths and storage,
current-width drag sizing, invalid drops, menu click/× handling, duplicate-divider
paint checks, API validation, locks, modal cancellation and existing header chrome.
Local release packaging, installed native captures and deployment receipt follow
in the focused sidebar recovery checkpoint; the older receipt below is historical.

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
