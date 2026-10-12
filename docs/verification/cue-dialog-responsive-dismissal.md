# Responsive native cue properties

The cue dialog shares its editor/model with the docked Effect Controls panel.
At 680 points of usable body width it uses a 40/60 two-column layout: identity
and name on the left; timing and output/routing on the right. The docked panel
and narrower dialog retain stacked cards. A bounded scrolling body leaves the
header and completion actions visible on shorter viewports.

The header has a centered title/icon and an upper-right 32-point square ×.
Card icon tiles are exact 32-point squares, time steppers are 28-point squares,
and measured title/subtitle groups center beside their icons. Selected cards
use one accent outline, without a competing left stripe.

## Dismissal

- Done accepts the current edits.
- × and Escape use the same dismissal decision. An unchanged existing cue
  closes immediately; an unchanged newly inserted draft is removed, preserving
  the existing insertion-cancellation contract.
- Changed cues prompt for Save and close, Discard changes/cue, or Keep editing.
  Closing the confirmation returns to editing, without losing changes.
- Discard of an existing cue restores the opening timeline and undo snapshot.
  Discard of a new cue restores the pre-insertion timeline/undo snapshot.
- Output controls edit authored values; they do not actuate the live board.

## Verification boundary

Focused egui tests exercise narrow/wide rendered geometry, icon/title alignment,
square icon tiles, upper-right close hit-testing, unchanged close and changed
Escape confirmation. Parent-provided isolated configuration protects user state.

The maintained Win32 capture utility successfully captured the current real
Pealayer window. Desktop automation capture failed with `0x8007041D`; opening
the updated cue dialog for its specific live screenshot remains a human-assisted
check. A renderer test is not a screenshot or human visual acceptance.

Cafe player delivery remains blocked by the previously denied runtime smoke
launch. Source synchronization and local package tests do not establish Cafe
runtime delivery or physical board acceptance. Do not bypass the denial, build
Rust on Cafe, or substitute another host's libmpv runtime.
