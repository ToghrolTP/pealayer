# FilmCraft & Craft UI Visuals Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate Craft UI widget palettes and FilmCraft design tokens into Pealayer's global visuals, monitor controls, and timeline panels with zero regression to control density or accessibility contracts.

**Architecture:** Connect `craft_theme::Tokens` and `craft_ui::theme::WidgetPalette` directly to `configure_native_visuals` in `src/ui/mod.rs`. Propagate FilmCraft surface, well, and timeline tokens into monitor controls and timeline panels while strictly preserving the 4-step control density (`ui::control_density`), modal close button geometry (`ui::dialog::close_button`), and WCAG >= 4.5 contrast guarantees.

**Tech Stack:** Rust 2024, `egui 0.37` (storytold fork), `craft-ui 0.1.0`, `eframe 0.37`.

**Spec:** `AGENTS.md` (Pealayer alpha contract) and FilmCraft design system tokens (`storytold/filmcraft`).

## Global Constraints

- Sizing must strictly come from `ui::control_density` (`UiControlDensity`: Compact 24, Regular 28, Comfortable 32, Spacious 36).
- Modal corner-close controls must use `ui::dialog::close_button` with exact square hit targets independent of text padding.
- `confirmation_modal` must be preserved for unsaved-change prompts.
- Relay ON/OFF actions use semantic green/red, never theme selection/accent colors.
- E-STOP release remains on GUI confirmation path; opening menus must not actuate hardware.
- Tests exercising persistence must receive an isolated `PEALAYER_CONFIG_FILE` from the parent process before startup.
- The egui `Context` lock is non-reentrant; never invoke context accessors or repaint callbacks from inside data transactions.

---

### Task 1: FilmCraft Widget Palette & Visuals Builder in `src/ui/craft_theme.rs`

**Files:**
- Modify: `src/ui/craft_theme.rs`
- Test: `src/ui/craft_theme.rs::tests`

**Interfaces:**
- Consumes: `Tokens`, `craft_ui::theme::WidgetPalette`, `UiControlDensity`
- Produces: `pub fn build_visuals(t: &Tokens) -> egui::Visuals`, `pub fn apply_to_style(style: &mut egui::Style, t: &Tokens, density: UiControlDensity)`

- [ ] **Step 1: Write failing unit test in `src/ui/craft_theme.rs`**

```rust
#[test]
fn build_visuals_applies_filmcraft_widget_palette_and_focus() {
    let dark = Tokens::for_kind(ThemeKind::Dark);
    let visuals = build_visuals(&dark);

    assert_eq!(visuals.panel_fill, dark.panel_bg);
    assert_eq!(visuals.extreme_bg_color, dark.field_bg);
    assert_eq!(visuals.widgets.inactive.bg_stroke.color, dark.field_border);
    assert_eq!(visuals.widgets.hovered.bg_stroke.color, dark.field_border);
    assert_eq!(visuals.widgets.hovered.bg_fill, dark.hover);
    assert_eq!(visuals.widgets.active.bg_fill, dark.pressed);
    assert_eq!(visuals.selection.bg_fill, dark.accent);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::craft_theme::tests::build_visuals_applies_filmcraft_widget_palette_and_focus`
Expected: FAIL with "cannot find function `build_visuals` in module"

- [ ] **Step 3: Write minimal implementation in `src/ui/craft_theme.rs`**

