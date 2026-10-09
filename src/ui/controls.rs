use crate::app::PealayerApp;
use eframe::egui;

const CONTROL_FADE_REPAINT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Transport intent is independent of the user's accent. All interaction
/// states keep the same stroke width, so hover/focus cannot shift the glyph.
pub(crate) fn transport_button<'a>(
    palette: crate::config::ColorPalette,
    ui: &egui::Ui,
    icon: &'a str,
    role: &str,
) -> egui::Button<'a> {
    let dark = ui.visuals().dark_mode;
    let intent = crate::ui::palette::color(palette, dark, role);
    let surface = crate::ui::palette::color(palette, dark, "surface-2");
    egui::Button::new(egui::RichText::new(icon).color(intent))
        .fill(egui::Color32::from_rgb(
            ((surface.r() as u16 * 88 + intent.r() as u16 * 12) / 100) as u8,
            ((surface.g() as u16 * 88 + intent.g() as u16 * 12) / 100) as u8,
            ((surface.b() as u16 * 88 + intent.b() as u16 * 12) / 100) as u8,
        ))
        .stroke(egui::Stroke::new(1.0, intent.gamma_multiply(0.45)))
}

pub(crate) fn playback_button_role(app: &PealayerApp) -> &'static str {
    if app.is_paused || app.is_playback_finished() { "green" } else { "amber" }
}

pub fn timecode_text(value: impl Into<String>) -> egui::RichText {
    egui::RichText::new(value).monospace()
}

/// Add a slider that consumes the horizontal space remaining in its row.
///
/// `Ui::add_sized` does not override egui's slider-track width; sliders read
/// `Spacing::slider_width` while laying themselves out. Keeping that detail in
/// one helper prevents the Simple and NLE transports from drifting back to the
/// short default slider width.
pub(crate) fn add_fill_width_slider(
    ui: &mut egui::Ui,
    enabled: bool,
    slider: egui::Slider<'_>,
) -> egui::Response {
    let seekbar_width = ui.available_width().max(1.0);
    let old_width = ui.spacing().slider_width;
    ui.spacing_mut().slider_width = seekbar_width;
    let response = ui.add_enabled(enabled, slider);
    ui.spacing_mut().slider_width = old_width;
    response
}

fn buffered_seekbar_rect(
    slider_rect: egui::Rect,
    playback_fraction: f32,
    buffered_fraction: f32,
    rail_height: f32,
) -> Option<egui::Rect> {
    if slider_rect.width() <= 0.0 || slider_rect.height() <= 0.0 {
        return None;
    }
    let playback_fraction = playback_fraction.clamp(0.0, 1.0);
    let buffered_fraction = buffered_fraction.clamp(0.0, 1.0);
    if buffered_fraction <= playback_fraction {
        return None;
    }

    // Match egui Slider's horizontal value range rather than its complete
    // response rectangle. Leaving the handle radius at the start prevents the
    // late buffer paint from crossing over the already-painted thumb.
    let handle_radius = slider_rect.height() / 2.5;
    let value_left = slider_rect.left() + handle_radius;
    let value_right = slider_rect.right() - handle_radius;
    if value_right <= value_left {
        return None;
    }
    let playback_x = egui::lerp(value_left..=value_right, playback_fraction);
    let buffered_x = egui::lerp(value_left..=value_right, buffered_fraction);
    let left = (playback_x + handle_radius * 0.78).min(buffered_x);
    if buffered_x - left < 0.75 {
        return None;
    }

    // The secondary range sits *inside* the real slider rail. It is slightly
    // slimmer than the playback fill so buffered media reads as supporting
    // information instead of a second competing progress value.
    let height = (rail_height * 0.58)
        .clamp(2.0, rail_height.max(2.0))
        .min(slider_rect.height());
    Some(egui::Rect::from_min_max(
        egui::pos2(left, slider_rect.center().y - height / 2.0),
        egui::pos2(buffered_x, slider_rect.center().y + height / 2.0),
    ))
}

/// Paint the cached/buffered range as a secondary segment inside an egui
/// slider's own rail. Call this after adding the slider: only the unplayed
/// portion is painted, and the segment begins beyond the thumb.
pub(crate) fn paint_buffered_seekbar(
    ui: &egui::Ui,
    response: &egui::Response,
    playback_fraction: f32,
    buffered_fraction: f32,
) {
    let rail_height = ui.spacing().slider_rail_height;
    let Some(buffered_rect) = buffered_seekbar_rect(
        response.rect,
        playback_fraction,
        buffered_fraction,
        rail_height,
    ) else {
        return;
    };
    let accent = ui.visuals().selection.bg_fill;
    let muted = ui.visuals().weak_text_color();
    let blend = |accent: u8, muted: u8| ((u16::from(accent) * 2 + u16::from(muted) * 3) / 5) as u8;
    let color = egui::Color32::from_rgba_unmultiplied(
        blend(accent.r(), muted.r()),
        blend(accent.g(), muted.g()),
        blend(accent.b(), muted.b()),
        185,
    );
    ui.painter()
        .rect_filled(buffered_rect, buffered_rect.height() / 2.0, color);
}

pub(crate) fn seekbar_value_range(rect: egui::Rect) -> std::ops::RangeInclusive<f32> {
    let radius = rect.height() / 2.5;
    rect.left() + radius..=rect.right() - radius
}

pub(crate) fn chapter_hover_label(chapters: &[crate::media_info::MediaChapter], seconds: f64) -> Option<String> {
    let chapter = chapters.iter().filter(|chapter| chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0 && chapter.time_seconds <= seconds)
        .max_by(|a,b| a.time_seconds.total_cmp(&b.time_seconds))?;
    Some(format!("Chapter {}{}", chapter.index + 1,
        if chapter.title.trim().is_empty() { String::new() } else { format!(" · {}", chapter.title) }))
}

fn chapter_near_pointer(chapters: &[crate::media_info::MediaChapter], rect: egui::Rect, duration: f64, pointer_x: f32) -> Option<&crate::media_info::MediaChapter> {
    if !duration.is_finite() || duration <= 0.0 { return None; }
    let range = seekbar_value_range(rect);
    if range.end() <= range.start() { return None; }
    let distance = |chapter: &crate::media_info::MediaChapter| (egui::lerp(range.clone(), (chapter.time_seconds / duration) as f32) - pointer_x).abs();
    chapters.iter().filter(|chapter| chapter.time_seconds.is_finite() && (0.0..=duration).contains(&chapter.time_seconds))
        .filter(|chapter| distance(chapter) <= 6.0)
        .min_by(|a,b| distance(a).total_cmp(&distance(b)))
}

