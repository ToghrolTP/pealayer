//! FilmCraft 3-tier NLE program monitor controls:
//! Tier 1: Controls row (blue timecode, zoom dropdown, res dropdown, wrench settings, duration)
//! Tier 2: Dedicated full-width mini-timeline scrub bar with ticks, pentagon markers, dual handles, playhead
//! Tier 3: Centered transport controls with trailing track selectors and sleek volume slider
use crate::app::PealayerApp;
use eframe::egui;

const VOLUME_WIDTH: f32 = 210.0;
const MIN_SEEK_WIDTH: f32 = 96.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rows {
    One,
    Two,
    Three,
    Four,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Transport,
    Timeline,
    Tracks,
    Volume,
}

#[derive(Clone, Copy)]
pub(super) struct Metrics {
    transport: f32,
    timeline_min: f32,
    tracks: f32,
    volume_min: f32,
    gap: f32,
    row_height: f32,
    gap_y: f32,
}

#[cfg(test)]
fn button_width(ui: &egui::Ui, label: &str) -> f32 {
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let color = ui.visuals().widgets.inactive.text_color();
    let text_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(label.to_owned(), font_id, color)
            .size()
            .x
    });
    let padding = ui.spacing().button_padding.x * 2.0;
    (text_width + padding).max(30.0)
}

fn button_height(ui: &egui::Ui) -> f32 {
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let text_height = ui.fonts_mut(|fonts| fonts.row_height(&font_id));
    let padding = ui.spacing().button_padding.y * 2.0;
    (text_height + padding)
        .max(ui.spacing().interact_size.y)
        .max(25.0)
}

impl Metrics {
    pub(super) fn measure(app: &PealayerApp, ui: &egui::Ui) -> Self {
        let gap = ui.spacing().item_spacing.x;
        let gap_y = ui.spacing().item_spacing.y;
        let tracks = super::control_bar::group_width(app, ui, crate::config::ControlBarSurface::Monitor, true);
        let transport = super::control_bar::group_width(app, ui, crate::config::ControlBarSurface::Monitor, false);
        Self {
            transport,
            timeline_min: MIN_SEEK_WIDTH,
            tracks,
            volume_min: 44.0 + 24.0 + gap,
            gap,
            row_height: button_height(ui).max(super::control_density::height(ui)),
            gap_y,
        }
    }

    fn rows(self, width: f32) -> Rows {
        if width >= 540.0 {
            Rows::One
        } else if width >= 500.0 {
            Rows::Two
        } else if width >= 260.0 {
            Rows::Three
        } else {
            Rows::Four
        }
    }

    pub(super) fn height(self, width: f32) -> f32 {
        let controls_h = if width >= 540.0 { 26.0 } else { 48.0 + self.gap_y };
        let timeline_h = 24.0;
        let transport_h = if width >= 500.0 {
            32.0
        } else if width >= 260.0 {
            56.0 + self.gap_y
        } else {
            82.0 + 2.0 * self.gap_y
        };
        controls_h + self.gap_y + timeline_h + self.gap_y + transport_h
    }
}

