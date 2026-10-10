//! Shared native download-center UI, usable by Pealayer and the standalone utility.
use crate::{AddRequest, Engine, EngineSettings, Manager, State};
use egui::{Color32, RichText, Stroke, Vec2};
use egui_phosphor::regular::{
    ARROW_DOWN, ARROW_UP, DOWNLOAD_SIMPLE, GAUGE, PAUSE, PLAY, PLUS, TRASH, WARNING,
};

#[derive(Default)]
pub struct View {
    url: String,
    use_proxy: bool,
    error: Option<String>,
    engine: Engine,
    connections: usize,
    aria2_endpoint: String,
    aria2_secret: String,
    pending: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
}

impl View {
    /// Returns only fully committed files requested for playback; partial data is never misrepresented as complete.
    pub fn draw(&mut self, ui: &mut egui::Ui, manager: &Manager) -> Option<String> {
        let snapshot = manager.snapshot();
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.error = result.err();
                    self.pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.error = Some("Engine connection worker stopped before replying".into());
                    self.pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if !snapshot.engines.contains(&self.engine) || self.engine == Engine::Ffmpeg {
            self.engine = Engine::Native;
        }
        self.connections = self.connections.max(1);
        let mut play = None;
        ui.horizontal(|ui| {
            ui.heading(format!("{DOWNLOAD_SIMPLE} Downloads"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!(
                    "{} active · {}/s",
                    snapshot.active,
                    bytes(snapshot.total_speed)
                ));
            });
        });
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.url)
                    .hint_text("https://…")
                    .desired_width(ui.available_width().max(200.0) - 115.0),
            );
            if ui
                .add_enabled(
                    !self.url.trim().is_empty(),
                    egui::Button::new(format!("{PLUS} Add")),
                )
                .clicked()
            {
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
        });
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("download-engine")
                .selected_text(engine_name(self.engine))
                .show_ui(ui, |ui| {
                    for engine in &snapshot.engines {
                        if *engine != Engine::Ffmpeg {
                            ui.selectable_value(&mut self.engine, *engine, engine_name(*engine));
                        }
                    }
                });
            if self.engine == Engine::Aria2 {
                ui.label("Connections per file");
                ui.add(egui::DragValue::new(&mut self.connections).range(1..=16));
            }
            ui.checkbox(&mut self.use_proxy, "Use proxy");
            ui.separator();
            ui.label("Parallel files");
            let mut concurrent = snapshot.max_concurrent;
            if ui
                .add(egui::DragValue::new(&mut concurrent).range(1..=8))
                .changed()
            {
                self.error = manager
                    .configure(concurrent, snapshot.bytes_per_second)
                    .err();
            }
            ui.label(format!("{GAUGE} Limit"));
            let mut limit = snapshot.bytes_per_second / 1024;
            if ui
                .add(
                    egui::DragValue::new(&mut limit)
                        .range(0..=1024 * 1024)
                        .suffix(" KiB/s"),
                )
                .changed()
            {
                self.error = manager
                    .configure(snapshot.max_concurrent, limit * 1024)
                    .err();
            }
            if limit == 0 {
                ui.weak("Unlimited");
            }
        });
        egui::CollapsingHeader::new("Engine settings").show(ui, |ui| {
            if self.aria2_endpoint.is_empty() {
                self.aria2_endpoint = snapshot
                    .aria2_endpoint
                    .clone()
                    .unwrap_or_else(|| "http://127.0.0.1:6800/jsonrpc".into());
            }
            ui.label("Local aria2 RPC");
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
            if ui
                .add_enabled(self.pending.is_none(), egui::Button::new("Connect aria2"))
                .clicked()
            {
                let manager = manager.clone();
                let settings = EngineSettings {
                    aria2_endpoint: Some(self.aria2_endpoint.trim().into()),
                    aria2_secret: (!self.aria2_secret.is_empty())
                        .then(|| self.aria2_secret.clone()),
                };
                let (tx, rx) = std::sync::mpsc::channel();
                match std::thread::Builder::new()
                    .name("download-engine-connect".into())
                    .spawn(move || {
                        let _ = tx.send(manager.configure_engines(settings));
                    }) {
                    Ok(_) => self.pending = Some(rx),
                    Err(error) => {
                        self.error = Some(format!("Cannot start engine connection: {error}"))
                    }
                }
                self.aria2_secret.clear();
            }
        });
        if let Some(error) = &self.error {
            ui.colored_label(
                Color32::from_rgb(230, 110, 95),
                format!("{WARNING} {error}"),
            );
        }
        ui.add_space(10.0);
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if snapshot.jobs.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(DOWNLOAD_SIMPLE).size(32.0).weak());
                        ui.label("Your download queue is empty");
                    });
                }
                for job in &snapshot.jobs {
                    ui.push_id(&job.id, |ui| {
                        let color = match job.state {
                            State::Complete => Color32::from_rgb(53, 183, 112),
                            State::Failed | State::Cancelled => Color32::from_rgb(216, 89, 91),
                            State::Paused => Color32::from_rgb(222, 160, 63),
                            _ => ui.visuals().selection.stroke.color,
                        };
                        egui::Frame::group(ui.style())
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.set_min_width((ui.available_width() - 2.0).max(0.0));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(DOWNLOAD_SIMPLE).color(color).size(22.0),
                                    );
                                    ui.vertical(|ui| {
                                        ui.set_max_width(ui.available_width().max(80.0));
                                        ui.add(
                                            egui::Label::new(RichText::new(&job.filename).strong())
                                                .truncate(),
                                        )
                                        .on_hover_text(&job.filename);
                                        ui.add(
                                            egui::Label::new(RichText::new(&job.source).weak())
                                                .truncate(),
                                        )
                                        .on_hover_text(&job.source);
                                    });
                                });
                                ui.add_space(8.0);
                                if let Some(total) = job.total.filter(|n| *n > 0) {
                                    ui.add(
                                        egui::ProgressBar::new(
                                            (job.downloaded as f64 / total as f64).min(1.0) as f32,
                                        )
                                        .fill(color)
                                        .text(format!(
                                            "{} / {} · {:.1}%",
                                            bytes(job.downloaded),
                                            bytes(total),
                                            job.downloaded as f64 / total as f64 * 100.0
                                        )),
                                    );
                                } else {
                                    ui.label(format!("{} downloaded", bytes(job.downloaded)));
                                }
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(color, format!("{:?}", job.state));
                                    ui.weak(engine_name(job.engine));
                                    ui.weak(format!("{}/s", bytes(job.speed)));
                                    if let Some(eta) = job.eta_seconds {
                                        ui.weak(format!("{}m {}s left", eta / 60, eta % 60));
                                    }
                                    for (action, icon, caption) in
                                        [("pause", PAUSE, "Pause"), ("resume", PLAY, "Resume")]
                                    {
                                        if job.actions.contains(&action)
                                            && ui.button(format!("{icon} {caption}")).clicked()
                                        {
                                            self.error = manager.action(&job.id, action).err();
                                        }
                                    }
                                    if job.state == State::Complete && job.output.is_some() {
                                        if ui.button(format!("{PLAY} Play in Pealayer")).clicked() {
                                            play = job.output.clone();
                                        }
                                    }
                                    if job.actions.contains(&"cancel") {
                                        if ui.button("Cancel").clicked() {
                                            self.error = manager.action(&job.id, "cancel").err();
                                        }
                                    }
                                    if job.actions.contains(&"remove")
                                        && ui.button(format!("{TRASH} Remove from queue")).clicked()
                                    {
                                        self.error = manager.action(&job.id, "remove").err();
                                    }
                                    for (action, caption) in [
                                        ("remux_mp4", "Remux MP4"),
                                        ("extract_audio", "Extract audio"),
                                    ] {
                                        if job.actions.contains(&action)
                                            && ui.button(caption).clicked()
                                        {
                                            self.error = manager.action(&job.id, action).err();
                                        }
                                    }
                                    if ui
                                        .add_enabled(
                                            job.actions.contains(&"move_up"),
                                            egui::Button::new(ARROW_UP).small(),
                                        )
                                        .on_hover_text("Move earlier in queue")
                                        .clicked()
                                    {
                                        self.error = manager.action(&job.id, "move_up").err();
                                    }
                                    if ui
                                        .add_enabled(
                                            job.actions.contains(&"move_down"),
                                            egui::Button::new(ARROW_DOWN).small(),
                                        )
                                        .on_hover_text("Move later in queue")
                                        .clicked()
                                    {
                                        self.error = manager.action(&job.id, "move_down").err();
                                    }
                                });
                                if let Some(error) = &job.error {
                                    ui.colored_label(Color32::from_rgb(216, 89, 91), error);
                                }
                                // Reserve the chart footprint before samples arrive, avoiding
                                // shifting the actions and the following queue rows on refresh.
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 32.0),
                                    egui::Sense::hover(),
                                );
                                if job.samples.len() > 1 {
                                    let peak = job.samples.iter().copied().max().unwrap_or(1).max(1)
                                        as f32;
                                    let points = job
                                        .samples
                                        .iter()
                                        .enumerate()
                                        .map(|(index, speed)| {
                                            egui::pos2(
                                                rect.left()
                                                    + index as f32 / (job.samples.len() - 1) as f32
                                                        * rect.width(),
                                                rect.bottom()
                                                    - *speed as f32 / peak * (rect.height() - 2.0),
                                            )
                                        })
                                        .collect();
                                    ui.painter()
                                        .add(egui::Shape::line(points, Stroke::new(1.5, color)));
                                }
                            });
                    });
                    ui.add_space(8.0);
                }
            });
        if self.pending.is_some()
            || snapshot.active > 0
            || snapshot.jobs.iter().any(|job| job.state == State::Queued)
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        play
    }
}

pub fn engine_name(engine: Engine) -> &'static str {
    match engine {
        Engine::Native => "Built-in Rust",
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
        let context = egui::Context::default();
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
        let context = egui::Context::default();
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
