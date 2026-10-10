//! Intrinsic-width NLE transport: one row when it fits, with track/volume
//! controls at the trailing end of the last row when the monitor narrows.
use crate::app::{MediaTrackType, PealayerApp};
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

fn button_width(ui: &egui::Ui, label: &str) -> f32 {
    let style = ui.style().button_style(
        &egui::widget_style::Classes::default(),
        egui::widget_style::WidgetState::Inactive,
    );
    let text_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(
                label.to_owned(),
                style.text_style.font_id,
                style.text_style.color,
            )
            .size()
            .x
    });
    (text_width + f32::from(style.frame.inner_margin.left + style.frame.inner_margin.right))
        .max(30.0)
}

impl Metrics {
    pub(super) fn measure(app: &PealayerApp, ui: &egui::Ui) -> Self {
        let gap = ui.spacing().item_spacing.x;
        let tracks = [
            MediaTrackType::Video,
            MediaTrackType::Audio,
            MediaTrackType::Subtitle,
        ]
        .map(|kind| button_width(ui, &super::media_tracks::menu_button_label(kind)))
        .iter()
        .sum::<f32>()
            + 2.0 * gap;
        let total = match app.media_timeline_state() {
            crate::media::MediaTimelineState::Determining => format!(
                "{} {}",
                super::icons::HOURGLASS_MEDIUM,
                app.tr("Determining…")
            ),
            crate::media::MediaTimelineState::Live => {
                format!("{} {}", super::icons::BROADCAST, app.tr("LIVE"))
            }
            crate::media::MediaTimelineState::NoMedia => format!("{} --:--", super::icons::CLOCK),
            crate::media::MediaTimelineState::Finite { .. } => super::controls::format_player_time(
                if app.show_remaining_time {
                    -app.duration
                } else {
                    app.duration
                },
                app.duration >= 3600.0,
                app.show_subseconds,
            ),
        };
        // The total is monospace for finite timelines and body text otherwise.
        let font = if matches!(
            app.media_timeline_state(),
            crate::media::MediaTimelineState::Finite { .. }
        ) {
            egui::TextStyle::Monospace.resolve(ui.style())
        } else {
            egui::TextStyle::Body.resolve(ui.style())
        };
        let total_width = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(total, font, ui.visuals().text_color())
                .size()
                .x
        });
        Self {
            transport: 2.0 * 30.0 + 2.0 * 44.0 + 3.0 * gap,
            timeline_min: 104.0
                + MIN_SEEK_WIDTH
                + total_width
                + button_width(ui, super::icons::ARROWS_OUT)
                + 3.0 * gap,
            tracks,
            volume_min: 30.0 + 44.0 + 24.0 + 2.0 * gap,
            gap,
            row_height: ui.spacing().interact_size.y.max(24.0),
            gap_y: ui.spacing().item_spacing.y,
        }
    }

    fn rows(self, width: f32) -> Rows {
        let complete =
            self.transport + self.timeline_min + self.tracks + VOLUME_WIDTH + 3.0 * self.gap;
        if width >= complete {
            Rows::One
        } else if width >= self.transport + self.timeline_min + self.gap {
            Rows::Two
        } else if width >= self.tracks + self.volume_min + self.gap {
            Rows::Three
        } else {
            Rows::Four
        }
    }

    pub(super) fn height(self, width: f32) -> f32 {
        let rows = match self.rows(width) {
            Rows::One => 1.0,
            Rows::Two => 2.0,
            Rows::Three => 3.0,
            Rows::Four => 4.0,
        };
        rows * self.row_height + (rows - 1.0) * self.gap_y
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
    draw_group(
        ui,
        metrics,
        Group::Volume,
        audio_width - metrics.tracks - metrics.gap,
        draw,
    );
}

