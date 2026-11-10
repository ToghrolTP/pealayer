# What's changed in Pealayer

User-facing changes are grouped by what you can do, not by commit order.
Version notes describe the source at that release tag; later fixes are not
retroactively attributed to an older download.

## Next milestone · unreleased

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

### 🖥️ Windows taskbar and appearance

- Changing the application icon also updates Explorer's taskbar group artwork
  and matching pinned launcher, not just the titlebar and notification area.
  Repeated changes refresh the running taskbar button without changing its
  application identity or losing its playback actions.
- Closing while minimized no longer reopens a tiny window. Valid normal placement
  has one owner; maximized/fullscreen clients do not replace the Restore size.
- Task View and Peek receive a separate full-resolution application composite
  when Windows requests it, instead of being left black in custom-thumbnail mode.
- Video-only taskbar thumbnails follow new rendered frames instead of stopping
  when a short hover refresh timer expires. Paused frames remain unchanged;
  the full-window Peek preview is not replaced by the thumbnail bitmap.
- Click the control-density value capsule to advance through all five sizes,
  wrapping back to Extra compact. Native and Web controls retain stable layout,
  keyboard adjustment and disabled-state behavior.

### 📦 Updating Pealayer

- The command-line updater shows clear upload/download, verification, restart
  and acknowledged-completion phases, with transfer speed and ETA when known.
  Semantic terminal colors follow terminal support; redirected logs stay plain
  and bounded. Use the update command's new `--json` option for automation.

### 🪟 Workspace chrome

- egui Dock restores its compact header height, centers tab captions and close
  icons consistently, and clips scrolling tab controls correctly. Its close-all
  control has no stray left-hand rule. CraftUI keeps its existing styling.

### 🎛️ Hardware controls

- Hardware connection cards use a short, actionable explanation by default.
  Hover or click the circled information button for the original technical
  diagnostic and a copy action, in native and Web UI.

- PWM indicators and On/Off selection share confirmed power state. Unknown or
  unavailable samples stay neutral; queued requests no longer fake PWM status.

- RF transport errors show a concise translated timeout/connection headline;
  original diagnostics remain under Details. Connection-loss toasts and OSD
  stay actionable rather than displaying nested operating-system paragraphs.
- Secondary clicks over native and Web MOSFET sliders open the card menu without changing
  brightness or starting a drag. Primary dragging and keyboard edits remain available.

- Channel Settings exposes the coordinator's shared Full/Restore On policy and
  confirmed remembered brightness, with explicit Apply and live event updates.
  Unknown-memory fallback remains visible; absolute setters and
  existing autonomous RF/motion mappings are unchanged.

- MOSFET cards put distinct minus/plus actions beside the slider. Hardware
  preferences independently hide action buttons and choose Toggle/On/Off for
  the indicator; these dispatch shared coordinator actions, not guessed values.
- Start an RF assignment directly from a channel card's context menu or its
  detail page, choosing the learned signal and action before saving.
- RF assignment editors precede saved buttons; saved gesture rules group by the
  complete waveform and scope to the selected channel. PWM recall reports
  controller-confirmed profile persistence rather than assuming a restart reset.

### 🎬 Playback and audio

- Publish factual OS media metadata and contextual transport/timeline state;
  Windows supports speed requests and cached alpha artwork with complete
  callback cleanup, while Web clears stale live/idle session positions.

- Reach important connected-board controls from the Windows tray's PCController
  submenu, with contextual availability and the existing E-STOP release guard.

- Refresh physical status RGB and hardware readings in native peer clients when
  authoritative snapshots change, including while playback is paused.

- Use external mpv as the player, alongside a muted internal preview, or control
  an existing mpv through event-driven socket/named-pipe IPC.
- Import, preview and schedule sound effects independently of movie audio, with
  discovered main, SFX and per-sound audio output selection.
- Restore the last media, paused state and bounded saved playback positions.
- Use a fixed-format segmented elapsed-time editor, configurable scrub modes,
  chapter-aware seeking, playback speed, native media keys and an optional
  libmpv metadata inspector.
- Reach volume and mute controls in narrow NLE panels; transport actions now
  show semantic playback states without shifting their geometry.

### 🎞️ Timeline and effects

- Repeated steps and whole effects show linked, dimmed cue projections by
  default. Select any occurrence, drag spacing, resize its shared length or
  edit repeat counts from its menu. Escape cancels a projection drag without
  changing the source or closing its containing Web designer.
- Create direct hardware cues by track interaction or shortcut; distinguish
  move-only value markers and recorded clips from resizable holds and PWM ramps.
- Resize streamed lighting's active window without changing its cycle speed;
  keep finite recordings fixed-length and preserve independent cue durations
  when dropping another effect or refreshing the controller catalog.
