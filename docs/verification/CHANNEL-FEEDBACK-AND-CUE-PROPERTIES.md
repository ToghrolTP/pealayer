# Channel feedback and cue editing checkpoint

## User requests and implementation

| Request | Change and cause |
| --- | --- |
| Orange/Blue motion feedback | Rust publishes the native computed indicator color and semantic direction to Web. The Web indicator previously ignored both and always used green. Native unknown motion now stays unknown instead of being incorrectly reported OFF. `seat.a` / `seat.b` select the authoritative PCController motion sides without interpreting captions or raw relay edges. Up defaults to Orange; Down to Blue; per-channel overrides remain supported. |
| Keyframe insertion moves the head | Insertion already has no seek command and the ruler captures context-menu time before opening. All playhead insertion entry points now read the visible logical seek target rather than the older decoded position during settlement. Regression assertions preserve both playback time and pending seek. No claim of live-host reproduction is made. |
| Insert first, then edit | Double-click / A validates the track and creates/selects one actual instance before opening Cue properties. The modal stores that UUID, not a second draft. Closing retains the cue; Delete removes it. |
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
- `src/four_d/engine.rs`: cross-hardware solo and direct PWM exclusions.
- `src/four_d/media_timeline.rs`: explicit zero for suppressed prepared PWM.
- `src/ui/effects_library.rs`: fixed effects color-option geometry.
- Web hardware, timeline and CSS: consume shared indicator fields, provide
  semantic fixed-size track actions and reserve readable header width.

The large layout diff is the deliberate relocation of the existing editor,
with `self.app` changed to the function's `app` argument. It is not a whole-file
formatting pass. Only that extracted function was formatted in isolation.

## Verification and delivery gates

TypeScript compilation, Vite production build and PWA generation passed.
Native release BuildOnly passed on the initial functional set; the final exact
head build result is recorded in the PR checkpoint. Regression sources were
updated/added but not executed here, following the user's request to defer tests.

No canonical executable, session, board output or installation was changed by
this worktree. The coordinated release owner must independently review/test,
merge, package with the correct host-specific libmpv DLLs and gracefully deploy
to Cafe and David. Installed acceptance remains pending: Orange/Blue feedback,
non-seeking keyframe insertion during decoder settlement and remote consumption,
one-instance cue editing, matching Library edges, hover stability and muted/solo
PWM output. Motion/seat actuation requires a safe explicit testing arrangement.

Web's existing direct-cue form remains its existing draft/save workflow in this
checkpoint; the insert-first shared dock/modal change is native. Full Web
cue-editor consolidation remains a parity follow-up, not a claimed completion.
