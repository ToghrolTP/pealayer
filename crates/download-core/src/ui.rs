//! Download Center shared by the player and standalone utility.
use crate::{AddRequest, Engine, EngineSettings, Manager, PublicJob, Snapshot, State};
use palette::controls::CheckboxUiExt;
use egui::{self, Color32, RichText, Stroke, TextStyle, Vec2};
use egui_phosphor::regular::{
    ARROW_DOWN, ARROW_UP, CARET_DOWN, CHECK_CIRCLE, CLOCK, COPY, DOTS_THREE, DOWNLOAD_SIMPLE,
    GAUGE, GEAR, LINK, MAGNIFYING_GLASS, PAUSE, PLAY, PLUS, STOP, TRASH, WARNING,
};
use std::{path::PathBuf, sync::mpsc};

#[path = "../../../assets/themes/native_palette.rs"]
pub mod palette;

#[derive(Default, PartialEq, Eq)]
enum Section {
    #[default]
    Queue,
    Settings,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Active,
    Finished,
}

impl Filter {
    fn includes(self, state: State) -> bool {
        let finished = matches!(state, State::Complete | State::Cancelled | State::Failed);
        match self {
            Self::All => true,
            Self::Active => !finished,
            Self::Finished => finished,
        }
    }
}

#[derive(Default)]
pub struct View {
    section: Section,
    filter: Filter,
    search: String,
    url: String,
    use_proxy: bool,
    error: Option<String>,
    engine: Engine,
    connections: usize,
    limits: Option<(usize, u64)>,
    aria2_endpoint: String,
    aria2_secret: String,
    aria2_executable: String,
    resolved: Option<(String, Result<PathBuf, String>)>,
    resolver: Option<(String, mpsc::Receiver<Result<PathBuf, String>>)>,
    pending: Option<mpsc::Receiver<Result<(), String>>>,
}