- Record app/board activity into sequences, edit channel lanes, and place effects
  from the library without treating selection as seeking.
- Use Effects Designer's Ctrl-drag copies, compatible seat-lane moves and scoped
  bulk selection/duplicate/delete actions without changing other shared placements.
- Repeat individual steps alongside whole-effect loops. Dimmed timeline copies
  select the original step; drag them to adjust spacing or resize their shared
  length without adding duplicate saved cues.
- Author finite whole-effect repetition and Start/Length/Finish timing; select
  advertised capture channels/opcodes explicitly and Publish & Run after the
  controller acknowledges the definition. Paired installed acceptance remains
  required before deploying the new recording contract over existing definitions.
- Use magnetic keyframes, cursor-anchored zoom, two-axis panning, configurable
  wheel actions, smooth playhead reveal/follow and reorderable toolbar controls.
- Manage track visibility, linking, mute, solo, lock, ordering and metadata
  consistently across native and Web commands.
- Refine library cards, group controls, icon/color pickers and the shared Cue
  properties/Effect Controls editor.
- Keep Effects Library and Hardware Monitor cards clear of their scrollbars;
  reveal library card actions and folder-add icons on hover or keyboard focus
  without shifting buttons or card edges.
- Align the Effects Library heading with Search and folder/card edges without
  changing editable-name alignment.

### 🔌 Hardware and remote control

- Discover advertised board channels/settings, manage channel presentation,
  bindings and order, and view live motion/PWM/status feedback.
- Keep the hidden-channel menu aligned above Hardware Monitor groups without
  reserving a large blank area in the sidebar.
- Right-click hardware section headings to manage persistent channel folders;
  rename, choose icons, move or drag channels, and delete folders without deleting
  outputs. Keep ungrouped channels distinct from named lighting folders and raw
  seat-wiring relays separate from operator outputs.
- Control the front panel, buzzer melody catalog and addressable strip through
  the controller contracts; receive catalog changes without rebuilding dialogs.
- Connect Pealayer peers with authoritative preferences/session commands and
  range-based media retrieval; negotiate playback authority and production locks.
- Prepare timed hardware plans ahead of playback and correlate clock/cue ACKs.
  Physical edge accuracy and load/soak acceptance remain release gates, not a
  promise of hard real-time or perfect frame synchronization.

### 🎨 Interface and browsing

- Keep Neutral/Studio palettes, configurable accents and live OS theme changes
  while adopting compact palette-safe Elegance cards, badges and callouts.
- Organize Preferences without repeated cards; preview changes live, Browse for
  custom icons, retain Current/Classic icons and propagate configured branding.
- Navigate open native dropdowns with arrows, Home/End and Enter while keeping
  searchable pickers editable. Color choices retain their closed-control swatch;
  full-width rows stay aligned on hover. Custom accents open one integrated HEX
  and color picker with app swatches and bounded session recents.
- Inspect copied links by default, refresh an existing source dialog, discover
  remote folders and choose a working proxy/direct route without overriding an
  explicit choice. Refreshes keep lists stable and stale replies cannot win.
- Improve Persian input/subtitles, numeric field actions, responsive controls,
  shared messages/OSD and persisted workspace profiles.

### 🛠️ Reliability and performance

- Stop taskbar thumbnail acknowledgements from renewing their own capture loop
  ([#81](https://github.com/ToghrolTP/pealayer/pull/81)).
- Reduce paused polling, redundant capability clones, JSON publication and
  translation work while preserving event-driven telemetry updates.
- Process commands while the native window is hidden; repair timeline lock
  re-entry, shell action routing and transactional peer update ownership.
- Retain intrinsic cue policies and correlated seeks through session changes,
  reconnects and catalog refreshes; fail safely without replaying failed output
  mutations.

See [current acceptance](docs/verification/CURRENT-ACCEPTANCE.md) for unfinished
work. These changes are merged source, not a declaration that Web parity,
physical timing, every GPU renderer or every platform has final acceptance.

## v0.2.0 · native shell and portable settings

- Added Windows taskbar thumbnail media actions, video-focused previews, tray
  controls and playback/error progress reporting.
- Added persistent application settings, portable configuration discovery and
  Windows Registry settings/history storage.
- Expanded CLI/IPC shell integration, media browsing and contextual controls.
- Improved scrub previews, slider synchronization and subtitle/audio menus.
- Added both macOS architectures to the release build and repaired linking,
  headless initialization and serialized test execution.

[Release notes](docs/releases/v0.2.0.md)

## v0.1.0 · first public release

- Established the native Rust/libmpv player and its initial interfaces.
- Added the multi-platform CI/build/release foundation.

[Release notes](docs/releases/v0.1.0.md)
