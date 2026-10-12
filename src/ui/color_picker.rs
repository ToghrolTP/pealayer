//! Compact RGB editor shared by channel presentation and custom accents.
//! The swatch is a click target inside the text field's reserved leading margin.
use eframe::egui;

pub fn configure_language(ctx: &egui::Context, language: crate::config::AppLanguage) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("color-picker-language"), language));
}
fn palette_tr(ui: &egui::Ui, key: &'static str) -> String {
    let language = ui.data(|data| data.get_temp::<crate::config::AppLanguage>(egui::Id::new("color-picker-language"))).unwrap_or(crate::config::AppLanguage::English);
    crate::ui::i18n::tr(language, key)
}

#[derive(serde::Deserialize)]
pub struct PaletteSwatch {
    pub label: String,
    pub hex: String,
}

pub fn semantic_swatches() -> &'static [PaletteSwatch] {
    static SWATCHES: std::sync::LazyLock<Vec<PaletteSwatch>> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("../../assets/themes/ui-colors.json"))
            .expect("valid shared UI swatches")
    });
    &SWATCHES
}

fn recent_id() -> egui::Id {
    egui::Id::new("pealayer-recent-colors")
}

pub fn remember_color(ctx: &egui::Context, color: egui::Color32) {
    ctx.data_mut(|data| {
        let mut colors = data
            .get_temp::<Vec<egui::Color32>>(recent_id())
            .unwrap_or_default();
        colors.retain(|previous| *previous != color);
        colors.insert(0, color);
        colors.truncate(12);
        data.insert_temp(recent_id(), colors);
    });
}

fn recent_row(ui: &mut egui::Ui) -> Option<egui::Color32> {
    let colors = ui.data(|data| {
        data.get_temp::<Vec<egui::Color32>>(recent_id())
            .unwrap_or_default()
    });
    if colors.is_empty() {
        return None;
    }
    ui.label(palette_tr(ui, "Recent colors"));
    let mut selected = None;
    ui.horizontal_wrapped(|ui| {
        for color in colors {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
            ui.painter().circle_filled(rect.center(), 7.0, color);
            ui.painter().circle_stroke(
                rect.center(),
                7.0,
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if response
                .on_hover_text(format!(
                    "#{:02X}{:02X}{:02X}",
                    color.r(),
                    color.g(),
                    color.b()
                ))
                .clicked()
            {
                selected = Some(color);
            }
        }
    });
    selected
}

fn swatch_row(ui: &mut egui::Ui) -> Option<egui::Color32> {
    let mut selected = recent_row(ui);
    ui.label(palette_tr(ui, "Pealayer colors"));
    ui.horizontal_wrapped(|ui| {
        for swatch in semantic_swatches() {
            let [r, g, b] = crate::config::parse_rgb_hex(&swatch.hex).expect("valid shared swatch");
            let color = egui::Color32::from_rgb(r, g, b);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
            ui.painter().circle_filled(rect.center(), 7.0, color);
            ui.painter().circle_stroke(
                rect.center(),
                7.0,
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            let label = match swatch.label.as_str() { "Red" => "Red", "Orange" => "Orange", "Amber" => "Amber",
                "Pealayer green" => "Pealayer green", "Blue" => "Blue", "Cyan" => "Cyan", "Violet" => "Violet", "Grey" => "Grey", "White" => "White", _ => "Black" };
            if response.on_hover_text(palette_tr(ui, label)).clicked() {
                selected = Some(color);
            }
        }
    });
    if let Some(color) = selected {
        remember_color(ui.ctx(), color);
    }
    selected
}

/// One full-width Custom choice. The inset is a preview, not a second item;
/// the actual editable HEX field lives in the palette opened by this row.
pub fn custom_menu_row(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    hex: &str,
    rgb: [u8; 3],
) -> egui::Response {
    let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let response = crate::ui::dropdown::choice_with_close(ui, selected, label, Some(color), false);
    let field = egui::Rect::from_center_size(
        egui::pos2(response.rect.right() - 70.0, response.rect.center().y),
        egui::vec2(124.0, (response.rect.height() - 4.0).max(1.0)),
    );
    ui.painter().rect(
        field,
        3.0,
        ui.visuals().extreme_bg_color,
        ui.visuals().widgets.inactive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().with_clip_rect(field.shrink(2.0)).text(
        field.left_center() + egui::vec2(7.0,0.0),
        egui::Align2::LEFT_CENTER,
        hex,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    response
}

/// This palette is anchored outside the closed dropdown: no competing nested
/// menu close event can eat the click that opens it.
pub fn palette_popup(
    ui: &mut egui::Ui,
    anchor: &egui::Response,
    id: egui::Id,
    open: bool,
    hex: &mut String,
    fallback: [u8; 3],
) -> bool {
    let keep_open = egui::Popup::is_id_open(ui.ctx(), id);
    let draft_id = id.with("hex-draft");
    let mut draft = ui
        .data(|data| data.get_temp::<String>(draft_id))
        .unwrap_or_else(|| hex.clone());
    if open {
        draft = hex.clone();
    }
    let mut changed = false;
    egui::Popup::menu(anchor)
        .id(id)
        .open_memory(Some(egui::SetOpenCommand::Bool(open || keep_open)))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(280.0);
            if let Some(color) = swatch_row(ui) {
                draft = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
                changed = true;
            }
            ui.separator();
            let field = ui.add_sized(
                [ui.available_width(), crate::ui::control_density::height(ui)],
                crate::ui::dialog::singleline_text_edit(ui, &mut draft)
                    .font(egui::TextStyle::Button.resolve(ui.style()))
                    .char_limit(7)
                    .hint_text("#RRGGBB"),
            );
            let rgb = crate::config::parse_rgb_hex(&draft).unwrap_or(fallback);
            let mut color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            if field.changed() && crate::config::parse_rgb_hex(&draft).is_some() {
                changed = true;
            }
            if egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            ) {
                draft = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
                changed = true;
            }
            if (field.lost_focus() || ui.input(|input| input.pointer.any_released()))
                && let Some([r, g, b]) = crate::config::parse_rgb_hex(&draft)
            {
                remember_color(ui.ctx(), egui::Color32::from_rgb(r, g, b));
            }
        });
    if changed {
        *hex = draft.clone();
    }
    if egui::Popup::is_id_open(ui.ctx(), id) {
        ui.data_mut(|data| data.insert_temp(draft_id, draft));
    } else {
        ui.data_mut(|data| data.remove::<String>(draft_id));
    }
    changed
}

/// Compact RGB-valued control with the same swatches as HEX fields.
pub fn color_button_srgba(ui: &mut egui::Ui, color: &mut egui::Color32) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::Vec2::splat(crate::ui::control_density::height(ui)), egui::Sense::click());
    ui.painter().circle_filled(rect.center(), 8.0, *color);
    ui.painter().circle_stroke(
        rect.center(),
        8.0,
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if let Some(selected) = swatch_row(ui) {
                *color = selected;
                response.mark_changed();
            }
            ui.separator();
            if egui::color_picker::color_picker_color32(
                ui,
                color,
                egui::color_picker::Alpha::OnlyBlend,
            ) {
                response.mark_changed();
            }
            if ui.input(|input| input.pointer.any_released()) {
                remember_color(ui.ctx(), *color);
            }
        });
    response
}