impl View {
    /// Returns only a committed file explicitly selected for playback.
    pub fn draw(&mut self, ui: &mut egui::Ui, manager: &Manager) -> Option<String> {
        let snapshot = manager.snapshot();
        self.poll();
        if !snapshot.engines.contains(&self.engine) || self.engine == Engine::Ffmpeg {
            self.engine = Engine::Native;
        }
        if self.connections == 0 {
            self.connections = 4;
        }

        ui.horizontal_wrapped(|ui| {
            ui.label(icon(DOWNLOAD_SIMPLE).size(22.0).color(accent(ui)));
            ui.heading("Download Center");
        });
        ui.horizontal_wrapped(|ui| {
            let completed = snapshot
                .jobs
                .iter()
                .filter(|job| job.state == State::Complete)
                .count();
            for (glyph, caption) in [
                (GAUGE, format!("{}/s", bytes(snapshot.total_speed))),
                (CLOCK, format!("{} active", snapshot.active)),
                (CHECK_CIRCLE, format!("{completed} complete")),
            ] {
                ui.label(icon_caption(
                    ui,
                    glyph,
                    RichText::new(caption).weak(),
                    TextStyle::Body,
                ));
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.section == Section::Queue,
                    action_caption(ui, DOWNLOAD_SIMPLE, "Queue"),
                )
                .clicked()
            {
                self.section = Section::Queue;
            }
            if ui
                .selectable_label(
                    self.section == Section::Settings,
                    action_caption(ui, GEAR, "Settings"),
                )
                .clicked()
            {
                self.section = Section::Settings;
            }
        });
        ui.separator();
        let mut play = None;
        egui::ScrollArea::vertical()
            .id_salt("download-center-content")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(6.0);
                if let Some(error) = &self.error {
                    let text = RichText::new(error).color(ui.visuals().error_fg_color);
                    ui.label(icon_caption(ui, WARNING, text, TextStyle::Body));
                    ui.add_space(6.0);
                }
                match self.section {
                    Section::Queue => play = self.queue(ui, manager, &snapshot),
                    Section::Settings => self.settings(ui, manager, &snapshot),
                }
            });
        if self.pending.is_some()
            || self.resolver.is_some()
            || snapshot.active > 0
            || snapshot.jobs.iter().any(|job| job.state == State::Queued)
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        play
    }

    fn poll(&mut self) {
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.error = result.err();
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error = Some("Engine connection worker stopped before replying".into());
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some((query, receiver)) = &self.resolver {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Executable check stopped before replying".into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                self.resolved = Some((query.clone(), result));
                self.resolver = None;
            }
        }
    }

    fn queue(
        &mut self,
        ui: &mut egui::Ui,
        manager: &Manager,
        snapshot: &Snapshot,
    ) -> Option<String> {
        panel(ui, |ui| {
            ui.strong("New download");
            let enabled = !self.url.trim().is_empty();
            let mut add = false;
            let mut compose = |ui: &mut egui::Ui, width: f32| {
                let input = ui.add(
                    egui::TextEdit::singleline(&mut self.url)
                        .hint_text("Paste a file or media link")
                        .desired_width(width.max(1.0)),
                );
                let clicked = ui
                    .add_enabled(enabled, primary(ui, PLUS, "Add download"))
                    .clicked();
                add = clicked
                    || (!self.url.trim().is_empty()
                        && input.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            };
            if ui.available_width() >= 420.0 {
                ui.horizontal(|ui| {
                    let caption = action_caption(ui, PLUS, "Add download");
                    let button_width = ui.fonts_mut(|fonts| fonts.layout_job(caption)).size().x
                        + 2.0 * ui.spacing().button_padding.x
                        + 8.0;
                    compose(
                        ui,
                        ui.available_width() - button_width - ui.spacing().item_spacing.x,
                    );
                });
            } else {
                compose(ui, ui.available_width());
            }
            if add {
                match manager.add(AddRequest {
                    url: self.url.trim().into(),
                    use_proxy: self.use_proxy,
                    proxy_url: None,
                    filename: None,
                    engine: self.engine,
                    connections: if self.engine == Engine::Aria2 {
                        self.connections
                    } else {
                        1
                    },
                }) {
                    Ok(_) => {
                        self.url.clear();
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            ui.horizontal_wrapped(|ui| {
                ui.weak("Engine");
                egui::ComboBox::from_id_salt("download-engine")
                    .selected_text(engine_name(self.engine))
                    .icon(|ui, rect, visuals, _| {
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            CARET_DOWN,
                            icon_font_id(ui, TextStyle::Small),
                            visuals.text_color(),
                        );
                    })
                    .show_ui(ui, |ui| {
                        for engine in snapshot
                            .engines
                            .iter()
                            .filter(|engine| **engine != Engine::Ffmpeg)
                        {
                            ui.selectable_value(&mut self.engine, *engine, engine_name(*engine));
                        }
                    });
                if self.engine == Engine::Aria2 {
                    ui.weak("Connections");
                    ui.add(egui::DragValue::new(&mut self.connections).range(1..=16));
                }
                ui.themed_checkbox(&mut self.use_proxy, "Use proxy");
            });
        });

        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("Queue · {}", snapshot.jobs.len()));
            for (filter, label) in [
                (Filter::All, "All"),
                (Filter::Active, "Pending"),
                (Filter::Finished, "Finished"),
            ] {
                ui.selectable_value(&mut self.filter, filter, label);
            }
        });
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Filter by name or source")
                .margin(egui::Margin {
                    left: 26,
                    right: 8,
                    top: 4,
                    bottom: 4,
                })
                .desired_width(ui.available_width()),
        );
        ui.painter().text(
            search.rect.left_center() + egui::vec2(13.0, 0.0),
            egui::Align2::CENTER_CENTER,
            MAGNIFYING_GLASS,
            icon_font_id(ui, TextStyle::Body),
            ui.visuals().weak_text_color(),
        );
        let query = self.search.trim().to_lowercase();
        let filter = self.filter;
        let mut visible = snapshot
            .jobs
            .iter()
            .filter(|job| {
                filter.includes(job.state)
                    && (query.is_empty()
                        || job.filename.to_lowercase().contains(&query)
                        || job.source.to_lowercase().contains(&query))
            })
            .peekable();
        ui.add_space(4.0);
        if visible.peek().is_none() {
            panel(ui, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(icon(DOWNLOAD_SIMPLE).size(30.0).color(accent(ui)));
                    if snapshot.jobs.is_empty() {
                        ui.strong("No downloads yet");
                        ui.weak("Paste a link above to start your first download.");
                    } else {
                        ui.strong("No matching downloads");
                        ui.weak("Try a different filter or search term.");
                    }
                });
                ui.add_space(20.0);
            });
        }
        let mut play = None;
        for job in visible {
            ui.push_id(&job.id, |ui| {
                panel(ui, |ui| self.job(ui, manager, job, &mut play));
            });
            ui.add_space(8.0);
        }
        play
    }

    fn job(
        &mut self,
        ui: &mut egui::Ui,
        manager: &Manager,
        job: &PublicJob,
        play: &mut Option<String>,
    ) {
        let (status, glyph, color) = state_style(ui, job.state);
        ui.horizontal(|ui| {
            ui.label(icon(DOWNLOAD_SIMPLE).size(19.0).color(color));
            ui.add(egui::Label::new(RichText::new(&job.filename).strong()).truncate())
                .on_hover_text(&job.filename);
        });
        ui.add(egui::Label::new(RichText::new(&job.source).weak().small()).truncate())
            .on_hover_text(&job.source);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(icon_caption(
                ui,
                glyph,
                RichText::new(status).color(color),
                TextStyle::Body,
            ));
            ui.weak(engine_name(job.engine));
            if job.engine == Engine::Aria2 {
                ui.weak(format!("{} connections", job.connections));
            }
            if let Some(total) = job.total.filter(|total| *total > 0) {
                ui.weak(format!("{} / {}", bytes(job.downloaded), bytes(total)));
            } else {
                ui.weak(format!("{} downloaded", bytes(job.downloaded)));
            }
            if job.speed > 0 {
                ui.weak(format!("{}/s", bytes(job.speed)));
            }
            if let Some(eta) = job.eta_seconds {
                ui.weak(format!("{}m {}s left", eta / 60, eta % 60));
            }
        });
        if let Some(total) = job.total.filter(|total| *total > 0) {
            let fraction = (job.downloaded as f64 / total as f64).clamp(0.0, 1.0) as f32;
            ui.add(
                egui::ProgressBar::new(fraction)
                    .fill(color)
                    .desired_height(6.0),
            )
            .on_hover_text(format!("{:.1}% downloaded", fraction * 100.0));
        }
        if job.samples.len() > 1 {
            speed_chart(ui, &job.samples, color);
        }
        if let Some(error) = &job.error {
            let text = RichText::new(error).color(ui.visuals().error_fg_color);
            ui.label(icon_caption(ui, WARNING, text, TextStyle::Body));
        }
        ui.horizontal_wrapped(|ui| {
            for (action, icon, label) in [("pause", PAUSE, "Pause"), ("resume", PLAY, "Resume")] {
                if job.actions.contains(&action)
                    && ui.button(action_caption(ui, icon, label)).clicked()
                {
                    self.error = manager.action(&job.id, action).err();
                }
            }
            if job.state == State::Complete
                && job.output.is_some()
                && ui.add(primary(ui, PLAY, "Play in Pealayer")).clicked()
            {
                *play = job.output.clone();
            }
            let actions = [
                ("move_up", ARROW_UP, "Move earlier"),
                ("move_down", ARROW_DOWN, "Move later"),
                ("remux_mp4", DOWNLOAD_SIMPLE, "Remux MP4"),
                ("extract_audio", DOWNLOAD_SIMPLE, "Extract audio"),
                ("cancel", STOP, "Cancel"),
                ("remove", TRASH, "Remove from queue"),
            ];
            if actions
                .iter()
                .any(|(action, _, _)| job.actions.contains(action))
            {
                ui.menu_button(action_caption(ui, DOTS_THREE, "More"), |ui| {
                    for (action, icon, label) in actions {
                        if job.actions.contains(&action)
                            && ui.button(action_caption(ui, icon, label)).clicked()
                        {
                            self.error = manager.action(&job.id, action).err();
                            ui.close();
                        }
                    }
                });
            }
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui, manager: &Manager, snapshot: &Snapshot) {
        panel(ui, |ui| {
            ui.label(icon_caption(
                ui,
                GAUGE,
                RichText::new("Transfer limits").strong(),
                TextStyle::Body,
            ));
            let current = (snapshot.max_concurrent, snapshot.bytes_per_second);
            let (concurrent, limit) = self.limits.get_or_insert(current);
            ui.horizontal_wrapped(|ui| {
                ui.label("Parallel files");
                ui.add(egui::DragValue::new(concurrent).range(1..=8));
                ui.label("Bandwidth limit");
                let mut kib = *limit as f64 / 1024.0;
                if ui
                    .add(
                        egui::DragValue::new(&mut kib)
                            .range(0.0..=1048576.0)
                            .suffix(" KiB/s"),
                    )
                    .changed()
                {
                    *limit = (kib * 1024.0).round() as u64;
                }
                if *limit == 0 {
                    ui.weak("Unlimited");
                }
            });
            if ui
                .add_enabled(
                    (*concurrent, *limit) != current,
                    egui::Button::new("Apply limits"),
                )
                .clicked()
            {
                self.error = manager.configure(*concurrent, *limit).err();
                if self.error.is_none() {
                    self.limits = None;
                }
            }
        });

        ui.add_space(8.0);
        panel(ui, |ui| {
            ui.label(icon_caption(
                ui,
                GEAR,
                RichText::new("aria2 executable").strong(),
                TextStyle::Body,
            ));
            ui.weak("Find a bundled or installed executable by name or full path.");
            if self.aria2_executable.is_empty()
                && self.resolved.is_none()
                && self.resolver.is_none()
            {
                self.aria2_executable = "aria2c".into();
                self.resolve_aria2(ui.ctx());
            }
            let input = ui.add(
                egui::TextEdit::singleline(&mut self.aria2_executable)
                    .hint_text("aria2c")
                    .desired_width(ui.available_width()),
            );
            let resolve = ui
                .button(action_caption(ui, MAGNIFYING_GLASS, "Resolve path"))
                .clicked();
            if input.lost_focus() || resolve {
                self.resolve_aria2(ui.ctx());
            }
            if self
                .resolver
                .as_ref()
                .is_some_and(|(query, _)| query == self.aria2_executable.trim())
            {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak("Looking for aria2…");
                });
            } else if let Some((query, result)) = &self.resolved
                && query == self.aria2_executable.trim()
            {
                match result {
                    Ok(path) => {
                        let path = path.display().to_string();
                        ui.horizontal(|ui| {
                            if ui
                                .button(icon(COPY))
                                .on_hover_text("Copy executable path")
                                .clicked()
                            {
                                ui.ctx().copy_text(path.clone());
                            }
                            let caption = icon_caption(
                                ui,
                                CHECK_CIRCLE,
                                RichText::new(&path),
                                TextStyle::Body,
                            );
                            ui.add(egui::Label::new(caption).truncate())
                                .on_hover_text(path);
                        });
                    }
                    Err(error) => {
                        ui.colored_label(ui.visuals().warn_fg_color, error);
                    }
                }
            }
        });

        ui.add_space(8.0);
        panel(ui, |ui| {
            ui.label(icon_caption(
                ui,
                LINK,
                RichText::new("Local aria2 connection").strong(),
                TextStyle::Body,
            ));
            if let Some(endpoint) = &snapshot.aria2_endpoint {
                ui.weak(format!("Configured for {endpoint}"));
            } else {
                ui.weak("Connect to an aria2 service running on this computer.");
            }
            if self.aria2_endpoint.is_empty() {
                self.aria2_endpoint = snapshot
                    .aria2_endpoint
                    .clone()
                    .unwrap_or_else(|| "http://127.0.0.1:6800/jsonrpc".into());
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.aria2_endpoint)
                    .desired_width(ui.available_width()),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.aria2_secret)
                    .password(true)
                    .hint_text("RPC secret (optional)")
                    .desired_width(ui.available_width()),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(self.pending.is_none(), primary(ui, LINK, "Connect aria2"))
                    .clicked()
                {
                    let manager = manager.clone();
                    let settings = EngineSettings {
                        aria2_endpoint: Some(self.aria2_endpoint.trim().into()),
                        aria2_secret: (!self.aria2_secret.is_empty())
                            .then(|| std::mem::take(&mut self.aria2_secret)),
                    };
                    let (sender, receiver) = mpsc::channel();
                    let ctx = ui.ctx().clone();
                    match std::thread::Builder::new()
                        .name("download-engine-connect".into())
                        .spawn(move || {
                            let _ = sender.send(manager.configure_engines(settings));
                            ctx.request_repaint();
                        }) {
                        Ok(_) => self.pending = Some(receiver),
                        Err(error) => {
                            self.error = Some(format!("Cannot start engine connection: {error}"))
                        }
                    }
                }
                if self.pending.is_some() {
                    ui.spinner();
                    ui.weak("Checking the connection…");
                }
            });
        });
    }

    fn resolve_aria2(&mut self, ctx: &egui::Context) {
        let query = self.aria2_executable.trim().to_owned();
        if self
            .resolver
            .as_ref()
            .is_some_and(|(active, _)| active == &query)
        {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let input = query.clone();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("aria2-discovery".into())
            .spawn(move || {
                let _ = sender.send(crate::tools::resolve_aria2(&input));
                ctx.request_repaint();
            }) {
            Ok(_) => self.resolver = Some((query, receiver)),
            Err(_) => self.resolved = Some((query, Err("Could not start executable check".into()))),
        }
    }
}