/// Chapters are subtle contained ticks; exact project keyframes retain taller
/// red dividers. Geometry, hover and chapter snapping share the thumb range.
pub(crate) fn paint_seekbar_markers(
    ui: &egui::Ui,
    response: &egui::Response,
    app: &PealayerApp,
) -> Option<f64> {
    let duration = app.duration;
    if duration <= 0.0 || !duration.is_finite() {
        return None;
    }
    let chapters = app.media_chapters();
    let rect = response.rect;
    let range = seekbar_value_range(rect);
    if range.end() <= range.start() {
        return None;
    }
    let rgb = |hex: &str, fallback| { let [r,g,b] = crate::config::parse_rgb_hex(hex).unwrap_or(fallback); egui::Color32::from_rgb(r,g,b) };
    let chapter_color = rgb(&app.seekbar_markers.chapter_color, [150,150,150]);
    let active_color = rgb(&app.seekbar_markers.active_chapter_color, [176,176,176]);
    let keyframe_color = rgb(&app.seekbar_markers.keyframe_color, [239,68,68]);
    let x_for = |time: f64| egui::lerp(range.clone(), (time / duration).clamp(0.0,1.0) as f32);
    let current_time = app.seek_pos.unwrap_or(app.playback_time);
    if let Some(active) = chapters.iter().filter(|chapter| chapter.time_seconds.is_finite()
        && (0.0..=duration).contains(&chapter.time_seconds) && chapter.time_seconds <= current_time)
        .max_by(|a,b| a.time_seconds.total_cmp(&b.time_seconds)) {
        let end = chapters.iter().filter(|chapter| chapter.time_seconds > active.time_seconds)
            .map(|chapter| chapter.time_seconds).min_by(f64::total_cmp).unwrap_or(duration);
        let active_rect = egui::Rect::from_min_max(egui::pos2(x_for(active.time_seconds), rect.center().y - ui.spacing().slider_rail_height * 0.4),
            egui::pos2(x_for(end), rect.center().y + ui.spacing().slider_rail_height * 0.4));
        ui.painter().rect_filled(active_rect, 1.0, active_color.gamma_multiply(0.24));
    }
    let pointer = ui.ctx().pointer_hover_pos().filter(|point| rect.contains(*point));
    let nearest_chapter = pointer.and_then(|point| chapter_near_pointer(&chapters, rect, duration, point.x));
    let half_height = ui.spacing().slider_rail_height * 0.35;
    for chapter in &chapters {
        if !chapter.time_seconds.is_finite()
            || !(0.0..=duration).contains(&chapter.time_seconds)
        {
            continue;
        }
        let x = x_for(chapter.time_seconds);
        ui.painter().line_segment(
            [egui::pos2(x, rect.center().y - half_height),
             egui::pos2(x, rect.center().y + half_height)],
            egui::Stroke::new(1.0, chapter_color),
        );
    }
    for marker in &app.timeline.keyframes {
        let time = marker.time_ms as f64 / 1000.0;
        if time > duration { continue; }
        let x = x_for(time);
        let height = (ui.spacing().slider_rail_height * 0.5 + 3.0).min(rect.height() * 0.5);
        ui.painter().line_segment([egui::pos2(x,rect.center().y-height),egui::pos2(x,rect.center().y+height)], egui::Stroke::new(1.5,keyframe_color));
    }
    if let Some(point) = pointer {
        let nearest_keyframe = app.timeline.keyframes.iter().filter(|marker| marker.time_ms as f64 / 1000.0 <= duration)
            .filter(|marker| (x_for(marker.time_ms as f64/1000.0)-point.x).abs() <= 6.0)
            .min_by_key(|marker| ((x_for(marker.time_ms as f64/1000.0)-point.x).abs()*100.0) as u64);
        let label = if let Some(marker) = nearest_keyframe {
            format!("Keyframe{}\n{}", if marker.label.is_empty() { String::new() } else { format!(" · {}", marker.label) },
                crate::duration::format_time_value_ms(marker.time_ms))
        } else {
            let time = nearest_chapter.map_or_else(|| duration * ((point.x-range.start()) / (range.end()-range.start())).clamp(0.0,1.0) as f64, |chapter| chapter.time_seconds);
            chapter_hover_label(&chapters,time).unwrap_or_default()
        };
        if !label.is_empty() && !response.dragged() { response.clone().on_hover_text(label); }
    }
    let operating = response.changed() || response.clicked() || response.drag_stopped();
    nearest_chapter.filter(|_| operating).map(|chapter| chapter.time_seconds)
}