pub fn color_button_srgb(ui: &mut egui::Ui, rgb: &mut [u8; 3]) -> egui::Response {
    let mut color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let response = color_button_srgba(ui, &mut color);
    if response.changed() {
        *rgb = [color.r(), color.g(), color.b()];
    }
    response
}

pub fn color_field(
    ui: &mut egui::Ui,
    hex: &mut String,
    fallback: [u8; 3],
    width: f32,
) -> egui::Response {
    let id = ui.next_auto_id().with("rgb-color-field");
    let mut response = ui.add_sized(
        [width, crate::ui::control_density::height(ui)],
        crate::ui::dialog::singleline_text_edit(ui, hex)
            .font(egui::TextStyle::Button.resolve(ui.style()))
            .id(id)
            .desired_width(width)
            .margin(egui::Margin {
                left: 30,
                right: 6,
                top: 0,
                bottom: 0,
            })
            .char_limit(7)
            .hint_text("#RRGGBB"),
    );
    let rgb = crate::config::parse_rgb_hex(hex).unwrap_or(fallback);
    let swatch_rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 15.0, response.rect.center().y),
        egui::vec2(20.0, 20.0),
    );
    let swatch = ui
        .interact(swatch_rect, id.with("swatch"), egui::Sense::click())
        .on_hover_text("Choose color");
    ui.painter().circle_filled(
        swatch_rect.center(),
        6.0,
        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
    );
    ui.painter().circle_stroke(
        swatch_rect.center(),
        6.0,
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    egui::Popup::menu(&swatch)
        .id(id.with("palette"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if let Some(selected) = swatch_row(ui) {
                *hex = format!(
                    "#{:02X}{:02X}{:02X}",
                    selected.r(),
                    selected.g(),
                    selected.b()
                );
                response.mark_changed();
            }
            ui.separator();
            let mut color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            if egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            ) {
                *hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
                response.mark_changed();
            }
            if ui.input(|input| input.pointer.any_released()) {
                remember_color(ui.ctx(), color);
            }
        });
    if response.lost_focus()
        && let Some([r, g, b]) = crate::config::parse_rgb_hex(hex)
    {
        remember_color(ui.ctx(), egui::Color32::from_rgb(r, g, b));
    }
    response
}

