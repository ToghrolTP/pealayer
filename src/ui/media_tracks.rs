use crate::app::{MediaTrackInfo, MediaTrackKey, MediaTrackType, PealayerApp};
use crate::ui::dropdown::DropdownUiExt;
use eframe::egui;

pub(crate) fn icon(kind: MediaTrackType) -> &'static str {
    match kind {
        MediaTrackType::Video => crate::ui::icons::FILE_VIDEO,
        MediaTrackType::Audio => crate::ui::icons::SPEAKER_HIGH,
        MediaTrackType::Subtitle => crate::ui::icons::SUBTITLES,
    }
}

pub(crate) fn track_label(track: &MediaTrackInfo) -> String {
    crate::ui::menu::format_track_label(track.id, track.language.as_deref(), track.title.as_deref())
}

pub(crate) fn current_track_label(app: &PealayerApp, kind: MediaTrackType) -> String {
    let current = app.current_media_track_id(kind);
    if current == "no" {
        return disabled_label(app, kind);
    }
    app.media_tracks
        .iter()
        .find(|track| track.kind == kind && track.id.to_string() == current)
        .map(track_label)
        .unwrap_or_else(|| format!("{} {current}", app.tr("Track")))
}

fn disabled_label(app: &PealayerApp, kind: MediaTrackType) -> String {
    app.tr(match kind {
        MediaTrackType::Video => "Video disabled",
        MediaTrackType::Audio => "No audio track",
        MediaTrackType::Subtitle => "Subtitles hidden",
    })
}

pub(crate) fn draw_track_menu(app: &mut PealayerApp, ui: &mut egui::Ui, kind: MediaTrackType) {
    ui.set_min_width(230.0);
    ui.horizontal(|ui| {
        ui.label(icon(kind));
        ui.strong(app.tr(kind.label()));
    });
    ui.label(
        egui::RichText::new(current_track_label(app, kind))
            .small()
            .weak(),
    );
    ui.separator();

    if kind == MediaTrackType::Audio {
        let mut muted = app.is_muted;
        if ui
            .checkbox(&mut muted, app.tr("Mute audio"))
            .on_hover_text(app.tr("Mute playback without changing the selected audio track."))
            .changed()
        {
            app.set_audio_muted(muted);
        }
        ui.separator();
    }

    let tracks = app
        .media_tracks
        .iter()
        .filter(|track| track.kind == kind)
        .cloned()
        .collect::<Vec<_>>();
    let current = app.current_media_track_id(kind).to_string();
    let disabled = current == "no";
    if ui
        .dropdown_choice(disabled, disabled_label(app, kind))
        .clicked()
    {
        app.disable_media_track(kind);
        ui.close();
    }
    ui.separator();

    if tracks.is_empty() {
        ui.add_enabled(false, egui::Label::new(app.tr("No tracks available")));
        return;
    }
    for track in tracks {
        let selected = current == track.id.to_string();
        if ui.dropdown_choice(selected, track_label(&track)).clicked() {
            app.select_media_track(MediaTrackKey { kind, id: track.id });
            ui.close();
        }
    }
}

pub(crate) fn menu_button_label(kind: MediaTrackType) -> String {
    format!("{} {}", icon(kind), crate::ui::icons::CARET_DOWN)
}

pub(crate) const HEADER_BUTTON_HEIGHT: f32 = 22.0;
pub(crate) const HEADER_SELECTOR_WIDTH: f32 = 42.0;

/// Timeline bands have fixed heights independent of the general input density.
/// Allocate and paint the closed trigger only; popup content uses the parent UI.
pub(crate) fn header_button(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    glyph: &str,
    dropdown: bool,
    enabled: bool,
    active: bool,
    help: &str,
) -> egui::Response {
    let id = ui.make_persistent_id(id_salt);
    ui.add_enabled_ui(enabled, |ui| {
        let width = if dropdown {
            HEADER_SELECTOR_WIDTH
        } else {
            HEADER_BUTTON_HEIGHT
        };
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, HEADER_BUTTON_HEIGHT),
            egui::Sense::hover(),
        );
        let response = ui.interact(rect, id, egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, help));
        let open = egui::Popup::is_id_open(ui.ctx(), id.with("popup"));
        let tokens = super::craft_theme::current(ui.ctx());
        let style = super::craft_theme::icon_button_style(&tokens, true);
        let visual = if active || open {
            style.selected
        } else if response.is_pointer_button_down_on() {
            style.pressed.unwrap_or(style.hovered)
        } else if response.hovered() {
            style.hovered
        } else {
            style.normal
        };
        let ink = if enabled {
            visual.icon
        } else {
            ui.visuals().weak_text_color()
        };
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
        painter.rect(
            rect.shrink(0.5),
            style.corner_radius,
            visual.fill,
            egui::Stroke::new(1.0, visual.stroke.color),
            egui::StrokeKind::Inside,
        );
        let icon_rect = if dropdown {
            egui::Rect::from_min_size(rect.min, egui::vec2(24.0, rect.height()))
        } else {
            rect
        };
        let galley = painter.layout_no_wrap(glyph.into(), egui::FontId::proportional(13.0), ink);
        painter.galley(
            icon_rect.center() - galley.mesh_bounds.center().to_vec2(),
            galley,
            ink,
        );
        if dropdown {
            super::control_density::paint_chevron(
                ui,
                egui::Rect::from_min_max(egui::pos2(rect.right() - 18.0, rect.top()), rect.max),
                ink,
            );
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.shrink(1.0),
                style.corner_radius,
                style.focus_stroke,
                egui::StrokeKind::Inside,
            );
        }
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(help)
    })
    .inner
}