fn panel(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(visuals.window_fill)
        .stroke(visuals.window_stroke)
        .corner_radius(visuals.window_corner_radius)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width().max(0.0));
            body(ui);
        });
}

fn accent(ui: &egui::Ui) -> Color32 {
    ui.visuals().selection.bg_fill
}

fn icon(glyph: &str) -> RichText {
    RichText::new(glyph).family(egui::FontFamily::Name("phosphor".into()))
}

fn icon_font_id(ui: &egui::Ui, style: TextStyle) -> egui::FontId {
    egui::FontId::new(
        style.resolve(ui.style()).size,
        egui::FontFamily::Name("phosphor".into()),
    )
}

fn icon_caption(
    ui: &egui::Ui,
    glyph: &str,
    caption: RichText,
    style: TextStyle,
) -> egui::text::LayoutJob {
    let mut layout = egui::text::LayoutJob::default();
    caption.append_to(&mut layout, ui.style(), style.into(), egui::Align::Center);
    let format = layout.sections[0].format.clone();
    let mut label = egui::text::LayoutJob::default();
    label.append(
        &format!("{glyph} "),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(
                format.font_id.size,
                egui::FontFamily::Name("phosphor".into()),
            ),
            ..format.clone()
        },
    );
    label.append(&layout.text, 0.0, format);
    label
}

