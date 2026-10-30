# Effects Library: double-click to Manage

Double-clicking the primary body of a native Effects Library card selects that
effect and opens the existing Manage/Effects Designer dialog. Context-menu Manage
and double-click share the same selection/open path for sequences, strip effects
and audio effects. Stable effect identities and the existing catalog lookup remain
authoritative; unavailable effects do not open an unrelated draft.

The card accepts click-and-drag instead of drag-only input. Its existing action
gutter still excludes Run, Add and menu controls; double-click during inline rename
does not replace the edited draft. Single click, secondary click and drag retain
their distinct purposes. Opening Manage does not run, publish or save the effect.

Regression coverage feeds actual egui pointer press/release frames through the
card: primary double-click versus single/secondary clicks, inline-rename guard,
action-button ownership and absence of a drag payload. Existing pointer drag/drop
and scroll-area gesture tests remain in place. No local Rust build/test is run on
the production host; hosted CI is the execution gate.

This pass requests source changes and a GitHub push only. No runtime is replaced;
installed-build double-click/drag/menu acceptance requires the matching hosted
package and a coordinated deployment with authoring-state checks. Source/CI
results must not be described as deployed visual acceptance.
