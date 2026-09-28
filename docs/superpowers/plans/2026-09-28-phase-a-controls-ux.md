# Phase A: Controls Fade Transition & Seekbar/Mute Layout Overlap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the two user interface defects reported in GitHub Issue #2: (1) smooth, unified opacity fading for the entire controls bar and all its children (buttons, icons, labels, sliders), and (2) dynamic, non-overlapping layout between the seekbar, mute button, volume slider, and right controls across all window sizes and DPI scaling factors.

**Architecture:**
1. Use `ui.set_opacity(alpha)` on the egui `Ui` container and set `style.visuals.override_text_color = Some(style.visuals.text_color().linear_multiply(alpha))` inside `multiply_style_opacity` so that every button, glyph, border, and background fades out smoothly in lockstep with the frame.
2. Replace hardcoded right controls width (`375.0`) with a robust dynamic layout: compute the right controls width dynamically based on actual widget spacing and font metrics, guaranteeing a safety margin so the mute button (`🔇`) and seekbar never overlap under any window width (from 400px to 4K) or High-DPI scale (100%–200%).
3. Disable interaction when `alpha < 0.05` to prevent ghost clicks while fading out.

**Tech Stack:** Rust 2024, `eframe` / `egui 0.34.3`.

**Spec:** GitHub Issue #2 ("Player UX feedback" - Interface Controls checklist items: control bar fade transition & mute button overlap).

## Global Constraints
- Must maintain 100% backward compatibility with all existing timeline, IPC, and 4D engine controls.
- All 88 existing unit and integration tests must continue to pass without regressions.
- Cross-platform compilation on both Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-gnu`) must remain clean (0 errors, 0 warnings).

---

### Task 1: Smooth Control Fade Transition

**Files:**
- Modify: `src/ui/controls.rs`
- Test: `src/ui/controls.rs` (unit tests)

**Interfaces:**
- Updates:
  ```rust
  pub fn multiply_style_opacity(style: &mut egui::Style, alpha: f32);
  ```

- [ ] **Step 1: Write unit tests for text and widget opacity multiplication**

In `src/ui/controls.rs` `mod tests`:
```rust
#[test]
fn test_multiply_style_opacity_handles_none_override_text_color() {
    let mut style = egui::Style::default();
    style.visuals.override_text_color = None;
    let initial_text_color = style.visuals.text_color();

    multiply_style_opacity(&mut style, 0.5);

    assert_eq!(
        style.visuals.override_text_color,
        Some(initial_text_color.linear_multiply(0.5))
    );
}