Implement `build_visuals(t: &Tokens) -> egui::Visuals` applying the `widget_palette`, field borders, selection fill, and subtle corner radius.

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::craft_theme::tests::build_visuals_applies_filmcraft_widget_palette_and_focus`
Expected: PASS (ok)

- [ ] **Step 5: Commit**

```bash
git add src/ui/craft_theme.rs
git commit -m "feat(ui): add build_visuals helper to craft_theme matching FilmCraft widget palette"
```

---

### Task 2: Global Visuals Integration in `src/ui/mod.rs`

**Files:**
- Modify: `src/ui/mod.rs`
- Test: `src/ui/mod.rs::tests`

**Interfaces:**
- Consumes: `craft_theme::Tokens`, `craft_theme::build_visuals`
- Produces: `configure_native_visuals(&egui::Context, &AppConfig)` with FilmCraft default widget fills and borders

- [ ] **Step 1: Write failing regression test in `src/ui/mod.rs`**

```rust
#[test]
fn native_visuals_use_craft_theme_tokens_and_widget_palette() {
    let ctx = eframe::egui::Context::default();
    let config = crate::config::AppConfig {
        theme: crate::config::AppTheme::Dark,
        ..Default::default()
    };
    super::configure_native_visuals(&ctx, &config);
    let dark_style = ctx.style_of(eframe::egui::Theme::Dark);
    let tokens = super::craft_theme::current(&ctx);

    assert_eq!(dark_style.visuals.widgets.hovered.bg_fill, tokens.hover);
    assert_eq!(dark_style.visuals.widgets.active.bg_fill, tokens.pressed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::tests::native_visuals_use_craft_theme_tokens_and_widget_palette`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation in `src/ui/mod.rs`**

Integrate `craft_theme::Tokens` into `configure_native_visuals`, setting the widget interaction fills, strokes, and context tokens while maintaining palette contrast assertions.

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::tests`
Expected: PASS (all tests pass, including contrast and density checks)

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): wire craft_theme widget palette into configure_native_visuals"
```

---

### Task 3: Monitor Transport Controls Token Alignment in `src/ui/monitor_controls.rs`

**Files:**
- Modify: `src/ui/monitor_controls.rs`
- Test: `src/ui/monitor_controls.rs::tests`

**Interfaces:**
- Consumes: `craft_theme::current(ctx)`
- Produces: Monitor transport button hover, pressed, and pill toggle styling aligned with FilmCraft tokens

- [ ] **Step 1: Write failing test in `src/ui/monitor_controls.rs`**

```rust
#[test]
fn monitor_transport_buttons_use_craft_theme_pill_and_hover_tokens() {
    let ctx = eframe::egui::Context::default();
    let config = crate::config::AppConfig::default();
    crate::ui::configure_native_visuals(&ctx, &config);
    let tokens = crate::ui::craft_theme::current(&ctx);

    assert_eq!(tokens.pill_active_bg, eframe::egui::Color32::from_rgb(0x3a, 0x3a, 0x3a));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::monitor_controls::tests`
Expected: Verify test fails if conditions are unmet.

- [ ] **Step 3: Write minimal implementation in `src/ui/monitor_controls.rs`**

Ensure monitor controls retrieve `craft_theme::current(ui.ctx())` for active pill toggles and icon tints.

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::monitor_controls::tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/monitor_controls.rs
git commit -m "feat(ui): align monitor controls styling with Craft UI tokens"
```

---

### Task 4: Timeline Surface & Ruler Token Alignment in `src/ui/layout.rs`

**Files:**
- Modify: `src/ui/layout.rs`
- Test: `src/ui/layout.rs::timeline_row_tests`

**Interfaces:**
- Consumes: `craft_theme::current(ctx)`
- Produces: Timeline track fills, ruler background, and playhead colors from FilmCraft tokens

- [ ] **Step 1: Write failing test in `src/ui/layout.rs`**

```rust
#[test]
fn timeline_ruler_inherits_craft_theme_ruler_tokens() {
    let ctx = eframe::egui::Context::default();
    let config = crate::config::AppConfig::default();
    crate::ui::configure_native_visuals(&ctx, &config);
    let tokens = crate::ui::craft_theme::current(&ctx);

    assert_eq!(tokens.tl_ruler_bg, eframe::egui::Color32::from_rgb(0x1d, 0x1d, 0x1d));
    assert_eq!(tokens.tl_ruler_tick, eframe::egui::Color32::from_rgb(0x8d, 0x8d, 0x8d));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::layout::timeline_row_tests::timeline_ruler_inherits_craft_theme_ruler_tokens`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation in `src/ui/layout.rs`**

Reference `craft_theme::current(ui.ctx())` for timeline background and ruler painting.

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::layout::timeline_row_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs
git commit -m "feat(ui): connect timeline track and ruler painting to craft_theme tokens"
```

---

### Task 5: Full Regression & Verification Gate

**Files:**
- Test: entire crate test suite

- [ ] **Step 1: Run full unit test suite**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib`
Expected: All 937+ tests PASS with 0 failures.

- [ ] **Step 2: Run downloader crate tests**

Run: `cargo test -p pealayer-downloader`
Expected: PASS with 0 failures.

- [ ] **Step 3: Run compiler checks across all targets**

Run: `cargo check --lib --bins`
Expected: Clean exit 0 with 0 errors.
