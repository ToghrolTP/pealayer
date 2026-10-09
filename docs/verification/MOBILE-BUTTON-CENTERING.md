# Mobile button content alignment

Trackers: [#80](https://github.com/ToghrolTP/pealayer/issues/80) and
[Web parity #63](https://github.com/ToghrolTP/pealayer/issues/63).

## Source audit and repair

The older bottom-navigation problem already has a specific repair: Ant's
collapsed-menu selectors otherwise keep a hidden label in layout. The current
phone rules hide that label and center the entire icon; those rules are retained.

Remaining button paths used inconsistent geometry. NLE play and volume/mute
overrode Ant Design's inline flex with inline grid. Ant Design 6 also inserts an
NBSP pseudo-element before button icon content to influence text baselines, while
the SVG itself is inline. That line box is not the glyph's visual extent, so its
baseline spacing can offset icons in compact zero-padding and mixed-size buttons.

The repair keeps a single centered inline-flex geometry for button roots and icon
wrappers, makes button SVGs block-level within those wrappers, and removes only
the button-icon baseline spacer. It covers normal buttons, icon-only buttons and
buttons in portaled dialogs/dropdowns; colors, dimensions, loading animations,
focus treatment, RTL ordering, labels and command handlers are unchanged.

The NLE play and volume-specific grid overrides now use that same flex layout.
Short-screen/mobile dialog footers explicitly center mixed-height buttons too.
There are no icon top offsets, transforms or mobile-only pixel nudges.
Intentional left alignment in stacked page headers and bottom alignment of
labeled sequence-editor inputs is not mistaken for button-content misalignment.

## Verification limits

TypeScript and production/PWA generation passed. Responsive CSS guardrails were
extended for root/wrapper/SVG layout, spacer removal and transport/footer overrides.
They remain unexecuted under the user's standing request to defer test suites and
slow CI waits. A current screenshot identifying any remaining affected mobile
button was requested; real phone/tablet/desktop visual acceptance is pending.
Do not turn source inspection or compilation into a claim of screenshot proof.

Installed package evidence will be recorded below after destination-validated,
graceful deployment. This pass changes presentation only, not Rust/API contracts.