fn compact_number(value: f64) -> String {
    let mut rendered = format!("{value:.3}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportNudgeMode {
    Seek,
    FrameStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportNudgeDensity {
    Compact,
    Labeled,
}

const fn transport_nudge_mode(is_paused: bool) -> TransportNudgeMode {
    if is_paused {
        TransportNudgeMode::FrameStep
    } else {
        TransportNudgeMode::Seek
    }
}

const fn transport_nudge_width(density: TransportNudgeDensity) -> f32 {
    match density {
        TransportNudgeDensity::Compact => 44.0,
        TransportNudgeDensity::Labeled => 72.0,
    }
}

/// Draw one transport nudge whose purpose follows playback state.
///
/// While playing it seeks by the configured quick-seek interval. While
/// paused it steps by the configured frame count. Both captions occupy one
/// stable button and crossfade/slide between states, avoiding duplicate
/// controls and layout jumps.
pub fn draw_contextual_transport_nudge(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    id: egui::Id,
    direction: i32,
    density: TransportNudgeDensity,
) -> egui::Response {
    let mode = transport_nudge_mode(app.is_paused);
    let paused_t = ui.ctx().animate_bool_with_time_and_easing(
        id.with("paused-mode"),
        matches!(mode, TransportNudgeMode::FrameStep),
        0.18,
        egui::emath::easing::cubic_out,
    );
    let backwards = direction < 0;
    let quick_text = compact_number(app.quick_seek_seconds);
    let seek_icon = if backwards {
        crate::ui::icons::REWIND
    } else {
        crate::ui::icons::FAST_FORWARD
    };
    let frame_icon = if backwards {
        crate::ui::icons::SKIP_BACK
    } else {
        crate::ui::icons::SKIP_FORWARD
    };
    let width = transport_nudge_width(density);
    let (seek_caption, frame_caption, font_size) = match density {
        TransportNudgeDensity::Compact => (
            format!("{seek_icon} {quick_text}"),
            frame_icon.to_string(),
            11.5,
        ),
        TransportNudgeDensity::Labeled => (
            format!("{seek_icon} {quick_text} {}", app.tr("sec")),
            format!("{frame_icon} {}", app.frame_step_count),
            11.0,
        ),
    };

    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let font = egui::FontId::proportional(font_size);
        let text_color = visuals.text_color();
        let travel = 4.0;
        ui.painter().text(
            rect.center() - egui::vec2(0.0, travel * paused_t),
            egui::Align2::CENTER_CENTER,
            seek_caption,
            font.clone(),
            text_color.gamma_multiply(1.0 - paused_t),
        );
        ui.painter().text(
            rect.center() + egui::vec2(0.0, travel * (1.0 - paused_t)),
            egui::Align2::CENTER_CENTER,
            frame_caption,
            font,
            text_color.gamma_multiply(paused_t),
        );
    }

    let action_name = if backwards {
        app.tr("Back")
    } else {
        app.tr("Forward")
    };
    let tooltip = match mode {
        TransportNudgeMode::Seek => format!(
            "{} {} {} ({})",
            if backwards {
                app.tr("Seek backward")
            } else {
                app.tr("Seek forward")
            },
            quick_text,
            app.tr("seconds"),
            if backwards { "←" } else { "→" }
        ),
        TransportNudgeMode::FrameStep => format!(
            "{} {} {} ({})",
            action_name,
            app.frame_step_count,
            app.tr("frames"),
            if backwards { "Ctrl+← / [" } else { "Ctrl+→ / ]" }
        ),
    };
    let response = response.on_hover_text(tooltip);
    response.context_menu(|ui| transport_context_menu(app, ui));
    if response.clicked() {
        match mode {
            TransportNudgeMode::Seek => {
                app.seek_relative(direction as f64 * app.quick_seek_seconds)
            }
            TransportNudgeMode::FrameStep => app.step_frames(direction),
        }
    }
    response
}

pub fn begin_elapsed_edit(app: &mut PealayerApp) {
    let elapsed = resolve_display_time(app.seek_pos, app.playback_time);
    app.elapsed_time_input = format_player_time(elapsed, app.duration >= 3600.0, true);
    app.elapsed_time_original = app.elapsed_time_input.clone();
    app.elapsed_time_group = 0;
    app.elapsed_time_group_digits = 0;
    app.editing_elapsed_time = true;
    app.elapsed_edit_focus_requested = true;
}

fn timecode_groups(value: &str) -> Vec<std::ops::Range<usize>> {
    let mut groups = Vec::new();
    let mut start = None;
    for (index, byte) in value.bytes().enumerate() {
        if byte.is_ascii_digit() {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            groups.push(start..index);
        }
    }
    if let Some(start) = start {
        groups.push(start..value.len());
    }
    groups
}

/// A segment editor never inserts or deletes punctuation: every edit replaces
/// digits inside an existing group, and a completed group advances itself.
fn edit_timecode_segment(value: &mut String, group: &mut usize, digits: &mut usize, event: &egui::Event) {
    let groups = timecode_groups(value);
    if groups.is_empty() {
        return;
    }
    *group = (*group).min(groups.len() - 1);
    match event {
        egui::Event::Key { key: egui::Key::ArrowLeft, pressed: true, .. } => {
            *group = group.saturating_sub(1);
            *digits = 0;
        }
        egui::Event::Key { key: egui::Key::ArrowRight, pressed: true, .. } => {
            *group = (*group + 1).min(groups.len() - 1);
            *digits = 0;
        }
        egui::Event::Key { key: egui::Key::Home, pressed: true, .. } => {
            *group = 0;
            *digits = 0;
        }
        egui::Event::Key { key: egui::Key::End, pressed: true, .. } => {
            *group = groups.len() - 1;
            *digits = 0;
        }
        egui::Event::Key { key: egui::Key::Backspace | egui::Key::Delete, pressed: true, .. } => {
            let range = groups[*group].clone();
            value.replace_range(range.clone(), &"0".repeat(range.len()));
            *digits = 0;
        }
        egui::Event::Text(text) => {
            for character in text.chars() {
                if character == ':' || character == '.' {
                    *group = (*group + 1).min(groups.len() - 1);
                    *digits = 0;
                    continue;
                }
                let Some(digit) = character.to_digit(10).filter(|_| character.is_ascii_digit()) else {
                    continue;
                };
                let range = groups[*group].clone();
                let previous = if *digits == 0 { 0 } else { value[range.clone()].parse::<u32>().unwrap_or(0) };
                let maximum = if *group == groups.len() - 1 { 999 } else if groups.len() == 4 && *group == 0 { 99 } else { 59 };
                let next = (previous * 10 + digit).min(maximum);
                value.replace_range(range.clone(), &format!("{next:0width$}", width = range.len()));
                *digits += 1;
                if *digits >= range.len() {
                    *group = (*group + 1).min(groups.len() - 1);
                    *digits = 0;
                }
            }
        }
        egui::Event::Paste(text) => {
            if let Some(seconds) = parse_timecode(text) {
                let has_hours = groups.len() == 4;
                let limit = if has_hours { 360_000.0 } else { 3_600.0 };
                if seconds < limit {
                    *value = format_player_time(seconds, has_hours, true);
                    *group = groups.len() - 1;
                    *digits = 0;
                }
            }
        }
        _ => {}
    }
}

/// Draw the elapsed timestamp as an in-place editor. Clicking the timestamp
/// swaps only that label for a fixed-width field, so transport geometry does
/// not jump while an exact value is entered.
pub fn draw_elapsed_editor(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    id_source: impl std::hash::Hash + std::fmt::Debug,
    enabled: bool,
) -> egui::Response {
    let elapsed = resolve_display_time(app.seek_pos, app.playback_time);
    let rendered = format_player_time(elapsed, app.duration >= 3600.0, app.show_subseconds);
    let desired_width = if app.duration >= 3600.0 { 104.0 } else { 82.0 };
    let edit_id = ui.make_persistent_id(id_source);
    let editing = app.editing_elapsed_time && enabled;
    // When this editor owns keyboard focus, intercept text actions before
    // TextEdit can alter the fixed separators or length. Pointer events remain
    // with egui so clicking still selects a segment naturally.
    let mut timecode_events = Vec::new();
    if editing && ui.memory(|memory| memory.has_focus(edit_id)) {
        ui.input_mut(|input| input.events.retain(|event| {
            let captured = match event {
                egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Cut | egui::Event::Ime(_) => true,
                egui::Event::Key { key, .. } => *key != egui::Key::Tab,
                _ => false,
            };
            if captured { timecode_events.push(event.clone()); }
            !captured
        }));
    }
    let mut escape = false;
    let mut enter = false;
    for event in &timecode_events {
        match event {
            egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => escape = true,
            egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => enter = true,
            _ => edit_timecode_segment(
                &mut app.elapsed_time_input,
                &mut app.elapsed_time_group,
                &mut app.elapsed_time_group_digits,
                event,
            ),
        }
    }
    let mut display_buffer = rendered;
    let buffer = if editing {
        &mut app.elapsed_time_input
    } else {
        &mut display_buffer
    };

    // Render both states with the same widget metrics. The former Label ->
    // TextEdit swap used different font padding and allocation rules, which
    // made the time jump when editing began even though the outer row stayed
    // in place.
    let editor = crate::ui::dialog::singleline_text_edit(buffer)
        .id(edit_id)
        .font(egui::TextStyle::Monospace)
        .horizontal_align(egui::Align::Center)
        .vertical_align(egui::Align::Center)
        .margin(egui::Margin::symmetric(4, 2))
        .interactive(editing)
        .frame(if editing {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .stroke(ui.visuals().widgets.active.bg_stroke)
                .corner_radius(ui.visuals().widgets.active.corner_radius)
        } else {
            egui::Frame::NONE
        });
    let widget_response =
        ui.add_enabled_ui(enabled, |ui| ui.add_sized([desired_width, 22.0], editor));
    let mut response = widget_response.inner;

    if editing {
        response =
            response.on_hover_text(app.tr(
                "Type digits in each time group; Left/Right switches groups. Enter seeks; Escape cancels.",
            ));
        if app.elapsed_edit_focus_requested {
            response.request_focus();
            app.elapsed_edit_focus_requested = false;
        }
        if response.has_focus() {
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit_id) {
                let groups = timecode_groups(&app.elapsed_time_input);
                if response.clicked() {
                    if let Some(cursor) = state.cursor.char_range() {
                        let index = cursor.primary.index.0;
                        if let Some(selected) = groups.iter().position(|range| index <= range.end) {
                            app.elapsed_time_group = selected;
                            app.elapsed_time_group_digits = 0;
                        }
                    }
                }
                if let Some(range) = groups.get(app.elapsed_time_group) {
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(range.start),
                        egui::text::CCursor::new(range.end),
                    )));
                    state.store(ui.ctx(), edit_id);
                }
            }
        }
        if escape {
            app.elapsed_time_input = app.elapsed_time_original.clone();
            app.editing_elapsed_time = false;
        } else if enter {
            match parse_timecode(&app.elapsed_time_input) {
                Some(seconds) => {
                    app.seek_absolute(seconds);
                    app.editing_elapsed_time = false;
                }
                None => app.set_osd(app.tr("Enter a valid playback time")),
            }
        } else if response.lost_focus() {
            if app.elapsed_time_input != app.elapsed_time_original {
                if let Some(seconds) = parse_timecode(&app.elapsed_time_input) {
                    app.seek_absolute(seconds);
                }
            }
            app.editing_elapsed_time = false;
        }
    } else {
        // A non-interactive TextEdit deliberately has no click sense. Layer a
        // stable activation target over its exact rectangle so display and
        // editing keep identical text geometry.
        if enabled {
            let activation = ui.interact(
                response.rect,
                edit_id.with("activate"),
                egui::Sense::click(),
            );
            response = response.union(activation);
        }
        response = response.on_hover_text(app.tr("Click to enter an exact playback time."));
        if response.clicked() {
            begin_elapsed_edit(app);
        }
    }

    response.context_menu(|ui| {
        let copied_time = if app.editing_elapsed_time {
            app.elapsed_time_input.clone()
        } else {
            format_player_time(
                resolve_display_time(app.seek_pos, app.playback_time),
                app.duration >= 3600.0,
                app.show_subseconds,
            )
        };
        if ui
            .button(format!("{} {}", crate::ui::icons::COPY, app.tr("Copy")))
            .clicked()
        {
            ui.ctx().copy_text(copied_time);
            ui.close();
        }
        if ui
            .add_enabled(
                enabled,
                egui::Button::new(format!(
                    "{} {}",
                    crate::ui::icons::CLIPBOARD,
                    app.tr("Paste")
                )),
            )
            .clicked()
        {
            if !app.editing_elapsed_time {
                begin_elapsed_edit(app);
            }
            ui.ctx().memory_mut(|memory| memory.request_focus(edit_id));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            ui.close();
        }
        ui.separator();
        transport_context_menu(app, ui);
    });
    response
}

