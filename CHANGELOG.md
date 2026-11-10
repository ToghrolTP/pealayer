# 📝 Pealayer changelog

Versioned history replaces the former single unreleased list. [Read the chronological release index](docs/releases/README.md) from oldest to newest; newest entries appear first here. The alpha suffix explicitly preserves development/acceptance status.

## 🚧 Unreleased

- Command palette/toolbar sizing, disabled paint and drag-to-hide completion: [#277](https://github.com/ToghrolTP/pealayer/pull/277).
- Proportional/tabular FPS digit alignment: [#278](https://github.com/ToghrolTP/pealayer/pull/278).
- Full projects/scenes: [#226](https://github.com/ToghrolTP/pealayer/issues/226), draft [#227](https://github.com/ToghrolTP/pealayer/pull/227).
- Optional FilmCraft monitor and timeline styling, with the Compact and Standard options retained ([#248](https://github.com/ToghrolTP/pealayer/pull/248)).

### 🎚️ Playback controls

### ⬇️ Download Center

- Browse a searchable queue with clear transfer states, speed history, and
  separate settings. The standalone window follows Pealayer’s saved theme,
  accent, and control density.
- Find aria2 by executable name or path and inspect the resolved location
  before connecting to its local RPC service.

- The video context menu offers a horizontal playback-speed slider with
  nine presets and normal speed at the center.

### 🎬 Subtitle timeline

- Optionally show timed subtitle cues as separate segments, with the gaps
  between them visible. Toggle this from the subtitle track menu or Playback
  preferences.

### 🗂️ Remote locations

- Browse files in a saved List or Grid layout, with consistent hover, selection
  and keyboard-focus styling that follows the application's palette.
- See when thumbnails are loading, inspect failures and retry unavailable previews.
- Open item Properties from the context menu; long filenames stay bounded in
  menus and wrap in the details dialog. Search now shares the corrected icon
  picker header, with its magnifier inside the field and view controls aligned.

### 🖱️ Contextual mouse controls

- Configure Back/Forward buttons separately for video, timelines, effects and
  remote folders, including Ctrl/Command, Alt and Shift variants.
- Middle-click video defaults to fullscreen; use configurable modifier drags
  for seeking or native window movement. A right-click opens the menu, while
  right-drag seeks; Shift+right always opens the menu.
- Remote folders gain shared Back/Forward history that advances only when the
  destination loads successfully. Failed navigation keeps the last good view.

## 🧰 v0.11.0-alpha.1 — Command tools, remote folders and sidebar recovery

Source checkpoint: **2026-11-10**. The standard package gains optional command tools and a real dockable Terminal, while native FPS sizing, remote folder encoding and remembered sidebar placement are repaired.

- ✨ **[Added]** Grouped command toolbar, trailing overflow/context menus, reorderable editor and Ctrl+Shift+P command palette ([#273](https://github.com/ToghrolTP/pealayer/pull/273)).
- 🛠️ **[Fixed]** Center actual CraftUI tab icon/title/close ink rather than the mixed-font line box ([#271](https://github.com/ToghrolTP/pealayer/pull/271)).
- 🧪 **[Experimental]** Opening a pane or prompt executes nothing; owned terminal sessions retain bounded output, cancellation and local-IPC fencing. Linux/macOS discovery, full-screen TUI and physical acceptance remain separate ([#272](https://github.com/ToghrolTP/pealayer/issues/272)).
- 🐞 **[Known issue]** The initial palette can reserve excessive empty height; toolbar icons/disabled paint and drag-to-hide still need [#277](https://github.com/ToghrolTP/pealayer/pull/277). The remaining FPS icon/digit-center discrepancy is addressed by [#278](https://github.com/ToghrolTP/pealayer/pull/278), outside this release.

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.11.0-alpha.1.md)

## ▶️ v0.10.0-alpha.1 — Playback recovery and native media sessions

Source checkpoint: **2026-11-09**. The accumulated native/timeline/media integration strengthens frame publication, external-player recovery, workspace controls, safe imports and factual OS now-playing state.

- ✨ **[Added]** Editable repeat projections with correct cancellation, reviewed external asset/timeline imports and contextual disconnected Hardware Monitor behavior ([#232](https://github.com/ToghrolTP/pealayer/pull/232), [#233](https://github.com/ToghrolTP/pealayer/pull/233), [#234](https://github.com/ToghrolTP/pealayer/pull/234), [#235](https://github.com/ToghrolTP/pealayer/pull/235)).
- 🛠️ **[Fixed]** Retain last published video frame across failed render/resize; remove Preferences per-frame filesystem/translation work ([#247](https://github.com/ToghrolTP/pealayer/pull/247), [#246](https://github.com/ToghrolTP/pealayer/pull/246)).
- 🧪 **[Experimental]** The consolidation preserves all contributing heads; [#268](https://github.com/ToghrolTP/pealayer/pull/268) and [#270](https://github.com/ToghrolTP/pealayer/pull/270) are integration owners, while the individual PR list below records the included histories.
- 🐞 **[Known issue]** The exact post-merge run fails a Windows remote-parent HTTP fixture with WouldBlock (error 10035), and Intel macOS is cancelled. Linux and Apple-silicon packages passed. This does not negate earlier PR evidence, but the exact merge matrix is not green.

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.10.0-alpha.1.md)

## 🗂️ v0.9.0-alpha.1 — Persistent cues and CraftUI workspaces

Source checkpoint: **2026-11-05**. Persistent authored cues and guarded playback clocks meet palette-aware CraftUI docking, property rails, native typography and a restored egui Dock alternative.

- ✨ **[Added]** Persist authored cues, templates, analog/keyframe tracks and mute/solo/lock state; Save shortcuts/menu, dirty baselines and non-destructive unsaved-change prompts ([#224](https://github.com/ToghrolTP/pealayer/pull/224)).
- 🛠️ **[Fixed]** Keep a fresh monotonic decoder-clock anchor; bind hardware admission to actual paused/fresh state and current Play/cancel intent, retaining the 50 ms / 250 ms limits ([#217](https://github.com/ToghrolTP/pealayer/pull/217)).
- 🧪 **[Experimental]** Cue/track persistence is implemented. Full project restore, scene snapshot execution and associated-media lifecycle remain in open draft [#227](https://github.com/ToghrolTP/pealayer/pull/227) / issue [#226](https://github.com/ToghrolTP/pealayer/issues/226); they are not shipped by this tag.
- 🐞 **[Known issue]** Native minimized-window placement, Task View/Peek and repeated dynamic taskbar artwork still need the later Windows work ([#239](https://github.com/ToghrolTP/pealayer/pull/239), [#245](https://github.com/ToghrolTP/pealayer/pull/245)).

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.9.0-alpha.1.md)

## 🧪 v0.8.0-alpha.1 — Virtual Board Demo and canonical opcodes

Source checkpoint: **2026-10-30**. A selectable Virtual Board Demo makes the 4D workflow explorable without physical hardware, alongside actionable errors and canonical controller opcode admission.

- ✨ **[Added]** Explicit Virtual Board Demo with supervised virtual-board/coordinator processes, temporary isolated settings, a showcase timeline and lifecycle cleanup ([#207](https://github.com/ToghrolTP/pealayer/pull/207)).
- 🛠️ **[Fixed]** Use the pinned PCController-owned opcode registry and reject foreign namespaces instead of guessing numeric commands ([#223](https://github.com/ToghrolTP/pealayer/pull/223)).
- 🧪 **[Experimental]** The demo executes a simulation and depends on the companion PCController/virtual-board tools. It does not prove physical relay, motor, RF, sensor or edge timing acceptance ([#207](https://github.com/ToghrolTP/pealayer/pull/207), [#80](https://github.com/ToghrolTP/pealayer/issues/80)).
- 🐞 **[Known issue]** This build predates cue/template/track-state dirty tracking and unsaved-change protection from [#224](https://github.com/ToghrolTP/pealayer/pull/224); do not treat its earlier sidecar support as complete project persistence.

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.8.0-alpha.1.md)

## 📡 v0.7.0-alpha.1 — RF learning, PWM controls and Wayland recovery

Source checkpoint: **2026-10-15**. RF learning/assignment and MOSFET controls become more usable and truthful; hardware-free development improves and Wayland focus/occlusion stops blocking the application.

- ✨ **[Added]** RF reception indicators and modal-opening counters for complete waveform identities, including unassigned/unlearned observations; shared timed/continuous learning preferences ([#194](https://github.com/ToghrolTP/pealayer/pull/194), [#200](https://github.com/ToghrolTP/pealayer/pull/200)).
- 🛠️ **[Fixed]** Restore short controller deadlines after preparation failures, retain authoritative live values and avoid redundant paused-video work ([#176](https://github.com/ToghrolTP/pealayer/pull/176), [#178](https://github.com/ToghrolTP/pealayer/pull/178), [#180](https://github.com/ToghrolTP/pealayer/pull/180)).
- 🧪 **[Experimental]** Hardware-free authoring is now testable against the shared native firmware; the selectable supervised Virtual Board Demo comes in v0.8.0-alpha.1 ([#207](https://github.com/ToghrolTP/pealayer/pull/207)).
- 🐞 **[Known issue]** Hardware timing, sustained rapid-motion/idle CPU, real RF/strip behavior and full native/Web parity still require acceptance ([#61](https://github.com/ToghrolTP/pealayer/issues/61), [#62](https://github.com/ToghrolTP/pealayer/issues/62), [#63](https://github.com/ToghrolTP/pealayer/issues/63), [#80](https://github.com/ToghrolTP/pealayer/issues/80)).

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.7.0-alpha.1.md)

## 🎛️ v0.6.0-alpha.1 — Effects Designer and customizable live status

Source checkpoint: **2026-10-12**. Effects Designer gains repeat-aware gestures and capture selection, while status widgets, contextual tray/media controls and hardware preparation become more capable.

- ✨ **[Added]** Ctrl-drag copies, compatible seat-lane moves, scoped bulk edits, finite whole-effect repeats, exact timing and advertised capture channels/opcodes; Publish & Run waits for acknowledgement ([#142](https://github.com/ToghrolTP/pealayer/pull/142)).
- 🛠️ **[Fixed]** Keep configuration I/O and routine metadata off hardware clock streams; expose bounded timing/coordinator diagnostics ([#132](https://github.com/ToghrolTP/pealayer/pull/132), [#133](https://github.com/ToghrolTP/pealayer/pull/133), [#136](https://github.com/ToghrolTP/pealayer/pull/136), [#137](https://github.com/ToghrolTP/pealayer/pull/137), [#138](https://github.com/ToghrolTP/pealayer/pull/138)).
- 🧪 **[Experimental]** Step repeats return here, then gain interactive projected-occurrence editing/cancellation in v0.10.0-alpha.1 ([#232](https://github.com/ToghrolTP/pealayer/pull/232), [#235](https://github.com/ToghrolTP/pealayer/pull/235)). Recording execution/repeat policy remains separately tracked by [#114](https://github.com/ToghrolTP/pealayer/issues/114) and [#141](https://github.com/ToghrolTP/pealayer/issues/141).
- 🐞 **[Known issue]** The exact checkpoint's CI failed Windows native tests; Apple-silicon macOS failed while downloading the Web artifact. Only Linux and Intel macOS packages are recovered from that run. These are recorded failures, not a green aggregate.

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.6.0-alpha.1.md)

## 🎧 v0.5.0-alpha.1 — Audio, external mpv and authoring workspace

Source checkpoint: **2026-10-10**. The versioned home of the previously date-named October 10 comparison: sound effects, independent audio outputs, external mpv, better source browsing and a much fuller authoring workspace.

- ✨ **[Added]** Import, preview and schedule sound effects with main/SFX/per-effect audio-output discovery and routing ([#104](https://github.com/ToghrolTP/pealayer/pull/104)).
- 🛠️ **[Fixed]** Hidden-window command processing, updater/lifecycle routing and Preferences organization ([#87](https://github.com/ToghrolTP/pealayer/pull/87)); proper contextual transport/volume in narrower NLE panels ([#95](https://github.com/ToghrolTP/pealayer/pull/95), [#96](https://github.com/ToghrolTP/pealayer/pull/96), [#124](https://github.com/ToghrolTP/pealayer/pull/124)).
- 🧪 **[Experimental]** SFX now has a real implementation, superseding the v0.4.0-alpha.1 scaffold; human listening and physical output routing remain open ([#104](https://github.com/ToghrolTP/pealayer/pull/104), [#80](https://github.com/ToghrolTP/pealayer/issues/80)).
- 🐞 **[Known issue]** Prepared hardware playback can be starved by configuration I/O and metadata work; v0.6.0-alpha.1 removes that work and repairs pending/empty-plan recovery ([#132](https://github.com/ToghrolTP/pealayer/pull/132), [#133](https://github.com/ToghrolTP/pealayer/pull/133), [#156](https://github.com/ToghrolTP/pealayer/pull/156)).

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.5.0-alpha.1.md)

## 🌐 v0.4.0-alpha.1 — Web Studio and live effects recording

Source checkpoint: **2026-10-08**. The integrated hardware workspace and Web Studio add live effect recording, controller-owned catalogs, palettes, remote authority and richer timeline interaction.

- ✨ **[Added]** Responsive native/Web hardware workspace, channel manager, front-panel and board settings, RF/melody/strip controls and shared appearance/preferences ([#46](https://github.com/ToghrolTP/pealayer/pull/46)).
- 🛠️ **[Fixed]** Finish now consumes the controller's 'effect … saved' response, selects the saved sequence and refreshes recording state from push events; stale unpublished drafts get a useful error ([#58](https://github.com/ToghrolTP/pealayer/pull/58)).
- 🧪 **[Experimental]** Web Studio is implemented, but [#63](https://github.com/ToghrolTP/pealayer/issues/63) still tracks full parity. A working recorder is not proof of physical motion timing, speaker routing or sustained load acceptance ([#80](https://github.com/ToghrolTP/pealayer/issues/80), [#61](https://github.com/ToghrolTP/pealayer/issues/61)).
- 🐞 **[Known issue]** The exact main checkpoint's aggregate CI was cancelled after Linux, Windows and Intel macOS succeeded; Apple-silicon macOS did not finish in that run.

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.4.0-alpha.1.md)

## 🔌 v0.3.0-alpha.1 — Capability workspace and unified control port

Source checkpoint: **2026-10-01**. A capability-led hardware workspace, saved dock layouts, live/remote media and a single control port replace the earlier fragmented workflow.

- ✨ **[Added]** Contextual live hardware cards, board identity, configurable status visibility, user-managed addressable effects and cross-platform vector icons ([#45](https://github.com/ToghrolTP/pealayer/pull/45)).
- 🛠️ **[Fixed]** Preserve the complete Windows config while loading isolated test profiles; retain live semantic controls separately from raw relay state ([#47](https://github.com/ToghrolTP/pealayer/pull/47), [#45](https://github.com/ToghrolTP/pealayer/pull/45)).
- 🧪 **[Experimental]** The first capability workspace is subsequently expanded by the large [#46](https://github.com/ToghrolTP/pealayer/pull/46) integration in v0.4.0-alpha.1. Later screenshots of that integration do not depict this tag.
- 🐞 **[Known issue]** The original aggregate CI run was cancelled: Linux, Windows and Intel macOS jobs succeeded, but Apple-silicon macOS was cancelled. A cancelled matrix is not a four-platform pass.

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.3.0-alpha.1.md)

## 🎬 v0.2.0 — Native shell and semantic hardware authoring

Source checkpoint: **2026-09-30**. Native Windows shell integration, persistent portable/system settings and semantic PCController authoring, with the first packages for both macOS architectures.

- ✨ **[Added]** Taskbar media buttons, video thumbnails, progress/error state, system tray controls and branded native GUI ownership ([#25](https://github.com/ToghrolTP/pealayer/pull/25), [#35](https://github.com/ToghrolTP/pealayer/pull/35)).
- 🛠️ **[Fixed]** Slider snap-backs, scrub-time labels, delay bounds and audio/subtitle menus ([#5](https://github.com/ToghrolTP/pealayer/pull/5), 2912d77); smoother seeking/frame previews, control fade and seekbar/mute layout (b080831, f49cb88).
- 🧪 **[Experimental]** Semantic hardware editing and macro placements are early implementations; capability-led workspace/port unification follows in v0.3.0-alpha.1 ([#45](https://github.com/ToghrolTP/pealayer/pull/45), [#50](https://github.com/ToghrolTP/pealayer/pull/50)).
- 🐞 **[Known issue]** Professional responsive hardware layout, durable pane restoration and live/buffered remote-media work are still pending at this tag; see v0.3.0-alpha.1 ([#45](https://github.com/ToghrolTP/pealayer/pull/45), [#47](https://github.com/ToghrolTP/pealayer/pull/47), [#49](https://github.com/ToghrolTP/pealayer/pull/49)).

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.2.0.md)

## 🌱 v0.1.0 — First public cross-platform packages

Source checkpoint: **2026-09-24**. The first public packages brought together the June player, docked 4D timeline, July remote controls and September binary hardware/curve-authoring work, then added the cross-platform build foundation.

- ✨ **[Added]** Native Rust/libmpv playback; a 4D editor with F1–F8 macro recording, relay indicators, timecode validation and template creation (June 4–15).
- 🛠️ **[Fixed]** High-DPI video placement ([#3](https://github.com/ToghrolTP/pealayer/issues/3); e9c391c), render/input lock re-entry (e722507, 2a474c1), duplicate Web seek dispatch and seek-drag notification flooding (eddaa15, bd7e247).
- 🧪 **[Experimental]** Early 4D execution and direct serial control were development groundwork. Later semantic PCController authoring arrives in v0.2.0; live recording is repaired in v0.4.0-alpha.1 ([#58](https://github.com/ToghrolTP/pealayer/pull/58)).
- 🐞 **[Known issue]** Slider snap-backs, scrub labels, incomplete audio/subtitle menus and control fade/layout problems still need the fixes later shipped in v0.2.0 ([#5](https://github.com/ToghrolTP/pealayer/pull/5), [#2](https://github.com/ToghrolTP/pealayer/issues/2)).

[📖 Complete notes, screenshots, packages, known problems, PRs and issues](docs/releases/v0.1.0.md)
