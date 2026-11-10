# Light Theme Timeline Elements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure all timeline UI elements (time ruler, track headers, grid lines, analog curve tracks, keyframe markers, snap lines, and clip handles) adapt dynamically to egui's Light Theme with accessible WCAG AA contrast (≥ 4.5:1).

**Architecture:** Introduce a dedicated `TimelineTheme` abstraction derived from `ui.visuals().dark_mode`. Replace hardcoded dark-mode RGB/RGBA constants across `src/ui/layout.rs` with `TimelineTheme` palette values that preserve dark Premiere styling in dark mode while delivering crisp, high-contrast typography, borders, and keyframe visibility in light mode.

**Tech Stack:** Rust 2021, egui 0.31, eframe.

**Spec:** Issue #28 ("Revamp dialogs, iconography, typography, themes, and Persian RTL") & Issue #26 ("Timeline zoom, contextual menus, click-to-play/pause, drag gesture preferences, light-theme completeness").

## Global Constraints

- Never use hardcoded white (`Color32::WHITE`) or bright cyan (`Color32::from_rgb(0, 220, 255)`) on surfaces that become white in light mode (`ui.visuals().extreme_bg_color`).
- All text and interactive icon contrasts on background fills must meet WCAG AA (≥ 4.5:1).
- Keyframe diamond borders in light mode must be dark enough to be instantly legible on `#ffffff` backgrounds.
- Zero breaking changes to existing dark theme visuals or timeline interaction behaviors (drag/drop, resize, snap).
- All changes must pass `cargo test --all-targets` and `cargo check --target x86_64-pc-windows-gnu`.

---

### Task 1: TimelineTheme Palette and WCAG Contrast Unit Tests

**Files:**
- Create: `tests/timeline_theme_test.rs`
- Modify: `src/ui/layout.rs:50-100`

**Interfaces:**
- Produces: `pub struct TimelineTheme`, `impl TimelineTheme { pub fn for_visuals(visuals: &egui::Visuals) -> Self }`

- [ ] **Step 1: Write the failing tests in `tests/timeline_theme_test.rs`**

```rust
use eframe::egui;
use pealayer::ui::layout::TimelineTheme;

fn relative_luminance(c: egui::Color32) -> f64 {
    let to_linear = |v: u8| -> f64 {
        let s = v as f64 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * to_linear(c.r()) + 0.7152 * to_linear(c.g()) + 0.0722 * to_linear(c.b())
}

fn contrast_ratio(c1: egui::Color32, c2: egui::Color32) -> f64 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[test]
fn test_timeline_theme_mode_detection() {
    let dark_visuals = egui::Visuals::dark();
    let light_visuals = egui::Visuals::light();

    let dark_theme = TimelineTheme::for_visuals(&dark_visuals);
    let light_theme = TimelineTheme::for_visuals(&light_visuals);

    assert!(dark_theme.is_dark);
    assert!(!light_theme.is_dark);
}

#[test]
fn test_timeline_theme_contrast_ratios() {
    let light_visuals = egui::Visuals::light();
    let light_theme = TimelineTheme::for_visuals(&light_visuals);

    let ruler_bg = light_theme.ruler_bg;
    let ruler_text_contrast = contrast_ratio(light_theme.ruler_text, ruler_bg);
    assert!(
        ruler_text_contrast >= 4.5,
        "Ruler text contrast ({:.2}) on light ruler bg must be >= 4.5",
        ruler_text_contrast
    );

    let track_bg = light_theme.row_bg;
    let channel_accent_contrast = contrast_ratio(light_theme.channel_accent, track_bg);
    assert!(
        channel_accent_contrast >= 4.0,
        "Channel accent contrast ({:.2}) on light track bg must be >= 4.0",
        channel_accent_contrast
    );

    let scale_tick_contrast = contrast_ratio(light_theme.scale_tick_text, track_bg);
    assert!(
        scale_tick_contrast >= 3.5,
        "Scale tick contrast ({:.2}) on light track bg must be >= 3.5",
        scale_tick_contrast
    );
}

#[test]
fn test_timeline_keyframe_visibility_in_light_mode() {
    let light_visuals = egui::Visuals::light();
    let light_theme = TimelineTheme::for_visuals(&light_visuals);

    // Keyframe stroke must not be white or near-white in light mode
    let stroke = light_theme.keyframe_stroke;
    let white_dist = ((255 - stroke.r() as i32).pow(2)
        + (255 - stroke.g() as i32).pow(2)
        + (255 - stroke.b() as i32).pow(2)) as f64;
    assert!(
        white_dist > 15000.0,
        "Keyframe stroke must not be near-white in light mode (distance: {:.1})",
        white_dist
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test timeline_theme_test`
Expected: FAIL with "cannot find type `TimelineTheme` in module `pealayer::ui::layout`"

