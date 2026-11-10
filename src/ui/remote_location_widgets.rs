//! Shared List/Grid entry and metadata presentation. No directory discovery here.
use crate::remote_location::{BrowserState, RemoteEntry};
use crate::ui::{dialog, icons};
use eframe::egui;

#[derive(Clone, Default)]
struct PreviewCache {
    textures: std::collections::HashMap<egui::Id, egui::TextureHandle>,
    recent: std::collections::VecDeque<egui::Id>,
}
impl PreviewCache {
    fn get(&mut self, id: egui::Id) -> Option<egui::TextureHandle> {
        let value = self.textures.get(&id)?.clone();
        self.recent.retain(|entry| *entry != id);
        self.recent.push_back(id);
        Some(value)
    }
    fn insert(&mut self, id: egui::Id, texture: egui::TextureHandle) {
        self.recent.retain(|entry| *entry != id);
        self.recent.push_back(id);
        self.textures.insert(id, texture);
        while self.recent.len() > 96 {
            if let Some(old) = self.recent.pop_front() {
                self.textures.remove(&old);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    Open(String),
    Select(String, bool),
    Inspect(String),
    Retry(String),
}

fn mix(base: egui::Color32, tint: egui::Color32, amount: f32) -> egui::Color32 {
    egui::lerp(egui::Rgba::from(base)..=egui::Rgba::from(tint), amount).into()
}

pub(super) fn apply_style(ui: &mut egui::Ui) {
    let accent = crate::ui::craft_theme::current(ui.ctx()).accent;
    let base = ui.visuals().widgets.inactive.bg_fill;
    let hover = ui.visuals().widgets.hovered.bg_fill;
    let text = ui.visuals().text_color();
    let visuals = ui.visuals_mut();
    visuals.selection.bg_fill = mix(base, accent, 0.17);
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.active.bg_fill = mix(hover, accent, 0.12);
    visuals.widgets.active.weak_bg_fill = visuals.widgets.active.bg_fill;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.active.bg_stroke = visuals.widgets.hovered.bg_stroke;
}

pub(super) fn item_geometry(ui: &egui::Ui, grid: bool) -> (usize, f32, f32) {
    let scroll = &ui.spacing().scroll;
    let width = ui.available_width()
        - if scroll.floating {
            0.0
        } else {
            scroll.bar_width + scroll.bar_inner_margin + scroll.bar_outer_margin
        };
    let columns = if grid {
        ((width + 8.0) / 190.0).floor().max(1.0) as usize
    } else {
        1
    };
    let labels = ui.text_style_height(&egui::TextStyle::Body)
        + ui.text_style_height(&egui::TextStyle::Small);
    let height = if grid {
        (116.0 + labels).max(158.0)
    } else {
        (labels + 20.0).max(56.0)
    };
    (
        columns,
        ((width - 8.0 * columns.saturating_sub(1) as f32) / columns as f32).max(1.0),
        height,
    )
}

fn surface(ui: &egui::Ui, response: &egui::Response, selected: bool) {
    let pressed = response.is_pointer_button_down_on();
    let hover = response.contains_pointer() || response.highlighted();
    let base = ui.visuals().widgets.inactive.weak_bg_fill;
    let accent = crate::ui::craft_theme::current(ui.ctx()).accent;
    let fill = if selected {
        mix(
            if hover {
                ui.visuals().widgets.hovered.bg_fill
            } else {
                base
            },
            accent,
            if pressed { 0.25 } else { 0.16 },
        )
    } else if pressed {
        ui.visuals().widgets.active.bg_fill
    } else if hover {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        base
    };
    let stroke = if response.has_focus() {
        ui.visuals().selection.stroke
    } else if selected {
        egui::Stroke::new(
            1.0,
            mix(ui.visuals().widgets.inactive.bg_stroke.color, accent, 0.5),
        )
    } else if hover {
        ui.visuals().widgets.hovered.bg_stroke
    } else {
        ui.visuals().widgets.inactive.bg_stroke
    };
    ui.painter()
        .rect(response.rect, 7.0, fill, stroke, egui::StrokeKind::Inside);
}

fn activate(entry: &RemoteEntry, play: bool) -> Action {
    if entry.is_dir {
        Action::Open(entry.url.clone())
    } else {
        Action::Select(entry.url.clone(), play)
    }
}

pub(super) fn entry(
    ui: &mut egui::Ui,
    entry: &RemoteEntry,
    parent: bool,
    state: &BrowserState,
    grid: bool,
    size: egui::Vec2,
) -> Option<Action> {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let response = response.on_hover_cursor(if state.loading {
        egui::CursorIcon::Progress
    } else {
        egui::CursorIcon::PointingHand
    });
    surface(ui, &response, state.selected.as_deref() == Some(&entry.url));
    let thumb = if grid {
        egui::Rect::from_min_max(
            rect.min + egui::vec2(7.0, 7.0),
            egui::pos2(rect.right() - 7.0, rect.top() + 96.0),
        )
    } else {
        egui::Rect::from_min_size(
            rect.min + egui::vec2(7.0, 7.0),
            egui::vec2(68.0, size.y - 14.0),
        )
    };
    thumbnail(ui, entry, parent, state.thumbnails, thumb);
    let text = if grid {
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 10.0, thumb.bottom() + 7.0),
            rect.max - egui::vec2(38.0, 4.0),
        )
    } else {
        egui::Rect::from_min_max(
            rect.min + egui::vec2(85.0, 7.0),
            rect.max - egui::vec2(43.0, 4.0),
        )
    };
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(text)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.set_clip_rect(ui.clip_rect().intersect(text));
            ui.add(egui::Label::new(&entry.name).selectable(false).truncate())
                .on_hover_text(&entry.name);
            let details = super::entry_details(entry, parent).unwrap_or_else(|| {
                if parent {
                    "Go up one level"
                } else if entry.is_dir {
                    "Folder"
                } else if entry.playable {
                    "Media file"
                } else {
                    "File"
                }
                .into()
            });
            ui.add(
                egui::Label::new(egui::RichText::new(details).small().weak())
                    .selectable(false)
                    .truncate(),
            );
        },
    );
    let mut action = None;
    let button = egui::Rect::from_center_size(
        egui::pos2(
            rect.right() - 20.0,
            if grid {
                rect.bottom() - 25.0
            } else {
                rect.center().y
            },
        ),
        egui::vec2(28.0, 28.0),
    );
    let enabled = !state.loading && (entry.playable || entry.is_dir);
    let action_response = ui
        .interact(
            button,
            ui.id().with("activate"),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        )
        .on_hover_cursor(if enabled {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::NotAllowed
        });
    if action_response.hovered() && enabled {
        ui.painter()
            .rect_filled(button, 5.0, ui.visuals().widgets.hovered.bg_fill);
    }
    centered(
        ui,
        button,
        if entry.is_dir {
            icons::ARROW_RIGHT
        } else {
            icons::PLAY
        },
        18.0,
        if enabled {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color().gamma_multiply(0.5)
        },
    );
    action_response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            enabled,
            if entry.is_dir { "Open folder" } else { "Play" },
        )
    });
    if action_response
        .on_hover_text(if entry.is_dir { "Open folder" } else { "Play" })
        .clicked()
        && enabled
    {
        action = Some(activate(entry, true));
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            !state.loading,
            state.selected.as_deref() == Some(&entry.url),
            &entry.name,
        )
    });
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(ui.id().with("entry-geometry"), (rect, thumb, text, button))
    });
    if action.is_none() && !state.loading {
        if response.double_clicked() {
            if entry.is_dir || entry.playable {
                action = Some(activate(entry, true));
            }
        } else if response.clicked()
            || response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
        {
            response.request_focus();
            action = Some(activate(entry, false));
        }
    }
    response.context_menu(|ui| {
        apply_style(ui);
        ui.set_width(240.0);
        if !parent {
            ui.add(egui::Label::new(egui::RichText::new(&entry.name).small().weak()).truncate());
            ui.separator();
        }
        if ui
            .add_enabled(
                !state.loading && (entry.is_dir || entry.playable),
                egui::Button::new(format!(
                    "{}  {}",
                    if entry.is_dir {
                        icons::FOLDER_OPEN
                    } else {
                        icons::PLAY
                    },
                    if entry.is_dir { "Open folder" } else { "Play" }
                )),
            )
            .clicked()
        {
            action = Some(activate(entry, true));
            ui.close();
        }
        if !parent && ui.button(format!("{}  Properties", icons::INFO)).clicked() {
            action = Some(Action::Inspect(entry.url.clone()));
            ui.close();
        }
        if state.thumbnails
            && state
                .thumbnail_status
                .get(&entry.url)
                .is_some_and(|status| status.starts_with("error:"))
            && ui
                .button(format!("{}  Retry thumbnail", icons::ARROW_CLOCKWISE))
                .clicked()
        {
            action = Some(Action::Retry(entry.url.clone()));
            ui.close();
        }
        ui.separator();
        if ui.button(format!("{}  Copy link", icons::COPY)).clicked() {
            ui.ctx().copy_text(entry.url.clone());
            ui.close();
        }
        if !parent && ui.button(format!("{}  Copy name", icons::TEXT_T)).clicked() {
            ui.ctx().copy_text(entry.name.clone());
            ui.close();
        }
    });
    action
}

