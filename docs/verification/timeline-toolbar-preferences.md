# Direct Timeline Toolbar preferences

The Timeline Toolbar overflow menu previously called the generic Preferences
opener. That intentionally retained the last selected page, so the toolbar entry
could land on Appearance or another unrelated page.

The entry now uses the existing process-local Preferences navigation contract:
`pealayer.preferences.navigate` with
`{"section":"timeline_toolbar","scroll_y":0}`. The dedicated page contains the
toolbar visibility setting and the same native visibility/order/reset editor as
the toolbar menu. It works with the separate native Preferences helper and the
embedded dialog, including an already-open draft. Navigation does not save,
discard or otherwise modify that draft.

The section is appended to the schema so persisted numeric selections for the
existing pages keep their meaning. Its visibility control is moved, not copied,
from Input. The navigation rail measures captions instead of truncating the new
page name to a fixed rail width. The Web schema and navigation icon recognize the
same page; the native customization editor remains shared with its toolbar menu.

## Regression gates

- Real rendered overflow-menu pointer click opens the exact page.
- Helper-message serialization and process-local JSON-RPC accept the same target.
- Repeated navigation at phone, compact-window and desktop widths preserves
  unsaved changes and resets only the destination scroll position.
- Existing Preferences section/scroll, draft, file-picker and layout tests remain
  applicable. Rendering and navigation do not actuate hardware.

Deployment receipts and native visual evidence belong to the owning PR. Cafe
deployment remains deferred until the user restores its normal connection.
