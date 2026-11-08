# Remote folder navigation that reflects availability

## Cause and correction

The remote browser advertised a parent URL computed from the path, even when
the host forbade listing that folder. Direct media discovery also advertised
its sibling folder after discovery had failed. The native list rendered that
navigation action as a file with “Date unavailable” and “Size unavailable”.

Rust now exposes `parent_url` only after a bounded GET returns a supported
directory index at the expected same-origin physical or query-based directory.
Forbidden, missing, timed-out, unsupported and redirected-to-self/unrelated
parents are omitted. A valid empty index remains navigable. Direct media stays
available if the sibling directory is not. Empty query directories normalize
to `/`, not `//`.

Both native and Web browsers label the action **Parent folder**, since this is
not navigation history. It has a navigation icon and no date/size metadata.
Single-clicking folders navigates; files retain selection and double-click
playback. Folder menus do not offer file information or disabled Play actions.
Loading disables navigation/play controls. If an advertised parent becomes
unavailable during navigation, that action is revoked while the last good
listing is retained; unrelated failures do not remove it.

No new wire field, parsing implementation or UI-side network probe was added.
All surfaces consume the existing authoritative remote-browser snapshot.
Optional parent verification is limited to two seconds and the existing 4 MiB
directory limit, on the selected proxy/direct route and worker thread.

## Verification

- Focused native discovery/model/presentation tests use isolated configuration
  and local HTTP fixtures: forbidden media parent, forbidden directory parent,
  self redirect, accessible empty parent, query-directory parent, stale action
  revocation, real metadata only, and existing parser/sorting/route tests.
  Final result: **16 passed, zero failed**.
- TypeScript compilation and installed Edge tests render the real dialog with
  a memory-only command sink at 390, 768 and 1440 CSS pixels in light and dark.
  Checks cover parent presence/absence, blank navigation metadata, one command
  on folder click, modal bounds and no document overflow. No media or hardware
  is opened. Screenshots remain in the private verification directory.
- Native packaging, installed API behavior and host deployment receipts are
  recorded separately below when completed. Source/fixture success is not
  acceptance of the user's remote host or physical hardware.

Cafe deployment remains user-deferred until its connection is restored.