fn action_caption(ui: &egui::Ui, glyph: &str, label: &str) -> egui::text::LayoutJob {
    icon_caption(ui, glyph, RichText::new(label), TextStyle::Button)
}

fn primary(ui: &egui::Ui, glyph: &str, label: &str) -> egui::Button<'static> {
    let label = RichText::new(label).color(ui.visuals().selection.stroke.color);
    egui::Button::new(icon_caption(ui, glyph, label, TextStyle::Button))
        .fill(accent(ui))
        .min_size(egui::vec2(0.0, ui.spacing().interact_size.y))
}

fn state_style(ui: &egui::Ui, state: State) -> (&'static str, &'static str, Color32) {
    let visuals = ui.visuals();
    match state {
        State::Queued => ("Queued", CLOCK, visuals.weak_text_color()),
        State::Connecting => ("Connecting", LINK, accent(ui)),
        State::Downloading => ("Downloading", DOWNLOAD_SIMPLE, accent(ui)),
        State::Paused => ("Paused", PAUSE, visuals.warn_fg_color),
        State::Complete => (
            "Complete",
            CHECK_CIRCLE,
            palette::color("native", visuals.dark_mode, "green"),
        ),
        State::Failed => ("Failed", WARNING, visuals.error_fg_color),
        State::Cancelled => ("Cancelled", STOP, visuals.weak_text_color()),
    }
}

