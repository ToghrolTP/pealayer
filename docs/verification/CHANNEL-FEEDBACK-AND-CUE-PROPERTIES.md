# Channel feedback and cue editing checkpoint

## User requests and implementation

| Request | Change and cause |
| --- | --- |
| Orange/Blue motion feedback | Rust publishes the native computed indicator color and semantic direction to Web. The Web indicator previously ignored both and always used green. Native unknown motion now stays unknown instead of being incorrectly reported OFF. `seat.a` / `seat.b` select the authoritative PCController motion sides without interpreting captions or raw relay edges. Up defaults to Orange; Down to Blue; per-channel overrides remain supported. |
| Keyframe insertion moves the head | Insertion already has no seek command and the ruler captures context-menu time before opening. All playhead insertion entry points now read the visible logical seek target rather than the older decoded position during settlement. Regression assertions preserve both playback time and pending seek. No claim of live-host reproduction is made. |
| Insert first, then edit | Double-click / A validates the track and creates/selects one actual instance before opening Cue properties. Preferences → Hardware → Timeline chooses a modal or the Effect Controls panel. In modal mode, **Done** retains a new cue while **Discard cue** or Escape restores the exact pre-insertion timeline and undo history. Panel mode retains the cue immediately. |
| Existing cue double-click | Double-clicking an existing cue uses the same preference: the shared properties modal opens, or Effect Controls is brought into view and focused. Existing-cue close never deletes the cue. |
| Clear modal dismissal | The ambiguous compact multiplication-sign close control is suppressed for Cue properties. Labeled footer actions make the destructive new-cue discard path explicit. |
| Semantic seat authoring | `seat.a` and `seat.b` now create finite Up/Down cues with a mandatory Stop edge. Live fallback and prepared playback retain PCController's semantic `motion` command instead of lowering seat movement to independent relay toggles. |
| One properties editor | The former `TabViewer::ui` EffectControls arm is moved, not replaced, into `draw_effect_controls(app, ui, show_header)`. The dock calls it with its header; the modal calls the identical renderer without a duplicate panel header. Timing, routing, value, behavior, duplication, deletion and undo/commit controls are preserved. |
| Effects Library trailing edges | The toolbar adds trailing actions first and truncates the heading inside the same width as search/groups/cards. Compact icon actions retain full tooltips. Previously the heading plus two full captions could exceed the allocated row width. |
| Stable, semantic Mute/Solo/Lock | Native already used fixed 22px squares and inside strokes. Web now exposes the same three actions with reserved transparent borders. Red marks Mute, amber Solo, neutral Lock; native/Web dim muted and solo-excluded tracks. Hardware solo applies across relay and PWM, not separate visual buses. |
| Track state actually affects PWM cues | Direct PWM cues previously bypassed analog mute/solo. They now use effective track flags in live and prepared playback. The prepared plan explicitly emits zero for suppressed PWM tracks so removing a cue does not leave its last value latched. Lock still protects editing; it does not mute playback. |
| Stable color choices | Native effects color rows paint inside fixed rectangles with a reserved swatch/text lane. Hover/selection cannot change padding, border width or text origin. The collapsed selection uses the same spacing. |

## Review map

- `src/ui/layout.rs`: shared editor extraction; insert-first modal; toolbar width;
  semantic track visuals; logical keyframe insertion; regression fixtures.
- `src/app.rs`: computed motion fields in Web snapshot; logical-head accessor;
  effective analog flags passed to live/prepared compilers; contract regression.
- `src/four_d/engine.rs`: cross-hardware solo, direct PWM exclusions, and
  semantic motion start/stop compilation.
- `src/four_d/media_timeline.rs`: explicit zero for suppressed prepared PWM
  and PCController-native prepared `motion` steps.
- `src/ui/effects_library.rs`: fixed effects color-option geometry.
- Web hardware, timeline and CSS: consume shared indicator fields, provide
  semantic fixed-size track actions and reserve readable header width.

The large layout diff is the deliberate relocation of the existing editor,
with `self.app` changed to the function's `app` argument. It is not a whole-file
formatting pass. Only that extracted function was formatted in isolation.

## Verification and delivery gates

TypeScript compilation, Vite production build, PWA verification, Rust typecheck,
all 721 Rust library cases (720 passed, one pre-existing ignored) and the 22-case
timeline clip-controls integration suite passed on David-PC. Packaging and
runtime deployment use the clean merged commit and each destination's retained
libmpv; no Rust compilation occurs on Cafe.

Cafe's live PCController presentation was also reconciled without output
activation: both `seat.a` and `seat.b` now advertise `Cinema motion`. The
`controller-effect:composite` track is hidden only in Cafe's stored timeline
view and remains linked. Media remained paused at 660.8 seconds and no motion
or seat output was actuated. Interactive installed acceptance of the actual
modal/panel gestures remains separate from these source and API checks.

Web's direct-cue form remains draft/save rather than insert-first, so Cancel has
no inserted cue to roll back. It now includes semantic seat tracks and exposes
Up/Down with a timed Stop contract. Native owns the configurable modal/panel
presentation because Effect Controls is a native workspace panel.