fn draw_group(
    ui: &mut egui::Ui,
    metrics: Metrics,
    group: Group,
    width: f32,
    draw: &mut impl FnMut(Group, &mut egui::Ui),
) {
    ui.allocate_ui_with_layout(
        egui::vec2(width.max(1.0), metrics.row_height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| draw(group, ui),
    );
}

fn draw_audio_row(
    ui: &mut egui::Ui,
    metrics: Metrics,
    width: f32,
    draw: &mut impl FnMut(Group, &mut egui::Ui),
) {
    let audio_width = (metrics.tracks + metrics.gap + VOLUME_WIDTH).min(width);
    ui.add_space((width - audio_width).max(0.0));
    draw_group(ui, metrics, Group::Tracks, metrics.tracks, draw);
    let vol_w = ui.available_width();
    draw_group(
        ui,
        metrics,
        Group::Volume,
        vol_w,
        draw,
    );
}

fn draw_layout(ui: &mut egui::Ui, metrics: Metrics, mut draw: impl FnMut(Group, &mut egui::Ui)) {
    let width = ui.available_width();
    ui.horizontal(|ui| {
        draw_group(ui, metrics, Group::Timeline, width, &mut draw);
    });
    if width >= 500.0 {
        ui.horizontal(|ui| {
            let transport_w = metrics.transport.min(width);
            let audio_w = (metrics.tracks + metrics.gap + VOLUME_WIDTH).min(width);
            let left_pad = ((width - transport_w) / 2.0).min((width - audio_w - transport_w).max(0.0)).max(0.0);
            if left_pad > 0.0 {
                ui.add_space(left_pad);
            }
            draw_group(ui, metrics, Group::Transport, transport_w, &mut draw);
            let remaining = (ui.available_width() - audio_w).max(0.0);
            if remaining > 0.0 {
                ui.add_space(remaining);
            }
            draw_group(ui, metrics, Group::Tracks, metrics.tracks, &mut draw);
            let vol_w = ui.available_width();
            draw_group(ui, metrics, Group::Volume, vol_w, &mut draw);
        });
    } else {
        ui.horizontal(|ui| {
            draw_group(ui, metrics, Group::Transport, metrics.transport.min(width), &mut draw);
        });
        ui.horizontal(|ui| {
            draw_audio_row(ui, metrics, width, &mut draw);
        });
    }
}

pub(super) fn draw(app: &mut PealayerApp, ui: &mut egui::Ui, metrics: Metrics) {
    let width = ui.available_width();
    let tokens = crate::ui::craft_theme::current(ui.ctx());
    let control_owner = ui.make_persistent_id(("control-bar", crate::config::ControlBarSurface::Monitor));
    let has_video = app.current_video_path.is_some();
    let can_seek = has_video && app.is_seekable && app.duration > 0.0;

    // Tier 1: Controls row
    draw_controls_row(app, ui, width, &tokens, can_seek, metrics.gap_y);

    // Tier 2: Dedicated full-width mini-timeline scrub bar
    draw_mini_timeline(app, ui, width, &tokens, can_seek);

    // Tier 3: Transport and Audio row
    draw_transport_row(app, ui, width, metrics, &tokens, control_owner);
}

fn draw_controls_row(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    width: f32,
    tokens: &crate::ui::craft_theme::Tokens,
    can_seek: bool,
    gap_y: f32,
) {
    let elapsed = app.seek_pos.unwrap_or(app.playback_time);
    let include_hours = app.duration >= 3600.0;
    let displayed_total = if app.show_remaining_time {
        -(app.duration - elapsed).max(0.0)
    } else {
        app.duration
    };
    let timeline_state = app.media_timeline_state();
    let (total_label, total_tooltip, finite_timeline) = match timeline_state {
        crate::media::MediaTimelineState::Determining => (
            format!(
                "{} {}",
                crate::ui::icons::HOURGLASS_MEDIUM,
                app.tr("Determining…")
            ),
            app.tr("MPV is still reading media metadata; duration and seeking will update when available."),
            false,
        ),
        crate::media::MediaTimelineState::Live => (
            format!(
                "{} {}",
                crate::ui::icons::BROADCAST,
                app.tr("LIVE")
            ),
            app.tr("This live or duration-less source has no fixed endpoint."),
            false,
        ),
        crate::media::MediaTimelineState::Finite { .. } => (
            crate::ui::controls::format_player_time(
                displayed_total,
                include_hours,
                app.show_subseconds,
            ),
            app.tr("Toggle duration / remaining time"),
            true,
        ),
        crate::media::MediaTimelineState::NoMedia => (
            format!("{} --:--", crate::ui::icons::CLOCK),
            app.tr("Open media to see its duration."),
            false,
        ),
    };
    let total_text = if finite_timeline {
        crate::ui::controls::timecode_text(total_label.clone()).color(tokens.text_dim)
    } else {
        egui::RichText::new(total_label.clone()).color(tokens.text_dim)
    };

    let zoom_label = if (app.video_aspect_ratio - 16.0 / 9.0).abs() < 0.01 {
        "16:9"
    } else if (app.video_aspect_ratio - 4.0 / 3.0).abs() < 0.01 {
        "4:3"
    } else if (app.video_aspect_ratio - 2.35).abs() < 0.01 {
        "2.35:1"
    } else {
        "Fit"
    };

    if width >= 540.0 {
        let (row_rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
        let margin = 14.0_f32;
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(row_rect.left() + margin, row_rect.top()),
            egui::pos2(row_rect.right() - margin, row_rect.bottom()),
        );

        // Sub-ui for left side: elapsed editor + zoom dropdown
        let left_w = 104.0 + 8.0 + 60.0;
        let left_rect = egui::Rect::from_min_size(content_rect.min, egui::vec2(left_w, content_rect.height()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                crate::ui::controls::draw_elapsed_editor(app, ui, "nle-elapsed-editor", can_seek);

                let (z_rect, z_resp) = ui.allocate_exact_size(egui::vec2(60.0, 22.0), egui::Sense::click());
                crate::ui::craft_theme::dropdown_text(ui, z_rect, zoom_label, tokens, ui.make_persistent_id("monitor-zoom-dd"));
                let z_resp = z_resp.on_hover_text(app.tr("Zoom / Aspect Ratio"));
                egui::Popup::menu(&z_resp).show(|ui| {
                    if ui.selectable_label(app.video_aspect_ratio == 0.0, app.tr("Fit")).clicked() {
                        app.video_aspect_ratio = 0.0;
                        app.save_config();
                        ui.close();
                    }
                    ui.separator();
                    for (label, ratio) in [("16:9", 16.0 / 9.0), ("4:3", 4.0 / 3.0), ("2.35:1", 2.35)] {
                        if ui.selectable_label((app.video_aspect_ratio - ratio).abs() < 0.01, label).clicked() {
                            app.video_aspect_ratio = ratio;
                            app.save_config();
                            ui.close();
                        }
                    }
                });
            });
        });

        // Sub-ui for right side: res dropdown + wrench + duration
        let font = if finite_timeline {
            egui::TextStyle::Monospace.resolve(ui.style())
        } else {
            egui::TextStyle::Body.resolve(ui.style())
        };
        let dur_w = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(total_label, font, tokens.text_dim)
                .size()
                .x
        });
        let right_w = 54.0 + 6.0 + 22.0 + 8.0 + dur_w;
        let right_rect = egui::Rect::from_min_max(
            egui::pos2((content_rect.right() - right_w).max(content_rect.left()), content_rect.top()),
            content_rect.max,
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                let (res_rect, res_resp) = ui.allocate_exact_size(egui::vec2(54.0, 22.0), egui::Sense::click());
                crate::ui::craft_theme::dropdown_text(ui, res_rect, "1/2", tokens, ui.make_persistent_id("monitor-res-dd"));
                let res_resp = res_resp.on_hover_text(app.tr("Playback resolution"));
                egui::Popup::menu(&res_resp).show(|ui| {
                    for res in ["Full", "1/2", "1/4", "1/8"] {
                        if ui.selectable_label(res == "1/2", res).clicked() {
                            ui.close();
                        }
                    }
                });

                let w_resp = crate::ui::craft_theme::icon_button_glyph(
                    ui,
                    &app.tr("Settings"),
                    crate::ui::icons::GEAR,
                    egui::vec2(22.0, 22.0),
                    false,
                    false,
                );
                egui::Popup::menu(&w_resp).show(|ui| {
                    crate::ui::controls::transport_context_menu(app, ui);
                });

                ui.spacing_mut().item_spacing.x = 8.0;
                let total_resp = ui.add(egui::Label::new(total_text).sense(if finite_timeline {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                })).on_hover_text(total_tooltip);
                total_resp.context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));
                if finite_timeline && total_resp.clicked() {
                    app.show_remaining_time = !app.show_remaining_time;
                    app.save_config();
                }
            });
        });
    } else {
        // Stacks to 2 sub-rows
        let (row_rect, _) = ui.allocate_exact_size(egui::vec2(width, 48.0 + gap_y), egui::Sense::hover());
        let margin = 14.0_f32;
        let sub1_rect = egui::Rect::from_min_max(
            egui::pos2(row_rect.left() + margin, row_rect.top()),
            egui::pos2(row_rect.right() - margin, row_rect.top() + 24.0),
        );
        let sub2_rect = egui::Rect::from_min_max(
            egui::pos2(row_rect.left() + margin, row_rect.top() + 24.0 + gap_y),
            egui::pos2(row_rect.right() - margin, row_rect.bottom()),
        );

        // Sub-row 1
        ui.scope_builder(egui::UiBuilder::new().max_rect(sub1_rect), |ui| {
            ui.horizontal_centered(|ui| {
                crate::ui::controls::draw_elapsed_editor(app, ui, "nle-elapsed-editor", can_seek);

                if width >= 260.0 {
                    let font = if finite_timeline {
                        egui::TextStyle::Monospace.resolve(ui.style())
                    } else {
                        egui::TextStyle::Body.resolve(ui.style())
                    };
                    let dur_w = ui.fonts_mut(|fonts| {
                        fonts
                            .layout_no_wrap(total_label, font, tokens.text_dim)
                            .size()
                            .x
                    });
                    let spacer = (ui.available_width() - dur_w).max(0.0);
                    ui.add_space(spacer);

                    let total_resp = ui.add(egui::Label::new(total_text).sense(if finite_timeline {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    })).on_hover_text(total_tooltip);
                    total_resp.context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));
                    if finite_timeline && total_resp.clicked() {
                        app.show_remaining_time = !app.show_remaining_time;
                        app.save_config();
                    }
                }
            });
        });

        // Sub-row 2
        ui.scope_builder(egui::UiBuilder::new().max_rect(sub2_rect), |ui| {
            ui.horizontal_centered(|ui| {
                let (z_rect, z_resp) = ui.allocate_exact_size(egui::vec2(60.0, 22.0), egui::Sense::click());
                crate::ui::craft_theme::dropdown_text(ui, z_rect, zoom_label, tokens, ui.make_persistent_id("monitor-zoom-dd"));
                let z_resp = z_resp.on_hover_text(app.tr("Zoom / Aspect Ratio"));
                egui::Popup::menu(&z_resp).show(|ui| {
                    if ui.selectable_label(app.video_aspect_ratio == 0.0, app.tr("Fit")).clicked() {
                        app.video_aspect_ratio = 0.0;
                        app.save_config();
                        ui.close();
                    }
                    ui.separator();
                    for (label, ratio) in [("16:9", 16.0 / 9.0), ("4:3", 4.0 / 3.0), ("2.35:1", 2.35)] {
                        if ui.selectable_label((app.video_aspect_ratio - ratio).abs() < 0.01, label).clicked() {
                            app.video_aspect_ratio = ratio;
                            app.save_config();
                            ui.close();
                        }
                    }
                });

                let show_res = width >= 260.0;
                let right_w = if show_res { 54.0 + 6.0 + 22.0 } else { 22.0 };
                let spacer = (ui.available_width() - right_w).max(0.0);
                ui.add_space(spacer);

                if show_res {
                    let (res_rect, res_resp) = ui.allocate_exact_size(egui::vec2(54.0, 22.0), egui::Sense::click());
                    crate::ui::craft_theme::dropdown_text(ui, res_rect, "1/2", tokens, ui.make_persistent_id("monitor-res-dd"));
                    let res_resp = res_resp.on_hover_text(app.tr("Playback resolution"));
                    egui::Popup::menu(&res_resp).show(|ui| {
                        for res in ["Full", "1/2", "1/4", "1/8"] {
                            if ui.selectable_label(res == "1/2", res).clicked() {
                                ui.close();
                            }
                        }
                    });

                    ui.spacing_mut().item_spacing.x = 6.0;
                }

                let w_resp = crate::ui::craft_theme::icon_button_glyph(
                    ui,
                    &app.tr("Settings"),
                    crate::ui::icons::GEAR,
                    egui::vec2(22.0, 22.0),
                    false,
                    false,
                );
                egui::Popup::menu(&w_resp).show(|ui| {
                    crate::ui::controls::transport_context_menu(app, ui);
                });
            });
        });
    }
}

