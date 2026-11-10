# Interface Bug Fixes & Menu Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve UI interactive slider snap-backs, seekbar scrub time indicator, audio/subtitle delay limits, and top-bar menu completeness adapted from PR #5.

**Architecture:** Update egui immediate-mode state synchronization in `src/ui/controls.rs`, `src/ui/subtitles.rs`, `src/ui/audio.rs`, and `src/ui/menu.rs` so that UI sliders write-through to app state immediately, delay ranges allow ±600.0s, seekbar time reflects active scrubbing, and menu bar offers full Subtitle Track and dialog actions.

**Tech Stack:** Rust, eframe/egui 0.34, libmpv2

**Spec:** PR #5 ("Addressing interface bugs and features")

## Global Constraints
- Preserve existing styling, layouts, and dark theme visuals.
- Maintain immediate-mode responsiveness: ensure MPV set_property calls succeed without blocking UI rendering.
- All unit tests must pass via `cargo test --locked`.
- Zero compiler warnings or lint regressions.

---

### Task 1: Seekbar Scrub Time Display in `src/ui/controls.rs`

**Files:**
- Modify: `src/ui/controls.rs:39-45`
- Test: `src/ui/controls.rs:270-305`

**Interfaces:**
- Consumes: `app.seek_pos: Option<f64>`, `app.playback_time: f64`
- Produces: Correct scrub time in time label while dragging seekbar

- [ ] **Step 1: Write failing test in `src/ui/controls.rs`**
Add unit test `test_display_time_calculation_scrubbing_vs_playback`:
```rust
#[test]
fn test_display_time_calculation_scrubbing_vs_playback() {
    let playback_time = 45.0;
    let seek_pos = Some(120.0);
    let resolved_time = seek_pos.unwrap_or(playback_time);
    assert_eq!(resolved_time, 120.0);

    let no_seek: Option<f64> = None;
    let fallback_time = no_seek.unwrap_or(playback_time);
    assert_eq!(fallback_time, 45.0);
}
```

- [ ] **Step 2: Run test to verify**
Run `cargo test --bin pealayer test_display_time_calculation_scrubbing_vs_playback`

- [ ] **Step 3: Update `src/ui/controls.rs`**
Update line 39:
```rust
let elapsed_time = app.seek_pos.unwrap_or(app.playback_time);
```

- [ ] **Step 4: Run all controls tests**
Run `cargo test ui::controls::tests`

---

### Task 2: Subtitle Font Size & Delay Immediate State Update in `src/ui/subtitles.rs`

**Files:**
- Modify: `src/ui/subtitles.rs:90-125`
- Test: `src/ui/subtitles.rs` (add unit tests module)

**Interfaces:**
- Consumes: `app.sub_font_size`, `app.sub_delay`, `app.mpv`
- Produces: Immediate local state write-through to prevent slider snap-back; expanded `[-600.0, 600.0]` delay range

- [ ] **Step 1: Write unit tests in `src/ui/subtitles.rs`**
Add tests module testing delay range bounds and reset constants.

- [ ] **Step 2: Update `src/ui/subtitles.rs`**
- Set `app.sub_font_size = font_size;` when slider changed
- Set `app.sub_delay = delay;` when drag value changed
- Set `app.sub_delay = 0.0;` when Reset clicked
- Expand delay range to `-600.0..=600.0`

- [ ] **Step 3: Verify tests pass**
Run `cargo test ui::subtitles::tests`

---

### Task 3: Audio Delay Immediate State Update in `src/ui/audio.rs`

**Files:**
- Modify: `src/ui/audio.rs:80-105`
- Test: `src/ui/audio.rs` (add unit tests module)

**Interfaces:**
- Consumes: `app.audio_delay`, `app.mpv`
- Produces: Immediate local state write-through to prevent slider snap-back; expanded `[-600.0, 600.0]` delay range

- [ ] **Step 1: Write unit tests in `src/ui/audio.rs`**
Add tests module testing delay range bounds and reset constants.

- [ ] **Step 2: Update `src/ui/audio.rs`**
- Set `app.audio_delay = delay;` when drag value changed
- Set `app.audio_delay = 0.0;` when Reset clicked
- Expand delay range to `-600.0..=600.0`

- [ ] **Step 3: Verify tests pass**
Run `cargo test ui::audio::tests`

---

### Task 4: Complete Subtitles & Audio Menus in `src/ui/menu.rs`

**Files:**
- Modify: `src/ui/menu.rs:125-175`
- Test: `src/ui/menu.rs` (add unit tests for track label formatting)

**Interfaces:**
- Consumes: `app.sub_tracks`, `app.current_sid`, `app.sub_visibility`, `app.current_aid`, `app.audio_tracks`
- Produces: Complete Subtitle Track selector with "None", Subtitle Settings modal trigger, Audio Settings modal trigger

- [ ] **Step 1: Write test for track label formatting helper**
Ensure track formatting handles missing lang/title gracefully with fallback to track ID.

- [ ] **Step 2: Implement track submenu & settings dialog triggers in `src/ui/menu.rs`**
- In `Subtitles` menu:
  - Add `Subtitle Track` submenu with "None" and dynamic track list
  - Separator
  - Enable Subtitles checkbox
  - Separator
  - "Subtitle Settings..." button -> `app.show_sub_settings = true`
- In `Audio` menu:
  - Add Separator
  - "Audio Settings..." button -> `app.show_audio_settings = true`

- [ ] **Step 3: Verify build and tests pass**
Run `cargo test ui::menu::tests` and `cargo check --locked`

---

### Task 5: End-to-End Verification & Cleanup

- [ ] **Step 1: Run full test suite**
Run `cargo test --locked`

- [ ] **Step 2: Verify binary compilation**
Run `cargo build --locked`

- [ ] **Step 3: Commit changes with conventional commits**
Commit changes to `main`.

- [ ] **Step 4: Close PR #5 as resolved**
Close PR #5 on GitHub referencing the implemented fixes and delete its branch.
