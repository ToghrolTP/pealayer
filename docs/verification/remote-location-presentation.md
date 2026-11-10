# Remote Location presentation and item properties

Owner: [PR #291](https://github.com/ToghrolTP/pealayer/pull/291), continuing
[#80](https://github.com/ToghrolTP/pealayer/issues/80).

## What was incomplete

The native list painted a selected background only. Hover, pressed and keyboard
focus did not have explicit row presentation; selection reused a strong accent
fill. Its filter placed a separate magnifier label outside the edit. Although
the icon picker already had a corrected measured header, the browser did not
use it. There was no native or Web grid renderer. Native thumbnail requests
discarded pending/error results and silently retained the file glyph. Web had
pending/retry mechanics without a visible progress state. The native item menu
included an unconstrained filename label and no Properties action.

Local native navigation/selection also mutated the owner after the current
snapshot had already been painted, without consistently requesting the next
frame. These actions now use the typed command handler and explicit repaint
notifications, including selection, sort, inspection, retry and Close.

## Implemented behavior

- Native search and trailing List/Grid controls reuse the **same** measured
  header as the icon picker: contained prefix, centered controls and stable
  trailing view switch. This does not change the icon picker's saved layout.
- List and Grid use one entry renderer/action model. Rows/cards have distinct
  hover, pressed, selected and keyboard-focus states. Subtle palette-derived
  fills replace solid accent blocks; borders paint inside stable hit bounds.
  Styling is scoped to this dialog, not a new global theme.
- Grid layout adapts to available width and font size, virtualizes rows and
  preserves aspect ratio. List and Grid keep separate native scroll identities;
  filenames elide with full-name hints. The filename/metadata labels do not
  steal the containing item's selection click.
- `remote_folder_view` is a shared persisted `list`/`grid` setting, default List,
  also exposed in Preferences → Playback → Remote folders. Native/Web use the
  host's `view_mode`, not independent renderer-owned settings. Existing verified
  sibling discovery and Back/Forward/Parent behavior are preserved.
- Pending previews show an indeterminate spinner; failures show a diagnostic
  hint and a retry action. Retry clears only a verified item's failed state,
  never an in-flight worker. Existing two-worker request limits are retained.
  Native decoded texture retention is bounded to 96 recently used previews;
  Web aborts obsolete requests and revokes object URLs.
- The native context menu has fixed bounded width and an elided caption. Both
  surfaces offer Properties, Copy link/name and the appropriate Open/Play action.
  Properties is a real read-only child modal with full wrapping filename, type,
  host, supplied size/date and location. Missing data is not invented; navigation
  rows have no fabricated file metadata or Properties action. Escape closes the
  child inspection rather than its parent browser.
- Properties uses metadata from the last verified listing and does not select,
  play or probe an arbitrary URL. Native/Web share `inspected` state through the
  existing authoritative browser snapshot. Navigation clears the inspection.
- Web gets matching restrained states, icon-prefixed filtering, List/Grid
  controls, paging, keyboard activation, delayed stable directory loading and
  a proper Properties modal instead of a second inline information block.

## Current control contract

Use the ordinary configuration API to set `remote_folder_view` to `grid` or
`list`. The command API accepts:

```json
{"command":"inspect_remote","target":"https://files.invalid/movie.mkv"}
{"command":"inspect_remote","target":null}
{"command":"retry_remote_thumbnail","target":"https://files.invalid/movie.mkv"}
```

Equivalent JSON-RPC methods are `pealayer.remote.inspect` and
`pealayer.remote.thumbnail.retry`. Targets must belong to the verified listing;
retry additionally requires playable media. Invalid values/URLs are rejected.
Inspection with null closes the child. Original encoded playback/copy targets
remain unchanged; displayed native locations redact URL userinfo.

The command permission audit also found that the recently added history command
was missing the Web host-file-access classification used by Browse. History,
inspection and retry now share that gate; closing an inspection/browser remains
possible without file access. No security preference is loosened.

## Evidence and remaining acceptance

The isolated native suite passes **1,379 tests, zero failures, six opt-in
ignored**. Native pointer/paint tests cover both layouts, both palettes and
light/dark states, narrow/wide widths and normal/enlarged fonts. They check
actual hit/paint geometry, selection versus Play, bounded long-name menus,
Properties activation/wrapping/Escape, verified metadata and safe retry.
JSON/IPC/RPC parity and invalid-input checks cover the new commands. A subsequent
focused remote/permission pass verifies the final file-access classification.
TypeScript
and focused Web contract, clipboard, mouse and application-parity checks pass.

These are source/input/paint checks, **not installed native screenshots**. The
loopback-only Web harness reuses the actual component and shared palette:

```powershell
cd web_ui
node scripts/serve-remote-location-review.mjs
```

Open `/scripts/fixtures/remote-location.html?mode=grid` on its printed loopback
address. Optional query values select `mode=list`, `theme=light` and
`palette=native`. It simulates thumbnail states with fixture data only: no real
host, user files, playback, configuration or outputs. Its module transformed
successfully over HTTP. The browser surface failed to attach on two attempts;
there is no screenshot or visual-browser acceptance claim from this run.

The previous source checkpoint and running apps remain preserved. Package
completion still needs the bounded interactive helper because of the confirmed
review-held scratch executable and the earlier tool-policy denial of generated
output cleanup. The helper stays paused during authoring and is repinned only
after the source is committed/frozen. Installed runtime/visual verification and
user-deferred Cafe deployment remain open. No source guard is bypassed and no
unknown draft, generated output or running app is discarded by this pass.