fn speed_chart(ui: &mut egui::Ui, samples: &std::collections::VecDeque<u64>, color: Color32) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), egui::Sense::hover());
    let peak = samples.iter().copied().max().unwrap_or(1).max(1);
    let plot = rect.shrink2(egui::vec2(4.0, 3.0));
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().faint_bg_color);
    painter.hline(plot.x_range(), plot.center().y, ui.visuals().window_stroke);
    let points = samples
        .iter()
        .enumerate()
        .map(|(index, speed)| {
            let ratio = index as f32 / (samples.len() - 1) as f32;
            egui::pos2(
                plot.left() + ratio * plot.width(),
                plot.bottom() - *speed as f32 / peak as f32 * plot.height(),
            )
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    response.on_hover_text(format!("Recent download speed · peak {}/s", bytes(peak)));
}

pub fn engine_name(engine: Engine) -> &'static str {
    match engine {
        Engine::Native => "Built-in",
        Engine::Aria2 => "aria2",
        Engine::YtDlp => "yt-dlp",
        Engine::Ffmpeg => "FFmpeg",
    }
}

pub fn bytes(value: u64) -> String {
    let (divisor, suffix) = if value >= 1024 * 1024 * 1024 {
        (1024.0 * 1024.0 * 1024.0, "GiB")
    } else if value >= 1024 * 1024 {
        (1024.0 * 1024.0, "MiB")
    } else if value >= 1024 {
        (1024.0, "KiB")
    } else {
        (1.0, "B")
    };
    format!("{:.1} {suffix}", value as f64 / divisor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> egui::Context {
        let context = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        context.set_fonts(fonts);
        context
    }

    #[test]
    fn queue_and_settings_fit_a_narrow_window() {
        let root = std::env::temp_dir().join(format!("pealayer-ui-{}", uuid::Uuid::new_v4()));
        let manager = Manager::open(root.clone()).unwrap();
        let context = test_context();
        let mut view = View::default();
        for section in [Section::Queue, Section::Settings] {
            view.section = section;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(340.0, 520.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let right = ui.max_rect().right();
                    view.draw(ui, &manager);
                    assert!(ui.min_rect().right() <= right + 1.0);
                },
            );
            output.textures_delta.clear();
        }
        manager.shutdown(std::time::Duration::from_secs(2)).unwrap();
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rendered_resume_button_changes_the_shared_queue() {
        let root = std::env::temp_dir().join(format!("pealayer-ui-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        // This exercises the real widget and queue actions, not the HTTP
        // worker. Keep the scheduler out of the fixture: a refused connection
        // could otherwise fail before Pause, or race the post-click snapshot.
        let manager = Manager(std::sync::Arc::new(std::sync::Mutex::new(crate::Inner {
            jobs: Vec::new(),
            root: root.clone(),
            max_concurrent: 2,
            bytes_per_second: 0,
            revision: 0,
            _queue_lock: std::fs::File::create(root.join("queue.lock")).unwrap(),
            engines: EngineSettings::default(),
            shutting_down: false,
        })));
        let id = manager
            .add(AddRequest {
                url: "http://127.0.0.1:9/synthetic.bin".into(),
                use_proxy: false,
                proxy_url: None,
                filename: None,
                engine: Engine::Native,
                connections: 1,
            })
            .unwrap();
        manager.action(&id, "pause").unwrap();
        let mut view = View::default();
        let context = test_context();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            view.draw(ui, &manager);
        });
        let position = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text().ends_with(" Resume") => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("Paused job renders its actual Resume button");
        output.textures_delta.clear();
        let pointer = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut output = context.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(position),
                    pointer(true),
                    pointer(false),
                ],
                ..Default::default()
            },
            |ui| {
                view.draw(ui, &manager);
            },
        );
        output.textures_delta.clear();
        assert_eq!(
            manager.snapshot().jobs[0].state,
            State::Queued,
            "A widget click must invoke the backend, not just change UI state"
        );
        manager.shutdown(std::time::Duration::from_secs(2)).unwrap();
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lost_engine_reply_releases_the_connect_control() {
        let root = std::env::temp_dir().join(format!("pealayer-ui-{}", uuid::Uuid::new_v4()));
        let manager = Manager::open(root.clone()).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(sender);
        let mut view = View {
            pending: Some(receiver),
            ..Default::default()
        };
        let context = test_context();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            view.draw(ui, &manager);
        });
        output.textures_delta.clear();
        assert!(view.pending.is_none());
        assert!(
            view.error
                .as_deref()
                .unwrap()
                .contains("stopped before replying")
        );
        manager.shutdown(std::time::Duration::from_secs(2)).unwrap();
        drop(manager);
        std::fs::remove_dir_all(root).unwrap();
    }
}