/// Shared seek/action menu for the NLE and Simple transports.
pub fn transport_context_menu(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let has_video = app.current_video_path.is_some();
    let can_seek = has_video && app.is_seekable;
    let quick = app.quick_seek_seconds;
    let quick_text = compact_number(quick);
    let frames = app.frame_step_count;
    if let Some(target) = app.current_video_path.as_ref().and_then(|path| path.to_str()) {
        if crate::remote_location::playback_proxy_for(target).is_some() {
            let can_previous = crate::remote_location::step(target,-1,false).is_some();
            let can_next = crate::remote_location::step(target,1,false).is_some();
            for (enabled,icon,label,command) in [(can_previous,crate::ui::icons::SKIP_BACK,"Previous file",crate::platform::interop::InteropCommand::Previous),(can_next,crate::ui::icons::SKIP_FORWARD,"Next file",crate::platform::interop::InteropCommand::Next)] {
                if ui.add_enabled(enabled,egui::Button::new(format!("{icon} {label}"))).clicked() { let ctx=ui.ctx().clone();app.apply_interop_command(&ctx,command,"Transport menu");ui.close(); }
            }
            ui.separator();
        }
    }

    ui.add_enabled_ui(has_video, |ui| {
        if ui
            .button(format!(
                "{} {}",
                if app.is_paused {
                    crate::ui::icons::PLAY
                } else {
                    crate::ui::icons::PAUSE
                },
                if app.is_paused {
                    app.tr("Play")
                } else {
                    app.tr("Pause")
                }
            ))
            .clicked()
        {
            app.toggle_playback();
            ui.close();
        }
    });
    ui.separator();
    if app.is_paused {
        ui.add_enabled_ui(has_video, |ui| {
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::SKIP_BACK,
                    app.tr("Back"),
                    frames,
                    app.tr("frames")
                ))
                .clicked()
            {
                app.step_frames(-1);
                ui.close();
            }
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::SKIP_FORWARD,
                    app.tr("Forward"),
                    frames,
                    app.tr("frames")
                ))
                .clicked()
            {
                app.step_frames(1);
                ui.close();
            }
        });
    } else {
        ui.add_enabled_ui(can_seek, |ui| {
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::REWIND,
                    app.tr("Back"),
                    quick_text,
                    app.tr("seconds")
                ))
                .clicked()
            {
                app.seek_relative(-quick);
                ui.close();
            }
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::FAST_FORWARD,
                    app.tr("Forward"),
                    quick_text,
                    app.tr("seconds")
                ))
                .clicked()
            {
                app.seek_relative(quick);
                ui.close();
            }
        });
    }
    ui.separator();
    ui.add_enabled_ui(can_seek, |ui| {
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::CLOCK,
                app.tr("Enter exact time…")
            ))
            .clicked()
        {
            begin_elapsed_edit(app);
            ui.close();
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::SKIP_BACK,
                app.tr("Go to beginning")
            ))
            .clicked()
        {
            app.seek_absolute(0.0);
            ui.close();
        }
    });
    ui.separator();
    let mut config_changed = false;
    let milliseconds_label = app.tr("Show milliseconds");
    let remaining_label = app.tr("Show time remaining");
    config_changed |= ui
        .checkbox(&mut app.show_subseconds, milliseconds_label)
        .changed();
    config_changed |= ui
        .checkbox(&mut app.show_remaining_time, remaining_label)
        .changed();
    if config_changed {
        app.save_config();
    }
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let controls_label = app.tr("Controls");
    let time_since_activity = app.last_mouse_activity.elapsed().as_secs_f32();
    let alpha = if app.pin_controls {
        1.0
    } else {
        (3.0 - time_since_activity).clamp(0.0, 1.0)
    };

    if alpha > 0.0 {
        let window_width = ui.available_width() - 20.0;
        let mut controls_frame = egui::Frame::window(ui.style()).multiply_with_opacity(alpha);
        controls_frame.inner_margin.right = 0;

        egui::Window::new(controls_label)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
            .min_width(window_width)
            .default_width(window_width)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .interactable(is_controls_interactable(alpha))
            .frame(controls_frame)
            .show(&ctx, |ui| {
                ui.set_opacity(alpha);
                multiply_style_opacity(ui.style_mut(), alpha);
                let has_video = app.current_video_path.is_some();
                let timeline_state = app.media_timeline_state();
                let can_seek = matches!(
                    timeline_state,
                    crate::media::MediaTimelineState::Finite {
                        seekable: true,
                        ..
                    }
                );
                let elapsed_time = resolve_display_time(app.seek_pos, app.playback_time);
                let is_long_video = app.duration >= 3600.0;
                let show_subseconds = app.show_subseconds;
                let display_total = if app.show_remaining_time {
                    -(app.duration - elapsed_time)
                } else {
                    app.duration
                };
                let (total_text, total_tooltip, total_is_toggle) = match timeline_state {
                    crate::media::MediaTimelineState::NoMedia => (
                        format!("{} --:--", crate::ui::icons::CLOCK),
                        app.tr("Open media to see its duration."),
                        false,
                    ),
                    crate::media::MediaTimelineState::Determining => (
                        format!(
                            "{} {}",
                            crate::ui::icons::HOURGLASS_MEDIUM,
                            app.tr("Determining duration…")
                        ),
                        app.tr("MPV is still reading media metadata. Duration and seeking will update when available."),
                        false,
                    ),
                    crate::media::MediaTimelineState::Live => (
                        format!("{} {}", crate::ui::icons::BROADCAST, app.tr("LIVE")),
                        app.tr("This live or duration-less source has no fixed endpoint or seek range."),
                        false,
                    ),
                    crate::media::MediaTimelineState::Finite { .. } => (
                        format_player_time(display_total, is_long_video, show_subseconds),
                        if app.show_remaining_time {
                            app.tr("Showing time remaining. Click to show total duration.")
                        } else {
                            app.tr("Showing total duration. Click to show time remaining.")
                        },
                        true,
                    ),
                };
                let seek_tooltip = match timeline_state {
                    crate::media::MediaTimelineState::NoMedia => app.tr("Open media to seek."),
                    crate::media::MediaTimelineState::Determining => app.tr(
                        "Duration is still being determined; seeking will become available when MPV reports a timeline.",
                    ),
                    crate::media::MediaTimelineState::Live => app.tr(
                        "This live or duration-less source has no fixed seek range.",
                    ),
                    crate::media::MediaTimelineState::Finite {
                        seekable: false, ..
                    } => app.tr("This media reports a duration but does not support seeking."),
                    crate::media::MediaTimelineState::Finite { .. } => {
                        app.tr("Seek through the media timeline.")
                    }
                };

                let fullscreen_tooltip = format!("{} (F)", app.tr("Fullscreen"));
                let pin_tooltip = if app.pin_controls {
                    app.tr("Unpin Controls")
                } else {
                    app.tr("Pin Controls")
                };
                let audio_tooltip = app.tr("Audio Settings...");
                let workspace_tooltip = app.tr("Switch workspace");
                let subtitles_tooltip = app.tr("Subtitle Settings...");
                let mute_tooltip = format!(
                    "{} (M)",
                    if app.is_muted {
                        app.tr("Unmute")
                    } else {
                        app.tr("Mute")
                    }
                );
                let pin_icon = if app.pin_controls {
                    crate::ui::icons::PUSH_PIN_SLASH
                } else {
                    crate::ui::icons::PUSH_PIN
                };
                let mute_icon = if app.is_muted {
                    crate::ui::icons::SPEAKER_SLASH
                } else {
                    crate::ui::icons::SPEAKER_HIGH
                };
                let transport_palette = app.color_palette;
                let mute_role = if app.is_muted { "amber" } else { "muted" };
                let mut volume = app.volume;
                let mut toggle_fullscreen = false;
                let mut toggle_pin = false;
                let mut toggle_audio = false;
                let mut toggle_workspace = false;
                let mut toggle_subtitles = false;
                let mut toggle_mute = false;
                let mut toggle_total_mode = false;
                let mut volume_update = None;

                egui::containers::Sides::new().shrink_left().show(
                    ui,
                    |ui| {
                        ui.add_enabled_ui(has_video, |ui| {
                            let mut toggle_playback_requested = false;
                            draw_contextual_transport_nudge(
                                app,
                                ui,
                                ui.make_persistent_id("simple-transport-back"),
                                -1,
                                TransportNudgeDensity::Labeled,
                            );

                            let play_icon = if app.is_playback_finished() {
                                crate::ui::icons::ARROW_COUNTER_CLOCKWISE
                            } else if app.is_paused {
                                crate::ui::icons::PLAY
                            } else {
                                crate::ui::icons::PAUSE
                            };
                            let play_tooltip = if app.is_playback_finished() {
                                app.tr("Replay")
                            } else if app.is_paused {
                                app.tr("Play")
                            } else {
                                app.tr("Pause")
                            };
                            let play_response = ui
                                .add_sized([30.0, 22.0], transport_button(transport_palette, ui, play_icon, playback_button_role(app)))
                                .on_hover_text(play_tooltip);
                            play_response.context_menu(|ui| transport_context_menu(app, ui));
                            if play_response.clicked() {
                                // Apply after both contextual buttons are drawn
                                // so one frame cannot render mismatched modes.
                                toggle_playback_requested = true;
                            }

                            draw_contextual_transport_nudge(
                                app,
                                ui,
                                ui.make_persistent_id("simple-transport-forward"),
                                1,
                                TransportNudgeDensity::Labeled,
                            );
                            if toggle_playback_requested {
                                app.toggle_playback();
                            }
                        });

                        draw_elapsed_editor(
                            app,
                            ui,
                            "simple-elapsed-editor",
                            has_video && can_seek,
                        );

                        let mut current_pos = if has_video {
                            app.seek_pos.unwrap_or(app.playback_time)
                        } else {
                            0.0
                        };
                        let max_duration = app.duration.max(1.0);
                        let slider = egui::Slider::new(&mut current_pos, 0.0..=max_duration)
                            .show_value(false)
                            .trailing_fill(true);
                        let response = add_fill_width_slider(ui, can_seek, slider);
                        let response = if can_seek {
                            if app.seekbar_hover_thumbnails {
                                response
                            } else {
                                response.on_hover_text(&seek_tooltip)
                            }
                        } else {
                            response.on_disabled_hover_text(&seek_tooltip)
                        };
                        response.context_menu(|ui| transport_context_menu(app, ui));

                        if let Some(buffered_until) = app.buffered_until() {
                            paint_buffered_seekbar(
                                ui,
                                &response,
                                (current_pos / app.duration).clamp(0.0, 1.0) as f32,
                                (buffered_until / app.duration).clamp(0.0, 1.0) as f32,
                            );
                        }
                        if let Some(chapter_time) = paint_seekbar_markers(ui, &response, app) { current_pos = chapter_time; }
                        let show_seek_preview = app.seekbar_hover_thumbnails;
                        crate::ui::seek_preview::draw(
                            app,
                            ui,
                            &response,
                            show_seek_preview,
                            "simple-seekbar-preview",
                        );
                        // `changed` covers both dragging and a single click on
                        // the seekbar. Restricting this to `dragged` left click
                        // seeks without a preview request on some egui input
                        // paths, so the thumb and decoded frame disagreed.
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
                    },
                    |ui| {
                        toggle_fullscreen = ui
                            .button(crate::ui::icons::ARROWS_OUT)
                            .on_hover_text(fullscreen_tooltip)
                            .clicked();
                        toggle_pin = ui.button(pin_icon).on_hover_text(pin_tooltip).clicked();
                        toggle_audio = ui
                            .button(crate::ui::icons::MUSIC_NOTE)
                            .on_hover_text(audio_tooltip)
                            .clicked();
                        toggle_workspace = ui
                            .button(crate::ui::icons::TABS)
                            .on_hover_text(workspace_tooltip)
                            .clicked();
                        toggle_subtitles = ui
                            .button(crate::ui::icons::SUBTITLES)
                            .on_hover_text(subtitles_tooltip)
                            .clicked();

                        ui.add_enabled_ui(has_video, |ui| {
                            let volume_slider =
                                egui::Slider::new(&mut volume, 0.0..=130.0).show_value(false);
                            let volume_response = ui.add_sized([80.0, 15.0], volume_slider);
                            if volume_response.changed() {
                                volume_update = Some(volume);
                            }
                            if volume_response.hovered() {
                                let scroll = ui.input(|input| {
                                    let mut delta = input.smooth_scroll_delta;
                                    if delta.x == 0.0 && delta.y == 0.0 {
                                        for event in &input.events {
                                            if let egui::Event::MouseWheel {
                                                delta: wheel_delta,
                                                ..
                                            } = event
                                            {
                                                delta += *wheel_delta;
                                            }
                                        }
                                    }
                                    delta
                                });
                                if scroll.y != 0.0 {
                                    volume_update = Some(
                                        (volume + if scroll.y > 0.0 { 2.0 } else { -2.0 })
                                            .clamp(0.0, 130.0),
                                    );
                                }
                            }
                            toggle_mute = ui
                                .add(transport_button(transport_palette, ui, mute_icon, mute_role))
                                .on_hover_text(mute_tooltip)
                                .clicked();
                        });

                        let total_label = if total_is_toggle {
                            timecode_text(total_text)
                        } else {
                            egui::RichText::new(total_text)
                        };
                        let total_response = ui
                            .add(egui::Label::new(total_label).sense(if total_is_toggle {
                                egui::Sense::click()
                            } else {
                                egui::Sense::hover()
                            }))
                            .on_hover_text(total_tooltip);
                        toggle_total_mode = total_is_toggle && total_response.clicked();
                    },
                );

                if toggle_fullscreen {
                    app.toggle_fullscreen(&ctx);
                }
                if toggle_pin {
                    app.pin_controls = !app.pin_controls;
                    app.set_osd(if app.pin_controls {
                        app.tr("Controls Pinned")
                    } else {
                        app.tr("Controls Unpinned")
                    });
                    app.save_config();
                }
                if toggle_audio {
                    app.show_audio_settings = !app.show_audio_settings;
                }
                if toggle_workspace {
                    app.cycle_workspace_profile(&ctx);
                }
                if toggle_subtitles {
                    app.show_sub_settings = !app.show_sub_settings;
                }
                if let Some(new_volume) = volume_update {
                    let _ = app.mpv.set_property("volume", new_volume);
                    app.volume = new_volume;
                    app.set_osd(format!("{}: {:.0}%", app.tr("Volume"), new_volume));
                    app.save_config();
                }
                if toggle_mute {
                    app.toggle_audio_muted();
                }
                if toggle_total_mode {
                    app.show_remaining_time = !app.show_remaining_time;
                    app.save_config();
                }
            });

        if time_since_activity < 3.0 && !app.pin_controls {
            // The old 16 ms loop repainted the entire application at 60 Hz
            // whenever controls were visible, even while media was paused.
            // Ten fade samples per second remain visually smooth while keeping
            // the idle renderer reactive rather than continuously busy.
            ctx.request_repaint_after(CONTROL_FADE_REPAINT_INTERVAL);
        }
    }
}

