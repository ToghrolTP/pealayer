use crate::app::PealayerApp;
use eframe::egui;
use reqwest::header::{
    ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG,
    LAST_MODIFIED, SERVER,
};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

const PROBE_DEBOUNCE: Duration = Duration::from_millis(700);
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedMediaUrl {
    pub normalized: String,
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub file_name: Option<String>,
    pub query_parameters: usize,
    pub supports_http_probe: bool,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteMediaInfo {
    pub requested_url: String,
    pub final_url: String,
    pub status: u16,
    pub status_text: String,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub file_name: Option<String>,
    pub accept_ranges: Option<String>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub server: Option<String>,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeStatus {
    Idle,
    Checking(String),
    Ready(RemoteMediaInfo),
    Failed { url: String, message: String },
    NotApplicable { url: String, message: String },
}

#[derive(Clone, Debug)]
struct ProbeResult {
    generation: u64,
    result: Result<RemoteMediaInfo, String>,
}

pub struct UrlInspector {
    tx: Sender<ProbeResult>,
    rx: Receiver<ProbeResult>,
    generation: u64,
    last_input: String,
    last_changed: Instant,
    requested_input: Option<String>,
    pub status: ProbeStatus,
}

impl Default for UrlInspector {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            generation: 0,
            last_input: String::new(),
            last_changed: Instant::now(),
            requested_input: None,
            status: ProbeStatus::Idle,
        }
    }
}

impl UrlInspector {
    fn update(&mut self, input: &str, ctx: &egui::Context) {
        while let Ok(probe) = self.rx.try_recv() {
            if probe.generation != self.generation {
                continue;
            }
            self.status = match probe.result {
                Ok(info) => ProbeStatus::Ready(info),
                Err(message) => ProbeStatus::Failed {
                    url: self.last_input.clone(),
                    message,
                },
            };
        }

        let trimmed = input.trim();
        if self.last_input != trimmed {
            self.generation = self.generation.wrapping_add(1);
            self.last_input = trimmed.to_owned();
            self.last_changed = Instant::now();
            self.requested_input = None;
            self.status = ProbeStatus::Idle;
        }

        let Ok(validated) = validate_media_url(trimmed) else {
            return;
        };
        if !validated.supports_http_probe {
            self.status = ProbeStatus::NotApplicable {
                url: validated.normalized,
                message: "This protocol is accepted by the media engine, but does not expose HTTP file metadata before playback.".to_string(),
            };
            return;
        }

        if self.requested_input.as_deref() == Some(trimmed) {
            return;
        }
        let elapsed = self.last_changed.elapsed();
        if elapsed >= PROBE_DEBOUNCE {
            self.start_probe(validated.normalized, ctx.clone());
        } else {
            ctx.request_repaint_after(PROBE_DEBOUNCE - elapsed);
        }
    }

    fn inspect_now(&mut self, input: &str, ctx: &egui::Context) {
        match validate_media_url(input.trim()) {
            Ok(validated) if validated.supports_http_probe => {
                self.generation = self.generation.wrapping_add(1);
                self.last_input = input.trim().to_owned();
                self.start_probe(validated.normalized, ctx.clone());
            }
            Ok(validated) => {
                self.status = ProbeStatus::NotApplicable {
                    url: validated.normalized,
                    message: "This protocol is accepted by the media engine, but does not expose HTTP file metadata before playback.".to_string(),
                };
            }
            Err(message) => {
                self.status = ProbeStatus::Failed {
                    url: input.trim().to_owned(),
                    message,
                };
            }
        }
    }

