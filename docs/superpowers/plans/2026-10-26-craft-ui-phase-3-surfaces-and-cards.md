# Craft UI Phase 3: Surfaces, Cards & Wells Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply FilmCraft and Craft UI surface, well, card, and separator tokens across the Status Bar, Dialogs/Modals, and Effects Library & Hardware cards with zero regression to geometry, control density, or accessibility contracts.

**Architecture:** Retrieve `craft_theme::Tokens` via `craft_theme::current(ctx)` across native UI panels. Replace hardcoded panel fills and borders in `src/ui/status_bar.rs`, `src/ui/dialog.rs`, `src/ui/effects_library.rs`, and `src/ui/layout.rs` with `tokens.well_bg`, `tokens.card_bg`, `tokens.panel_bg`, and `tokens.separator`. Preserve `ui::dialog::close_button` geometry and `AGENTS.md` contracts.

**Tech Stack:** Rust 2024, `egui 0.37` (storytold fork), `craft-ui 0.1.0`, `eframe 0.37`.

**Spec:** `AGENTS.md` (Pealayer alpha contract), FilmCraft design tokens, and Phase 3 roadmap specifications.

## Global Constraints

- Sizing must strictly come from `ui::control_density` (`UiControlDensity`: Compact 24, Regular 28, Comfortable 32, Spacious 36).
- Modal corner-close controls must use `ui::dialog::close_button` with exact square hit targets independent of text padding.
- `confirmation_modal` must be preserved for unsaved-change prompts.
- Relay ON/OFF actions use semantic green/red, never theme selection/accent colors.
- E-STOP release remains on GUI confirmation path; opening menus must not actuate hardware.
- Effects Library display/rename headers must share grip/icon/name slots, action gutter, font/line-height and inset.
- Tests exercising persistence must receive an isolated `PEALAYER_CONFIG_FILE` from the parent process before startup.
- The egui `Context` lock is non-reentrant; never invoke context accessors or repaint callbacks from inside data transactions.

---

### Task 1: Status Bar Surface, Wells, and Separator Fills

**Files:**
- Modify: `src/ui/status_bar.rs`
- Test: `src/ui/status_bar.rs::tests`

**Interfaces:**
- Consumes: `craft_theme::current(ctx) -> Tokens`
- Produces: Bottom status bar panel styled with `tokens.well_bg` and a 1px top stroke with `tokens.separator`.

- [x] **Step 1: Write failing unit test in `src/ui/status_bar.rs`**

```rust
#[test]
fn status_bar_uses_craft_theme_surface_and_separator_tokens() {
    let ctx = egui::Context::default();
    let config = crate::config::AppConfig::default();
    crate::ui::configure_native_visuals(&ctx, &config);
    let tokens = crate::ui::craft_theme::current(&ctx);

    let frame = status_bar_panel_frame(&tokens);
    assert_eq!(frame.fill, tokens.well_bg);
    assert_eq!(frame.stroke.color, tokens.separator);
    assert_eq!(frame.stroke.width, 1.0);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::status_bar::tests::status_bar_uses_craft_theme_surface_and_separator_tokens`
Expected: FAIL with "cannot find function `status_bar_panel_frame`"

- [x] **Step 3: Write minimal implementation in `src/ui/status_bar.rs`**

Implement `pub(crate) fn status_bar_panel_frame(tokens: &crate::ui::craft_theme::Tokens) -> egui::Frame` that returns `egui::Frame::none().fill(tokens.well_bg).stroke(egui::Stroke::new(1.0, tokens.separator))`.
In `pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui)`:
Replace the unstyled `egui::Panel::bottom("status_bar")` with `.frame(status_bar_panel_frame(&tokens))`.

- [x] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::status_bar::tests::status_bar_uses_craft_theme_surface_and_separator_tokens`
Expected: PASS (ok)

- [x] **Step 5: Run all status bar tests to verify no regressions**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::status_bar::tests`
Expected: PASS (all 19 passed)

- [x] **Step 6: Commit**

```bash
git add src/ui/status_bar.rs
git commit -m "feat(ui): style status bar panel with craft_theme well_bg and separator"
```

---

### Task 2: Modal & Dialog Surfaces Alignment

**Files:**
- Modify: `src/ui/dialog.rs`
- Test: `src/ui/dialog.rs::tests`

**Interfaces:**
- Consumes: `craft_theme::current(ctx) -> Tokens`
- Produces: `confirmation_modal` and dialog frame styled with `tokens.panel_bg`, `tokens.border_subtle`, and `tokens.modal_backdrop`.

