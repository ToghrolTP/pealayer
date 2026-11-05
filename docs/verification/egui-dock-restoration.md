# egui Dock: restore application chrome and header interactions

## Cause

The original renderer customized more than its tab content. It supplied the
application palette, Phosphor disclosure carets, a focused-panel outline, tab
response geometry, and an empty-header context menu.

- `967b297` introduced the shared disclosure icons.
- `0ec9ffc` retained the empty-header panel menu.
- The CraftUI renderer migration (`37c8acb`) removed those egui Dock hooks.
- The selectable egui Dock adapter (`1fbed60`) brought the renderer back with
  stock `Style::from_egui`, without restoring those hooks. Tab-specific context
  menus survived; right-clicking unused header space did not.

These are source-history findings, not evidence that the user's settings or
custom icon assets were deleted. The adapter omitted their presentation paths.

## Restored behavior

- egui Dock reads the same live application palette as CraftUI: tab surfaces,
  text, separators, hover, close controls, focus and active-panel outline.
- The Phosphor right/down caret replaces the stock filled triangle, while the
  existing upstream hit target and collapse behavior remain intact. Header
  height grows for larger UI fonts. Floating panels paint on their own layer.
- Right-click unused tab-bar space to show/hide panels or reset the workspace.
  Right-click a tab for its contextual actions, Preferences, docking and Close.
  The empty-header menu does not claim body clicks or neighboring panels just
  because they share the same vertical coordinates.
- Explicit panel reveal expands collapsed ancestors and queues a repaint.
- Modal dialogs retain input ownership, and locking prevents detach via the
  tab menu. Custom chrome remains painted underneath a modal backdrop.

CraftUI remains selectable and unchanged. Both renderers still use one canonical
workspace, panel implementation and hidden-panel restore model. There is no new
persisted layout schema, reset-on-upgrade or dependency/theme replacement.

## Verification and remaining delivery gates

Native tests exercise actual egui pointer events and paint output, not only
configuration flags: both tab/header menus, body-click isolation, collapse and
reveal, floating layers, hide/restore, modal ownership, and existing workspace
projection invariants. Caret containment is checked in light/dark at 100%, 125%
and 200% display scale. Fixtures use a parent-selected isolated configuration
and the memory-only hardware engine; they do not actuate outputs.

Build, installed screenshot and deployment receipts will be recorded in the PR.
Source tests do not establish a deployed runtime or interactive acceptance.
Cafe-PC's configured SSH route refused the connection during this pass; this is
an explicit remote-deployment blocker, not a successful installation.