    fn start_probe(&mut self, url: String, ctx: egui::Context) {
        let generation = self.generation;
        self.requested_input = Some(self.last_input.clone());
        self.status = ProbeStatus::Checking(url.clone());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = probe_remote_media(&url);
            let _ = tx.send(ProbeResult { generation, result });
            ctx.request_repaint();
        });
    }
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_open_url_dialog {
        return;
    }

    let mut open_requested = false;
    let mut close_requested = crate::ui::dialog::escape_pressed(ui.ctx());
    let mut inspect_requested = false;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(700.0), bounds.height().min(620.0));
    let default_size = egui::vec2(max_size.x.min(620.0), max_size.y.min(500.0));
    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);

    app.url_inspector.update(&app.url_input_buffer, ui.ctx());
    let validation = validate_media_url(app.url_input_buffer.trim());
    let can_open = validation.is_ok();
    let open_shortcut = if app.open_url_multiline {
        "Ctrl+Enter"
    } else {
        "Enter"
    };

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::ARROW_SQUARE_OUT,
        app.tr("Open Location / URL")
    ))
    .id(egui::Id::new("open_location_dialog_inspector_v1"))
    .collapsible(false)
    .resizable(true)
    .default_rect(default_rect)
    .min_size([max_size.x.min(420.0), max_size.y.min(330.0)])
    .max_size(max_size)
    .constrain_to(bounds)
    .movable(true)
    .show(ui.ctx(), |ui| {
        ui.with_layout(crate::ui::i18n::vertical_layout(app.rtl), |ui| {
            let edit_id = ui.make_persistent_id("open_location_url_editor");
            let body_height = (ui.available_height() - 54.0).max(180.0);
            crate::ui::dialog::scroll_column(
                ui,
                "open_location_inspector_body",
                Some(body_height),
                |ui| {
                    ui.add(
                        egui::Label::new(app.tr(
                            "Enter a media URL (HTTP/HTTPS, HLS, RTSP, RTMP, SRT, UDP, or TCP):",
                        ))
                        .wrap(),
                    );
                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        let wrap_label = format!(
                            "{}  {}",
                            crate::ui::icons::TEXT_ALIGN_LEFT,
                            app.tr("Wrap long URLs in a text area")
                        );
                        if ui
                            .checkbox(&mut app.open_url_multiline, wrap_label)
                            .changed()
                        {
                            app.save_config();
                        }
                        if ui
                            .add_enabled(
                                can_open,
                                egui::Button::new(format!(
                                    "{}  {}",
                                    crate::ui::icons::MAGNIFYING_GLASS,
                                    app.tr("Inspect now")
                                )),
                            )
                            .clicked()
                        {
                            inspect_requested = true;
                        }
                    });

                    let edit = if app.open_url_multiline {
                        egui::TextEdit::multiline(&mut app.url_input_buffer)
                            .id(edit_id)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .hint_text("https://...")
                    } else {
                        egui::TextEdit::singleline(&mut app.url_input_buffer)
                            .id(edit_id)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://...")
                    };
                    let mut edit_output = edit.show(ui);
                    text_edit_context_menu(
                        ui,
                        &mut edit_output,
                        &mut app.url_input_buffer,
                        app.language,
                    );

                    ui.add_space(8.0);
                    draw_validation(ui, app, validation.as_ref());
                    ui.add_space(6.0);
                    draw_remote_info(ui, app, &app.url_inspector.status);
                },
            );

            ui.add_space(8.0);
            ui.separator();
            crate::ui::dialog::action_bar(
                ui,
                app.rtl,
                |ui| {
                    if crate::ui::dialog::action_button(
                        ui,
                        crate::ui::icons::CLIPBOARD,
                        &app.tr("Paste"),
                    )
                    .on_hover_text("Ctrl+V")
                    .clicked()
                    {
                        ui.ctx().memory_mut(|memory| memory.request_focus(edit_id));
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    }
                },
                |ui| {
                    // Cancel intentionally precedes Open in this trailing-edge
                    // layout so their visual positions are swapped as requested.
                    if crate::ui::dialog::action_button(ui, crate::ui::icons::X, &app.tr("Cancel"))
                        .on_hover_text("Esc")
                        .clicked()
                    {
                        close_requested = true;
                    }
                    if ui
                        .add_enabled_ui(can_open, |ui| {
                            crate::ui::dialog::action_button(
                                ui,
                                crate::ui::icons::ARROW_SQUARE_OUT,
                                &app.tr("Open"),
                            )
                            .on_hover_text(open_shortcut)
                        })
                        .inner
                        .clicked()
                    {
                        open_requested = true;
                    }
                },
            );
        });
    });

    if inspect_requested {
        app.url_inspector
            .inspect_now(&app.url_input_buffer, ui.ctx());
    }
    let keyboard_open = ui.ctx().input(|input| {
        input.key_pressed(egui::Key::Enter)
            && if app.open_url_multiline {
                input.modifiers.command
            } else {
                !input.modifiers.shift
            }
    });
    if can_open && keyboard_open {
        open_requested = true;
    }

    if open_requested {
        let url = validation
            .map(|validated| validated.normalized)
            .unwrap_or_else(|_| app.url_input_buffer.trim().to_owned());
        app.load_url(&url);
        app.url_input_buffer.clear();
        app.url_inspector = UrlInspector::default();
        app.show_open_url_dialog = false;
    } else if close_requested {
        app.show_open_url_dialog = false;
    }
}