pub struct ColorChoice { pub value: String, pub label: String, pub rgb: [u8; 3] }

fn palette_contents(ui: &mut egui::Ui, id: egui::Id, hex: &mut String, fallback: [u8; 3]) -> bool {
    let (mut draft, baseline) = ui.data(|data| data.get_temp::<(String, String)>(id)).unwrap_or_else(|| (hex.clone(), hex.clone()));
    if baseline != *hex { draft = hex.clone(); }
    let mut changed = false;
    if let Some(color) = swatch_row(ui) {
        draft = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        changed = true;
    }
    ui.separator();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let frame = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 0)).fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke).corner_radius(3.0);
    let field = ui.add_sized([ui.available_width(), crate::ui::control_density::height(ui)], crate::ui::dialog::singleline_text_edit(ui, &mut draft)
        .id(id.with("input")).font(font).frame(frame).char_limit(7).hint_text("#RRGGBB"));
    let rgb = crate::config::parse_rgb_hex(&draft).unwrap_or(fallback);
    let mut color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    changed |= field.changed() && crate::config::parse_rgb_hex(&draft).is_some();
    if egui::color_picker::color_picker_color32(ui, &mut color, egui::color_picker::Alpha::Opaque) {
        draft = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b()); changed = true;
    }
    if changed { *hex = draft.clone(); }
    if (field.lost_focus() || ui.input(|input| input.pointer.any_released())) && crate::config::parse_rgb_hex(&draft).is_some() {
        remember_color(ui.ctx(), color);
    }
    ui.data_mut(|data| data.insert_temp(id, (draft, hex.clone())));
    changed
}