pub(crate) fn draw_timeline(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let tokens = crate::ui::craft_theme::current(ui.ctx());
    let has_video = app.current_video_path.is_some();
    let can_seek = has_video && app.is_seekable && app.duration > 0.0;
    let width = ui.available_width();
    draw_mini_timeline(app, ui, width, &tokens, can_seek);
}

fn draw_mini_timeline(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    width: f32,
    tokens: &crate::ui::craft_theme::Tokens,
    can_seek: bool,
) {
    let has_video = app.current_video_path.is_some();
    let mut current_pos = if has_video {
        app.seek_pos.unwrap_or(app.playback_time)
    } else {
        0.0
    };
    let max_dur = if has_video && app.duration > 0.0 {
        app.duration
    } else {
        1.0
    };

    let (row_rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
    let margin = 14.0_f32.min((width - MIN_SEEK_WIDTH) / 2.0).max(0.0);
    let seek_rect = egui::Rect::from_min_max(
        egui::pos2(row_rect.left() + margin, row_rect.top()),
        egui::pos2(row_rect.right() - margin, row_rect.bottom()),
    );
    let response = ui.interact(
        seek_rect,
        ui.make_persistent_id("nle-mini-timeline"),
        if can_seek {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::hover()
        },
    );

    let track_h = 13.0_f32;
    let track_y = seek_rect.bottom() - 8.5;
    let track_rect = egui::Rect::from_min_max(
        egui::pos2(seek_rect.left(), track_y - track_h / 2.0),
        egui::pos2(seek_rect.right(), track_y + track_h / 2.0),
    );
    let r = track_h / 2.0;
    let p = ui.painter();

            // Track rail
            p.rect_filled(track_rect, r, tokens.separator);

            // Buffered range fill
            if let Some(buffered_until) = app.buffered_until() {
                let buf_frac = (buffered_until / max_dur).clamp(0.0, 1.0) as f32;
                let buf_w = (track_rect.width() * buf_frac).max(0.0);
                let buf_rect = egui::Rect::from_min_max(
                    track_rect.min,
                    egui::pos2(track_rect.min.x + buf_w, track_rect.max.y),
                );
                p.rect_filled(buf_rect, r, tokens.accent.gamma_multiply(0.25));
            }

            // Dual circular handles at track ends
            for handle_x in [track_rect.min.x + r, track_rect.max.x - r] {
                p.circle_filled(egui::pos2(handle_x, track_rect.center().y), 4.5, tokens.panel_bg);
                p.circle_stroke(
                    egui::pos2(handle_x, track_rect.center().y),
                    4.5,
                    egui::Stroke::new(1.5, tokens.icon_active),
                );
            }

            // Tick ladder above track
            let fps = if app.media_fps.is_finite() && app.media_fps > 0.0 {
                app.media_fps
            } else {
                30.0
            };
            let base = fps.clamp(12.0, 120.0).round() as i64;
            let frame_px = (track_rect.width() as f64) / (max_dur * fps);
            let (minor_step, major_step) = crate::ui::craft_theme::ruler_steps(frame_px, base);
            let total_frames = (max_dur * fps).ceil() as i64;
            let mut f = 0;
            let mut count = 0;
            while f <= total_frames && count < 256 {
                let frac = (f as f64 / (max_dur * fps)).clamp(0.0, 1.0) as f32;
                let x = track_rect.min.x + frac * track_rect.width();
                let is_major = f % major_step == 0;
                let h = if is_major { 8.0 } else { 3.5 };
                p.line_segment(
                    [egui::pos2(x, track_rect.min.y - h), egui::pos2(x, track_rect.min.y)],
                    egui::Stroke::new(
                        1.0,
                        if is_major {
                            tokens.text_dim
                        } else {
                            tokens.text_faint
                        },
                    ),
                );
                f += minor_step;
                count += 1;
            }

            // Pentagon markers (chapters and keyframes)
            let mut clicked_marker_time = None;
            let chapters = app.media_chapters();
            for (i, chapter) in chapters.iter().enumerate() {
                if !chapter.time_seconds.is_finite()
                    || chapter.time_seconds < 0.0
                    || chapter.time_seconds > max_dur
                {
                    continue;
                }
                let frac = (chapter.time_seconds / max_dur).clamp(0.0, 1.0) as f32;
                let x = track_rect.min.x + frac * track_rect.width();
                let marker_rect = egui::Rect::from_min_size(
                    egui::pos2(x - 3.5, track_rect.min.y - 12.0),
                    egui::vec2(7.0, 10.0),
                );
                let color = match i % 4 {
                    0 => egui::Color32::from_rgb(0x38, 0x8e, 0x3c), // green
                    1 => egui::Color32::from_rgb(0xc2, 0x18, 0x5b), // magenta
                    2 => egui::Color32::from_rgb(0xf5, 0x7c, 0x00), // amber
                    _ => egui::Color32::from_rgb(0x00, 0x97, 0xa7), // cyan
                };
                crate::ui::craft_theme::marker_glyph(
                    p,
                    marker_rect,
                    3.0,
                    crate::ui::craft_theme::MarkerPart::Whole,
                    color,
                );
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if marker_rect.expand(2.0).contains(pos) {
                            clicked_marker_time = Some(chapter.time_seconds);
                        }
                    }
                }
            }
            for keyframe in &app.timeline.keyframes {
                let time = keyframe.time_ms as f64 / 1000.0;
                if time >= 0.0 && time <= max_dur {
                    let frac = (time / max_dur).clamp(0.0, 1.0) as f32;
                    let x = track_rect.min.x + frac * track_rect.width();
                    let k_rect = egui::Rect::from_min_size(
                        egui::pos2(x - 3.5, track_rect.min.y - 12.0),
                        egui::vec2(7.0, 10.0),
                    );
                    crate::ui::craft_theme::marker_glyph(
                        p,
                        k_rect,
                        3.0,
                        crate::ui::craft_theme::MarkerPart::Whole,
                        egui::Color32::from_rgb(0xef, 0x44, 0x44),
                    );
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if k_rect.expand(2.0).contains(pos) {
                                clicked_marker_time = Some(time);
                            }
                        }
                    }
                }
            }

            // Playhead: precision downward pentagon and vertical stem line
            let frac = (current_pos / max_dur).clamp(0.0, 1.0) as f32;
            let x = track_rect.min.x + frac * track_rect.width();
            let hy = track_rect.min.y - 11.0;
            p.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x - 5.5, hy),
                    egui::pos2(x + 5.5, hy),
                    egui::pos2(x + 5.5, hy + 5.0),
                    egui::pos2(x, hy + 9.0),
                    egui::pos2(x - 5.5, hy + 5.0),
                ],
                tokens.playhead,
                egui::Stroke::NONE,
            ));
            p.line_segment(
                [egui::pos2(x, hy + 8.0), egui::pos2(x, track_rect.max.y)],
                egui::Stroke::new(1.0, tokens.playhead),
            );

            // Scrubbing and interaction
            if let Some(m_time) = clicked_marker_time {
                current_pos = m_time;
                if can_seek {
                    app.scrub_to(current_pos);
                    app.finish_scrub(current_pos);
                    ui.ctx().request_repaint();
                }
            } else if can_seek && (response.dragged() || response.clicked()) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let f = ((pos.x - track_rect.left()) / track_rect.width().max(1.0)).clamp(0.0, 1.0) as f64;
                    current_pos = f * max_dur;
                    app.scrub_to(current_pos);
                    ui.ctx().request_repaint();
                }
            }
            if can_seek
                && (response.drag_stopped()
                    || (app.is_scrubbing && !ui.input(|i| i.pointer.primary_down())))
            {
                app.finish_scrub(current_pos);
            }

            response.context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));
            let show_seek_preview = app.nle_seekbar_hover_thumbnails;
            crate::ui::seek_preview::draw(app, ui, &response, show_seek_preview, "nle-seekbar-preview");
}