- [ ] **Step 3: Implement `TimelineTheme` in `src/ui/layout.rs`**

```rust
#[derive(Clone, Copy, Debug)]
pub struct TimelineTheme {
    pub is_dark: bool,
    pub ruler_bg: egui::Color32,
    pub ruler_border: egui::Color32,
    pub ruler_major_tick: egui::Color32,
    pub ruler_minor_tick: egui::Color32,
    pub ruler_text: egui::Color32,
    pub grid_line: egui::Color32,
    pub row_bg: egui::Color32,
    pub row_alt_bg: egui::Color32,
    pub row_separator: egui::Color32,
    pub centerline: egui::Color32,
    pub header_bg: egui::Color32,
    pub header_text: egui::Color32,
    pub channel_accent: egui::Color32,
    pub scale_tick_text: egui::Color32,
    pub unarmed_icon: egui::Color32,
    pub curve_stroke: egui::Color32,
    pub curve_fill: egui::Color32,
    pub curve_muted_stroke: egui::Color32,
    pub curve_muted_fill: egui::Color32,
    pub keyframe_stroke: egui::Color32,
    pub keyframe_hover_stroke: egui::Color32,
    pub keyframe_hover_glow: egui::Color32,
    pub keyframe_selected: egui::Color32,
    pub playhead: egui::Color32,
    pub snap_line: egui::Color32,
    pub drop_highlight_border: egui::Color32,
    pub drop_highlight_fill: egui::Color32,
}

impl TimelineTheme {
    pub fn for_visuals(visuals: &egui::Visuals) -> Self {
        if visuals.dark_mode {
            Self {
                is_dark: true,
                ruler_bg: visuals.panel_fill,
                ruler_border: egui::Color32::from_rgb(50, 50, 50),
                ruler_major_tick: egui::Color32::from_rgb(100, 100, 100),
                ruler_minor_tick: egui::Color32::from_rgb(60, 60, 60),
                ruler_text: egui::Color32::from_rgb(170, 170, 170),
                grid_line: visuals.widgets.noninteractive.bg_stroke.color,
                row_bg: visuals.extreme_bg_color,
                row_alt_bg: visuals.faint_bg_color,
                row_separator: visuals.widgets.noninteractive.bg_stroke.color,
                centerline: egui::Color32::from_rgb(34, 42, 48),
                header_bg: visuals.faint_bg_color,
                header_text: visuals.text_color(),
                channel_accent: egui::Color32::from_rgb(0, 220, 255),
                scale_tick_text: egui::Color32::from_rgb(90, 90, 90),
                unarmed_icon: egui::Color32::from_rgb(120, 120, 120),
                curve_stroke: egui::Color32::from_rgb(0, 220, 255),
                curve_fill: egui::Color32::from_rgba_unmultiplied(0, 220, 255, 25),
                curve_muted_stroke: egui::Color32::from_rgb(110, 110, 110),
                curve_muted_fill: egui::Color32::from_rgba_unmultiplied(100, 100, 100, 20),
                keyframe_stroke: egui::Color32::WHITE,
                keyframe_hover_stroke: egui::Color32::from_rgb(255, 255, 180),
                keyframe_hover_glow: egui::Color32::from_rgba_unmultiplied(255, 255, 150, 45),
                keyframe_selected: egui::Color32::from_rgb(255, 230, 0),
                playhead: egui::Color32::RED,
                snap_line: egui::Color32::from_rgb(0, 255, 255),
                drop_highlight_border: egui::Color32::from_rgb(170, 130, 255),
                drop_highlight_fill: egui::Color32::from_rgba_unmultiplied(108, 76, 170, 55),
            }
        } else {
            Self {
                is_dark: false,
                ruler_bg: visuals.panel_fill,
                ruler_border: egui::Color32::from_rgb(205, 205, 205),
                ruler_major_tick: egui::Color32::from_rgb(135, 135, 135),
                ruler_minor_tick: egui::Color32::from_rgb(185, 185, 185),
                ruler_text: egui::Color32::from_rgb(60, 60, 60),
                grid_line: egui::Color32::from_rgb(228, 228, 228),
                row_bg: visuals.extreme_bg_color,
                row_alt_bg: visuals.faint_bg_color,
                row_separator: egui::Color32::from_rgb(218, 218, 218),
                centerline: egui::Color32::from_rgb(222, 226, 230),
                header_bg: visuals.faint_bg_color,
                header_text: visuals.text_color(),
                channel_accent: egui::Color32::from_rgb(0, 102, 184),
                scale_tick_text: egui::Color32::from_rgb(105, 105, 105),
                unarmed_icon: egui::Color32::from_rgb(135, 135, 135),
                curve_stroke: egui::Color32::from_rgb(0, 112, 192),
                curve_fill: egui::Color32::from_rgba_unmultiplied(0, 112, 192, 35),
                curve_muted_stroke: egui::Color32::from_rgb(150, 150, 150),
                curve_muted_fill: egui::Color32::from_rgba_unmultiplied(150, 150, 150, 25),
                keyframe_stroke: egui::Color32::from_rgb(32, 32, 32),
                keyframe_hover_stroke: egui::Color32::from_rgb(0, 85, 160),
                keyframe_hover_glow: egui::Color32::from_rgba_unmultiplied(0, 112, 192, 45),
                keyframe_selected: egui::Color32::from_rgb(220, 120, 0),
                playhead: egui::Color32::from_rgb(215, 38, 56),
                snap_line: egui::Color32::from_rgb(0, 102, 204),
                drop_highlight_border: egui::Color32::from_rgb(112, 64, 180),
                drop_highlight_fill: egui::Color32::from_rgba_unmultiplied(108, 76, 170, 35),
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test timeline_theme_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/timeline_theme_test.rs
git commit -m "feat(ui): define TimelineTheme with light mode palette and WCAG contrast checks"
```