/// Presets and the expanded Custom palette share one popup and one focus owner.
pub fn color_dropdown(ui: &mut egui::Ui, salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String, custom_hex: &mut String, options: &[ColorChoice], width: f32, custom_label: &str) -> egui::Response {
    let id = ui.make_persistent_id(egui::IdSalt::new(&salt));
    let expanded_id = id.with("custom-expanded");
    let draft_id = id.with("custom-hex");
    let mut expanded = ui.data(|data| data.get_temp::<bool>(expanded_id)).unwrap_or(false);
    let custom_rgb = crate::config::parse_rgb_hex(custom_hex).unwrap_or([0,120,212]);
    let selected = options.iter().find(|option| option.value == *value);
    let label = if value == "custom" { custom_label } else { selected.map(|option| option.label.as_str()).unwrap_or(value) };
    let rgb = selected.map(|option| option.rgb).unwrap_or(custom_rgb);
    let mut changed = false;
    let mut response = ui.push_id(&salt, |ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, crate::ui::control_density::height(ui)), egui::Sense::click());
        let visuals = ui.style().interact(&response);
        ui.painter().rect(rect, visuals.corner_radius, visuals.weak_bg_fill, visuals.bg_stroke, egui::StrokeKind::Inside);
        response
    }).inner;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), label));
    let galley = ui.painter().layout_no_wrap(label.to_owned(), egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color());
    ui.painter().with_clip_rect(response.rect.shrink(1.0)).galley(
        egui::pos2(response.rect.left()+27.0,response.rect.center().y-galley.size().y/2.0),galley,ui.visuals().text_color());
    crate::ui::dropdown::paint_swatch(ui, response.rect, egui::Color32::from_rgb(rgb[0],rgb[1],rgb[2]));
    crate::ui::control_density::paint_chevron(ui, egui::Rect::from_center_size(
        response.rect.right_center() - egui::vec2(10.0,0.0), egui::vec2(14.0,14.0)), ui.visuals().text_color());
    let max_height = (ui.ctx().content_rect().height() - 32.0).clamp(120.0,720.0);
    if response.has_focus() && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowDown)
        || input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowUp)) {
        egui::Popup::open_id(ui.ctx(),id.with("color-popup"));
    }
    let output = egui::Popup::menu(&response).id(id.with("color-popup")).width(width)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_width(width.max(310.0).min((ui.ctx().content_rect().width() - 32.0).max(120.0)));
            // Area remembers the collapsed size. Explicitly permit growth before
            // constructing the scroll viewport when Custom expands in place.
            ui.set_max_height(max_height);
            egui::ScrollArea::vertical().max_height(max_height).show(ui, |ui| {
                crate::ui::dropdown::menu_ui(ui, response.id, |ui| {
                    for option in options {
                        if crate::ui::dropdown::choice(ui, *value == option.value, &option.label,
                            Some(egui::Color32::from_rgb(option.rgb[0], option.rgb[1], option.rgb[2]))).clicked() {
                            *value = option.value.clone(); changed = true; ui.close();
                        }
                    }
                    if custom_menu_row(ui, *value == "custom", custom_label, custom_hex, custom_rgb).clicked() {
                        *value = "custom".into(); changed = true; expanded = !expanded;
                    }
                    if expanded {
                        ui.separator();
                        if palette_contents(ui, draft_id, custom_hex, custom_rgb) { *value = "custom".into(); changed = true; }
                    }
                });
            });
        });
    if output.is_some() { ui.data_mut(|data| data.insert_temp(expanded_id, expanded)); }
    else {
        crate::ui::dropdown::clear_menu(ui.ctx(),response.id);
        ui.data_mut(|data| { data.remove::<bool>(expanded_id); data.remove::<(String,String)>(draft_id); });
    }
    if changed { response.mark_changed(); }
    response
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_colors_are_session_only_bounded_and_most_recent_first() {
        let ctx = egui::Context::default();
        for red in 0..20 {
            remember_color(&ctx, egui::Color32::from_rgb(red, 0, 0));
        }
        remember_color(&ctx, egui::Color32::from_rgb(15, 0, 0));
        let colors = ctx.data(|data| data.get_temp::<Vec<egui::Color32>>(recent_id()).unwrap());
        assert_eq!(colors.len(), 12);
        assert_eq!(colors[0], egui::Color32::from_rgb(15, 0, 0));
        assert_eq!(colors.iter().filter(|color| color.r() == 15).count(), 1);
        let fresh = egui::Context::default();
        assert!(
            fresh
                .data(|data| data.get_temp::<Vec<egui::Color32>>(recent_id()))
                .is_none()
        );
    }

    #[test]
    fn shared_swatches_are_valid_and_include_semantic_marker_colors() {
        for swatch in semantic_swatches() {
            assert!(crate::config::parse_rgb_hex(&swatch.hex).is_some());
        }
        for expected in ["#EF4444", "#F59E0B", "#969696"] {
            assert!(
                semantic_swatches()
                    .iter()
                    .any(|swatch| swatch.hex == expected)
            );
        }
    }

    #[test]
    fn integrated_color_field_keeps_round_swatch_inside_text_field_in_both_themes() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut hex = "#38D27A".to_owned();
            let mut rect = egui::Rect::NOTHING;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                rect = color_field(ui, &mut hex, [0, 0, 0], 124.0).rect;
            });
            output.textures_delta.clear();
            assert!((rect.width() - 124.0).abs() < 0.1);
            let circles: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::epaint::Shape::Circle(circle) = &shape.shape {
                        Some(circle)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(circles.iter().any(|circle| circle.fill
                == egui::Color32::from_rgb(56, 210, 122)
                && rect.contains(circle.center)
                && circle.radius == 6.0));
            assert_eq!(hex, "#38D27A");
        }
    }

    #[test]
    fn color_field_hex_edit_and_swatch_popup_are_interactive() {
        let ctx = egui::Context::default();
        let mut hex = "#38D27A".to_owned();
        let frame = |events: Vec<egui::Event>, hex: &mut String| {
            let mut response = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    response = Some(color_field(ui, hex, [0, 0, 0], 124.0));
                },
            );
            output.textures_delta.clear();
            response.unwrap()
        };
        let response = frame(vec![], &mut hex);
        let text_pos = egui::pos2(response.rect.left() + 60.0, response.rect.center().y);
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            vec![egui::Event::PointerMoved(text_pos), click(text_pos, true)],
            &mut hex,
        );
        frame(vec![click(text_pos, false)], &mut hex);
        let command = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let changed = frame(
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: command,
                },
                egui::Event::Text("#123ABC".to_owned()),
            ],
            &mut hex,
        );
        assert!(changed.changed());
        assert_eq!(hex, "#123ABC");
        let swatch_pos = egui::pos2(response.rect.left() + 15.0, response.rect.center().y);
        frame(
            vec![
                egui::Event::PointerMoved(swatch_pos),
                click(swatch_pos, true),
            ],
            &mut hex,
        );
        frame(vec![click(swatch_pos, false)], &mut hex);
        assert!(
            egui::Popup::is_id_open(&ctx, response.id.with("palette")),
            "leading swatch opens the actual color picker"
        );
    }
}
