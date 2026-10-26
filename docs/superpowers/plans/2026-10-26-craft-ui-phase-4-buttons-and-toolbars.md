# Craft UI Phase 4: Buttons & Toolbars Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate `craft_ui::buttons::IconButtonStyle`, `craft_ui::buttons::IconButton`, and `craft_ui::layout::toolbar::fit_toolbar` across Pealayer's action control bars, monitor transport controls, and timeline toolbars with zero regression to drag-and-drop, density, or accessibility contracts.

**Architecture:** Implement `craft_theme::icon_button_style` returning `craft_ui::buttons::IconButtonStyle` calibrated from `craft_theme::Tokens`. Refactor `src/ui/action_customization.rs::control` to consume this style for interaction fills, strokes, focus outline, and pill selection. Integrate `craft_ui::layout::toolbar::fit_toolbar` in `src/ui/control_bar.rs` to compute toolbar fits and overflow.

**Tech Stack:** Rust 2024, `egui 0.37` (storytold fork), `craft-ui 0.1.0` (`craft_ui::buttons`, `craft_ui::layout::toolbar`), `eframe 0.37`.

**Spec:** `AGENTS.md` (Pealayer alpha contract), FilmCraft design tokens, and Craft UI component APIs.

## Global Constraints

- Sizing must strictly come from `ui::control_density` (`UiControlDensity`: Compact 24, Regular 28, Comfortable 32, Spacious 36).
- Modal corner-close controls must use `ui::dialog::close_button` with exact square hit targets independent of text padding.
- `confirmation_modal` must be preserved for unsaved-change prompts.
- Relay ON/OFF actions use semantic green/red, never theme selection/accent colors.
- E-STOP release remains on GUI confirmation path; opening menus must not actuate hardware.
- Control bar drag-and-drop registration (`click_and_drag()`) and payload routing must remain intact.
- Tests exercising persistence must receive an isolated `PEALAYER_CONFIG_FILE` from the parent process before startup.
- The egui `Context` lock is non-reentrant; never invoke context accessors or repaint callbacks from inside data transactions.

---

### Task 1: `IconButtonStyle` Factory in `src/ui/craft_theme.rs`

**Files:**
- Modify: `src/ui/craft_theme.rs`
- Test: `src/ui/craft_theme.rs::tests`

**Interfaces:**
- Consumes: `craft_theme::Tokens`, `craft_ui::buttons::{IconButtonStyle, ButtonVisuals}`
- Produces: `pub fn icon_button_style(tokens: &Tokens, frame: bool) -> craft_ui::buttons::IconButtonStyle`

- [x] **Step 1: Write failing unit test in `src/ui/craft_theme.rs`**

```rust
#[test]
fn icon_button_style_applies_filmcraft_pill_and_hover_tokens() {
    let dark = Tokens::for_kind(ThemeKind::Dark);
    let style = icon_button_style(&dark, false);

    assert_eq!(style.normal.fill, egui::Color32::TRANSPARENT);
    assert_eq!(style.hovered.fill, dark.hover);
    assert_eq!(style.pressed.map(|p| p.fill), Some(dark.pressed));
    assert_eq!(style.selected.fill, dark.pill_active_bg);
    assert_eq!(style.selected.stroke.color, dark.accent);
    assert_eq!(style.focus_stroke.color, dark.focus);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::craft_theme::tests::icon_button_style_applies_filmcraft_pill_and_hover_tokens`
Expected: FAIL with "cannot find function `icon_button_style`"

- [x] **Step 3: Write minimal implementation in `src/ui/craft_theme.rs`**

Implement `pub fn icon_button_style(tokens: &Tokens, frame: bool) -> craft_ui::buttons::IconButtonStyle` configuring:
- normal: transparent fill (or `tokens.panel_bg` if frame), stroke `tokens.field_border` (if frame) or none, text color `tokens.text_muted`.
- hovered: fill `tokens.hover`, stroke `tokens.field_border` (if frame) or none, icon color `tokens.text`.
- pressed: fill `tokens.pressed`, stroke `tokens.accent`, icon color `tokens.text`.
- selected: fill `tokens.pill_active_bg`, stroke `Stroke::new(1.0, tokens.accent)`, icon color `tokens.accent`.
- corner_radius: `CornerRadius::same(4)`.
- focus_stroke: `Stroke::new(1.0, tokens.focus)`.

- [x] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::craft_theme::tests::icon_button_style_applies_filmcraft_pill_and_hover_tokens`
Expected: PASS (ok)

- [x] **Step 5: Run all craft_theme tests to verify no regressions**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::craft_theme::tests`
Expected: PASS

- [x] **Step 6: Commit**

```bash
git add src/ui/craft_theme.rs
git commit -m "feat(ui): add icon_button_style factory in craft_theme matching Craft UI tokens"
```