pub fn is_controls_interactable(alpha: f32) -> bool {
    alpha >= 0.05
}

pub fn multiply_style_opacity(style: &mut egui::Style, alpha: f32) {
    let fade_color = |color: &mut egui::Color32| {
        *color = color.linear_multiply(alpha);
    };

    // When override_text_color is None, egui uses visuals.text_color().
    // Set override_text_color explicitly to faded text_color so all labels,
    // button text, and icon glyphs fade smoothly.
    let base_text_color = style
        .visuals
        .override_text_color
        .unwrap_or_else(|| style.visuals.text_color());
    style.visuals.override_text_color = Some(base_text_color.linear_multiply(alpha));

    fade_color(&mut style.visuals.warn_fg_color);
    fade_color(&mut style.visuals.error_fg_color);
    fade_color(&mut style.visuals.hyperlink_color);
    fade_color(&mut style.visuals.extreme_bg_color);
    fade_color(&mut style.visuals.faint_bg_color);
    fade_color(&mut style.visuals.code_bg_color);
    fade_color(&mut style.visuals.window_stroke.color);

    let widgets = &mut style.visuals.widgets;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        fade_color(&mut state.bg_fill);
        fade_color(&mut state.fg_stroke.color);
        fade_color(&mut state.bg_stroke.color);
    }

    fade_color(&mut style.visuals.selection.bg_fill);
    fade_color(&mut style.visuals.selection.stroke.color);
}

