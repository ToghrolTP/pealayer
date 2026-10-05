use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui;

const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(120);
const PREVIEW_SIZE: egui::Vec2 = egui::vec2(160.0, 90.0);
const PREVIEW_GAP: f32 = 8.0;
const TIMECODE_ROW_HEIGHT: f32 = 18.0;

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
        }
    }
}

impl SeekbarThumbnailPreview {
    fn clear_hover(&mut self) {
        self.desired = None;
        self.failed = None;
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
    let fraction = ((pointer_x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
    Some((duration * fraction).min((duration - 0.001).max(0.0)))
}

fn preview_top(seekbar_top: f32, popup_height: f32) -> f32 {
    seekbar_top - popup_height - PREVIEW_GAP
}

pub fn draw(
    app: &mut crate::app::PealayerApp,
    ui: &egui::Ui,
    response: &egui::Response,
    enabled: bool,
    id_source: impl std::hash::Hash + std::fmt::Debug,
) {
    app.seekbar_thumbnail_preview.poll(ui.ctx());
    if !enabled || !app.is_seekable || app.duration <= 0.0 {
        app.seekbar_thumbnail_preview.clear_hover();
        return;
    }
    let Some(pointer) = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|pointer| response.rect.contains(*pointer))
    else {
        app.seekbar_thumbnail_preview.clear_hover();
        return;
    };
    let Some(seconds) = hovered_seek_time(response.rect, pointer.x, app.duration) else {
        return;
    };
    let Some(media_target) = app
        .current_video_path
        .as_ref()
        .map(|target| target.to_string_lossy().into_owned())
    else {
        return;
    };
    let key = PreviewKey {
        media_target,
        second: seconds.floor() as u64,
        use_proxy: app.open_url_use_proxy,
        proxy_url: (!app.open_url_proxy_url.trim().is_empty())
            .then(|| app.open_url_proxy_url.trim().to_string()),
    };
    app.seekbar_thumbnail_preview.request(key.clone(), ui.ctx());

    let frame = egui::Frame::popup(ui.style())
        .inner_margin(egui::Margin::same(6))
        .corner_radius(7.0);
    let popup_size = PREVIEW_SIZE + egui::vec2(12.0, TIMECODE_ROW_HEIGHT + 15.0);
    let bounds = ui.ctx().content_rect();
    let x = (pointer.x - popup_size.x / 2.0).clamp(
        bounds.left() + 8.0,
        (bounds.right() - popup_size.x - 8.0).max(bounds.left() + 8.0),
    );
    // Keep the preview entirely above the control. Clamping its top edge to
    // the viewport used to push a tall card back over the seekbar.
    let y = preview_top(response.rect.top(), popup_size.y);
    egui::Area::new(egui::Id::new(("seekbar-thumbnail-preview", id_source)))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(x, y))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                ui.set_min_width(PREVIEW_SIZE.x);
                ui.set_max_width(PREVIEW_SIZE.x);
                if let Some((loaded, texture)) = app
                    .seekbar_thumbnail_preview
                    .texture
                    .as_ref()
                    .filter(|(loaded, _)| loaded == &key)
                {
                    debug_assert_eq!(loaded.second, key.second);
                    ui.add(egui::Image::new(texture).fit_to_exact_size(PREVIEW_SIZE));
                } else {
                    let (rect, _) = ui.allocate_exact_size(PREVIEW_SIZE, egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                    if app.seekbar_thumbnail_preview.failed.as_ref() == Some(&key) {
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            app.tr("Preview unavailable"),
                            egui::FontId::proportional(12.0),
                            ui.visuals().weak_text_color(),
                        );
                    } else {
                        ui.put(
                            egui::Rect::from_center_size(rect.center(), egui::vec2(20.0, 20.0)),
                            egui::Spinner::new().size(18.0),
                        );
                    }
                }
                ui.add_space(3.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(PREVIEW_SIZE.x, TIMECODE_ROW_HEIGHT),
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(
                            crate::ui::controls::timecode_text(
                                crate::ui::controls::format_player_time(
                                    seconds,
                                    app.duration >= 3600.0,
                                    false,
                                ),
                            )
                            .strong(),
                        );
                    },
                );
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
        let popup_height = PREVIEW_SIZE.y + TIMECODE_ROW_HEIGHT + 15.0;
        let top = preview_top(220.0, popup_height);
        assert_eq!(top + popup_height + PREVIEW_GAP, 220.0);
    }
}
