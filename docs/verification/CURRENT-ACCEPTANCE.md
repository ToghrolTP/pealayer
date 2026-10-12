# Current Pealayer acceptance and next steps

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Web counterpart: [issue #63](https://github.com/ToghrolTP/pealayer/issues/63)
and [product ledger](../WEB-APPLICATION-PARITY.md).

This is the current index. Older handoffs and installed-build receipts remain
historical evidence; their old open-PR, missing-feature or disconnected-runtime
statements are not the present product state.

## Delivered source and deployment

- Hardware preparation recovery handles acknowledged empty plans, preserves new
  errors against old clock ACKs and pauses the coordinator before replacement.
  [Transition contract and regressions](hardware-preparation-recovery.md) retain
  real timing faults, strict deadlines and the physical/Cafe acceptance gates.

- Windows tray menus expose a capability-driven PCController board submenu,
  with monitor reveal, guarded E-STOP release and tracked stop/clear/automatic
  LED actions. [Tray contract and verification](windows-tray-board-actions.md)
  retain source-switch, physical-click and Cafe runtime boundaries.

- Modal corner-close controls share a fixed square with a larger, ink-centered ×.
  Unsaved Preferences/cue confirmations retain non-destructive dismissal.
  [Geometry contract and native fixture](square-modal-close.md) prevent regressions
  from text padding, font metrics or display scaling.

- Publishing-authority prompts share a non-mutating Dismiss action, corner close
  and Escape behavior on native/Web. [Dismissal semantics and regression](authority-modal-dismissal.md)
  distinguish dismissal from handoff rejection and retain the live/Cafe boundaries.

- Program Monitor reserves the automatic video/footer spacing and behaves as a
  fitted viewport rather than a scrollable document. [Viewport contract and
  regression](program-monitor-viewport.md) covers panel sizes and display scales;
  Cafe runtime delivery remains separately blocked.

- Status widgets share independent context menus and drag ordering, three FPS
  sources, compact telemetry, an optional bounded RAM chart, real operation
  progress and authoritative HOT alerts. The Effects Library heading is start
  aligned. [Widget contract and evidence](status-bar-widgets.md) retains the Cafe
  runtime blocker and physical/human acceptance boundaries.
  Peer RGB-only snapshot updates now invalidate the native hardware cache before
  repaint; paused rendering and native/Web projections have focused regressions.

- Native cue properties now has guarded ×/Escape dismissal, square icon controls,
  centered caption groups and a viewport-bounded two-column layout on wider
  screens. [Dialog contract and checks](cue-dialog-responsive-dismissal.md)
  separates rendered tests from the human-assisted live-dialog screenshot gate.

- Designer UI consistency is implemented across native/Web authoring: caption
  metrics, preference-governed fades, acknowledged draft identities, a shared
  Custom color popup, contextual actions, Technical disclosure and lane-targeted
  step insertion. [Contract and verification](designer-interaction-consistency.md)
  keeps live UI/source proof separate from Cafe runtime and physical acceptance.

- Routine dev/test builds suppress symbols and incremental snapshots while
  retaining the shared dependency cache. Debug-only cleanup verifies cache,
  path/link and active-writer safety before deletion; no runtime rebuild or
  replacement is needed for this build-policy change.
  [Low-write workflow](LOCAL-WORKSPACE-HYGIENE.md) retains diagnostic opt-ins and
  measured RAM-disk sizing constraints.
- Individual step repeats coexist with whole-effect repeats. Native/Web designer
  ghosts select their source and expose interval/length manipulation without
  creating persisted duplicate cues: [step repeat contract](step-repeat-ghosts.md).
  Installed delivery and physical acceptance are distinct from source coverage.
- Erfan's current-source package is installed and reopened in its signed-in NLE
  workspace with Cafe's real board connected while paused; original cues and
  private work are preserved. Publishing ownership changed after a repeated clean
  exit, and a metadata timeout needs resolution before hardware preview readiness.
  [Authoring readiness](ERFAN-AUTHORING-READINESS.md)
  records the current-source runtime/ACK evidence and a genuine native-window
  capture. This is editing readiness, not full interactive or physical acceptance.
- PRs #77 and #79 and CPU fixes #78/#81/#82 are merged. Subsequent feature and
  recovery PRs through #108 are integrated; none should be fetched as a stale
  replacement for current main.
- SFX loading, scheduled playback and audio routing are implemented, not merely
  a preference scaffold: [SFX acceptance](SFX-AUDIO-DELIVERY.md).
- Clipboard inspection/source browsing and event-driven external mpv modes are
  implemented: [browsing](../remote-folders.md) and
  [external player acceptance](EXTERNAL-MPV-DELIVERY.md).
- Preferences, live branding, responsive volume, elapsed entry, buzzer catalog,
  Effect Controls and palette-safe widgets have focused delivery records.
  [Widget delivery](PALETTE-SAFE-ELEGANCE.md) and the subsequent focused records
  below retain exact source/runtime identities, not product summaries.
- The production controller startup service exists. The 2026-10-10 folder update
  preserved the verified interactive primary on port 8787 and SCM Stopped/Auto;
  do not infer a running service or reboot acceptance from application health.
  The live player remained paused, connected and without a hardware error.
  This is not a CPU soak or physical-output timing acceptance test.
- Native NLE transport now uses contextual enabled/disabled styling, a square
  Stop, and intrinsic-width responsive rows with trailing track/volume controls:
  [transport verification](nle-transport.md).
- Native timeline actions use accent-matched outlines, mouse-down-preserving
  drag offsets and reversible removal previews over Trash:
  [toolbar verification](timeline-toolbar-drag-feedback.md).
- Effect Controls can show its selected cue in Timeline without seeking or
  changing hardware track state: [inspector verification](EFFECT-CONTROLS-INSPECTOR.md).
- Hardware Monitor's hidden-channel menu uses a content-height, right-aligned
  row without a viewport-sized gap: [disclosure verification](hardware-hidden-disclosure.md).
- Hardware section context menus, persistent channel folders, explicit Ungrouped
  and same-section folder drops are implemented together with the controller API:
  [folder verification](channel-folders.md).
- Effects Library and Hardware Monitor share a compact non-overlapping scrollbar
  lane. Library action icons and folder-add buttons reveal on hover or keyboard
  focus without shifting geometry: [library verification](effects-library-hover-spacing.md).
- Native dropdowns share keyboard choice navigation and stable full-width rows;
  closed color indicators and the unified Custom picker retain app/session
  swatches: [dropdown verification](dropdown-navigation-colors.md).
- Seat titles in hardware descriptions, NLE tracks and native/Web Effects
  Designer use current controller presentation names while preserving stable
  keys and action IDs: [seat-title verification](renamed-seat-titles.md).
  Cafe installed acceptance remains blocked by the recorded launch denial.

## Remaining gates · do not silently mark complete

- [ ] Complete paired Effects Designer/capture deployment and installed acceptance.
  [PR #142](https://github.com/ToghrolTP/pealayer/pull/142) has verified source
  implementation and [PCController PR #634](https://github.com/atomicdeploy/PCController/pull/634)
  is merged; see [source and delivery gates](effects-designer-gestures.md).
  PR #142 and its integrated main have passed platform CI. Preserve exact persisted
  per-step repetition schedules across paired contract deployment; the restored
  step-repeat feature must not automatically decompile explicitly reauthored work.
  Cafe's recorded smoke-launch tool denial remains a deployment blocker unless
  explicitly cleared; its staged package is not delivery evidence. The controller
  Web/TUI chooser and autonomous command-tail gates remain in its owning tracker.
- [ ] Independently exercise elapsed-entry cancel/blur/Enter, Browse, toolbar
  drag/hide, Add cue backdrop and native Appearance/Audio/Subtitle visuals.
- [ ] Verify phone/tablet/desktop Web gestures, dialogs and interactions against
  the corresponding native features; complete the missing rows in the parity
  ledger, including negotiated low-latency video fallback.
- [ ] Confirm SFX audibility, output routing/hotplug and interactive editor use.
  Real decoder/EOF/seek checks do not prove which physical speaker produced sound.
- [x] Installed-runtime service restart/reconnect and prepared timeline re-arming
  passed with the same Pealayer process, paused position and current cues retained.
  See [cue/startup acceptance](CUE-STARTUP-ACCEPTANCE.md).
- [ ] Complete safe authorized physical cue testing and sustained timing/load
  checks. VirtualBoard ACK timing is not physical edge measurement. Only the
  currently authorized R5–R7 may be used; no seat or R8 test is implied.
  A later explicit, bounded Seat Left Down authorization was exercised in the
  [2026-10-11 timing tests](seat-cue-timing.md): the earlier attempt failed closed;
  the subsequent CI1459 test ACKed Down and STOP without a timing fault.
  Human-observed movement/STOP and sustained timing acceptance remain pending.
  That receipt is not broader seat/relay permission or completed acceptance.
- [ ] Exercise recording, offline editing, RF assignments and strip controls
  end to end with real advertised hardware and retain explicit human acceptance.
  [Behavior-based cue duration](../HARDWARE-CUE-MODES.md) corrects strip windows
  and finite recordings; explicit recording repeat-window/count execution is
  unfinished and tracked in [#114](https://github.com/ToghrolTP/pealayer/issues/114).
- [ ] Recheck idle/playing/taskbar-preview CPU and responsiveness for both
  applications in two separated live samples. Keep #62 open until its gates pass.
- [ ] Resolve remaining GPU/native-surface and exact-navigation gates in
  #64–#66; experimental D3D11 must remain optional/default-off until accepted.
- [ ] Clear applicable platform CI/runtime/package checks and privacy-safe
  screenshots before selecting the next release version and publishing packages.

## Successor instructions

Inspect current main, linked issues and fresh process-local APIs before editing.
Coordinate controller/firmware and shared worktree/cache/updater ownership.
Implement a focused current-contract slice, retain exact test limitations, build
on CI or the faster authorized host, and deploy through the product updater.
Never compile/link Rust on the production host or replace its validated libmpv
with a different host's DLL. Reverify health, actual playback, controller state
and error/CPU evidence after delivery; preserve private settings and original cues.

For genuine human intervention, use an advertised suitable buzzer melody and
a large Persian message through the existing APIs. Attention must be bounded,
acknowledgement-aware and stopped when answered; never use motion/relays as an
alert or leave an unbounded sound loop. No alert is needed for routine progress.

Update human-facing issues with outcomes, remaining gates and useful links;
keep hashes, host identities and measured timestamps in technical evidence only.
Maintain [the changelog](../../CHANGELOG.md), curated version notes and genuine
screenshot evolution. Publish a new milestone only when its stated scope is
stable and verified; do not claim that every parity/backlog item is finished.
