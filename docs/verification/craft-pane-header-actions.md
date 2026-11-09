# CraftUI pane header actions

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

The requested header behavior supersedes the earlier menu-chevron decision:

| Control | Action | Default |
| --- | --- | --- |
| × beside each visible tab title | Close that specific panel, retaining its restore location | Shown |
| Three dots at the end of the tab strip | Open the contextual panel menu | Shown |
| Chevron beside the menu | Collapse or expand the panel group; never open a menu | Hidden |

Appearance → Workspace panels exposes separate **Show CraftUI panel close
buttons** and **Show CraftUI collapse buttons** settings. Both use the shared
Preferences contract, live appearance preview and the existing configuration
API (`craft_show_close_buttons`, `craft_show_collapse_buttons`). The egui Dock
adapter keeps its own native controls and geometry.

Close controls use actual visible upstream tab responses and current title
paint, including the floating layer and clip. Hidden overflow tabs get no
invisible close target. A fixed title gutter and inside focus outline keep
hover/press from shifting the title or its neighbors. An inactive tab's × closes
that tab, not the group's selected panel. Lock and modal shielding remain in
effect.

Collapse is not Close. Existing dock groups and floating windows use Craft's
minimization action. Split-tree tab groups retain their panel membership and
normal split dimensions; a persisted local view projects collapsed rows to
header height. Side-by-side cells retain their columns. Expansion restores the
original split dimensions, and explicit panel reveal expands the complete tab
group. The three-dot menu also exposes Expand/Collapse, so hiding the optional
chevron cannot strand a collapsed panel. Workspace/session snapshots retain the
view state through the existing egui persistence path.

## Verification

The native pointer/paint suite covers individual and inactive tab close,
three-dot menu activation, lock/modal guards, theme/scale/RTL alignment,
visibility defaults, exact collapse/expand geometry and explicit reveal.
Shared Preferences tests cover serialized values and cross-window preview.
The egui Dock regressions are run alongside these tests, not assumed unaffected.

All 56 focused native tests passed, including the shared Preferences contract,
egui Dock and CraftUI layout/header suites. Narrow-header checks cover 160, 240,
420, 900 and 1440 logical pixels in both directions. Header interaction checks
also cover both themes and display scales from 1× to 2×. Tests use isolated
configuration/instance identity and the command-free in-memory engine.

Packaging, actual native captures and installed-runtime checks are separate
gates and are recorded here after completion. Cafe deployment remains
user-deferred while its normal connection is unavailable. No hardware actuation
is needed to verify these controls. The separate taskbar-artwork gate in
[PR #260](https://github.com/ToghrolTP/pealayer/pull/260) remains unresolved.
