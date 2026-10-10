# What's changed in Pealayer

User-facing changes are grouped by what you can do, not by commit order.
Version notes describe the source at that release tag; later fixes are not
retroactively attributed to an older download.

## Next milestone · unreleased

### 🎬 Playback and audio

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

- Create direct hardware cues by track interaction or shortcut; distinguish
  move-only value markers and recorded clips from resizable holds and PWM ramps.
- Resize streamed lighting's active window without changing its cycle speed;
  keep finite recordings fixed-length and preserve independent cue durations
  when dropping another effect or refreshing the controller catalog.
- Record app/board activity into sequences, edit channel lanes, and place effects
  from the library without treating selection as seeking.
- Use magnetic keyframes, cursor-anchored zoom, two-axis panning, configurable
  wheel actions, smooth playhead reveal/follow and reorderable toolbar controls.
- Manage track visibility, linking, mute, solo, lock, ordering and metadata
  consistently across native and Web commands.
- Refine library cards, group controls, icon/color pickers and the shared Cue
  properties/Effect Controls editor.
- Keep Effects Library and Hardware Monitor cards clear of their scrollbars;
  reveal library card actions and folder-add icons on hover or keyboard focus
  without shifting buttons or card edges.

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