/// Explicit bounded groups keep the elastic seekbar from forcing a wrap and
/// reserve exactly the number of rows used before the video is allocated.
fn draw_layout(ui: &mut egui::Ui, metrics: Metrics, mut draw: impl FnMut(Group, &mut egui::Ui)) {
    let width = ui.available_width();
    let audio_width = (metrics.tracks + metrics.gap + VOLUME_WIDTH).min(width);
    let rows = metrics.rows(width);
    if matches!(rows, Rows::Three | Rows::Four) {
        ui.horizontal(|ui| {
            draw_group(
                ui,
                metrics,
                Group::Transport,
                metrics.transport.min(width),
                &mut draw,
            )
        });
        ui.horizontal(|ui| draw_group(ui, metrics, Group::Timeline, width, &mut draw));
        if rows == Rows::Four {
            ui.horizontal(|ui| {
                draw_group(
                    ui,
                    metrics,
                    Group::Tracks,
                    metrics.tracks.min(width),
                    &mut draw,
                )
            });
            ui.horizontal(|ui| {
                let volume_width = VOLUME_WIDTH.min(width);
                ui.add_space((width - volume_width).max(0.0));
                draw_group(ui, metrics, Group::Volume, volume_width, &mut draw);
            });
        } else {
            ui.horizontal(|ui| draw_audio_row(ui, metrics, width, &mut draw));
        }
    } else {
        // Single row: transport, full timeline, selectors, volume.
        // Two rows: transport/timeline, then trailing selectors/volume.
        ui.horizontal(|ui| {
            draw_group(ui, metrics, Group::Transport, metrics.transport, &mut draw);
            let tail = if rows == Rows::One {
                audio_width + metrics.gap
            } else {
                0.0
            };
            draw_group(
                ui,
                metrics,
                Group::Timeline,
                width - metrics.transport - metrics.gap - tail,
                &mut draw,
            );
            if rows == Rows::One {
                draw_group(ui, metrics, Group::Tracks, metrics.tracks, &mut draw);
                draw_group(ui, metrics, Group::Volume, VOLUME_WIDTH, &mut draw);
            }
        });
        if rows == Rows::Two {
            ui.horizontal(|ui| draw_audio_row(ui, metrics, width, &mut draw));
        }
    }
}

pub(super) fn draw(app: &mut PealayerApp, ui: &mut egui::Ui, metrics: Metrics) {
    draw_layout(ui, metrics, |group, ui| match group {
        Group::Transport => draw_transport(app, ui),
        Group::Timeline => draw_timeline(app, ui),
        Group::Tracks => {
            ui.add_enabled_ui(app.current_video_path.is_some(), |ui| {
                for (kind, id) in [
                    (MediaTrackType::Video, "nle-video-track-menu"),
                    (MediaTrackType::Audio, "nle-audio-track-menu"),
                    (MediaTrackType::Subtitle, "nle-subtitle-track-menu"),
                ] {
                    super::media_tracks::menu_button(app, ui, kind, id);
                }
            });
        }
        Group::Volume => {
            ui.add_enabled_ui(app.current_video_path.is_some(), |ui| {
                super::controls::draw_volume_strip(app, ui);
            });
        }
    });
}

fn draw_transport(app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.add_enabled_ui(app.current_video_path.is_some(), |ui| {
        let finished = app.is_playback_finished();
        let active = app.is_active_playback();
        let (play_icon, play_tooltip) = if finished {
            (super::icons::ARROW_COUNTER_CLOCKWISE, app.tr("Replay"))
        } else if app.is_paused {
            (super::icons::PLAY, app.tr("Play"))
        } else {
            (super::icons::PAUSE, app.tr("Pause"))
        };
        let play = super::controls::draw_contextual_transport_button(
            app.color_palette,
            ui,
            play_icon,
            super::controls::playback_button_role(app),
            active,
        )
        .on_hover_text(play_tooltip)
        .on_disabled_hover_text(app.tr("Open media to play."));
        play.context_menu(|ui| super::controls::transport_context_menu(app, ui));
        let stop = super::controls::draw_contextual_transport_button(
            app.color_palette,
            ui,
            super::icons::STOP,
            "red",
            false,
        )
        .on_hover_text(app.tr("Stop"))
        .on_disabled_hover_text(app.tr("Open media to play."));
        stop.context_menu(|ui| super::controls::transport_context_menu(app, ui));
        super::controls::draw_contextual_transport_nudge(
            app,
            ui,
            ui.make_persistent_id("nle-transport-back"),
            -1,
            super::controls::TransportNudgeDensity::Compact,
        );
        super::controls::draw_contextual_transport_nudge(
            app,
            ui,
            ui.make_persistent_id("nle-transport-forward"),
            1,
            super::controls::TransportNudgeDensity::Compact,
        );
        // Apply after rendering so one frame cannot mix old and new states.
        if play.clicked() {
            app.toggle_playback();
        }
        if stop.clicked() {
            app.apply_interop_command(
                &ui.ctx().clone(),
                crate::platform::interop::InteropCommand::Stop,
                "Program monitor",
            );
        }
    });
}