#[test]
fn test_controls_interactable_threshold() {
    let alpha_active = 0.5;
    let alpha_faded = 0.02;
    assert!(is_controls_interactable(alpha_active));
    assert!(!is_controls_interactable(alpha_faded));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_multiply_style_opacity_handles_none_override_text_color`
Expected: FAIL (assertion failed because `override_text_color` remained `None`)

- [ ] **Step 3: Implement smooth fade in src/ui/controls.rs**

In `src/ui/controls.rs`:
```rust
pub fn is_controls_interactable(alpha: f32) -> bool {
    alpha >= 0.05
}

pub fn multiply_style_opacity(style: &mut egui::Style, alpha: f32) {
    let fade_color = |color: &mut egui::Color32| {
        *color = color.linear_multiply(alpha);
    };

    // When override_text_color is None, egui uses visuals.text_color().
    // Set override_text_color explicitly to faded text_color so all labels,
    // button text, and icon glyphs fade smoothly.
    let base_text_color = style.visuals.override_text_color.unwrap_or_else(|| style.visuals.text_color());
    style.visuals.override_text_color = Some(base_text_color.linear_multiply(alpha));

    fade_color(&mut style.visuals.warn_fg_color);
    fade_color(&mut style.visuals.error_fg_color);
    fade_color(&mut style.visuals.hyperlink_color);
    fade_color(&mut style.visuals.extreme_bg_color);
    fade_color(&mut style.visuals.faint_bg_color);
    fade_color(&mut style.visuals.code_bg_color);
    fade_color(&mut style.visuals.window_stroke.color);

    let widgets = &mut style.visuals.widgets;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        fade_color(&mut state.bg_fill);
        fade_color(&mut state.fg_stroke.color);
        fade_color(&mut state.bg_stroke.color);
    }

    fade_color(&mut style.visuals.selection.bg_fill);
    fade_color(&mut style.visuals.selection.stroke.color);
}
```
And inside `draw(app: &mut PealayerApp, ui: &mut egui::Ui)`:
```rust
        egui::Window::new("Controls")
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
            .min_width(window_width)
            .default_width(window_width)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .interactable(is_controls_interactable(alpha))
            .frame(egui::Frame::window(ui.style()).multiply_with_opacity(alpha))
            .show(&ctx, |ui| {
                ui.set_opacity(alpha);
                multiply_style_opacity(ui.style_mut(), alpha);
                ...
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_multiply_style_opacity_handles_none_override_text_color`
Expected: PASS

---

### Task 2: Eliminate Mute Button & Seekbar Overlap (Dynamic Layout)

**Files:**
- Modify: `src/ui/controls.rs`
- Test: `src/ui/controls.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn compute_controls_layout(available_width: f32, left_width: f32, right_width: f32, spacing: f32) -> (f32, f32);
  ```

- [ ] **Step 1: Write unit tests for layout dimensions and overlap avoidance**

In `src/ui/controls.rs` `mod tests`:
```rust
#[test]
fn test_compute_controls_layout_prevents_overlap() {
    // Standard 800px window
    let available_w = 780.0;
    let left_w = 120.0;
    let right_w = 380.0;
    let spacing = 8.0;

    let (seekbar_w, gap) = compute_controls_layout(available_w, left_w, right_w, spacing);
    assert!(seekbar_w >= 40.0);
    assert_eq!(left_w + spacing + seekbar_w + gap + right_w, available_w);
    assert!(gap >= spacing);

    // Narrow 500px window
    let available_w_narrow = 520.0;
    let (seekbar_w_narrow, gap_narrow) = compute_controls_layout(available_w_narrow, left_w, right_w, spacing);
    assert_eq!(seekbar_w_narrow, 40.0); // clamped to min width
    assert!(left_w + seekbar_w_narrow <= available_w_narrow);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_compute_controls_layout_prevents_overlap`
Expected: FAIL (missing `compute_controls_layout`)

- [ ] **Step 3: Implement compute_controls_layout and dynamic sizing**

In `src/ui/controls.rs`:
```rust
pub fn compute_controls_layout(
    available_width: f32,
    left_width: f32,
    right_width: f32,
    spacing: f32,
) -> (f32, f32) {
    let min_seekbar_width = 40.0;
    let fixed_widths = left_width + right_width + (spacing * 2.0);
    if available_width > fixed_widths {
        let seekbar_width = (available_width - fixed_widths).max(min_seekbar_width);
        let remaining_gap = (available_width - (left_width + spacing + seekbar_width + right_width)).max(spacing);
        (seekbar_width, remaining_gap)
    } else {
        (min_seekbar_width, spacing)
    }
}
```

In `draw`:
- Measure or calculate exact right controls width:
  - 5 icon buttons (Fullscreen, Pin, Audio, 4D, Subs) = 5 * 24.0 = 120.0
  - Volume slider = 80.0
  - Mute button = 24.0
  - Duration label = ~65.0
  - Spacings (7 item gaps * 8.0) = 56.0
  - Total right width = ~345.0
- Left controls width:
  - Play button = 30.0
  - Elapsed label = ~55.0
  - Gap = 8.0
  - Total left width = 93.0
- Seekbar width and right controls start are dynamically derived with `compute_controls_layout`.
- Render left controls -> Render seekbar with exact clamped `seekbar_w` -> Render right controls via right-aligned sub-UI with guaranteed zero overlap.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_compute_controls_layout_prevents_overlap`
Expected: PASS

---

### Task 3: Visual Audit & Full Suite Verification

**Files:**
- Audit: `src/ui/controls.rs`
- Run: Full test suite

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass (0 failures)

- [ ] **Step 2: Run Windows cross-compilation check**

Run: `cargo check --target x86_64-pc-windows-gnu`
Expected: 0 errors, 0 warnings

- [ ] **Step 3: Verify git status is clean**
