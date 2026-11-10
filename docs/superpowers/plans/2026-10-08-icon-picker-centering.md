# Icon Picker Centering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct grid tile alignment in the searchable icon picker dialog so icons and labels are properly centered horizontally and vertically inside their tile cells.

**Architecture:** Replace standard multi-line `egui::Button` tile rendering with dedicated `icon_grid_tile` widget logic. In `egui` 0.36, `AtomLayout` hardcodes painter origins at `item_rect.min`, causing `LayoutJob`s with `halign: Align::Center` (whose galley bounding box is centered around `x = 0`) to be drawn left-shifted by `galley.rect.width() / 2`. `icon_grid_tile` calculates exact center coordinates `rect.center() - galley.rect.center()` and draws the tile frame and centered galley.

**Tech Stack:** Rust, eframe/egui 0.36, egui-phosphor.

**Spec:** User reported icon picker dialog grid tiles not centered properly; pixel analysis confirmed text and glyphs shifted ~18px left within each tile.

## Global Constraints
- Preserve existing Phosphor icon sizing (18.0px) and caption text sizing (11.0px).
- Maintain 54.0px tile height and responsive column calculation.
- Ensure selection highlight, hover feedback, border stroke, and tooltips match existing visuals.
- All unit tests in `src/ui/icons.rs` must pass without regressions.

---

### Task 1: Custom Centered Grid Tile Widget in Icon Picker

**Files:**
- Modify: `src/ui/icons.rs`
- Test: `src/ui/icons.rs` (unit tests)

**Interfaces:**
- Produces: `fn icon_grid_tile(ui: &mut eframe::egui::Ui, size: eframe::egui::Vec2, text: eframe::egui::text::LayoutJob, selected: bool, hover_label: &str) -> eframe::egui::Response`

- [x] **Step 1: Write test asserting tile text is horizontally centered in button tile**
Added assertion to `grid_tiles_render_two_small_lines_with_borders_and_combobox_width_is_not_capped`:
```rust
let tile_rect = output.shapes.iter().find_map(|shape| match &shape.shape {
    egui::epaint::Shape::Rect(rect)
        if (rect.rect.height() - 54.0).abs() < 0.1
            && rect.stroke.width > 0.0
            && rect.rect.contains(tile.pos) =>
    {
        Some(rect.rect)
    }
    _ => None,
}).expect("enclosing button rect for tile");
let text_center_x = tile.pos.x + tile.galley.rect.center().x;
assert!(
    (text_center_x - tile_rect.center().x).abs() < 1.0,
    "icon tile text must be horizontally centered in button tile"
);
```

- [x] **Step 2: Run test to verify failure on uncentered implementation**
Prior to fix: `text_center_x` differed from `tile_rect.center().x` by ~18.0px (FAIL).

- [x] **Step 3: Implement `icon_grid_tile` and integrate into `searchable_icon_picker_contents`**
Defined `icon_grid_tile` to compute `pos = rect.center() - galley.rect.center().to_vec2()`, paint tile rect with interactive visuals, and paint galley at centered position.

- [x] **Step 4: Run test to verify it passes**
Run `cargo test --lib icons`.
Expected: PASS (all 13 tests passed).

- [x] **Step 5: Run integration tests**
Run `cargo test --test workspace_dock_test`.
Expected: PASS.