fn draw_transport_row(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    width: f32,
    metrics: Metrics,
    _tokens: &crate::ui::craft_theme::Tokens,
    owner: egui::Id,
) {
    let vol_enabled = app.current_video_path.is_some();
    if width >= 500.0 {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 32.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let transport_w = metrics.transport.min(width);
                let tracks_w = metrics.tracks;
                let vol_w = VOLUME_WIDTH.min((width - transport_w - tracks_w - 20.0).max(metrics.volume_min));
                let audio_w = (tracks_w + metrics.gap + vol_w).min(width);

                let max_transport_right = (width - audio_w - metrics.gap).max(0.0);
                let target_center_x = (width - transport_w) / 2.0;
                let left_pad = target_center_x.min(max_transport_right - transport_w).max(0.0);

                ui.add_space(left_pad);
                super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, false, owner);

                let trailing_space = (width - left_pad - transport_w - audio_w).max(0.0);
                ui.add_space(trailing_space);

                super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, true, owner);
                ui.add_enabled_ui(vol_enabled, |ui| {
                    super::controls::draw_volume_slider(app, ui);
                });
            },
        );
    } else if width >= 260.0 {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 30.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let transport_w = metrics.transport.min(width);
                let left_pad = ((width - transport_w) / 2.0).max(0.0);
                ui.add_space(left_pad);
                super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, false, owner);
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(width, 26.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let tracks_w = metrics.tracks;
                let vol_w = VOLUME_WIDTH.min(width - tracks_w - metrics.gap).max(metrics.volume_min);
                let audio_w = (tracks_w + metrics.gap + vol_w).min(width);
                ui.add_space((width - audio_w).max(0.0));
                super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, true, owner);
                ui.add_enabled_ui(vol_enabled, |ui| {
                    super::controls::draw_volume_slider(app, ui);
                });
            },
        );
    } else {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 30.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, false, owner);
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(width, 26.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, true, owner);
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(width, 26.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_enabled_ui(vol_enabled, |ui| {
                    super::controls::draw_volume_slider(app, ui);
                });
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MediaTrackType;

    fn metrics() -> Metrics {
        Metrics {
            transport: 172.0,
            timeline_min: 300.0,
            tracks: 130.0,
            volume_min: 114.0,
            gap: 8.0,
            gap_y: 4.0,
            row_height: 24.0,
        }
    }

    #[test]
    fn smallest_number_of_rows_is_selected_at_exact_fit() {
        let m = metrics();
        assert_eq!(m.rows(1400.0), Rows::One);
        assert_eq!(m.rows(540.0), Rows::One);
        assert_eq!(m.rows(539.9), Rows::Two);
        assert_eq!(m.rows(500.0), Rows::Two);
        assert_eq!(m.rows(499.9), Rows::Three);
        assert_eq!(m.rows(260.0), Rows::Three);
        assert_eq!(m.rows(259.9), Rows::Four);
        assert_eq!(m.height(1400.0), 90.0);
        assert_eq!(m.height(700.0), 90.0);
        assert_eq!(m.height(400.0), 144.0);
        assert_eq!(m.height(240.0), 174.0);
    }

    #[test]
    fn all_widths_use_bounded_rows_and_trailing_track_volume_order() {
        for width in [1200.0, 836.0, 835.0, 600.0, 480.0, 400.0, 300.0, 240.0] {
            let ctx = egui::Context::default();
            let mut rects = Vec::new();
            let mut area = egui::Rect::NOTHING;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 200.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(metrics().gap, metrics().gap_y);
                    area = ui.available_rect_before_wrap();
                    draw_layout(ui, metrics(), |group, ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 24.0),
                            egui::Sense::hover(),
                        );
                        rects.push((group, rect));
                    });
                },
            );
            output.textures_delta.clear();
            assert_eq!(
                rects.iter().map(|(group, _)| *group).collect::<Vec<_>>(),
                vec![
                    Group::Timeline,
                    Group::Transport,
                    Group::Tracks,
                    Group::Volume
                ]
            );
            let tracks = rects[2].1;
            let volume = rects[3].1;
            if area.width() >= 500.0 {
                assert_eq!(tracks.top(), volume.top(), "width {width}");
                assert!(tracks.right() <= volume.left(), "width {width}");
            }
            assert!(
                (volume.right() - area.right()).abs() < 1.0,
                "volume must trail at {width}"
            );
            for (_, rect) in &rects {
                assert!(
                    rect.left() >= area.left() - 0.5 && rect.right() <= area.right() + 0.5,
                    "overflow at {width}: {rect:?} vs {area:?}"
                );
            }
        }
    }

    #[test]
    fn real_track_buttons_and_volume_strip_fit_the_measured_slots() {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);
        let mut measured = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for kind in [
                MediaTrackType::Video,
                MediaTrackType::Audio,
                MediaTrackType::Subtitle,
            ] {
                let label = super::super::media_tracks::menu_button_label(kind);
                let width = button_width(ui, &label);
                let response =
                    egui::containers::menu::MenuButton::from_button(egui::Button::new(label))
                        .ui(ui, |_| {})
                        .0;
                measured.push((width, response.rect.width()));
            }
            for width in [114.0, VOLUME_WIDTH] {
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let left = ui.max_rect().left();
                        super::super::controls::draw_volume_widgets(
                            ui,
                            crate::config::ColorPalette::Studio,
                            false,
                            130.0,
                            "Mute",
                            "Volume",
                        );
                        measured.push((width, ui.min_rect().right() - left));
                    },
                );
            }
        });
        output.textures_delta.clear();
        for (allocated, used) in measured {
            assert!(used <= allocated + 0.5, "{used} overflowed {allocated}");
        }
    }

    #[test]
    fn stop_uses_a_square_not_the_record_like_circled_glyph() {
        assert_ne!(super::super::icons::STOP, super::super::icons::STOP_CIRCLE);
        assert_eq!(super::super::icons::STOP, egui_phosphor::regular::STOP);
    }

    #[test]
    fn customized_real_controls_fit_the_reserved_monitor_rows() {
        let _lock = crate::app::tests::lock_app_tests();
        for hidden in [true, false] {
            for width in [240.0, 400.0, 700.0, 1400.0] {
                let ctx = egui::Context::default();
                let mut app = PealayerApp::default();
                app.control_bars.monitor.hidden = [hidden; crate::config::ControlBarAction::COUNT];
                for _ in 0..3 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, 600.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            let available = ui.available_rect_before_wrap();
                            let metrics = Metrics::measure(&app, ui);
                            draw(&mut app, ui, metrics);
                            let actual = ui.min_rect();
                            assert!(
                                actual.right() <= available.right() + 1.0,
                                "width {width}, hidden={hidden}: {actual:?} vs {available:?}"
                            );
                            assert!(
                                actual.height() <= metrics.height(available.width()) + 1.0,
                                "reserved height at {width}, hidden={hidden}: {actual:?}"
                            );
                        },
                    );
                    output.textures_delta.clear();
                }
            }
        }
    }

    #[test]
    fn monitor_controls_respect_craft_theme_tokens() {
        for palette in [crate::config::ColorPalette::Native, crate::config::ColorPalette::Studio] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let ctx = egui::Context::default();
                ctx.set_theme(theme);
                let config = crate::config::AppConfig { color_palette: palette,
                    accent_color: crate::config::AccentColor::PealayerGreen, ..Default::default() };
                crate::ui::configure_native_visuals(&ctx, &config);
                let tokens = crate::ui::craft_theme::current(&ctx);
                let selected = super::super::craft_theme::icon_button_style(&tokens, false).selected;
                assert_eq!(tokens.pill_active_bg, ctx.style_of(theme).visuals.selection.bg_fill);
                assert_eq!(selected.icon, super::super::accent_foreground(tokens.pill_active_bg));
                assert_ne!(selected.icon, selected.fill);
                assert_eq!(tokens.playhead, tokens.focus);
            }
        }
    }

    #[test]
    fn filmcraft_monitor_timeline_renders_custom_handles_and_playhead() {
        let _lock = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let config = crate::config::AppConfig::default();
        crate::ui::configure_native_visuals(&ctx, &config);
        let mut app = PealayerApp::default();
        app.current_video_path = Some(std::path::PathBuf::from("/mock/video.mp4"));
        app.duration = 60.0;
        app.is_seekable = true;
        app.playback_time = 15.0;

        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 100.0),
                )),
                ..Default::default()
            },
            |ui| {
                draw_timeline(&mut app, ui);
            },
        );
        output.textures_delta.clear();
    }
}