fn draw_timeline(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let has_video = app.current_video_path.is_some();
    let can_seek = has_video && app.is_seekable && app.duration > 0.0;
    let elapsed = app.seek_pos.unwrap_or(app.playback_time);
    let include_hours = app.duration >= 3600.0;
    crate::ui::controls::draw_elapsed_editor(app, ui, "nle-elapsed-editor", can_seek);

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
        crate::ui::controls::timecode_text(total_label)
    } else {
        egui::RichText::new(total_label)
    };

    // Anchor the duration and fullscreen affordance to the right
    // edge first, then give the seekbar the exact remaining width.
    // A former fixed 180 px reservation was wider than these
    // controls and left a visible, useless tail after fullscreen.
    let response = ui
        .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let fullscreen_response = ui
                .button(crate::ui::icons::ARROWS_OUT)
                .on_hover_text(format!("{} (F)", app.tr("Fullscreen")));
            fullscreen_response
                .context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));
            if fullscreen_response.clicked() {
                app.set_fullscreen(ui.ctx(), true);
            }

            let total_response = ui
                .add(egui::Label::new(total_text).sense(if finite_timeline {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                }))
                .on_hover_text(total_tooltip);
            total_response.context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));
            if finite_timeline && total_response.clicked() {
                app.show_remaining_time = !app.show_remaining_time;
                app.save_config();
            }

            let slider = egui::Slider::new(&mut current_pos, 0.0..=max_dur)
                .show_value(false)
                .trailing_fill(true);
            let response = crate::ui::controls::add_fill_width_slider(ui, can_seek, slider);
            response.context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));
            response
        })
        .inner;

    if let Some(buffered_until) = app.buffered_until() {
        crate::ui::controls::paint_buffered_seekbar(
            ui,
            &response,
            (current_pos / app.duration).clamp(0.0, 1.0) as f32,
            (buffered_until / app.duration).clamp(0.0, 1.0) as f32,
        );
    }
    if let Some(chapter_time) = crate::ui::controls::paint_seekbar_markers(ui, &response, app) {
        current_pos = chapter_time;
    }
    let show_seek_preview = app.nle_seekbar_hover_thumbnails;
    crate::ui::seek_preview::draw(app, ui, &response, show_seek_preview, "nle-seekbar-preview");

    if can_seek && response.changed() {
        app.scrub_to(current_pos);
        ui.ctx().request_repaint();
    }
    if can_seek
        && (response.drag_stopped()
            || response.clicked()
            || (app.is_scrubbing && !ui.input(|i| i.pointer.primary_down())))
    {
        app.finish_scrub(current_pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(m.rows(836.0), Rows::One);
        assert_eq!(m.rows(835.9), Rows::Two);
        assert_eq!(m.rows(480.0), Rows::Two);
        assert_eq!(m.rows(479.9), Rows::Three);
        assert_eq!(m.rows(252.0), Rows::Three);
        assert_eq!(m.rows(251.9), Rows::Four);
        assert_eq!(m.height(836.0), 24.0);
        assert_eq!(m.height(480.0), 52.0);
        assert_eq!(m.height(400.0), 80.0);
        assert_eq!(m.height(240.0), 108.0);
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
                        // Exercise the production allocator, not a mirrored layout.
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
                    Group::Transport,
                    Group::Timeline,
                    Group::Tracks,
                    Group::Volume
                ]
            );
            let tracks = rects[2].1;
            let volume = rects[3].1;
            if metrics().rows(area.width()) != Rows::Four {
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
            let rendered_height = rects.last().unwrap().1.bottom() - rects[0].1.top();
            assert!(
                (rendered_height - metrics().height(area.width())).abs() < 1.0,
                "footer at {width}"
            );
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
}