---

### Task 2: Connect `IconButtonStyle` into `src/ui/action_customization.rs`

**Files:**
- Modify: `src/ui/action_customization.rs`
- Test: `src/ui/action_customization.rs::tests`

**Interfaces:**
- Consumes: `craft_theme::icon_button_style`, `craft_theme::current(ctx)`
- Produces: `action_customization::control` styled using Craft UI button visuals and focus stroke.

- [ ] **Step 1: Write failing regression test in `src/ui/action_customization.rs`**

```rust
#[test]
fn action_control_renders_with_craft_icon_button_visuals() {
    let ctx = egui::Context::default();
    let config = crate::config::AppConfig::default();
    crate::ui::configure_native_visuals(&ctx, &config);
    let tokens = crate::ui::craft_theme::current(&ctx);

    let style = crate::ui::craft_theme::icon_button_style(&tokens, false);
    assert_eq!(style.selected.fill, tokens.pill_active_bg);
    assert_eq!(style.selected.stroke.color, tokens.accent);
}
```

- [ ] **Step 2: Run test to verify it compiles and runs**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::action_customization::tests::action_control_renders_with_craft_icon_button_visuals`
Expected: PASS or compile check.

- [ ] **Step 3: Refactor `control` in `src/ui/action_customization.rs`**

In `pub(crate) fn control(...) -> Response`:
Query `let btn_style = super::craft_theme::icon_button_style(&tokens, frame);`
Use `btn_style` to provide fill, stroke, and focus indicators for the button:
- When selected: fill `btn_style.selected.fill`, stroke `btn_style.selected.stroke`.
- When hovered: fill `btn_style.hovered.fill`, stroke `btn_style.hovered.stroke`.
- When pressed: fill `btn_style.pressed.unwrap().fill`, stroke `btn_style.pressed.unwrap().stroke`.
- Focus outline: when `response.has_focus()`, paint `btn_style.focus_stroke`.
- Preserve `ui.interact(rect, id, Sense::click_and_drag())`, accessibility `WidgetInfo`, and font rendering.

- [ ] **Step 4: Run all control bar and action tests to verify no regressions**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::control_bar::tests ui::timeline_toolbar::tests ui::monitor_controls::tests`
Expected: PASS (all tests pass)

- [ ] **Step 5: Commit**

```bash
git add src/ui/action_customization.rs
git commit -m "feat(ui): refactor action_customization control to use craft_theme icon_button_style"
```

---

### Task 3: Integrate `craft_ui::layout::toolbar::fit_toolbar` in `src/ui/control_bar.rs`

**Files:**
- Modify: `src/ui/control_bar.rs`
- Test: `src/ui/control_bar.rs::tests`

**Interfaces:**
- Consumes: `craft_ui::layout::toolbar::fit_toolbar`
- Produces: Precise toolbar group fitting and overflow calculation.

- [ ] **Step 1: Write failing test in `src/ui/control_bar.rs`**

```rust
#[test]
fn control_bar_computes_fit_with_craft_toolbar_fit() {
    let natural = [32.0, 32.0, 32.0];
    let fit = craft_ui::layout::toolbar::fit_toolbar(&natural, 80.0, 4.0, 32.0);
    assert!(fit.is_some());
    let fit = fit.unwrap();
    assert_eq!(fit.shown.len(), 1);
    assert_eq!(fit.overflow.len(), 2);
}
```

- [ ] **Step 2: Run test to verify it passes / checks integration**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::control_bar::tests::control_bar_computes_fit_with_craft_toolbar_fit`
Expected: PASS (or compile if missing module import)

- [ ] **Step 3: Implement helper `compute_toolbar_fit` in `src/ui/control_bar.rs`**

Implement `pub(crate) fn compute_toolbar_fit(natural_widths: &[f32], available: f32, gap: f32, overflow_width: f32) -> craft_ui::layout::toolbar::ToolbarFit`.
Use `craft_ui::layout::toolbar::fit_toolbar` to calculate visible vs overflowing actions for customizable control bars.

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::control_bar::tests`
Expected: PASS (all control bar tests pass)

- [ ] **Step 5: Commit**

```bash
git add src/ui/control_bar.rs
git commit -m "feat(ui): integrate craft_ui fit_toolbar in control_bar"
```

---

### Task 4: Full Regression & Verification Gate

**Files:**
- Test: entire crate test suite

- [ ] **Step 1: Run full unit test suite**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib`
Expected: All 944+ tests PASS with 0 failures.

- [ ] **Step 2: Run downloader crate tests**

Run: `cargo test -p pealayer-downloader`
Expected: PASS with 0 failures.

- [ ] **Step 3: Run compiler checks across all targets**

Run: `cargo check --lib --bins`
Expected: Clean exit 0 with 0 errors.
