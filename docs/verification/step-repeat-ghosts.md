# Step repeats and designer ghost instances

Step repetition and whole-effect repetition are independent current-contract
features. This supersedes the replacement/removal decision recorded in
[the earlier designer pass](effects-designer-gestures.md).

Each step optionally owns `repeat_count` (once when omitted, 1–1000 when set)
and `repeat_interval_ms` (start-to-start, 0–3600000). Zero interval follows its
shared length, or 1ms for an instant command. Independent steps may interleave
or overlap; command releases retain stable authored ordering. Whole-effect
repetition uses the full span including all step repeats. Its explicit interval
must cover that span. Both controller executors consume the same bounded,
expanded volatile plan; persisted definitions keep their editable repeat fields.
No firmware schema or board flash is required.

Native and both Web designer entry points draw dimmed repeated instances on
the source lane, including repetitions of the entire effect and nested repeats.
**Show repeat ghosts** is enabled by default and saved as a local designer view
preference (native UI persistence / browser storage). It does not alter the
effect's execution or published repeat parameters. Whole-effect copies have
dashed borders; step-only copies have dotted borders. Originals retain solid
borders and draw above overlapping ghosts. Clicking, keyboard
selection and context actions bind to the original step; copies never become
extra saved steps. Drag a step ghost body/leading edge to change its step interval;
drag a whole-effect ghost to change the whole-effect interval. Movement of later
copies is divided by their repeat index. Whole-effect intervals are bounded by
the full sequence span and controller timestamp range. Arrow keys on a focused
ghost nudge its interval; its context menu exposes repeat count and interval.
Drag its
trailing resize handle to change the shared step length. Native spacing respects
the selected snap grid. Moving an original moves its entire repetition schedule.
Ghost edits never retarget a channel, seek media, play a preview or issue an
output command. Explicit Duplicate/Delete operate on selected sources once.
The count includes the original occurrence.

## Using the projections

1. Enable **Repeat step** in a step's properties, **Repeat effect** for the
   complete sequence, or both. **Show repeat ghosts** starts enabled; its saved
   view choice never changes the execution schedule.
2. Click an original or any ghost: every visible occurrence of that source
   shares selection. Delete and Duplicate operate on that source only once.
3. Drag a dotted step ghost to adjust the step's start-to-start interval. Drag
   a dashed whole-effect ghost to adjust the complete sequence's interval.
   Later copies divide pointer movement by their repetition index. Nested
   whole-effect ghosts edit the whole-effect interval, not the inner loop.
4. Drag the trailing edge to change the shared source length. Use arrow keys
   to nudge spacing, or right-click a ghost to edit its repeat count/interval.
   Double-click opens its original properties, never a detached copy.
5. Press **Escape** during a projection drag to cancel it. Native restores the
   source and effect interval captured at drag start; Web discards its preview.
   Releasing the pointer afterward cannot recommit the cancelled change.

### Follow-up verification

Native light/dark pointer tests cover selecting and moving step/whole-effect
projections and cancelling an actual changed drag. The disconnected browser
fixture covers default/saved visibility, both selection directions, one-source
Delete, interval dragging and arrow keys, shared-length resizing, Escape, count
menus and dotted/dashed styling at 390, 768 and 1440 pixels in both themes.
These exercise the real shared `SequenceTimeline` used by both Web designers.
No previews, publishing, hardware outputs or media seeking are involved.

Cafe deployment is deferred at the user's request while it is unavailable.
Local source, fixture and package verification are not installed Cafe acceptance.

Preferences → Hardware → Effect designer colors exposes background colors for
motion, relays, PWM, buzzer, status RGB, strip pixels, RF, displays, front-panel
actions and raw commands. The canonical palette asset supplies native/Web
defaults. Preferences use the existing config API and live state publication;
source and all its projections keep the same type color. Text contrast is chosen
from the configured background. Selection is an outline, not a replacement type
color. Reset uses the existing Preferences control reset operation.

Repeat previews are viewport-bounded, retaining an actively dragged instance
outside the viewport. Web cancellation, lost capture, an external draft
replacement or disabled authoring discards the preview. The source remains
unchanged until pointer release. English/Persian repeat controls are shared by
both Web entry points; native properties expose the same values.

Focused regressions cover exact microsecond spans, automatic intervals, nested
loops, interval/length edits, source binding, stable stop-event ordering,
overlap, timestamp/count/capacity limits and light/dark native pointer selection.
Run `web_ui/scripts/test-cue-authoring.mjs` and native Effects Library/controller
contract tests; the paired controller owns its stable-path Go suites.

Private existing effects are not automatically folded or decompiled into loops.
Preserve previously reauthored explicit schedules and any current working draft.
Source tests and non-actuating save/catalog checks are not human motion, firmware
timing or sustained output acceptance. Cafe Pealayer deployment retains its
existing smoke-launch-denial boundary; a local package is not Cafe delivery.