fn text_edit_context_menu(
    ui: &mut egui::Ui,
    output: &mut egui::text_edit::TextEditOutput,
    text: &mut String,
    language: crate::config::AppLanguage,
) {
    let ctx = ui.ctx().clone();
    let id = output.response.id;
    let mut state = output.state.clone();
    let initial_range = state
        .cursor
        .char_range()
        .or(output.cursor_range)
        .unwrap_or_else(|| {
            egui::text::CCursorRange::one(egui::text::CCursor::new(text.chars().count()))
        });
    let selected_text =
        (!initial_range.is_empty()).then(|| initial_range.slice_str(text).to_owned());

    output.response.context_menu(|ui| {
        let mut close = false;
        let current = (initial_range, text.clone());
        let mut undoer = state.undoer();

        if ui
            .add_enabled(
                undoer.has_undo(&current),
                menu_button(
                    crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                    tr(language, "Undo"),
                    "Ctrl+Z",
                ),
            )
            .clicked()
        {
            if let Some((range, restored)) = undoer.undo(&current).cloned() {
                *text = restored;
                state.cursor.set_char_range(Some(range));
                state.set_undoer(undoer.clone());
                state.clone().store(&ctx, id);
            }
            close = true;
        }
        if ui
            .add_enabled(
                undoer.has_redo(&current),
                menu_button(
                    crate::ui::icons::ARROW_CLOCKWISE,
                    tr(language, "Redo"),
                    "Ctrl+Y",
                ),
            )
            .clicked()
        {
            if let Some((range, restored)) = undoer.redo(&current).cloned() {
                *text = restored;
                state.cursor.set_char_range(Some(range));
                state.set_undoer(undoer.clone());
                state.clone().store(&ctx, id);
            }
            close = true;
        }
        ui.separator();

        if ui
            .add_enabled(
                selected_text.is_some(),
                menu_button(crate::ui::icons::SCISSORS, tr(language, "Cut"), "Ctrl+X"),
            )
            .clicked()
        {
            if let Some(selected) = &selected_text {
                ctx.copy_text(selected.clone());
                replace_selection(text, initial_range, "", &mut state, &ctx, id);
            }
            close = true;
        }
        if ui
            .add_enabled(
                selected_text.is_some(),
                menu_button(crate::ui::icons::COPY, tr(language, "Copy"), "Ctrl+C"),
            )
            .clicked()
        {
            if let Some(selected) = &selected_text {
                ctx.copy_text(selected.clone());
            }
            close = true;
        }
        if ui
            .add(menu_button(
                crate::ui::icons::CLIPBOARD,
                tr(language, "Paste"),
                "Ctrl+V",
            ))
            .clicked()
        {
            ctx.memory_mut(|memory| memory.request_focus(id));
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            close = true;
        }
        if ui
            .add_enabled(
                selected_text.is_some(),
                menu_button(crate::ui::icons::TRASH, tr(language, "Delete"), "Del"),
            )
            .clicked()
        {
            replace_selection(text, initial_range, "", &mut state, &ctx, id);
            close = true;
        }
        ui.separator();
        if ui
            .add(menu_button(
                crate::ui::icons::SELECTION_ALL,
                tr(language, "Select All"),
                "Ctrl+A",
            ))
            .clicked()
        {
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(text.chars().count()),
            );
            state.cursor.set_char_range(Some(all));
            state.clone().store(&ctx, id);
            ctx.memory_mut(|memory| memory.request_focus(id));
            close = true;
        }

        if close {
            ui.close();
            ctx.request_repaint();
        }
    });
}

