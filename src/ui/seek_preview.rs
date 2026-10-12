use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui;

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(120);
const PREVIEW_SIZE: egui::Vec2 = egui::vec2(160.0, 90.0);
const PREVIEW_GAP: f32 = 8.0;
const PREVIEW_CAPTION_GAP: f32 = 3.0;
const TIMECODE_ROW_HEIGHT: f32 = 18.0;
const PREVIEW_FRAME_MARGIN: f32 = 6.0;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreviewKey {
    media_target: String,
    second: u64,
    use_proxy: bool,
    proxy_url: Option<String>,
}

struct PreviewPixels {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

struct PreviewResult {
    key: PreviewKey,
    result: Result<PreviewPixels, String>,
}

pub struct SeekbarThumbnailPreview {
    tx: Sender<PreviewResult>,
    rx: Receiver<PreviewResult>,
    desired: Option<PreviewKey>,
    desired_since: Instant,
    pending: Option<PreviewKey>,
    texture: Option<(PreviewKey, egui::TextureHandle)>,
    failed: Option<PreviewKey>,
    owner: Option<egui::Id>,
}

impl Default for SeekbarThumbnailPreview {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            desired: None,
            desired_since: Instant::now(),
            pending: None,
            texture: None,
            failed: None,
            owner: None,
        }
    }
}

impl SeekbarThumbnailPreview {
    fn clear_hover(&mut self, owner: egui::Id) {
        // Simple and a docked monitor may both render in one frame. A surface
        // that isn't hovered must not invalidate the other surface's request.
        if self.owner != Some(owner) {
            return;
        }
        self.desired = None;
        self.failed = None;
        self.owner = None;
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(completed) = self.rx.try_recv() {
            if self.pending.as_ref() == Some(&completed.key) {
                self.pending = None;
            }
            if self.desired.as_ref() != Some(&completed.key) {
                continue;
            }
            match completed.result {
                Ok(pixels) => {
                    let texture = ctx.load_texture(
                        format!(
                            "seek-preview-{}-{}",
                            completed.key.second,
                            stable_target_hash(&completed.key.media_target)
                        ),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [pixels.width, pixels.height],
                            &pixels.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    self.texture = Some((completed.key, texture));
                    self.failed = None;
                }
                Err(error) => {
                    log::debug!("Seekbar thumbnail is unavailable: {error}");
                    self.failed = Some(completed.key);
                }
            }
        }
    }

    fn request(&mut self, key: PreviewKey, ctx: &egui::Context) {
        if self.desired.as_ref() != Some(&key) {
            self.desired = Some(key.clone());
            self.desired_since = Instant::now();
            self.failed = None;
        }
        let already_loaded = self
            .texture
            .as_ref()
            .is_some_and(|(loaded, _)| loaded == &key);
        let failed = self.failed.as_ref() == Some(&key);
        if self.pending.is_none()
            && !already_loaded
            && !failed
            && self.desired_since.elapsed() >= PREVIEW_DEBOUNCE
        {
            self.pending = Some(key.clone());
            let tx = self.tx.clone();
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                let result = crate::server::thumbnails::get_or_generate_seek_thumbnail(
                    &key.media_target,
                    key.second as f64,
                    key.use_proxy,
                    key.proxy_url.as_deref(),
                )
                .and_then(|path| decode_thumbnail(&path));
                let _ = tx.send(PreviewResult { key, result });
                repaint.request_repaint();
            });
        } else if !already_loaded && !failed {
            ctx.request_repaint_after(PREVIEW_DEBOUNCE);
        }
    }
}

fn decode_thumbnail(path: &std::path::Path) -> Result<PreviewPixels, String> {
    let image = image::open(path)
        .map_err(|error| format!("Could not decode seek-preview image: {error}"))?
        .to_rgba8();
    Ok(PreviewPixels {
        width: image.width() as usize,
        height: image.height() as usize,
        rgba: image.into_raw(),
    })
}