---

### Task 2: Time Ruler and Grid Light Theme Integration

**Files:**
- Modify: `src/ui/layout.rs:1620-1715, 2715-2775`
- Test: `tests/timeline_theme_test.rs`

**Interfaces:**
- Consumes: `TimelineTheme::for_visuals(ui.visuals())`

- [ ] **Step 1: Write unit test in `tests/timeline_theme_test.rs` checking ruler line and text colors**

```rust
#[test]
fn test_timeline_ruler_palette_distinction() {
    let dark_theme = TimelineTheme::for_visuals(&egui::Visuals::dark());
    let light_theme = TimelineTheme::for_visuals(&egui::Visuals::light());

    assert_ne!(dark_theme.ruler_border, light_theme.ruler_border);
    assert_ne!(dark_theme.ruler_major_tick, light_theme.ruler_major_tick);
    assert_ne!(dark_theme.ruler_minor_tick, light_theme.ruler_minor_tick);
    assert_ne!(dark_theme.ruler_text, light_theme.ruler_text);
    assert_ne!(dark_theme.grid_line, light_theme.grid_line);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test timeline_theme_test`
Expected: PASS

- [ ] **Step 3: Update `layout.rs` ruler and grid rendering using `TimelineTheme`**

In `src/ui/layout.rs`:
Obtain `let tt = TimelineTheme::for_visuals(ui.visuals());` at start of `PealayerTab::Timeline` rendering:
1. Lines 1675-1682: Draw major grid lines with `tt.grid_line`.
2. Lines 1699-1703 & 1708-1712: Draw track separators with `tt.row_separator`.
3. Lines 2720-2725: Draw ruler bottom border with `tt.ruler_border`.
4. Lines 2738-2741: Draw major second tick with `tt.ruler_major_tick`.
5. Lines 2748-2751: Draw sub-second notches with `tt.ruler_minor_tick`.
6. Lines 2762-2768: Draw time label with `tt.ruler_text`.

- [ ] **Step 4: Verify full test suite passes**