fn menu_button(icon: &str, label: String, shortcut: &str) -> egui::Button<'static> {
    egui::Button::new(format!("{icon}  {label}")).right_text(shortcut.to_owned())
}

fn tr(language: crate::config::AppLanguage, english: &'static str) -> String {
    crate::ui::i18n::tr(language, english)
}

fn replace_selection(
    text: &mut String,
    range: egui::text::CCursorRange,
    replacement: &str,
    state: &mut egui::text_edit::TextEditState,
    ctx: &egui::Context,
    id: egui::Id,
) {
    let [start, end] = range.sorted_cursors();
    let start_byte = char_to_byte(text, start.index);
    let end_byte = char_to_byte(text, end.index);
    let mut undoer = state.undoer();
    undoer.add_undo(&(range, text.clone()));
    text.replace_range(start_byte..end_byte, replacement);
    let cursor = start.index + replacement.chars().count();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(cursor),
        )));
    state.set_undoer(undoer);
    state.clone().store(ctx, id);
    ctx.memory_mut(|memory| memory.request_focus(id));
}

fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(byte, _)| byte)
}

fn draw_validation(
    ui: &mut egui::Ui,
    app: &PealayerApp,
    validation: Result<&ValidatedMediaUrl, &String>,
) {
    match validation {
        Ok(validated) => {
            egui::Frame::group(ui.style())
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}  {}",
                                crate::ui::icons::CHECK,
                                app.tr("Valid media location")
                            ))
                            .strong()
                            .color(egui::Color32::from_rgb(34, 197, 94)),
                        );
                        ui.separator();
                        ui.label(format!("{}  {}", crate::ui::icons::GLOBE, validated.scheme));
                        if !validated.host.is_empty() {
                            ui.label(&validated.host);
                        }
                        if let Some(port) = validated.port {
                            ui.label(format!("port {port}"));
                        }
                        if validated.query_parameters > 0 {
                            ui.label(format!("{} query parameter(s)", validated.query_parameters));
                        }
                    });
                    if let Some(file_name) = &validated.file_name {
                        ui.label(format!("{}  {file_name}", crate::ui::icons::FILE_VIDEO));
                    }
                    if let Some(warning) = &validated.warning {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}  {warning}",
                                crate::ui::icons::WARNING
                            ))
                            .color(ui.visuals().warn_fg_color),
                        );
                    }
                });
        }
        Err(message) if !app.url_input_buffer.trim().is_empty() => {
            ui.label(
                egui::RichText::new(format!("{}  {message}", crate::ui::icons::WARNING))
                    .color(ui.visuals().error_fg_color),
            );
        }
        Err(_) => {
            ui.label(
                egui::RichText::new(app.tr("Enter a location to validate and inspect it.")).weak(),
            );
        }
    }
}

