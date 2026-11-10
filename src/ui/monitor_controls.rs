//! Content-measured NLE transport. Below the wide breakpoint seeking owns a
//! complete row; actions form balanced leading/trailing clusters below it.
use crate::app::PealayerApp;
use eframe::egui;

#[cfg(test)]
const VOLUME_WIDTH: f32 = 160.0;
const MIN_SEEK_WIDTH: f32 = 96.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rows {
    One,
    Two,
    Three,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Group {
    Transport,
    Timeline,
    Tracks,
    Elapsed,
    Duration,
    Seek,
}

#[derive(Clone, Copy)]
pub(super) struct Metrics {
    transport: f32,
    timeline_min: f32,
    tracks: f32,
    gap: f32,
    row_height: f32,
    gap_y: f32,
    rtl: bool,
    style: crate::config::NativeMonitorStyle,
    elapsed_width: f32,
    duration_width: f32,
}

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
        let tracks = super::control_bar::group_width(app, ui, crate::config::ControlBarSurface::Monitor, true);
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
            transport: super::control_bar::group_width(app, ui, crate::config::ControlBarSurface::Monitor, false),
            timeline_min: super::controls::elapsed_editor_size(ui).x
                + MIN_SEEK_WIDTH
                + total_width
                + 2.0 * gap,
            tracks,
            gap,
            row_height: button_height(ui).max(super::controls::elapsed_editor_size(ui).y),
            gap_y: ui.spacing().item_spacing.y,
            rtl: app.rtl,
            style: app.native_monitor_style,
            elapsed_width: 104.0 + 72.0 + gap,
            duration_width: total_width + 72.0 + button_height(ui) + 3.0 * gap,
        }
    }

    fn rows(self, width: f32) -> Rows {
        let complete =
            self.transport + self.timeline_min + self.tracks + 2.0 * self.gap;
        if width >= complete {
            Rows::One
        } else if width >= self.transport + self.tracks + self.cluster_gap() {
            Rows::Two
        } else {
            Rows::Three
        }
    }

    pub(super) fn height(self, width: f32) -> f32 {
        if self.style == crate::config::NativeMonitorStyle::FilmCraft {
            let rows = self.filmcraft_time_rows(width) + 1 + self.filmcraft_action_rows(width);
            return rows as f32 * self.row_height + (rows - 1) as f32 * self.gap_y;
        }
        let rows = match self.rows(width) {
            Rows::One => 1.0,
            Rows::Two => 2.0,
            Rows::Three => 3.0,
        };
        rows * self.row_height + (rows - 1.0) * self.gap_y
    }

    fn cluster_gap(self) -> f32 { (2.0*self.gap).max(24.0) }

    fn filmcraft_time_rows(self, width: f32) -> usize {
        if width >= self.elapsed_width + self.duration_width + self.gap { 1 } else { 2 }
    }

    fn filmcraft_action_rows(self, width: f32) -> usize {
        if width >= self.transport + self.tracks + self.cluster_gap() { 1 } else { 2 }
    }
}