Run: `cargo test`
Expected: 100% tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/timeline_theme_test.rs
git commit -m "feat(ui): adapt timeline ruler and grid separators to active theme"
```

---

### Task 3: Analog Curve Track, Amplitude Ticks, and Keyframe Visibility

**Files:**
- Modify: `src/ui/layout.rs:1500-1530, 2330-2460`
- Test: `tests/timeline_theme_test.rs`

**Interfaces:**
- Consumes: `TimelineTheme` (`centerline`, `scale_tick_text`, `curve_stroke`, `curve_fill`, `curve_muted_stroke`, `curve_muted_fill`, `keyframe_stroke`, `keyframe_hover_stroke`, `keyframe_hover_glow`, `keyframe_selected`)

- [ ] **Step 1: Write test in `tests/timeline_theme_test.rs` verifying curve and keyframe colors**

```rust
#[test]
fn test_analog_curve_and_keyframe_palette_contrast() {
    let light_theme = TimelineTheme::for_visuals(&egui::Visuals::light());
    assert_ne!(light_theme.curve_stroke, egui::Color32::from_rgb(0, 220, 255));
    assert_ne!(light_theme.keyframe_stroke, egui::Color32::WHITE);
    assert_ne!(light_theme.centerline, egui::Color32::from_rgb(34, 42, 48));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test timeline_theme_test`
Expected: PASS

- [ ] **Step 3: Update `layout.rs` analog track rendering using `TimelineTheme`**

1. Lines 1509-1528: Replace hardcoded `from_rgb(90, 90, 90)` and `from_rgb(70, 70, 70)` with `tt.scale_tick_text`.
2. Lines 2340-2343: Replace `egui::Color32::from_rgb(34, 42, 48)` centerline with `tt.centerline`.
3. Lines 2358-2362: Replace curve gradient fill with `if track.muted { tt.curve_muted_fill } else { tt.curve_fill }`.
4. Lines 2376-2384: Replace curve line stroke with `if track.muted { tt.curve_muted_stroke } else { tt.curve_stroke }`.
5. Lines 2426-2429: Keyframe fill diamond when selected uses `tt.keyframe_selected`.
6. Lines 2433-2436: Keyframe outline stroke uses `if is_hovered { egui::Stroke::new(2.0, tt.keyframe_hover_stroke) } else { egui::Stroke::new(1.2, tt.keyframe_stroke) }`.
7. Lines 2445-2450: Keyframe hover glow diamond uses `tt.keyframe_hover_glow`.

- [ ] **Step 4: Verify test suite passes**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/timeline_theme_test.rs
git commit -m "feat(ui): ensure analog curves and keyframe markers are legible in light mode"
```

---

### Task 4: Track Headers, Clip Handles, and Drag Highlights

**Files:**
- Modify: `src/ui/layout.rs:1530-1555, 2790-2825, 3500-3545`
- Test: `tests/timeline_theme_test.rs`

**Interfaces:**
- Consumes: `TimelineTheme` (`channel_accent`, `unarmed_icon`, `snap_line`, `drop_highlight_border`, `drop_highlight_fill`)

- [ ] **Step 1: Write test in `tests/timeline_theme_test.rs` verifying header accent and snap line**

```rust
#[test]
fn test_track_header_and_snap_line_contrast() {
    let light_theme = TimelineTheme::for_visuals(&egui::Visuals::light());
    assert_ne!(light_theme.channel_accent, egui::Color32::from_rgb(0, 220, 255));
    assert_ne!(light_theme.snap_line, egui::Color32::from_rgb(0, 255, 255));
    assert_ne!(light_theme.unarmed_icon, egui::Color32::from_rgb(120, 120, 120));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test timeline_theme_test`
Expected: PASS

- [ ] **Step 3: Update `layout.rs` headers, handles, and snap line with `TimelineTheme`**

1. Line 1535: Port channel name text uses `.color(tt.channel_accent)`.
2. Lines 1548-1551: Un-armed record dot uses `tt.unarmed_icon`.
3. Lines 2793-2797: Snap line stroke uses `tt.snap_line`.
4. Lines 2818-2820: Macro/strip drag highlight uses `tt.drop_highlight_fill` and `tt.drop_highlight_border`.
5. Lines 3500-3545: Pass `tt: TimelineTheme` or `is_dark: bool` to `render_clip_handles` so handle accent line uses `tt.channel_accent` instead of hardcoded cyan.

- [ ] **Step 4: Verify test suite passes**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/timeline_theme_test.rs
git commit -m "feat(ui): adapt track headers, clip handles, and snap lines to light theme"
```

---

### Task 5: End-to-End Theme Switching Verification

**Files:**
- Test: `tests/timeline_theme_test.rs`

- [ ] **Step 1: Add end-to-end simulated theme toggle integration test**

```rust
use pealayer::app::PealayerApp;
use pealayer::config::AppTheme;

#[test]
fn test_app_theme_switching_updates_visuals_and_timeline() {
    let ctx = egui::Context::default();
    let mut app = PealayerApp::default();

    // 1. Switch to Light Theme
    app.set_theme(&ctx, AppTheme::Light);
    let visuals_light = ctx.style().visuals.clone();
    assert!(!visuals_light.dark_mode, "Context visuals must be in light mode");
    let theme_light = TimelineTheme::for_visuals(&visuals_light);
    assert!(!theme_light.is_dark);
    assert_eq!(theme_light.keyframe_stroke, egui::Color32::from_rgb(32, 32, 32));

    // 2. Switch to Dark Theme
    app.set_theme(&ctx, AppTheme::Dark);
    let visuals_dark = ctx.style().visuals.clone();
    assert!(visuals_dark.dark_mode, "Context visuals must be in dark mode");
    let theme_dark = TimelineTheme::for_visuals(&visuals_dark);
    assert!(theme_dark.is_dark);
    assert_eq!(theme_dark.keyframe_stroke, egui::Color32::WHITE);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test timeline_theme_test`
Expected: PASS

- [ ] **Step 3: Run full cross-platform checks**

Run: `cargo check --target x86_64-pc-windows-gnu` and `cargo test`
Expected: 100% pass across all targets.

- [ ] **Step 4: Commit**

```bash
git add tests/timeline_theme_test.rs
git commit -m "test(ui): add end-to-end integration test for timeline light/dark theme switching"
```