pub(crate) fn menu_button(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    kind: MediaTrackType,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let tooltip = format!(
        "{}: {}",
        app.tr(kind.label()),
        current_track_label(app, kind)
    );
    let response = header_button(ui, id_salt, icon(kind), true, true, false, &tooltip);
    egui::Popup::menu(&response).show(|ui| draw_track_menu(app, ui, kind));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_triggers_stay_inside_their_bands_across_density_font_and_theme() {
        for density in crate::config::UiControlDensity::ALL {
            for dark in [false, true] {
                for scale in [1.0, 1.25, 2.0] {
                    let ctx = egui::Context::default();
                    super::super::i18n::configure_ui_fonts(&ctx, false);
                    ctx.set_visuals(if dark {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    });
                    ctx.set_pixels_per_point(scale);
                    ctx.all_styles_mut(|style| {
                        super::super::control_density::configure(style, density);
                        style
                            .text_styles
                            .insert(egui::TextStyle::Button, egui::FontId::proportional(24.0));
                    });
                    let mut before = None;
                    for hovered in [false, true, false] {
                        let mut rects = Vec::new();
                        let mut output = ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    egui::vec2(600.0, 300.0),
                                )),
                                events: if hovered {
                                    vec![egui::Event::PointerMoved(egui::pos2(225.0, 26.0))]
                                } else {
                                    vec![]
                                },
                                ..Default::default()
                            },
                            |ui| {
                                for (i, height) in [26.0, 32.0, 40.0].into_iter().enumerate() {
                                    let band = egui::Rect::from_min_size(
                                        egui::pos2(10.0, 10.0 + i as f32 * 60.0),
                                        egui::vec2(250.0, height),
                                    );
                                    let mut child = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(band.shrink2(egui::vec2(4.0, 0.0)))
                                            .layout(egui::Layout::right_to_left(
                                                egui::Align::Center,
                                            )),
                                    );
                                    let selector = header_button(
                                        &mut child,
                                        ("selector", i),
                                        icon(MediaTrackType::Audio),
                                        true,
                                        true,
                                        false,
                                        "Audio",
                                    );
                                    let action = header_button(
                                        &mut child,
                                        ("action", i),
                                        crate::ui::icons::SPEAKER_HIGH,
                                        false,
                                        true,
                                        false,
                                        "Mute",
                                    );
                                    assert_eq!(
                                        selector.rect.size(),
                                        egui::vec2(HEADER_SELECTOR_WIDTH, HEADER_BUTTON_HEIGHT)
                                    );
                                    assert_eq!(
                                        action.rect.size(),
                                        egui::vec2(HEADER_BUTTON_HEIGHT, HEADER_BUTTON_HEIGHT)
                                    );
                                    assert!(
                                        band.contains_rect(selector.rect)
                                            && band.contains_rect(action.rect)
                                    );
                                    assert!(
                                        (selector.rect.center().y - band.center().y).abs() < 0.01
                                    );
                                    assert!(action.rect.right() < selector.rect.left());
                                    rects.push((selector.rect, action.rect));
                                }
                            },
                        );
                        if let Some(previous) = &before {
                            assert_eq!(&rects, previous, "hover changed header geometry");
                        }
                        before = Some(rects);
                        output.textures_delta.clear();
                    }
                }
            }
        }
    }

    #[test]
    fn compact_timeline_selector_opens_a_full_size_menu_for_each_media_kind() {
        let _guard = crate::app::tests::lock_app_tests();
        for kind in [
            MediaTrackType::Video,
            MediaTrackType::Audio,
            MediaTrackType::Subtitle,
        ] {
            let ctx = egui::Context::default();
            let mut app = PealayerApp::default();
            app.engine_handle = crate::four_d::engine::memory_only_test_engine();
            let mut trigger = egui::Rect::NOTHING;
            for pass in 0..4 {
                let point = trigger.center();
                let events = match pass {
                    1 | 2 => vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed: pass == 1,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    _ => vec![],
                };
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(800.0, 500.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        let density = ui.spacing().interact_size.y;
                        trigger = menu_button(&mut app, ui, kind, "track-fixture").rect;
                        assert_eq!(
                            density,
                            ui.spacing().interact_size.y,
                            "trigger must not change popup or parent density"
                        );
                    },
                );
                if pass == 3 {
                    assert!(egui::Popup::is_any_open(&ctx), "selector failed to open");
                    assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("No tracks available"))));
                }
                output.textures_delta.clear();
            }
        }
    }
}