/// Explicit bounded groups keep the elastic seekbar from forcing a wrap and
/// reserve exactly the number of rows used before the video is allocated.
fn draw_layout(ui: &mut egui::Ui, metrics: Metrics, mut draw: impl FnMut(Group, &mut egui::Ui)) {
    let width = ui.available_width();
    let (bounds, _) = ui.allocate_exact_size(egui::vec2(width, metrics.height(width)), egui::Sense::hover());
    let mut slot = |group, row: usize, x: f32, w: f32| {
        let x = if metrics.rtl { width - x - w } else { x };
        let rect = egui::Rect::from_min_size(bounds.min + egui::vec2(x, row as f32 * (metrics.row_height + metrics.gap_y)),
            egui::vec2(w.max(1.0), metrics.row_height));
        let mut child = ui.new_child(egui::UiBuilder::new().id_salt(("monitor-control-group", group))
            .max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        draw(group, &mut child);
    };
    if metrics.style == crate::config::NativeMonitorStyle::FilmCraft {
        let time_rows = metrics.filmcraft_time_rows(width);
        slot(Group::Elapsed, 0, 0.0, metrics.elapsed_width.min(width));
        let duration_width = metrics.duration_width.min(width);
        slot(Group::Duration, time_rows - 1, width - duration_width, duration_width);
        // FilmCraft's middle tier gives the scrubber the entire available width.
        slot(Group::Seek, time_rows, 0.0, width);
        let transport = metrics.transport.min(width);
        let tracks = metrics.tracks.min(width);
        let action_row = time_rows + 1;
        if metrics.filmcraft_action_rows(width) == 1 {
            let centered = (width - transport) / 2.0;
            let available_before_tail = width - tracks - metrics.cluster_gap() - transport;
            slot(Group::Transport, action_row, centered.min(available_before_tail).max(0.0), transport);
            slot(Group::Tracks, action_row, width - tracks, tracks);
        } else {
            slot(Group::Transport, action_row, (width - transport) / 2.0, transport);
            slot(Group::Tracks, action_row + 1, width - tracks, tracks);
        }
        return;
    }
    match metrics.rows(width) {
        Rows::One => {
            let timeline = width - metrics.transport - metrics.tracks - 2.0 * metrics.gap;
            slot(Group::Transport, 0, 0.0, metrics.transport);
            slot(Group::Timeline, 0, metrics.transport + metrics.gap, timeline);
            slot(Group::Tracks, 0, width - metrics.tracks, metrics.tracks);
        }
        Rows::Two => {
            slot(Group::Timeline, 0, 0.0, width);
            slot(Group::Transport, 1, 0.0, metrics.transport);
            slot(Group::Tracks, 1, width - metrics.tracks, metrics.tracks);
        }
        Rows::Three => {
            let transport = metrics.transport.min(width);
            let tracks = metrics.tracks.min(width);
            slot(Group::Timeline, 0, 0.0, width);
            slot(Group::Transport, 1, 0.0, transport);
            slot(Group::Tracks, 2, width - tracks, tracks);
        }
    }
}

pub(super) fn draw(app: &mut PealayerApp, ui: &mut egui::Ui, metrics: Metrics) {
    let control_owner=ui.make_persistent_id(("control-bar",crate::config::ControlBarSurface::Monitor));
    draw_layout(ui, metrics, |group, ui| match group {
        Group::Transport => super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, false,control_owner),
        Group::Timeline => draw_timeline(app, ui),
        Group::Tracks => super::control_bar::draw_group(app, ui, crate::config::ControlBarSurface::Monitor, true,control_owner),
        Group::Elapsed => {
            let can_seek = app.current_video_path.is_some() && app.is_seekable && app.duration > 0.0;
            ui.scope(|ui| {
                let tokens = super::craft_theme::current(ui.ctx());
                ui.visuals_mut().override_text_color = Some(tokens.timecode);
                super::controls::draw_elapsed_editor(app, ui, "nle-elapsed-editor", can_seek);
                draw_preview_selector(app,ui,true);
            });
        }
        Group::Duration => {
            let layout = if app.rtl { egui::Layout::left_to_right(egui::Align::Center) }
                else { egui::Layout::right_to_left(egui::Align::Center) };
            ui.with_layout(layout, |ui| {
                draw_total(app, ui);
                let h=super::control_density::height(ui);
                let response=super::craft_theme::icon_button_glyph(ui,"Transport settings",super::icons::GEAR,
                    egui::vec2(h,h),false,false);
                egui::Popup::menu(&response).show(|ui| super::controls::transport_context_menu(app,ui));
                draw_preview_selector(app,ui,false);
            });
        }
        Group::Seek => { draw_seekbar(app, ui, true); }
    });
}