fn draw_remote_info(ui: &mut egui::Ui, app: &PealayerApp, status: &ProbeStatus) {
    let frame = egui::Frame::group(ui.style())
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(10));
    frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            egui::RichText::new(format!(
                "{}  {}",
                crate::ui::icons::LINK_SIMPLE,
                app.tr("Remote media information")
            ))
            .strong(),
        );
        ui.add_space(4.0);
        match status {
            ProbeStatus::Idle => {
                ui.label(
                    egui::RichText::new(app.tr("Waiting for a valid HTTP or HTTPS URL…")).weak(),
                );
            }
            ProbeStatus::Checking(url) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add(egui::Label::new(format!("{}  {url}", app.tr("Checking"))).truncate());
                });
            }
            ProbeStatus::Ready(info) => {
                let status_color = if (200..400).contains(&info.status) {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    ui.visuals().error_fg_color
                };
                egui::Grid::new("open_url_remote_metadata")
                    .num_columns(2)
                    .max_col_width((ui.available_width() * 0.68).max(180.0))
                    .spacing([18.0, 6.0])
                    .show(ui, |ui| {
                        metadata_row_colored(
                            ui,
                            &app.tr("Status"),
                            &format!("{} {}", info.status, info.status_text),
                            status_color,
                        );
                        metadata_row(ui, &app.tr("Final URL"), &info.final_url);
                        optional_metadata_row(ui, &app.tr("File name"), info.file_name.as_deref());
                        optional_metadata_row(
                            ui,
                            &app.tr("Content type"),
                            info.content_type.as_deref(),
                        );
                        if let Some(length) = info.content_length {
                            metadata_row(ui, &app.tr("Remote size"), &human_bytes(length));
                        }
                        optional_metadata_row(
                            ui,
                            &app.tr("Byte ranges"),
                            info.accept_ranges.as_deref(),
                        );
                        optional_metadata_row(
                            ui,
                            &app.tr("Last modified"),
                            info.last_modified.as_deref(),
                        );
                        optional_metadata_row(ui, "ETag", info.etag.as_deref());
                        optional_metadata_row(ui, &app.tr("Server"), info.server.as_deref());
                        metadata_row(
                            ui,
                            &app.tr("Response time"),
                            &format!("{} ms", info.elapsed_ms),
                        );
                    });
            }
            ProbeStatus::Failed { message, .. } => {
                ui.label(
                    egui::RichText::new(format!("{}  {message}", crate::ui::icons::WARNING))
                        .color(ui.visuals().error_fg_color),
                );
            }
            ProbeStatus::NotApplicable { message, .. } => {
                ui.label(format!("{}  {message}", crate::ui::icons::INFO));
            }
        }
    });
}

fn metadata_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).strong());
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
}

fn metadata_row_colored(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(label).strong());
    ui.label(egui::RichText::new(value).strong().color(color));
    ui.end_row();
}

fn optional_metadata_row(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        metadata_row(ui, label, value);
    }
}

pub fn validate_media_url(input: &str) -> Result<ValidatedMediaUrl, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Enter a media location.".to_string());
    }
    if input.chars().any(char::is_whitespace) {
        return Err("The location contains whitespace or a line break.".to_string());
    }
    let parsed = url::Url::parse(input).map_err(|_| {
        "Enter a complete URL including its protocol, such as https://.".to_string()
    })?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    let supported = [
        "http", "https", "rtsp", "rtsps", "rtmp", "rtmps", "srt", "udp", "tcp", "rist",
    ];
    if !supported.contains(&scheme.as_str()) {
        return Err(format!(
            "The {scheme} protocol is not supported by Open Location."
        ));
    }
    let host = parsed.host_str().unwrap_or_default().to_owned();
    if host.is_empty() {
        return Err("This protocol requires a host name or IP address.".to_string());
    }
    let path = parsed.path().to_owned();
    let file_name = parsed
        .path_segments()
        .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
        .map(str::to_owned);
    let warning = (!parsed.username().is_empty() || parsed.password().is_some()).then(|| {
        "This URL contains embedded credentials. They will be passed to the media engine."
            .to_string()
    });
    Ok(ValidatedMediaUrl {
        normalized: parsed.to_string(),
        scheme: scheme.clone(),
        host,
        port: parsed.port_or_known_default(),
        path,
        file_name,
        query_parameters: parsed.query_pairs().count(),
        supports_http_probe: matches!(scheme.as_str(), "http" | "https"),
        warning,
    })
}