fn stable_target_hash(target: &str) -> u64 {
    target.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn hovered_seek_time(rect: egui::Rect, pointer_x: f32, duration: f64) -> Option<f64> {
    if rect.width() <= 0.0 || !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let range = super::controls::seekbar_value_range(rect);
    if range.end() <= range.start() {
        return None;
    }
    let fraction =
        ((pointer_x - range.start()) / (range.end() - range.start())).clamp(0.0, 1.0) as f64;
    Some((duration * fraction).min((duration - 0.001).max(0.0)))
}

fn preview_position(
    bounds: egui::Rect,
    seekbar: egui::Rect,
    pointer_x: f32,
    size: egui::Vec2,
) -> egui::Pos2 {
    let left = bounds.left() + PREVIEW_GAP;
    let top = bounds.top() + PREVIEW_GAP;
    let above = seekbar.top() - size.y - PREVIEW_GAP;
    let below = seekbar.bottom() + PREVIEW_GAP;
    let y = if above >= top { above } else { below };
    egui::pos2(
        (pointer_x - size.x / 2.0).clamp(left, (bounds.right() - size.x - PREVIEW_GAP).max(left)),
        y.clamp(top, (bounds.bottom() - size.y - PREVIEW_GAP).max(top)),
    )
}

pub fn draw(
    app: &mut crate::app::PealayerApp,
    ui: &egui::Ui,
    response: &egui::Response,
    thumbnails_enabled: bool,
    id_source: impl std::hash::Hash + std::fmt::Debug,
) {
    app.seekbar_thumbnail_preview.poll(ui.ctx());
    let owner = egui::Id::new(id_source);
    // A thumbnail helps the user decide where to begin a seek. Once the
    // pointer is actively operating the slider, the thumb and decoded video
    // frame are the direct feedback; leaving a floating card above the
    // gesture only obscures the content and competes with the drag.
    if !response.contains_pointer() || response.is_pointer_button_down_on() || response.dragged() {
        app.seekbar_thumbnail_preview.clear_hover(owner);
        return;
    }
    let Some(pointer) = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|pointer| response.rect.contains(*pointer))
    else {
        app.seekbar_thumbnail_preview.clear_hover(owner);
        return;
    };
    app.seekbar_thumbnail_preview.owner = Some(owner);
    let seconds = hovered_seek_time(response.rect, pointer.x, app.duration);
    let key = app
        .current_video_path
        .as_ref()
        .zip(seconds)
        .filter(|_| thumbnails_enabled && app.is_seekable)
        .map(|(target, seconds)| PreviewKey {
            media_target: target.to_string_lossy().into_owned(),
            second: seconds.floor() as u64, // Decode bucket, not the displayed position.
            use_proxy: app.open_url_use_proxy,
            proxy_url: (!app.open_url_proxy_url.trim().is_empty())
                .then(|| app.open_url_proxy_url.trim().to_string()),
        });
    if let Some(key) = &key {
        app.seekbar_thumbnail_preview.request(key.clone(), ui.ctx());
    } else {
        app.seekbar_thumbnail_preview.desired = None;
    }

    let mut captions = Vec::new();
    if let Some(seconds) = seconds {
        captions.push((
            super::controls::format_player_time(
                seconds,
                app.duration >= 3600.0,
                app.show_subseconds,
            ),
            true,
        ));
        if let Some(chapter) = super::controls::chapter_hover_label(&app.media_chapters(), seconds)
        {
            captions.push((chapter.replacen("Chapter", &app.tr("Chapter"), 1), false));
        }
        let range = super::controls::seekbar_value_range(response.rect);
        if let Some(marker) = app
            .timeline
            .keyframes
            .iter()
            .filter(|marker| marker.time_ms as f64 / 1000.0 <= app.duration)
            .map(|marker| {
                (
                    marker,
                    (egui::lerp(
                        range.clone(),
                        (marker.time_ms as f64 / 1000.0 / app.duration) as f32,
                    ) - pointer.x)
                        .abs(),
                )
            })
            .filter(|(_, distance)| *distance <= 6.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(marker, _)| marker)
        {
            captions.push((
                format!(
                    "{}{}",
                    app.tr("Keyframe"),
                    if marker.label.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", marker.label)
                    }
                ),
                false,
            ));
        }
    }
    let reason = match app.media_timeline_state() {
        crate::media::MediaTimelineState::NoMedia => Some("Open media to seek."),
        crate::media::MediaTimelineState::Determining => Some("Duration is still being determined; seeking will become available when MPV reports a timeline."),
        crate::media::MediaTimelineState::Live => Some("This live or duration-less source has no fixed seek range."),
        crate::media::MediaTimelineState::Finite { seekable: false, .. } => Some("This media reports a duration but does not support seeking."),
        crate::media::MediaTimelineState::Finite { .. } => None,
    };
    if let Some(reason) = reason {
        captions.push((app.tr(reason), false));
    }
    let bounds = ui.ctx().content_rect();
    let width =
        240.0_f32.min((bounds.width() - 2.0 * (PREVIEW_GAP + PREVIEW_FRAME_MARGIN)).max(1.0));
    let captions: Vec<_> = captions
        .into_iter()
        .map(|(text, time)| {
            ui.painter().layout(
                text,
                if time {
                    egui::FontId::monospace(11.0)
                } else {
                    egui::FontId::proportional(12.0)
                },
                if time {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                },
                width,
            )
        })
        .collect();
    let image_size = egui::vec2(
        PREVIEW_SIZE.x.min(width),
        PREVIEW_SIZE.y * (width / PREVIEW_SIZE.x).min(1.0),
    );
    let image_height = if key.is_some() {
        image_size.y + PREVIEW_CAPTION_GAP
    } else {
        0.0
    };
    let content_size = egui::vec2(
        width,
        image_height
            + captions
                .iter()
                .map(|line| line.size().y.max(TIMECODE_ROW_HEIGHT))
                .sum::<f32>(),
    );

    let frame = egui::Frame::popup(ui.style())
        .inner_margin(egui::Margin::same(PREVIEW_FRAME_MARGIN as i8))
        .corner_radius(7.0);
    let popup_size = content_size + egui::Vec2::splat(PREVIEW_FRAME_MARGIN * 2.0);
    // One shared popup ID: chapter ticks and workspace surfaces never create
    // another hover object. Fit above the cursor, or below on a top-edge bar.
    egui::Area::new(egui::Id::new("seekbar-thumbnail-preview"))
        .order(egui::Order::Tooltip)
        .fixed_pos(preview_position(
            bounds,
            response.rect,
            pointer.x,
            popup_size,
        ))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                // Reserve the complete image + caption geometry before
                // painting either state. Image widgets, spinners, errors, and
                // decoded textures are all placed into these same rectangles,
                // so an image becoming ready cannot reflow the card or move
                // the timecode from inside the viewport to beneath it.
                let (content_rect, _) = ui.allocate_exact_size(content_size, egui::Sense::hover());
                let image_rect = egui::Rect::from_min_size(
                    content_rect.min + egui::vec2((width - image_size.x) / 2.0, 0.0),
                    image_size,
                );
                if let Some(key) = &key {
                    ui.painter()
                        .rect_filled(image_rect, 4.0, ui.visuals().extreme_bg_color);
                    if let Some((loaded, texture)) = app
                        .seekbar_thumbnail_preview
                        .texture
                        .as_ref()
                        .filter(|(loaded, _)| loaded == key)
                    {
                        debug_assert_eq!(loaded.second, key.second);
                        ui.put(
                            image_rect,
                            egui::Image::new(texture).fit_to_exact_size(image_size),
                        );
                    } else {
                        if app.seekbar_thumbnail_preview.failed.as_ref() == Some(key) {
                            ui.painter().text(
                                image_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                app.tr("Preview unavailable"),
                                egui::FontId::proportional(12.0),
                                ui.visuals().weak_text_color(),
                            );
                        } else {
                            ui.put(
                                egui::Rect::from_center_size(
                                    image_rect.center(),
                                    egui::vec2(20.0, 20.0),
                                ),
                                egui::Spinner::new().size(18.0),
                            );
                        }
                    }
                }
                let mut y = content_rect.top() + image_height;
                for line in &captions {
                    let height = line.size().y.max(TIMECODE_ROW_HEIGHT);
                    ui.painter().galley(
                        egui::pos2(
                            content_rect.center().x - line.size().x / 2.0,
                            y + (height - line.size().y) / 2.0,
                        ),
                        line.clone(),
                        ui.visuals().text_color(),
                    );
                    y += height;
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hovered_time_maps_and_clamps_to_media_range() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        assert_eq!(hovered_seek_time(rect, 100.0, 120.0), Some(0.0));
        assert_eq!(hovered_seek_time(rect, 300.0, 120.0), Some(60.0));
        assert_eq!(hovered_seek_time(rect, 600.0, 120.0), Some(119.999));
    }

    #[test]
    fn hovered_time_rejects_invalid_ranges() {
        let empty = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::ZERO);
        assert_eq!(hovered_seek_time(empty, 0.0, 120.0), None);
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 10.0));
        assert_eq!(hovered_seek_time(rect, 50.0, 0.0), None);
        assert_eq!(hovered_seek_time(rect, 50.0, f64::NAN), None);
    }

    #[test]
    fn preview_is_anchored_entirely_above_seekbar() {
        let size = egui::vec2(252.0, 141.0);
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let bar = egui::Rect::from_min_size(egui::pos2(100.0, 220.0), egui::vec2(600.0, 20.0));
        let pos = preview_position(bounds, bar, 450.0, size);
        assert_eq!(pos.y + size.y + PREVIEW_GAP, bar.top());
        assert_eq!(pos.x + size.x / 2.0, 450.0);
    }

    #[test]
    fn preview_fits_top_edge_and_horizontal_edges() {
        let size = egui::vec2(252.0, 141.0);
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 20.0));
        for x in [0.0, 200.0, 400.0] {
            let pos = preview_position(bounds, bar, x, size);
            assert!(bounds.contains_rect(egui::Rect::from_min_size(pos, size)));
            assert!(pos.y >= bar.bottom() + PREVIEW_GAP);
        }
    }

    #[test]
    fn inactive_surface_does_not_clear_active_request() {
        let mut preview = SeekbarThumbnailPreview::default();
        let owner = egui::Id::new("simple");
        let key = PreviewKey {
            media_target: "fixture".into(),
            second: 42,
            use_proxy: false,
            proxy_url: None,
        };
        preview.owner = Some(owner);
        preview.desired = Some(key.clone());
        preview.clear_hover(egui::Id::new("monitor"));
        assert_eq!(preview.desired, Some(key));
        preview.clear_hover(owner);
        assert!(preview.desired.is_none());
    }

    #[test]
    fn native_surfaces_render_one_cursor_card_in_all_image_states() {
        let _lock = crate::app::tests::lock_app_tests();
        for surface in ["simple-seekbar-preview", "nle-seekbar-preview"] {
            for state in ["no-image", "loading", "ready", "failed"] {
                let ctx = egui::Context::default();
                let mut app = crate::app::PealayerApp::default();
                app.current_video_path = Some("memory-only-preview-fixture".into());
                app.duration = 240.0;
                app.is_seekable = true;
                app.media_metadata_loaded = true;
                app.show_subseconds = true;
                app.cached_media_chapters = vec![crate::media_info::MediaChapter {
                    index: 8,
                    title: "At the cursor, not at its start".into(),
                    time_seconds: 110.125,
                }];
                app.timeline.keyframes = vec![crate::four_d::models::TimelineKeyframe {
                    id: uuid::Uuid::new_v4(),
                    time_ms: 132000,
                    label: "Named marker".into(),
                }];
                let mut bar = egui::Rect::NOTHING;
                let mut pointer = None;
                let mut final_text = Vec::new();
                let mut expected_time = String::new();
                for frame in 0..5 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(640.0, 480.0),
                            )),
                            time: Some(frame as f64),
                            events: pointer
                                .map(|pos| vec![egui::Event::PointerMoved(pos)])
                                .unwrap_or_default(),
                            ..Default::default()
                        },
                        |ui| {
                            ui.add_space(280.0);
                            let mut value = 0.0;
                            let response = super::super::controls::add_fill_width_slider(
                                ui,
                                true,
                                egui::Slider::new(&mut value, 0.0..=240.0).show_value(false),
                            );
                            bar = response.rect;
                            super::super::controls::paint_seekbar_markers(ui, &response, &app);
                            if let Some(pointer) = pointer {
                                let seconds =
                                    hovered_seek_time(bar, pointer.x, app.duration).unwrap();
                                expected_time = super::super::controls::format_player_time(
                                    seconds, false, true,
                                );
                                let key = PreviewKey {
                                    media_target: "memory-only-preview-fixture".into(),
                                    second: seconds.floor() as u64,
                                    use_proxy: app.open_url_use_proxy,
                                    proxy_url: (!app.open_url_proxy_url.trim().is_empty())
                                        .then(|| app.open_url_proxy_url.trim().to_owned()),
                                };
                                // Inject every state, with a pending guard. No fixture may
                                // spawn ffmpeg, read media, start a player or touch hardware.
                                app.seekbar_thumbnail_preview.desired = Some(key.clone());
                                app.seekbar_thumbnail_preview.pending = Some(key.clone());
                                if state == "ready"
                                    && app.seekbar_thumbnail_preview.texture.is_none()
                                {
                                    app.seekbar_thumbnail_preview.texture = Some((
                                        key.clone(),
                                        ctx.load_texture(
                                            "fixture",
                                            egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
                                            egui::TextureOptions::LINEAR,
                                        ),
                                    ));
                                }
                                if state == "failed" {
                                    app.seekbar_thumbnail_preview.failed = Some(key);
                                }
                            }
                            draw(&mut app, ui, &response, state != "no-image", surface);
                        },
                    );
                    final_text = output
                        .shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                            _ => None,
                        })
                        .collect();
                    output.textures_delta.clear();
                    let range = super::super::controls::seekbar_value_range(bar);
                    pointer = Some(egui::pos2(egui::lerp(range, 0.55), bar.center().y));
                }
                assert_eq!(
                    final_text
                        .iter()
                        .filter(|text| **text == expected_time)
                        .count(),
                    1,
                    "{surface}/{state}: {final_text:?}"
                );
                assert_eq!(
                    final_text
                        .iter()
                        .filter(|text| text.starts_with("Chapter 9"))
                        .count(),
                    1
                );
                assert_eq!(
                    final_text
                        .iter()
                        .filter(|text| text.as_str() == "Keyframe · Named marker")
                        .count(),
                    1
                );
                assert!(!final_text
                    .iter()
                    .any(|text| text.contains("01:50.125") || text.contains("Seek through")));
                assert_eq!(
                    final_text.iter().any(|text| text == "Preview unavailable"),
                    state == "failed"
                );
            }
        }
    }
}