fn draw_preview_selector(app:&mut PealayerApp,ui:&mut egui::Ui,aspect:bool) {
    use crate::config::{MonitorAspect as A,MonitorResolution as R};
    let label=if aspect {app.monitor_preview.aspect.label()} else {app.monitor_preview.resolution.label()};
    let width=ui.available_width().min(72.0).max(1.0);
    let h=super::control_density::height(ui);
    let (rect,_)=ui.allocate_exact_size(egui::vec2(width,h),egui::Sense::hover());
    // Trailing allocation is reversed even in LTR; paint direction belongs to
    // the application, not that allocation order.
    let mut field=ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(
        if app.rtl {egui::Layout::right_to_left(egui::Align::Center)} else {egui::Layout::left_to_right(egui::Align::Center)}));
    let response=super::craft_theme::dropdown_text(&mut field,rect,label,&super::craft_theme::current(ui.ctx()),
        ui.make_persistent_id(("monitor-preview-selector",aspect)))
        .on_hover_text(if aspect {"Preview aspect ratio; Fit uses the source. This does not change media metadata."}
            else {"Preview render resolution. Reduces GPU preview size, not the source file or external player."});
    egui::Popup::menu(&response).show(|ui| {
        let before=app.monitor_preview;
        if aspect {
            for value in [A::Source,A::Wide,A::Classic,A::Cinema] {
                if ui.selectable_label(before.aspect==value,value.label()).clicked() {
                    app.monitor_preview.aspect=value;ui.close();
                }
            }
        } else {
            for value in [R::Full,R::Half,R::Quarter,R::Eighth] {
                if ui.selectable_label(before.resolution==value,value.label()).clicked() {
                    app.monitor_preview.resolution=value;ui.close();
                }
            }
        }
        if app.monitor_preview!=before {app.save_config();ui.ctx().request_repaint();}
    });
}

fn draw_total(app: &mut PealayerApp, ui: &mut egui::Ui) {
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
    let total_response = crate::ui::controls::draw_duration_label(
        app, ui, total_label, total_tooltip, finite_timeline,
    );
    if finite_timeline && total_response.clicked() {
        app.show_remaining_time = !app.show_remaining_time;
        app.save_config();
    }
}

fn draw_timeline(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let can_seek = app.current_video_path.is_some() && app.is_seekable && app.duration > 0.0;
    super::controls::draw_elapsed_editor(app, ui, "nle-elapsed-editor", can_seek);
    // Anchor the duration and fullscreen affordance to the right
    // edge first, then give the seekbar the exact remaining width.
    // A former fixed 180 px reservation was wider than these
    // controls and left a visible, useless tail after fullscreen.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        draw_total(app, ui);
        draw_seekbar(app, ui, false);
    });
}

