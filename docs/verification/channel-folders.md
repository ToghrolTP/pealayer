# Hardware channel folders

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Controller contract: PCController `docs/Channel-Folders.md`.

## Delivered behavior

Motion / seat controls, Relay outputs and PWM outputs have section context menus,
even while collapsed. Manage channels, New folder, Manage folders and expand /
collapse actions are available. Native and Web folders can be created empty,
renamed, given an icon and deleted without deleting their channels. Folder deletion
asks for confirmation and moves all members, including hidden members, to Ungrouped.

Channel context menus offer Move to folder. Channel grips can drop onto same-section
folder headings (including empty folders and Ungrouped), with accent feedback.
Dropping on another operator card changes its folder and kind-local rank in one
presentation transaction. A wrong-kind or raw-seat-wiring drop cannot change an
operator folder. Raw relays is a fixed diagnostic heading for controller-declared
R1–R4 seat wiring; R5–R8 are not implicitly members.

The previous renderer put ungrouped cards immediately after a named folder without
another heading. This made unrelated MOSFET channels appear inside Cinema lighting.
A fresh Cafe catalog assigned only `pwm.0` there, so no bulk reassignment was needed:
the repaired rendering gives other channels an explicit Ungrouped heading. Output
type, channel name and relay number alone do not infer folder membership.

Folders persist in the controller's board profile, not a separate player store.
Revision-checked transactions reject stale edits; clients apply the complete
authoritative response before accepting its new revision. Empty folders remain
available after a move and have no empty expansion chevron. Native failed saves
retain draft values; Web completion waits for the corresponding host operation
sequence/acknowledgement rather than assuming queued means saved. Board changes
discard an editor for the previous board.

## Verification and boundaries

- Stable-path Go appconfig/ipcjson folder/profile/presentation/capability tests pass.
- Isolated shared-cache native `cargo test --lib --locked --jobs 1 hardware_ --
  --test-threads=1`: 48 passed. This includes real collapsed-header context menus,
  folder-drop release/invalid-kind checks, editor failure/success/board-change
  lifecycle, exact membership and hidden/empty-folder partitioning. Existing
  hidden-disclosure geometry and card/manager/drag regressions remain green.
- Web TypeScript build, source parity/responsive guardrails, PWA and asset validation
  pass. Executed pure TypeScript folder tests check exact R1–R4 membership, ordinary
  R1 rejection, lighting membership, hidden channels, section scoping and empty
  Ungrouped targets. These are not a claimed browser gesture or native screenshot.
- No folder operation energizes hardware or changes EEPROM, firmware, cues or media.
  Human native/Web gesture acceptance, sustained playback and physical timing
  acceptance remain separate gates.

Deployment receipt will be recorded here after verified coordinator-owned update
of both host applications, retaining private settings, exact media and cues.
