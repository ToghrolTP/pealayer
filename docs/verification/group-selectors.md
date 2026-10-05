# Group selectors

Channel management, inline channel grouping, effect properties and recording setup
use searchable single-value comboboxes instead of free-text group/category fields.
The egui and Web interfaces share the same behavior within their renderer.

## Use

1. Open Manage for a channel and expand the Group dropdown (or an effect's Category).
2. Choose an existing group, or type to filter the live choices.
3. To create a new group, type its name and choose **Create group**. In egui,
   Enter also accepts the name. Choose **Ungrouped** to clear the assignment.
4. Native channel dialogs retain **Save presentation**; inline channel edits retain
   their Save button. The Web channel dialog applies selections using
   `hardware.presentation.update`. Effect edits retain their existing save flow.

Choices come from PCController's current channel/effect catalog, with the current
draft retained even if absent from the latest catalog. No sample group names or
second group store were introduced. Searching alone neither modifies a draft nor
sends an RPC. Group names retain case, Unicode and existing authoritative identity.

## Verification

- `cargo test --lib --locked --jobs 1 group_picker -- --test-threads=1`:
  options/normalization, bounds and draft preservation, plus real egui pointer
  selection/search/Enter creation/clearing in light and dark themes.
- Existing mutable-presentation contract test checks PCController catalog parsing.
- `npm run build`: TypeScript, production bundle and offline PWA integrity.

Automated renderer/input checks are not physical-board or desktop screenshot proof.