fn draw_seekbar(app: &mut PealayerApp, ui: &mut egui::Ui, filmcraft: bool) -> egui::Response {
    let has_video = app.current_video_path.is_some();
    let can_seek = has_video && app.is_seekable && app.duration > 0.0;
    let mut current_pos = if has_video { app.seek_pos.unwrap_or(app.playback_time) } else { 0.0 };
    let max_dur = if has_video && app.duration > 0.0 { app.duration } else { 1.0 };
    let slider = egui::Slider::new(&mut current_pos, 0.0..=max_dur)
                .show_value(false)
                .trailing_fill(true);
    // Keep egui's input, keyboard focus, accessibility and exact value mapping;
    // only the FilmCraft presentation is application-painted.
    let response = ui.scope(|ui| {
        if filmcraft {ui.set_opacity(0.0);}
        crate::ui::controls::add_fill_width_slider(ui, can_seek, slider)
    }).inner;
    response.context_menu(|ui| crate::ui::controls::transport_context_menu(app, ui));

    if filmcraft {
        super::craft_theme::paint_monitor_seek(ui,&response,(current_pos/max_dur) as f32,
            app.buffered_until().map(|end|(end/max_dur) as f32));
    } else if let Some(buffered_until) = app.buffered_until() {
        crate::ui::controls::paint_buffered_seekbar(
            ui,
            &response,
            (current_pos / app.duration).clamp(0.0, 1.0) as f32,
            (buffered_until / app.duration).clamp(0.0, 1.0) as f32,
        );
    }
    if filmcraft { paint_seek_ticks(ui, &response, app); }
    if let Some(chapter_time) = crate::ui::controls::paint_seekbar_markers_presentation(ui, &response, app,filmcraft) {
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
    response
}

/// Decorative FilmCraft ticks share the actual slider's mapping with markers,
/// hover previews and seeks. They stay inside its rail and add no hit targets.
pub(crate) fn paint_seek_ticks(ui: &egui::Ui, response: &egui::Response, app: &PealayerApp) {
    if !app.duration.is_finite() || app.duration <= 0.0 { return; }
    let range = super::controls::seekbar_value_range(response.rect);
    let width = *range.end() - *range.start();
    if width <= 0.0 { return; }
    let fps = if app.media_fps.is_finite() && app.media_fps > 0.0 { app.media_fps } else { 30.0 };
    let frames = app.duration * fps;
    if !frames.is_finite() || frames <= 0.0 { return; }
    let (minor, major) = super::craft_theme::ruler_steps(f64::from(width) / frames, fps.round().clamp(1.0, 240.0) as i64);
    let tokens = super::craft_theme::current(ui.ctx());
    let painter = ui.painter().with_clip_rect(response.rect.intersect(ui.clip_rect()));
    for index in 0..256 {
        let frame = index as f64 * minor as f64;
        if frame > frames { break; }
        let x = egui::lerp(range.clone(), (frame / frames) as f32);
        let major_tick = (index as i64 * minor) % major == 0;
        let height = if major_tick { 5.0 } else { 2.5 };
        painter.line_segment([egui::pos2(x, response.rect.bottom() - height), egui::pos2(x, response.rect.bottom())],
            egui::Stroke::new(1.0, tokens.text_dim.gamma_multiply(if major_tick { 0.5 } else { 0.3 })));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filmcraft_preview_dropdowns_really_change_choices_without_seeking_or_overwriting_source() {
        let _lock=crate::app::tests::lock_app_tests();
        for (aspect,choice) in [(true,"4:3"),(false,"1/4")] {
            let ctx=egui::Context::default();let mut app=PealayerApp::default();
            app.workspace_rendering=true; // Save is deferred, as in a real panel body.
            app.video_aspect_ratio=2.39;app.playback_time=5.25;app.is_paused=true;
            let mut frame=|events| ctx.run_ui(egui::RawInput {
                screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(600.0,400.0))),events,..Default::default()
            },|ui|draw_preview_selector(&mut app,ui,aspect));
            frame(vec![]).drop_without_applying_deltas();
            let point=egui::pos2(35.0,12.0);
            let event=|pos,pressed|egui::Event::PointerButton{pos,pressed,button:egui::PointerButton::Primary,modifiers:Default::default()};
            frame(vec![egui::Event::PointerMoved(point),event(point,true)]).drop_without_applying_deltas();
            frame(vec![event(point,false)]).drop_without_applying_deltas();
            let output=frame(vec![]);
            let target=output.shapes.iter().find_map(|s|match &s.shape{
                egui::Shape::Text(t) if t.galley.text()==choice=>Some(t.pos+t.galley.rect.center().to_vec2()),_=>None,
            }).expect("actual preview choice popup");
            output.drop_without_applying_deltas();
            frame(vec![egui::Event::PointerMoved(target),event(target,true)]).drop_without_applying_deltas();
            frame(vec![event(target,false)]).drop_without_applying_deltas();
            drop(frame);
            if aspect {assert_eq!(app.monitor_preview.aspect,crate::config::MonitorAspect::Classic);}
            else {assert_eq!(app.monitor_preview.resolution,crate::config::MonitorResolution::Quarter);}
            assert_eq!(app.video_aspect_ratio,2.39);assert_eq!(app.playback_time,5.25);
            assert!(app.is_paused && app.seek_pos.is_none() && app.deferred_config_save);
        }
    }
    use crate::app::MediaTrackType;

    fn metrics() -> Metrics {
        Metrics {
            transport: 172.0,
            timeline_min: 300.0,
            tracks: 130.0,
            gap: 8.0,
            gap_y: 4.0,
            row_height: 24.0,
            rtl: false,
            style: crate::config::NativeMonitorStyle::Compact,
            elapsed_width: 104.0,
            duration_width: 96.0,
        }
    }

    #[test]
    fn smallest_number_of_rows_is_selected_at_exact_fit() {
        let m = metrics();
        assert_eq!(m.rows(618.0), Rows::One);
        assert_eq!(m.rows(617.9), Rows::Two);
        assert_eq!(m.rows(326.0), Rows::Two);
        assert_eq!(m.rows(325.9), Rows::Three);
        assert_eq!(m.rows(240.0), Rows::Three);
        assert_eq!(m.height(618.0), 24.0);
        assert_eq!(m.height(326.0), 52.0);
        assert_eq!(m.height(300.0), 80.0);
        assert_eq!(m.height(240.0), 80.0);
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
            assert_eq!(rects.len(),3);
            let find=|group|rects.iter().find(|(candidate,_)|*candidate==group).unwrap().1;
            let tracks = find(Group::Tracks);
            if metrics().rows(area.width()) == Rows::Two {
                assert_eq!(find(Group::Timeline).width(),area.width());
                assert!(find(Group::Timeline).bottom() < find(Group::Transport).top());
                assert_eq!(find(Group::Transport).top(),tracks.top());
                assert!(find(Group::Transport).right() <= tracks.left());
            }
            assert!(
                (tracks.right() - area.right()).abs() < 1.0,
                "configured trailing controls must trail at {width}"
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

    #[test]
    fn customized_real_controls_fit_the_reserved_monitor_rows() {
        let _lock=crate::app::tests::lock_app_tests();
        for presentation in [crate::config::NativeMonitorStyle::Compact, crate::config::NativeMonitorStyle::FilmCraft] {
        for hidden in [true,false] {
            for width in [240.0,400.0,700.0,1400.0] {
                let ctx=egui::Context::default();
                let mut app=PealayerApp::default();
                app.native_monitor_style = presentation;
                app.control_bars.monitor.hidden=[hidden;crate::config::ControlBarAction::COUNT];
                for _ in 0..3 {
                    let mut output=ctx.run_ui(egui::RawInput {
                        screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,600.0))),..Default::default()
                    },|ui|{
                        let available=ui.available_rect_before_wrap();
                        let metrics=Metrics::measure(&app,ui);
                        draw(&mut app,ui,metrics);
                        let actual=ui.min_rect();
                        assert!(actual.right()<=available.right()+1.0,"width {width}, hidden={hidden}: {actual:?} vs {available:?}");
                        assert!(actual.height()<=metrics.height(available.width())+1.0,"reserved height at {width}, hidden={hidden}: {actual:?}");
                    });output.textures_delta.clear();
                }
            }
        }
        }
    }

    #[test]
    fn actual_monitor_overflow_opens_after_reflow_and_in_rtl() {
        let _lock=crate::app::tests::lock_app_tests();
        for presentation in [crate::config::NativeMonitorStyle::Compact, crate::config::NativeMonitorStyle::FilmCraft] {
        for width in [260.0,400.0,700.0,1400.0] {
            for rtl in [false,true] {
                let ctx=egui::Context::default(); let mut app=PealayerApp::default(); app.rtl=rtl;
                app.native_monitor_style = presentation;
                let owner=std::cell::Cell::new(egui::Id::NULL);
                let mut render=|app:&mut PealayerApp,events| {
                    let mut output=ctx.run_ui(egui::RawInput {
                        screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,600.0))),events,..Default::default()
                    },|ui| {
                        owner.set(ui.make_persistent_id(("control-bar",crate::config::ControlBarSurface::Monitor)));
                        let metrics=Metrics::measure(app,ui); draw(app,ui,metrics);
                    }); output.textures_delta.clear(); output
                };
                render(&mut app,vec![]); render(&mut app,vec![]);
                // Match the same root UI used by the production draw call.
                let pos=ctx.read_response(owner.get().with("overflow")).unwrap().rect.center();
                let pointer=|pressed|egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
                render(&mut app,vec![egui::Event::PointerMoved(pos),pointer(true)]);
                render(&mut app,vec![pointer(false)]);
                let output=render(&mut app,vec![]);
                assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Show / hide actions"))),"overflow at width={width} rtl={rtl}");
            }
        }
        }
    }

    #[test]
    fn filmcraft_measured_tiers_mirror_and_keep_every_slot_inside_the_footer() {
        for width in [180.0, 240.0, 390.0, 600.0, 1400.0] {
            for row_height in [25.0, 36.0, 52.0] {
                for rtl in [false, true] {
                    let ctx = egui::Context::default();
                    ctx.run_ui(egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 500.0))),
                        ..Default::default()
                    }, |ui| {
                        let m = Metrics { style: crate::config::NativeMonitorStyle::FilmCraft, row_height, rtl, ..metrics() };
                        let available = ui.available_rect_before_wrap();
                        let mut slots = Vec::new();
                        draw_layout(ui, m, |group, ui| slots.push((group, ui.max_rect())));
                        assert_eq!(slots.len(), 5);
                        let find = |kind| slots.iter().find(|(group, _)| *group == kind).unwrap().1;
                        let seek = find(Group::Seek);
                        assert_eq!(seek.width(), available.width());
                        assert!(seek.top() >= find(Group::Elapsed).bottom());
                        assert!(seek.top() >= find(Group::Duration).bottom());
                        assert!(find(Group::Transport).top() > seek.bottom());
                        let tail = find(Group::Tracks);
                        assert!((if rtl { tail.left() - available.left() } else { tail.right() - available.right() }).abs() < 0.01);
                        for (_, rect) in &slots {
                            assert!(rect.left() >= available.left() && rect.right() <= available.right() + 0.01);
                            assert_eq!(rect.height(), row_height);
                        }
                        let bottom = slots.iter().map(|(_, rect)| rect.bottom()).fold(0.0_f32, f32::max);
                        assert!((bottom - available.top() - m.height(width)).abs() < 0.01);
                    }).drop_without_applying_deltas();
                }
            }
        }
    }

    #[test]
    fn filmcraft_seek_decoration_stays_inside_the_same_interactive_slider() {
        let _lock = crate::app::tests::lock_app_tests();
        for scale in [1.0, 1.25, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let mut app = PealayerApp::default();
            app.duration = 120.0;
            app.media_fps = 23.976;
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let (rect, response) = ui.allocate_exact_size(egui::vec2(400.0, 25.0), egui::Sense::hover());
                paint_seek_ticks(ui, &response, &app);
                assert_eq!(response.rect, rect);
            });
            let mut count = 0;
            for shape in &output.shapes {
                if let egui::Shape::LineSegment { points, .. } = &shape.shape {
                    count += 1;
                    assert!(shape.clip_rect.contains(points[0]) && shape.clip_rect.contains(points[1]));
                    assert!((points[0].x - points[1].x).abs() < f32::EPSILON);
                }
            }
            assert!(count > 2 && count <= 256);
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn both_seek_presentations_keep_pointer_mapping_and_keyboard_adjustment() {
        let _lock = crate::app::tests::lock_app_tests();
        for filmcraft in [false, true] {
            let ctx = egui::Context::default();
            let mut app = PealayerApp::default();
            app.current_video_path = Some(std::path::PathBuf::from("test-media.mp4"));
            app.duration = 120.0;
            app.playback_time = 30.0;
            app.is_paused = true;
            app.is_seekable = true;
            app.nle_seekbar_hover_thumbnails = false;
            let render = |app: &mut PealayerApp, events| {
                let mut response = None;
                ctx.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 160.0))),
                    events, ..Default::default()
                }, |ui| { response = Some(draw_seekbar(app, ui, filmcraft)); }).drop_without_applying_deltas();
                response.unwrap()
            };
            render(&mut app, vec![]);
            let response = render(&mut app, vec![]);
            let range = super::super::controls::seekbar_value_range(response.rect);
            let pos = egui::pos2(egui::lerp(range, 0.75), response.rect.center().y);
            let pointer = |pressed| egui::Event::PointerButton {
                pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default(),
            };
            render(&mut app, vec![egui::Event::PointerMoved(pos), pointer(true)]);
            render(&mut app, vec![pointer(false)]);
            assert!((app.seek_pos.unwrap() - 90.0).abs() < 0.2, "presentation={filmcraft}");
            let before = app.seek_pos.unwrap();
            ctx.memory_mut(|memory| memory.request_focus(response.id));
            render(&mut app, vec![egui::Event::Key {
                key: egui::Key::ArrowRight, physical_key: Some(egui::Key::ArrowRight),
                pressed: true, repeat: false, modifiers: Default::default(),
            }]);
            assert!(app.seek_pos.unwrap() > before, "keyboard seek lost in presentation={filmcraft}");
        }
    }

    #[test]
    fn responsive_groups_mirror_without_overlap_or_height_changes() {
        for width in [240.0,390.0,600.0,900.0,1400.0] {
            let render=|rtl| {
                let ctx=egui::Context::default(); let mut rects=Vec::new();
                ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,300.0))),..Default::default()},|ui| {
                    let m=Metrics {rtl,..metrics()};
                    draw_layout(ui,m,|group,ui| {rects.push((group,ui.max_rect()));});
                }).drop_without_applying_deltas(); rects
            };
            let left=render(false); let right=render(true);
            for (group,a) in left {
                let b=right.iter().find(|(candidate,_)|*candidate==group).unwrap().1;
                assert_eq!(a.size(),b.size()); assert_eq!(a.y_range(),b.y_range());
                assert!((a.left()+b.right()-width).abs()<1.0,"mirror at {width}: {a:?} vs {b:?}");
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
}