fn centered(ui: &egui::Ui, rect: egui::Rect, text: &str, size: f32, color: egui::Color32) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.into(), egui::FontId::proportional(size), color);
    let ink = if galley.mesh_bounds.is_finite() {
        galley.mesh_bounds
    } else {
        galley.rect
    };
    ui.painter()
        .galley(rect.center() - ink.center().to_vec2(), galley, color);
}

fn thumbnail(
    ui: &mut egui::Ui,
    entry: &RemoteEntry,
    parent: bool,
    enabled: bool,
    rect: egui::Rect,
) {
    ui.painter()
        .rect_filled(rect, 5.0, ui.visuals().extreme_bg_color);
    if enabled && entry.playable {
        match crate::remote_location::thumbnail(&entry.url, ui.ctx()) {
            Ok(Some(path)) => {
                let id = egui::Id::new(("remote-thumb", &entry.url));
                let cache = egui::Id::new("remote-preview-cache");
                let texture = ui
                    .ctx()
                    .data_mut(|data| data.get_temp_mut_or_default::<PreviewCache>(cache).get(id))
                    .or_else(|| {
                        let image = image::open(path).ok()?.thumbnail(360, 220).to_rgba8();
                        let texture = ui.ctx().load_texture(
                            format!("remote-thumb:{id:?}"),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [image.width() as usize, image.height() as usize],
                                image.as_raw(),
                            ),
                            egui::TextureOptions::LINEAR,
                        );
                        ui.ctx().data_mut(|data| {
                            data.get_temp_mut_or_default::<PreviewCache>(cache)
                                .insert(id, texture.clone())
                        });
                        Some(texture)
                    });
                if let Some(texture) = texture {
                    let size = texture.size_vec2();
                    let scale = (rect.width() / size.x).min(rect.height() / size.y);
                    ui.painter().image(
                        texture.id(),
                        egui::Rect::from_center_size(rect.center(), size * scale),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    return;
                }
                placeholder(
                    ui,
                    rect,
                    Some("The downloaded preview could not be decoded."),
                );
                return;
            }
            Ok(None) => {
                placeholder(ui, rect, None);
                return;
            }
            Err(error) => {
                placeholder(ui, rect, Some(&error));
                return;
            }
        }
    }
    centered(
        ui,
        rect,
        if parent {
            icons::ARROW_UP
        } else if entry.is_dir {
            icons::FOLDER_OPEN
        } else if entry.playable {
            icons::FILE_VIDEO
        } else {
            icons::FILE
        },
        24.0,
        ui.visuals().weak_text_color(),
    );
}