- [x] **Step 1: Write failing unit test in `src/ui/dialog.rs`**

```rust
#[test]
fn confirmation_modal_frame_uses_craft_theme_tokens() {
    let ctx = egui::Context::default();
    let config = crate::config::AppConfig::default();
    crate::ui::configure_native_visuals(&ctx, &config);
    let tokens = crate::ui::craft_theme::current(&ctx);

    let frame = confirmation_modal_frame(&tokens);
    assert_eq!(frame.fill, tokens.panel_bg);
    assert_eq!(frame.stroke.color, tokens.border_subtle);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::dialog::tests::confirmation_modal_frame_uses_craft_theme_tokens`
Expected: FAIL with "cannot find function `confirmation_modal_frame`"

- [x] **Step 3: Write minimal implementation in `src/ui/dialog.rs`**

Implement `pub(crate) fn confirmation_modal_frame(tokens: &crate::ui::craft_theme::Tokens) -> egui::Frame` using `tokens.panel_bg` and `tokens.border_subtle`.
In `confirmation_modal`:
Use `confirmation_modal_frame(&tokens)` and set `.backdrop_color(tokens.modal_backdrop)`.
Ensure `close_button` paint, geometry, and non-destructive escape logic remain completely untouched.

- [x] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::dialog::tests::confirmation_modal_frame_uses_craft_theme_tokens`
Expected: PASS (ok)

- [x] **Step 5: Run all dialog tests to verify no regressions**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::dialog::tests`
Expected: PASS (all 20 passed)

- [x] **Step 6: Commit**

```bash
git add src/ui/dialog.rs
git commit -m "feat(ui): align confirmation modal frame and backdrop with craft_theme tokens"
```

---

### Task 3: Card Well & Divider Styling in Effects Library & Sidebars

**Files:**
- Modify: `src/ui/effects_library.rs`, `src/ui/layout.rs`
- Test: `src/ui/effects_library.rs::tests`, `src/ui/layout.rs::timeline_row_tests`

**Interfaces:**
- Consumes: `craft_theme::current(ui.ctx()) -> Tokens`
- Produces: Effects library container and hardware cards styled with `tokens.well_bg`, `tokens.card_bg`, and `tokens.separator`.

- [ ] **Step 1: Write failing test in `src/ui/effects_library.rs`**

```rust
#[test]
fn effects_library_search_well_uses_craft_theme_field_tokens() {
    let ctx = egui::Context::default();
    let config = crate::config::AppConfig::default();
    crate::ui::configure_native_visuals(&ctx, &config);
    let tokens = crate::ui::craft_theme::current(&ctx);

    let frame = effects_search_well_frame(&tokens);
    assert_eq!(frame.fill, tokens.field_bg);
    assert_eq!(frame.stroke.color, tokens.field_border);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::effects_library::tests::effects_library_search_well_uses_craft_theme_field_tokens`
Expected: FAIL with "cannot find function `effects_search_well_frame`"

- [ ] **Step 3: Write minimal implementation in `src/ui/effects_library.rs` and `src/ui/layout.rs`**

Implement `pub(crate) fn effects_search_well_frame(tokens: &crate::ui::craft_theme::Tokens) -> egui::Frame` using `tokens.field_bg` and `tokens.field_border`.
In `effects_library.rs`, use `tokens.well_bg` for panel container background and `tokens.separator` for group divider strokes.
In `layout.rs`, ensure hardware card containers use `tokens.separator` for card outlines.

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::effects_library::tests::effects_library_search_well_uses_craft_theme_field_tokens`
Expected: PASS (ok)

- [ ] **Step 5: Run all effects library tests to verify no regressions**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib ui::effects_library::tests`
Expected: PASS (all 35 passed)

- [ ] **Step 6: Commit**

```bash
git add src/ui/effects_library.rs src/ui/layout.rs
git commit -m "feat(ui): align effects library wells and dividers with craft_theme tokens"
```

---

### Task 4: Full Regression & Verification Gate

**Files:**
- Test: entire crate test suite

- [ ] **Step 1: Run full unit test suite**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --lib`
Expected: All 941+ tests PASS with 0 failures.

- [ ] **Step 2: Run downloader crate tests**

Run: `cargo test -p pealayer-downloader`
Expected: PASS with 0 failures.

- [ ] **Step 3: Run compiler checks across all targets**

Run: `cargo check --lib --bins`
Expected: Clean exit 0 with 0 errors.