fn probe_remote_media(url: &str) -> Result<RemoteMediaInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(PROBE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(8))
        .user_agent(concat!("Pealayer/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not initialize the URL inspector: {error}"))?;

    let started = Instant::now();
    let mut response = client
        .head(url)
        .send()
        .map_err(|error| friendly_probe_error(error))?;
    if matches!(
        response.status(),
        reqwest::StatusCode::METHOD_NOT_ALLOWED
            | reqwest::StatusCode::NOT_IMPLEMENTED
            | reqwest::StatusCode::FORBIDDEN
    ) {
        response = client
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .map_err(|error| friendly_probe_error(error))?;
    }

    let headers = response.headers();
    let content_length = header_string(headers, CONTENT_RANGE)
        .and_then(|value| {
            value
                .rsplit_once('/')
                .and_then(|(_, total)| total.parse().ok())
        })
        .or_else(|| {
            header_string(headers, CONTENT_LENGTH).and_then(|value| value.parse::<u64>().ok())
        });
    let final_url = response.url().to_string();
    let file_name = header_string(headers, CONTENT_DISPOSITION)
        .and_then(|value| content_disposition_filename(&value))
        .or_else(|| {
            response
                .url()
                .path_segments()
                .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
                .map(str::to_owned)
        });
    let status = response.status();
    Ok(RemoteMediaInfo {
        requested_url: url.to_owned(),
        final_url,
        status: status.as_u16(),
        status_text: status
            .canonical_reason()
            .unwrap_or("Unknown status")
            .to_string(),
        content_type: header_string(headers, CONTENT_TYPE),
        content_length,
        file_name,
        accept_ranges: header_string(headers, ACCEPT_RANGES),
        last_modified: header_string(headers, LAST_MODIFIED),
        etag: header_string(headers, ETAG),
        server: header_string(headers, SERVER),
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn friendly_probe_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "The remote server did not respond before the inspection timeout.".to_string()
    } else if error.is_connect() {
        format!("Could not connect to the remote server: {error}")
    } else if error.is_redirect() {
        "The remote server exceeded the redirect limit.".to_string()
    } else {
        format!("Could not inspect the remote media: {error}")
    }
}

fn header_string(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn content_disposition_filename(value: &str) -> Option<String> {
    value.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        name.eq_ignore_ascii_case("filename")
            .then(|| value.trim().trim_matches('"').to_owned())
            .filter(|value| !value.is_empty())
    })
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.2} {} ({bytes} bytes)", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_media_urls_and_extracts_identity() {
        let value = validate_media_url("https://example.com:8443/media/movie.mp4?token=1").unwrap();
        assert_eq!(value.scheme, "https");
        assert_eq!(value.host, "example.com");
        assert_eq!(value.port, Some(8443));
        assert_eq!(value.file_name.as_deref(), Some("movie.mp4"));
        assert_eq!(value.query_parameters, 1);
        assert!(value.supports_http_probe);
    }

    #[test]
    fn accepts_stream_protocols_without_claiming_http_metadata() {
        let value = validate_media_url("srt://cinema.local:9000?mode=caller").unwrap();
        assert_eq!(value.scheme, "srt");
        assert!(!value.supports_http_probe);
    }

    #[test]
    fn rejects_missing_protocol_whitespace_and_local_file_scheme() {
        assert!(validate_media_url("example.com/video.mp4").is_err());
        assert!(validate_media_url("https://example.com/my video.mp4").is_err());
        assert!(validate_media_url("file:///C:/video.mp4").is_err());
    }

    #[test]
    fn extracts_remote_filename_and_formats_lengths() {
        assert_eq!(
            content_disposition_filename("attachment; filename=\"movie.mp4\""),
            Some("movie.mp4".to_string())
        );
        assert_eq!(human_bytes(6419456), "6.12 MiB (6419456 bytes)");
    }

    #[test]
    fn unicode_selection_replacement_uses_character_indices() {
        let text = "AسلامZ";
        assert_eq!(char_to_byte(text, 1), 1);
        assert_eq!(char_to_byte(text, 5), text.len() - 1);
    }
}