fn placeholder(ui: &mut egui::Ui, rect: egui::Rect, error: Option<&str>) {
    if error.is_some() {
        centered(
            ui,
            rect,
            icons::WARNING,
            20.0,
            ui.visuals().weak_text_color(),
        );
    } else {
        let center = rect.center() - egui::vec2(0.0, 7.0);
        let ink = ui.visuals().weak_text_color();
        ui.painter().circle_stroke(
            center,
            7.0,
            egui::Stroke::new(1.0, ink.gamma_multiply(0.42)),
        );
        egui::Spinner::new().size(16.0).paint_at(
            ui,
            egui::Rect::from_center_size(center, egui::Vec2::splat(16.0)),
        );
        centered(
            ui,
            egui::Rect::from_center_size(
                rect.center() + egui::vec2(0.0, 10.0),
                egui::vec2(rect.width(), 13.0),
            ),
            "Loading…",
            10.0,
            ink,
        );
    }
    ui.interact(rect, ui.id().with("preview-hint"), egui::Sense::hover())
        .on_hover_text(error.unwrap_or("Fetching thumbnail…"));
}

pub(super) fn properties(ctx: &egui::Context, entry: &RemoteEntry) -> bool {
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::new("remote-file-properties"))
        .frame(dialog::opaque_window_frame_from_context(ctx))
        .show(ctx, |ui| {
            apply_style(ui);
            let width = (ctx.content_rect().width() - 64.0).clamp(220.0, 540.0);
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{}  Properties", icons::INFO)).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= dialog::close_button(ui, "Close properties").clicked();
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((ctx.content_rect().height() - 220.0).max(120.0))
                .show(ui, |ui| {
                    let font = egui::TextStyle::Body.resolve(ui.style());
                    let label_width = ["Name", "Type", "Host", "Size", "Modified", "Location"]
                        .iter()
                        .map(|label| {
                            ui.painter()
                                .layout_no_wrap(
                                    (*label).into(),
                                    font.clone(),
                                    ui.visuals().text_color(),
                                )
                                .size()
                                .x
                        })
                        .fold(0.0_f32, f32::max);
                    let value_width = (width - label_width - 16.0).max(60.0);
                    egui::Grid::new("remote-property-fields")
                        .num_columns(2)
                        .spacing([16.0, 12.0])
                        .show(ui, |ui| {
                            let host = url::Url::parse(&entry.url)
                                .ok()
                                .and_then(|url| url.host_str().map(str::to_owned))
                                .unwrap_or_default();
                            for (label, value) in [
                                ("Name", entry.name.clone()),
                                (
                                    "Type",
                                    if entry.is_dir {
                                        "Folder"
                                    } else if entry.playable {
                                        "Playable media"
                                    } else {
                                        "File"
                                    }
                                    .into(),
                                ),
                                ("Host", host),
                                (
                                    "Size",
                                    if entry.is_dir {
                                        "Not applicable".into()
                                    } else if entry.size_bytes.is_none() {
                                        "Not provided by server".into()
                                    } else {
                                        super::format_bytes(entry.size_bytes)
                                    },
                                ),
                                (
                                    "Modified",
                                    entry
                                        .modified
                                        .clone()
                                        .unwrap_or_else(|| "Not provided by server".into()),
                                ),
                                ("Location", crate::media::redact_media_target(&entry.url)),
                            ] {
                                ui.weak(label);
                                ui.scope(|ui| {
                                    ui.set_max_width(value_width);
                                    ui.add(egui::Label::new(value).wrap().selectable(true));
                                });
                                ui.end_row();
                            }
                        });
                });
            ui.separator();
            ui.horizontal(|ui| {
                if dialog::action_button(ui, icons::COPY, "Copy link").clicked() {
                    ui.ctx().copy_text(entry.url.clone());
                }
                if dialog::action_button(ui, icons::X, "Close").clicked() {
                    close = true;
                }
            });
        });
    close || modal.should_close()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn properties_wrap_long_metadata_and_escape_closes_only_the_inspection() {
        let ctx = setup(true, crate::config::ColorPalette::Studio, 22.0);
        let file = fixture();
        let render = |events| {
            let mut close = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 500.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    close = properties(ui.ctx(), &file);
                },
            );
            output.textures_delta.clear();
            (output, close)
        };
        render(vec![]);
        let (output, close) = render(vec![]);
        assert!(!close);
        fn find(shape: &egui::Shape, name: &str) -> Option<egui::Rect> {
            match shape {
                egui::Shape::Text(text) if text.galley.job.text == name => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, name)),
                _ => None,
            }
        }
        let name = output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, &file.name))
            .expect("Properties displays the full name in a wrapping value field");
        assert!(name.width() < 300.0);
        let (_, close) = render(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }]);
        assert!(close);
    }
    fn fixture() -> RemoteEntry {
        RemoteEntry {
            name: format!("{} movie.mkv", "A long title ".repeat(40)),
            url: "https://files.invalid/movie.mkv".into(),
            is_dir: false,
            playable: true,
            size_bytes: Some(2048),
            modified: Some("2026-01-02".into()),
        }
    }
    fn setup(dark: bool, palette: crate::config::ColorPalette, font: f32) -> egui::Context {
        let ctx = egui::Context::default();
        let mut style = egui::Style::default();
        style.visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        crate::ui::palette::apply(&mut style.visuals, palette);
        for kind in [egui::TextStyle::Body, egui::TextStyle::Button] {
            style
                .text_styles
                .insert(kind, egui::FontId::proportional(font));
        }
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::proportional(font * 0.8),
        );
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ctx.set_style_of(ctx.theme(), style);
        ctx
    }
    fn frame(
        ctx: &egui::Context,
        selected: bool,
        grid: bool,
        width: f32,
        events: Vec<egui::Event>,
    ) -> (
        egui::FullOutput,
        (egui::Rect, egui::Rect, egui::Rect, egui::Rect),
        Option<Action>,
    ) {
        let file = fixture();
        let state = BrowserState {
            selected: selected.then(|| file.url.clone()),
            thumbnails: false,
            ..Default::default()
        };
        let mut geometry = None;
        let mut action = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width + 16.0, 500.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                apply_style(ui);
                let (_, item_width, height) = item_geometry(ui, grid);
                ui.horizontal(|ui| {
                    action = entry(
                        ui,
                        &file,
                        false,
                        &state,
                        grid,
                        egui::vec2(item_width, height),
                    );
                    geometry = ui.data(|data| data.get_temp(ui.id().with("entry-geometry")));
                });
            },
        );
        output.textures_delta.clear(); // Actual headless paint/input; no GPU uploads.
        (output, geometry.unwrap(), action)
    }
    #[test]
    fn file_captions_stack_even_inside_horizontal_rows_of_cards() {
        fn find(shape: &egui::Shape, expected: &str) -> Option<egui::Rect> {
            match shape {
                egui::Shape::Text(text) if text.galley.job.text == expected => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, expected)),
                _ => None,
            }
        }
        let file = fixture();
        let details = super::super::entry_details(&file, false).unwrap();
        for grid in [false, true] {
            let ctx = setup(true, crate::config::ColorPalette::Studio, 22.0);
            let (output, (_, _, text, _), _) = frame(&ctx, false, grid, 700.0, vec![]);
            let name = output
                .shapes
                .iter()
                .find_map(|shape| find(&shape.shape, &file.name))
                .unwrap();
            let meta = output
                .shapes
                .iter()
                .find_map(|shape| find(&shape.shape, &details))
                .unwrap();
            assert!(meta.top() >= name.bottom());
            assert!(text.contains_rect(meta));
        }
    }
    fn fill(output: &egui::FullOutput, rect: egui::Rect) -> egui::Color32 {
        fn find(shape: &egui::Shape, rect: egui::Rect) -> Option<egui::Color32> {
            match shape {
                egui::Shape::Rect(shape) if shape.rect == rect => Some(shape.fill),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, rect)),
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, rect))
            .unwrap()
    }
    #[test]
    fn list_and_grid_have_real_hover_pressed_selected_states_without_geometry_or_click_regressions()
    {
        for dark in [false, true] {
            for palette in [
                crate::config::ColorPalette::Native,
                crate::config::ColorPalette::Studio,
            ] {
                for grid in [false, true] {
                    for width in [280.0, 700.0] {
                        for font in [14.0, 28.0] {
                            let ctx = setup(dark, palette, font);
                            frame(&ctx, false, grid, width, vec![]);
                            let (normal, geometry, _) = frame(
                                &ctx,
                                false,
                                grid,
                                width,
                                vec![egui::Event::PointerMoved(egui::pos2(-10.0, -10.0))],
                            );
                            let (rect, thumb, text, button) = geometry;
                            assert!(
                                rect.contains_rect(thumb)
                                    && rect.contains_rect(text)
                                    && rect.contains_rect(button)
                            );
                            assert!(!thumb.intersects(text));
                            assert!(!text.intersects(button));
                            let point = text.left_top() + egui::vec2(10.0, 5.0);
                            let (hover, hover_geometry, _) = frame(
                                &ctx,
                                false,
                                grid,
                                width,
                                vec![egui::Event::PointerMoved(point)],
                            );
                            assert_eq!(geometry, hover_geometry);
                            assert_ne!(fill(&normal, rect), fill(&hover, rect));
                            let (pressed, _, _) = frame(
                                &ctx,
                                false,
                                grid,
                                width,
                                vec![egui::Event::PointerButton {
                                    pos: point,
                                    button: egui::PointerButton::Primary,
                                    pressed: true,
                                    modifiers: Default::default(),
                                }],
                            );
                            assert_ne!(fill(&hover, rect), fill(&pressed, rect));
                            let (_, released, action) = frame(
                                &ctx,
                                false,
                                grid,
                                width,
                                vec![egui::Event::PointerButton {
                                    pos: point,
                                    button: egui::PointerButton::Primary,
                                    pressed: false,
                                    modifiers: Default::default(),
                                }],
                            );
                            assert_eq!(released, geometry);
                            assert_eq!(action, Some(Action::Select(fixture().url, false)));
                            let (selected, _, _) = frame(
                                &ctx,
                                true,
                                grid,
                                width,
                                vec![egui::Event::PointerMoved(egui::pos2(-10.0, -10.0))],
                            );
                            assert_ne!(fill(&normal, rect), fill(&selected, rect));
                            assert_ne!(
                                fill(&selected, rect),
                                crate::ui::craft_theme::current(&ctx).accent
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn file_menu_is_bounded_and_properties_uses_the_inspection_action_not_playback() {
        let ctx = setup(true, crate::config::ColorPalette::Studio, 14.0);
        let (_, (_, _, text, _), _) = frame(&ctx, false, false, 500.0, vec![]);
        let point = text.center();
        frame(
            &ctx,
            false,
            false,
            500.0,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            &ctx,
            false,
            false,
            500.0,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        let (output, _, _) = frame(&ctx, false, false, 500.0, vec![]);
        fn collect(shape: &egui::Shape, texts: &mut Vec<(String, egui::Rect, f32)>) {
            match shape {
                egui::Shape::Text(text) => texts.push((
                    text.galley.job.text.clone(),
                    text.galley.rect.translate(text.pos.to_vec2()),
                    text.galley.job.sections[0].format.font_id.size,
                )),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, texts);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for shape in &output.shapes {
            collect(&shape.shape, &mut texts);
        }
        let mut property = None;
        for (text, rect, font) in &texts {
            if text.contains("Properties") {
                property = Some(*rect);
            }
            if *text == fixture().name && *font < 13.0 {
                assert!(rect.width() <= 261.0);
            }
        }
        assert!(
            property.is_some(),
            "Properties missing; popup={}, painted={:?}",
            egui::Popup::is_any_open(&ctx),
            texts.iter().map(|(text, _, _)| text).collect::<Vec<_>>()
        );
        let point = property
            .expect("Properties is offered in the actual popup")
            .center();
        frame(
            &ctx,
            false,
            false,
            500.0,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let (_, _, action) = frame(
            &ctx,
            false,
            false,
            500.0,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(action, Some(Action::Inspect(fixture().url)));
    }
}