pub fn resolve_display_time(seek_pos: Option<f64>, playback_time: f64) -> f64 {
    seek_pos.unwrap_or(playback_time)
}

pub fn format_player_time(time: f64, include_hours: bool, show_subseconds: bool) -> String {
    let negative = time < 0.0;
    let absolute = time.abs();
    let total_millis = (absolute * 1000.0).round() as i64;
    let whole = total_millis / 1000;
    let millis = total_millis % 1000;
    let formatted = if include_hours {
        if show_subseconds {
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                whole / 3600,
                (whole / 60) % 60,
                whole % 60,
                millis
            )
        } else {
            format!(
                "{:02}:{:02}:{:02}",
                whole / 3600,
                (whole / 60) % 60,
                whole % 60
            )
        }
    } else if show_subseconds {
        format!("{:02}:{:02}.{:03}", (whole / 60) % 60, whole % 60, millis)
    } else {
        format!("{:02}:{:02}", (whole / 60) % 60, whole % 60)
    };
    if negative {
        format!("-{formatted}")
    } else {
        formatted
    }
}

pub fn parse_timecode(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return None;
    }
    let fields = trimmed.split(':').collect::<Vec<_>>();
    if fields.len() > 3 {
        return None;
    }
    let seconds = match fields.as_slice() {
        [seconds] => seconds.parse::<f64>().ok()?,
        [minutes, seconds] => {
            let seconds = seconds.parse::<f64>().ok()?;
            (seconds < 60.0).then_some(())?;
            minutes.parse::<u64>().ok()? as f64 * 60.0 + seconds
        }
        [hours, minutes, seconds] => {
            let minutes = minutes.parse::<u64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            (minutes < 60 && seconds < 60.0).then_some(())?;
            hours.parse::<u64>().ok()? as f64 * 3600.0 + minutes as f64 * 60.0 + seconds
        }
        _ => return None,
    };
    seconds.is_finite().then_some(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_snapping_uses_thumb_range_and_preserves_exact_timestamp() {
        let chapters = vec![crate::media_info::MediaChapter { index: 0, title: "Opening".into(), time_seconds: 10.125 }];
        let rect = egui::Rect::from_min_size(egui::pos2(100.0,20.0), egui::vec2(400.0,20.0));
        let x = egui::lerp(seekbar_value_range(rect), 10.125 / 120.0);
        assert_eq!(chapter_near_pointer(&chapters, rect, 120.0, x + 5.5).unwrap().time_seconds, 10.125);
        assert!(chapter_near_pointer(&chapters, rect, 120.0, x + 6.1).is_none());
        assert!(chapter_near_pointer(&chapters, rect, f64::NAN, x).is_none());
        assert_eq!(chapter_hover_label(&chapters, 11.0).as_deref(), Some("Chapter 1 · Opening"));
        assert!(chapter_hover_label(&chapters, 9.0).is_none());
    }

    #[test]
    fn test_multiply_style_opacity_handles_none_override_text_color() {
        let mut style = egui::Style::default();
        style.visuals.override_text_color = None;
        let initial_text_color = style.visuals.text_color();

        multiply_style_opacity(&mut style, 0.5);

        assert_eq!(
            style.visuals.override_text_color,
            Some(initial_text_color.linear_multiply(0.5))
        );
    }

    #[test]
    fn test_controls_interactable_threshold() {
        let alpha_active = 0.5;
        let alpha_faded = 0.02;
        assert!(is_controls_interactable(alpha_active));
        assert!(!is_controls_interactable(alpha_faded));
    }

    #[test]
    fn test_multiply_style_opacity() {
        let mut style = egui::Style::default();
        style.visuals.override_text_color =
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200));
        let orig_fill = style.visuals.widgets.inactive.bg_fill;

        multiply_style_opacity(&mut style, 0.5);

        assert_eq!(
            style.visuals.override_text_color,
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200).linear_multiply(0.5))
        );
        assert_eq!(
            style.visuals.widgets.inactive.bg_fill,
            orig_fill.linear_multiply(0.5)
        );
    }

    #[test]
    fn test_multiply_style_opacity_zero() {
        let mut style = egui::Style::default();
        style.visuals.override_text_color =
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200));

        multiply_style_opacity(&mut style, 0.0);

        assert_eq!(
            style.visuals.override_text_color,
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200).linear_multiply(0.0))
        );
    }

    #[test]
    fn test_pin_controls_alpha_calculation() {
        let pin_controls = true;
        let time_since_activity: f32 = 10.0;
        let alpha = if pin_controls {
            1.0
        } else {
            (3.0 - time_since_activity).clamp(0.0, 1.0)
        };
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn test_play_button_fixed_size_constant() {
        let button_size = egui::vec2(30.0, 22.0);
        assert_eq!(button_size.x, 30.0);
        assert_eq!(button_size.y, 22.0);
    }

    #[test]
    fn contextual_nudges_seek_while_playing_and_step_while_paused() {
        assert_eq!(transport_nudge_mode(false), TransportNudgeMode::Seek);
        assert_eq!(transport_nudge_mode(true), TransportNudgeMode::FrameStep);
    }

    #[test]
    fn contextual_nudge_slots_keep_stable_geometry_during_transition() {
        assert_eq!(transport_nudge_width(TransportNudgeDensity::Compact), 44.0);
        assert_eq!(transport_nudge_width(TransportNudgeDensity::Labeled), 72.0);
    }

    #[test]
    fn test_display_total_time_calculation() {
        let playback_time = 83.0;
        let duration = 300.0;
        let show_remaining_time = true;

        let display_total = if show_remaining_time {
            -(duration - playback_time)
        } else {
            duration
        };
        assert_eq!(display_total, -217.0);
    }

    #[test]
    fn test_resolve_display_time_scrubbing_vs_playback() {
        let playback_time = 45.0;
        let seek_pos = Some(120.0);
        assert_eq!(resolve_display_time(seek_pos, playback_time), 120.0);

        let no_seek: Option<f64> = None;
        assert_eq!(resolve_display_time(no_seek, playback_time), 45.0);
    }

    #[test]
    fn subsecond_timecode_uses_a_decimal_separator() {
        assert_eq!(format_player_time(65.125, false, true), "01:05.125");
        assert_eq!(format_player_time(-3661.5, true, true), "-01:01:01.500");
    }

    #[test]
    fn exact_timecode_parser_accepts_player_formats() {
        assert_eq!(parse_timecode("90.5"), Some(90.5));
        assert_eq!(parse_timecode("01:30.500"), Some(90.5));
        assert_eq!(parse_timecode("01:02:03.250"), Some(3723.25));
        assert_eq!(parse_timecode("01:75"), None);
        assert_eq!(parse_timecode("-00:01"), None);
        assert_eq!(parse_timecode("not a time"), None);
    }

    #[test]
    fn elapsed_segment_editor_preserves_separators_and_advances_groups() {
        let mut value = "01:02:03.004".to_string();
        let mut group = 1;
        let mut digits = 0;
        edit_timecode_segment(&mut value, &mut group, &mut digits, &egui::Event::Text("45".into()));
        assert_eq!(value, "01:45:03.004");
        assert_eq!(group, 2);
        edit_timecode_segment(&mut value, &mut group, &mut digits, &egui::Event::Text("x".into()));
        assert_eq!(value, "01:45:03.004");
        edit_timecode_segment(&mut value, &mut group, &mut digits, &egui::Event::Text("7".into()));
        assert_eq!(value, "01:45:07.004");
        edit_timecode_segment(&mut value, &mut group, &mut digits, &egui::Event::Paste("00:01:20.250".into()));
        assert_eq!(value, "00:01:20.250");
        assert_eq!(timecode_groups(&value), vec![0..2, 3..5, 6..8, 9..12]);
    }

    #[test]
    fn test_seekbar_disabled_position_and_range() {
        let has_video = false;
        let playback_time = 0.0;
        let duration = 0.0;

        let current_pos = if has_video { playback_time } else { 0.0 };
        let max_dur = if has_video && duration > 0.0 {
            duration
        } else {
            1.0
        };

        assert_eq!(current_pos, 0.0);
        assert_eq!(max_dur, 1.0);
    }

    #[test]
    fn buffered_seekbar_segment_is_centered_inside_the_slider_rail() {
        let slider = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(240.0, 20.0));
        let segment = buffered_seekbar_rect(slider, 0.25, 0.75, 8.0).unwrap();
        assert_eq!(segment.center().y, slider.center().y);
        assert!(segment.height() < 8.0);
        assert!(segment.left() > slider.left());
        assert!(segment.right() < slider.right());
    }

    #[test]
    fn buffered_seekbar_does_not_paint_behind_played_or_unbuffered_media() {
        let slider = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 20.0));
        assert!(buffered_seekbar_rect(slider, 0.75, 0.50, 8.0).is_none());
        assert!(buffered_seekbar_rect(slider, 0.50, 0.50, 8.0).is_none());
    }

    #[test]
    fn fill_width_slider_uses_the_remaining_transport_width() {
        let mut measured_width = 0.0;
        let mut value = 25.0;

        egui::__run_test_ui(|ui| {
            ui.set_width(640.0);
            ui.horizontal(|ui| {
                ui.add_sized([120.0, 22.0], egui::Label::new("transport"));
                let slider = egui::Slider::new(&mut value, 0.0..=100.0).show_value(false);
                measured_width = add_fill_width_slider(ui, true, slider).rect.width();
            });
        });

        assert!(
            measured_width > 450.0,
            "seekbar should consume the remaining row, got {measured_width}px"
        );
    }
}
