# Preferences layout and live preview

Owner: [workspace UI PR #225](https://github.com/ToghrolTP/pealayer/pull/225),
with acceptance tracked in [#80](https://github.com/ToghrolTP/pealayer/issues/80).

## Repaired causes

- The native content scroller reused one ID for every section. Its identity now
  includes the stable section ID: navigation restores that section's scroll only.
  Web section navigation likewise retains independent content scroll offsets.
- File controls reserved a constant reset width but rendered a differently padded
  button. The extra width expanded the next row, producing a staircase. Input,
  Browse and Reset now occupy measured slots in a single allocated row.
- Scroll content used the outer width before subtracting the scrollbar. Native
  cards now use the content viewport; long selectors truncate and NIC details
  wrap. Web grid tracks and controls have explicit shrinkable bounds.
- The separate Preferences process addressed its owner by branding text, omitting
  the named-instance suffix. The parent now supplies the actual listening identity,
  retained for its lifetime even when branding is previewed. Preview and Discard
  use that endpoint; failed delivery does not advance the preview baseline.
- Color palette follows Theme in the shared contract. Native and Web dropdowns
  show miniature surface/text previews from `assets/themes/palettes.json`, using
  the active light/dark scheme, not unrelated hard-coded colors.

Persian UI uses installed Yekan Bakh VF when available and applicable; no private
font is bundled. Vazirmatn remains the bundled fallback. Explicit font choices
remain authoritative.

## Verification and boundaries

Focused tests cover equal file-row bounds across widths, themes and densities;
independent scroll positions; real Windows named-pipe preview/discard routing;
and shared control ordering. TypeScript, preference-file and responsive guards
are run separately. These are source/geometry/transport checks, not screenshots
or production deployment receipts.

Keep native live visual review and host-compatible deployment separate. The
configured Cafe SSH endpoint refused connections during this pass; do not mark
Cafe delivery complete without a fresh updater receipt and runtime verification.

The project/scene persistence draft belongs to issue #226. It must not be included
in a production package or described as complete merely because these Preferences
checks pass.

## Verified UI checkpoint

Executable source `d6feabf8c0956523aa5050bf978975cee8b6c431` was launched in the
isolated, hardware-disconnected review profile. Preferences remained a separate
native window. Process-local navigation selected/scrolled Appearance, Typography
and Web UI; read-only Win32 captures were inspected:

- all four icon-file rows have matching input, Browse and Reset edges;
- Color palette follows Theme with its surface/text preview;
- the long network-interface caption truncates and cards stay within the column;
- Typography controls and the Persian sample render without UI automation.

The exact UI-only branch passed 60 Preferences native tests, six Web appearance
tests, TypeScript and preference-file/responsive guards. Windows resource,
quick-action icon, player libmpv smoke and downloader smoke checks passed.

Publication was interrupted because the regular application was actually running
from the staging executable, locking that destination. This was confirmed with
`QueryFullProcessImageName`, not inferred from a generic access-denied message.
The subsequent graceful peer-updater invocation was explicitly denied by the
execution tool's policy. A manual updater action was requested; no restart,
canonical installation or Cafe deployment is claimed without its receipt.

Typography follow-up: inspect variable-font weight coordinates as well as family
selection. The Persian sample appears unusually light; resolving an installed
variable family alone does not prove its requested regular/semibold axis is used.
Do not mark that visual detail accepted solely from a successful font lookup.
