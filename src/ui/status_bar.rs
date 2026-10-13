use crate::app::PealayerApp;
use crate::config::{StatusBarFpsMode, StatusBarWidget as Widget, StatusChartMetric};
use crate::four_d::controller::{HardwareCapabilities, HardwareStatusLed};
use crate::ui::dropdown::DropdownUiExt;
use eframe::egui;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const ITEM_HEIGHT: f32 = 22.0;
const LOG_LIMIT: usize = 240;
const WORKSPACE_WIDTH: f32 = 190.0;

pub fn widget_label(item: Widget) -> &'static str {
    match item {
        Widget::Fps => "Frame rate",
        Widget::Hot => "HOT alerts",
        Widget::Estop => "E-STOP",
        Widget::Connection => "Hardware connection",
        Widget::StatusRgb => "Physical status RGB",
        Widget::Voltage => "Voltage",
        Widget::Current => "Current draw",
        Widget::Temperature => "Temperature",
        Widget::Warnings => "Hardware warnings",
        Widget::Timeline => "Hardware timing",
        Widget::Progress => "Operation progress",
        Widget::Chart => "Telemetry log chart",
        Widget::Message => "Status message",
        Widget::Workspace => "Workspace mode",
    }
}

fn widget_icon(item: Widget) -> &'static str {
    use crate::ui::icons::*;
    match item {
        Widget::Fps => GAUGE,
        Widget::Hot | Widget::Estop | Widget::Warnings => WARNING,
        Widget::Connection => PLUG,
        Widget::StatusRgb => LIGHTBULB,
        Widget::Voltage => LIGHTNING,
        Widget::Current => CIRCUITRY,
        Widget::Temperature => THERMOMETER,
        Widget::Chart => WAVEFORM,
        Widget::Timeline => CLOCK,
        Widget::Progress => ARROW_CLOCKWISE,
        Widget::Message => INFO,
        Widget::Workspace => APP_WINDOW,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct StatusProgress {
    pub id: String,
    pub source: String,
    pub label: String,
    pub phase: String,
    pub fraction: Option<f32>,
    pub detail: String,
}

pub fn active_operations(capabilities: Option<&HardwareCapabilities>) -> Vec<StatusProgress> {
    if let Some(snapshot) = crate::peer::client().and_then(|client| client.snapshot()) {
        return snapshot
            .session
            .status
            .get("status_operations")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
    }
    let mut result = Vec::new();
    let update = crate::update::manager().status();
    if update.active() {
        let fraction = update.progress_percent().map(|value| value as f32 / 100.0);
        result.push(StatusProgress {
            id: update.operation_id.unwrap_or_else(|| "app-update".into()),
            source: "app".into(),
            label: "App update".into(),
            phase: update.state,
            fraction,
            detail: update.message,
        });
    }
    if let Some(capabilities) = capabilities {
        for operation in capabilities
            .operations
            .iter()
            .filter(|operation| operation.active())
        {
            result.push(StatusProgress {
                id: operation.id.clone(),
                source: "controller".into(),
                label: operation.kind.clone(),
                phase: operation.stage.clone(),
                fraction: operation
                    .progress_percent
                    .map(|value| f32::from(value) / 100.0),
                detail: operation.detail.clone(),
            });
        }
    }
    if let Some(downloads) = crate::downloads::progress_if_initialized() {
        for job in downloads {
            result.push(StatusProgress {
                id: job.id.clone(),
                source: "download".into(),
                label: "Download".into(),
                phase: format!("{:?}", job.state),
                fraction: job
                    .total
                    .filter(|total| *total > 0)
                    .map(|total| (job.downloaded as f64 / total as f64).clamp(0.0, 1.0) as f32),
                detail: format!("{} bytes received · {} bytes/s", job.downloaded, job.speed),
            });
        }
    }
    result.truncate(16);
    result
}

#[derive(Clone)]
struct LogSample {
    time: Instant,
    values: [Option<f64>; 3],
}
#[derive(Clone, Default)]
struct TelemetryLog {
    samples: VecDeque<LogSample>,
}

impl TelemetryLog {
    fn sample(&mut self, now: Instant, hardware: Option<&HardwareCapabilities>) {
        if self
            .samples
            .back()
            .is_some_and(|last| now.duration_since(last.time) < Duration::from_millis(500))
        {
            return;
        }
        let live = hardware
            .filter(|hardware| hardware.board_connected)
            .map(|hardware| &hardware.telemetry);
        let values = live
            .map(|value| {
                [
                    value.bus_mv.map(|value| f64::from(value) / 1000.0),
                    value.current_ma.map(f64::from),
                    value
                        .led_temperature_centi_c
                        .map(|value| f64::from(value) / 100.0),
                ]
            })
            .unwrap_or([None; 3]);
        self.samples.push_back(LogSample { time: now, values });
        while self.samples.len() > LOG_LIMIT {
            self.samples.pop_front();
        }
    }
    fn index(metric: StatusChartMetric) -> usize {
        match metric {
            StatusChartMetric::Voltage => 0,
            StatusChartMetric::Current => 1,
            StatusChartMetric::Temperature => 2,
        }
    }
    fn summary(&self, metric: StatusChartMetric) -> String {
        let index = Self::index(metric);
        let (name, unit) = match metric {
            StatusChartMetric::Voltage => ("Voltage", "V"),
            StatusChartMetric::Current => ("Current draw", "mA"),
            StatusChartMetric::Temperature => ("Temperature", "°C"),
        };
        let values: Vec<_> = self
            .samples
            .iter()
            .filter_map(|sample| sample.values[index])
            .collect();
        let range = if values.is_empty() {
            "—".to_string()
        } else {
            format!(
                "{:.2}–{:.2} {unit}",
                values.iter().copied().fold(f64::INFINITY, f64::min),
                values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            )
        };
        format!(
            "{name} · {range} · {}/240 samples · RAM only",
            self.samples.len()
        )
    }
    fn paint(&self, ui: &mut egui::Ui, rect: egui::Rect, metric: StatusChartMetric) {
        let index = Self::index(metric);
        let values: Vec<_> = self
            .samples
            .iter()
            .filter_map(|sample| sample.values[index])
            .collect();
        let (Some(first), Some(last)) = (self.samples.front(), self.samples.back()) else {
            return;
        };
        if values.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "—",
                egui::FontId::proportional(12.0),
                ui.visuals().weak_text_color(),
            );
            return;
        }
        let low = values.iter().copied().fold(f64::INFINITY, f64::min);
        let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let pad = ((high - low) * 0.1).max(0.1);
        let span = last.time.duration_since(first.time).as_secs_f64().max(0.5);
        let mut previous = None;
        for sample in &self.samples {
            let point = sample.values[index].map(|value| {
                egui::pos2(
                    rect.left()
                        + rect.width()
                            * (sample.time.duration_since(first.time).as_secs_f64() / span) as f32,
                    rect.bottom()
                        - rect.height() * ((value - low + pad) / (high - low + 2.0 * pad)) as f32,
                )
            });
            if let (Some(start), Some(end)) = (previous, point) {
                ui.painter().line_segment(
                    [start, end],
                    egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
                );
            }
            previous = point;
        }
    }
}

fn current_status_message(app: &PealayerApp) -> Option<String> {
    let update = crate::update::manager().status();
    if update.active() && !app.status_bar.progress {
        return Some(update.message);
    }
    let (message, timestamp) = app.osd_message.as_ref()?;
    let lifetime = Duration::from_secs_f32(
        app.osd_display_options
            .as_ref()
            .and_then(|options| options.timeout_seconds)
            .unwrap_or(app.osd_timeout_seconds)
            .max(0.25),
    );
    (timestamp.elapsed() <= lifetime).then(|| message.clone())
}

fn status_led_display_color(led: &HardwareStatusLed) -> egui::Color32 {
    egui::Color32::from_rgb(led.red, led.green, led.blue)
}

fn metric_text(hardware: Option<&HardwareCapabilities>, item: Widget) -> Option<String> {
    let telemetry = &hardware
        .filter(|hardware| hardware.board_connected)?
        .telemetry;
    match item {
        Widget::Voltage => telemetry
            .bus_mv
            .map(|value| format!("{:.2} V", f64::from(value) / 1000.0)),
        Widget::Current => telemetry.current_ma.map(|value| format!("{value} mA")),
        Widget::Temperature => telemetry
            .led_temperature_centi_c
            .map(|value| format!("{:.1} °C", f64::from(value) / 100.0)),
        _ => None,
    }
}

fn status_font(ui: &egui::Ui, monospace: bool, strong: bool) -> egui::FontId {
    let mut font = if monospace {
        egui::FontId::monospace(12.0)
    } else {
        egui::FontId::proportional(12.0)
    };
    if strong {
        let family = egui::FontFamily::Name(
            if monospace {
                "pealayer-status-mono-strong"
            } else {
                "pealayer-status-strong"
            }
            .into(),
        );
        if ui.fonts_mut(|fonts| fonts.families().contains(&family)) {
            font.family = family;
        }
    }
    font
}

fn metric_index(item: Widget) -> Option<usize> { match item {
    Widget::Voltage => Some(0), Widget::Current => Some(1), Widget::Temperature => Some(2), _ => None,
} }
fn metric_color(ui: &egui::Ui, config: &crate::config::StatusBarConfig,
    hardware: Option<&HardwareCapabilities>, item: Widget) -> egui::Color32 {
    let muted = ui.visuals().weak_text_color();
    if !config.semantic_colors { return muted; }
    let Some(hardware) = hardware.filter(|h| h.board_connected) else { return muted; };
    if item == Widget::Temperature && hardware.telemetry.hot == Some(true) { return ui.visuals().error_fg_color; }
    let Some(index) = metric_index(item) else { return muted; };
    let value = [hardware.telemetry.bus_mv, hardware.telemetry.current_ma, hardware.telemetry.led_temperature_centi_c][index];
    let [low, high] = config.metric_limits[index];
    if value.is_some_and(|value| low.is_some_and(|low| value < low) || high.is_some_and(|high| value > high)) {
        ui.visuals().warn_fg_color
    } else { muted }
}

fn text_widget(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    monospace: bool,
    color: egui::Color32,
    strong: bool,
    align: egui::Align,
) -> egui::Response {
    let font = status_font(ui, monospace, strong);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, ITEM_HEIGHT), egui::Sense::click_and_drag());
    let galley = egui::WidgetText::from(egui::RichText::new(text).font(font).color(color))
        .into_galley(ui, Some(egui::TextWrapMode::Truncate), (width - 6.0).max(1.0), egui::TextStyle::Small);
    let x = match align { egui::Align::Min => rect.left() + 3.0,
        egui::Align::Center => rect.center().x - galley.size().x * 0.5,
        egui::Align::Max => rect.right() - 3.0 - galley.size().x };
    ui.painter().with_clip_rect(rect).galley(egui::pos2(x, rect.center().y - galley.size().y * 0.5), galley, color);
    response.on_hover_text(text)
}

fn workspace_trigger(ui: &mut egui::Ui, name: &str, icon: &str, rtl: bool,
    tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(WORKSPACE_WIDTH, ITEM_HEIGHT), egui::Sense::click_and_drag());
    let visuals = ui.style().interact(&response);
    // Private-use icon glyphs must use the normal icon-capable font family,
    // not the custom bold status-caption family.
    let icon_center = egui::pos2(if rtl { rect.right() - 9.0 } else { rect.left() + 9.0 }, rect.center().y);
    let chevron_center = egui::pos2(if rtl { rect.left() + 9.0 } else { rect.right() - 9.0 }, rect.center().y);
    ui.painter().text(icon_center, egui::Align2::CENTER_CENTER, icon,
        egui::FontId::proportional(12.0), visuals.text_color());
    let name_rect = rect.shrink2(egui::vec2(20.0, 0.0));
    let galley = egui::WidgetText::from(egui::RichText::new(name).font(status_font(ui, false, true)))
        .into_galley(ui, Some(egui::TextWrapMode::Truncate), name_rect.width(), egui::TextStyle::Small);
    ui.painter().with_clip_rect(name_rect).galley(egui::pos2(
        if rtl { name_rect.right() - galley.size().x } else { name_rect.left() },
        rect.center().y - galley.size().y * 0.5), galley, visuals.text_color());
    crate::ui::control_density::paint_chevron(ui,
        egui::Rect::from_center_size(chevron_center, egui::vec2(14.0, 14.0)), visuals.text_color());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), name));
    response.on_hover_text(format!("{tooltip} · {name}"))
}

#[derive(Debug, PartialEq, Eq)]
enum WorkspaceChoice { Restore(String), Manage }

fn workspace_dropdown(ui: &mut egui::Ui, profiles: &[(String, crate::config::WorkspaceProfile)],
    active: Option<&str>, name: &str, icon: &str, rtl: bool, tooltip: &str,
    manage_label: &str) -> (egui::Response, Option<WorkspaceChoice>) {
    let response = workspace_trigger(ui, name, icon, rtl, tooltip);
    let mut selected = None;
    let popup_id = response.id.with("workspace-menu");
    if response.has_focus() && ui.input_mut(|input|
        input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
        egui::Popup::open_id(ui.ctx(), popup_id);
    }
    let popup = egui::Popup::menu(&response).id(popup_id)
        .align(if rtl { egui::RectAlign::TOP_END } else { egui::RectAlign::TOP_START })
        .width(200.0).show(|ui| {
            crate::ui::dropdown::menu_ui(ui, response.id, |ui| {
                if let Some(id) = crate::ui::workspace_profiles::profile_choices(ui, profiles, active) {
                    selected = Some(WorkspaceChoice::Restore(id));
                }
                ui.separator();
                if ui.dropdown_choice(false, format!("{}  {}", crate::ui::icons::TABS,
                    manage_label)).clicked() {
                    selected = Some(WorkspaceChoice::Manage);
                    ui.close();
                }
            });
        });
    if popup.is_none() { crate::ui::dropdown::clear_menu(ui.ctx(), response.id); }
    (response, selected)
}

fn workspace_selector(app: &mut PealayerApp, ui: &mut egui::Ui) -> egui::Response {
    let profiles = app.ordered_workspace_profiles();
    let icon = app.active_workspace_profile.as_ref().and_then(|id| app.workspace_profiles.get(id))
        .map(|profile| crate::ui::icons::workspace_icon(&profile.icon)).unwrap_or(crate::ui::icons::APP_WINDOW);
    let (response, choice) = workspace_dropdown(ui, &profiles, app.active_workspace_profile.as_deref(),
        &format!("{}: {}", app.tr("Workspace"), app.active_workspace_name()), icon, app.rtl, &app.tr("Switch workspace"), &app.tr("Manage workspaces..."));
    match choice {
        Some(WorkspaceChoice::Restore(id)) => app.restore_workspace_profile(ui.ctx(), &id),
        Some(WorkspaceChoice::Manage) => app.show_workspace_profiles_dialog = true,
        None => {},
    }
    response
}

fn render_widget(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    item: Widget,
    hardware: Option<&HardwareCapabilities>,
    log: &TelemetryLog,
    progress: &[StatusProgress],
) -> Option<(egui::Response, String)> {
    let color = ui.visuals().text_color();
    let (text, width, mono, color) = match item {
        Widget::Fps => {
            let fps = app.current_fps_display(Instant::now())?;
            let mode = app.tr(match app.status_bar.fps_mode {
                StatusBarFpsMode::Media => "Media",
                StatusBarFpsMode::Ui => "Screen",
                StatusBarFpsMode::Auto => "Auto",
            });
            let text = format!(
                "{} {}",
                if fps.is_ui_rate {
                    crate::ui::icons::GAUGE
                } else {
                    crate::ui::icons::FILE_VIDEO
                },
                fps.formatted
            );
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(88.0, ITEM_HEIGHT),
                egui::Sense::click_and_drag(),
            );
            let icon = if fps.is_ui_rate {
                crate::ui::icons::GAUGE
            } else {
                crate::ui::icons::FILE_VIDEO
            };
            let icon_x = if app.rtl {
                rect.right() - 7.0
            } else {
                rect.left() + 7.0
            };
            ui.painter().text(
                egui::pos2(icon_x, rect.center().y),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(12.0),
                color,
            );
            let number_rect = if app.rtl {
                egui::Rect::from_min_max(rect.min, rect.max - egui::vec2(19.0, 0.0))
            } else {
                egui::Rect::from_min_max(rect.min + egui::vec2(19.0, 0.0), rect.max)
            };
            ui.painter().with_clip_rect(number_rect).text(
                number_rect.right_center() - egui::vec2(3.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                &fps.formatted,
                status_font(ui, app.status_bar.uses_monospace(item), true),
                color,
            );
            return Some((
                response.on_hover_text(format!("{mode}\n{}", fps.tooltip)),
                text,
            ));
        }
        Widget::Connection => {
            let requested = app
                .engine_handle
                .connection_requested
                .load(std::sync::atomic::Ordering::Relaxed);
            let availability = crate::four_d::programming::hardware_availability(app.is_connected, requested, hardware);
            let ready = availability == crate::four_d::programming::HardwareAvailability::Ready;
            let text = if ready {
                hardware
                    .map(|hardware| app.display_text(&hardware.board_name))
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| app.tr("Connected board"))
            } else {
                app.tr(availability.label())
            };
            let icon = match app.status_bar.connection_icon {
                crate::config::StatusConnectionIcon::Plug => crate::ui::icons::PLUG,
                crate::config::StatusConnectionIcon::Circle => "●",
                crate::config::StatusConnectionIcon::Controller => crate::ui::icons::CPU,
                crate::config::StatusConnectionIcon::None => "",
            };
            let mode = app.status_bar.connection_mode.then(|| app.engine_handle.active_transport.try_lock().ok()
                .and_then(|value| value.clone())).flatten();
            let text = match mode { Some(mode) => format!("{text} · {mode}"), None => text };
            (
                format!("{icon} {text}").trim().to_owned(),
                if app.status_bar.connection_mode { 252.0 } else { 148.0 },
                false,
                if ready && app.is_connected {
                    egui::Color32::from_rgb(46, 204, 113)
                } else {
                    ui.visuals().warn_fg_color
                },
            )
        }
        Widget::Voltage | Widget::Current | Widget::Temperature => (
            metric_text(hardware, item)?,
            if item == Widget::Temperature { 64.0 } else { 58.0 },
            app.status_bar.uses_monospace(item),
            metric_color(ui, &app.status_bar, hardware, item),
        ),
        Widget::StatusRgb => {
            let led = hardware
                .filter(|hardware| hardware.board_connected)?
                .status_led
                .as_ref()?;
            let (rect, response) = ui
                .allocate_exact_size(egui::vec2(20.0, ITEM_HEIGHT), egui::Sense::click_and_drag());
            ui.painter()
                .circle_filled(rect.center(), 6.0, status_led_display_color(led));
            return Some((
                response.on_hover_text(format!(
                    "RGB {}, {}, {} · brightness {}",
                    led.red, led.green, led.blue, led.brightness
                )),
                "RGB".into(),
            ));
        }
        Widget::Hot => {
            if hardware
                .filter(|hardware| hardware.board_connected)?
                .telemetry
                .hot
                != Some(true)
            {
                return None;
            }
            (
                format!(
                    "{} HOT · {}",
                    crate::ui::icons::WARNING,
                    app.tr("Temperature alert")
                ),
                160.0,
                false,
                ui.visuals().error_fg_color,
            )
        }
        Widget::Estop => {
            if !app.estop_active {
                return None;
            }
            (
                format!("{} {}", crate::ui::icons::WARNING, app.tr("E-STOP ACTIVE")),
                130.0,
                false,
                ui.visuals().error_fg_color,
            )
        }
        Widget::Warnings => {
            let warning = hardware?.warnings.first()?;
            let text = format!("{} {}", crate::ui::icons::WARNING, warning.code);
            return Some((
                text_widget(
                    ui,
                    &text,
                    150.0,
                    false,
                    ui.visuals().warn_fg_color,
                    true,
                    egui::Align::Min,
                )
                .on_hover_text(&warning.message),
                text,
            ));
        }
        Widget::Timeline => {
            if !app.is_connected {
                return None;
            }
            let plan = app.engine_handle.prepared_timeline.try_lock().ok()?;
            if !plan.has_items() && !plan.play_requested && plan.error.is_none() && plan.deferred_reason.is_none() {
                return None;
            }
            let state = plan.presentation_label();
            let text = format!("{} {}", crate::ui::icons::CLOCK, app.tr(state));
            let tooltip = plan
                .error
                .clone()
                .or_else(|| plan.deferred_reason.clone())
                .unwrap_or_else(|| {
                    format!(
                        "Revision {} · ACK {}",
                        plan.revision, plan.acknowledged_revision
                    )
                });
            let color = if plan.error.is_some() {
                ui.visuals().error_fg_color
            } else {
                color
            };
            drop(plan);
            return Some((
                text_widget(ui, &text, 148.0, false, color, false, egui::Align::Min)
                    .on_hover_text(tooltip),
                text,
            ));
        }
        Widget::Message => (current_status_message(app)?, 180.0, false, color),
        // The workspace has one dedicated, interactive trailing selector.
        Widget::Workspace => return None,
        Widget::Progress => {
            let operation = progress.first()?;
            let text = format!(
                "{} · {}{}",
                operation.label,
                operation.phase,
                operation
                    .fraction
                    .map(|value| format!(" · {:.0}%", value * 100.0))
                    .unwrap_or_default()
            );
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(168.0, ITEM_HEIGHT),
                egui::Sense::click_and_drag(),
            );
            if let Some(fraction) = operation.fraction {
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        rect.left_bottom() - egui::vec2(0.0, 3.0),
                        egui::vec2(rect.width() * fraction, 3.0),
                    ),
                    1.0,
                    ui.visuals().selection.bg_fill,
                );
            } else {
                ui.put(
                    egui::Rect::from_min_size(rect.left_top(), egui::vec2(14.0, 14.0)),
                    egui::Spinner::new().size(12.0),
                );
            }
            ui.painter().with_clip_rect(rect).text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &text,
                egui::FontId::proportional(11.0),
                color,
            );
            let tooltip = progress
                .iter()
                .map(|operation| {
                    format!(
                        "{} · {}\n{}",
                        operation.label, operation.phase, operation.detail
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some((response.on_hover_text(tooltip), text));
        }
        Widget::Chart => {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(100.0, ITEM_HEIGHT),
                egui::Sense::click_and_drag(),
            );
            log.paint(
                ui,
                rect.shrink2(egui::vec2(3.0, 3.0)),
                app.status_bar.chart_metric,
            );
            let label = app.tr("Telemetry log chart");
            let response = response.on_hover_ui(|ui| {
                ui.label(&label);
                ui.weak(log.summary(app.status_bar.chart_metric));
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(260.0, 90.0), egui::Sense::hover());
                log.paint(ui, rect, app.status_bar.chart_metric);
            });
            return Some((response, label));
        }
    };
    let strong = matches!(
        item,
        Widget::Connection
            | Widget::Fps
            | Widget::Hot
            | Widget::Estop
    );
    let align = if mono || metric_index(item).is_some() {
        egui::Align::Max
    } else {
        egui::Align::Min
    };
    Some((
        text_widget(ui, &text, width, mono, color, strong, align),
        text,
    ))
}

fn item_menu(app: &mut PealayerApp, ui: &mut egui::Ui, item: Widget, log_id: egui::Id) {
    ui.strong(format!(
        "{} {}",
        widget_icon(item),
        app.tr(widget_label(item))
    ));
    ui.separator();
    let mut changed = false;
    if item == Widget::Fps && let Some(fps) = app.current_fps_display(Instant::now()) {
        if ui.dropdown_choice(false, app.tr("Copy value")).clicked() { ui.ctx().copy_text(fps.formatted); ui.close(); }
        ui.weak(fps.tooltip);
    }
    if item == Widget::Estop && ui.dropdown_choice(false, app.tr("Release E-STOP...")).clicked() {
        app.request_emergency_stop_change(false); ui.close();
    }
    if matches!(item, Widget::Warnings | Widget::Hot) && let Some(hardware) = app.advertised_hardware() {
        if let Some(text) = metric_text(Some(&hardware), Widget::Temperature) { ui.label(text); }
        for warning in &hardware.warnings { ui.label(format!("{} · {}", warning.code, warning.message)); }
        if ui.dropdown_choice(false, app.tr("Show in chart")).clicked() {
            app.status_bar.chart_metric = StatusChartMetric::Temperature; app.status_bar.chart = true; changed = true; ui.close();
        }
    }
    if item == Widget::Connection {
        ui.menu_button(app.tr("Connection icon"), |ui| {
            use crate::config::StatusConnectionIcon;
            for (value, label) in [(StatusConnectionIcon::Plug, "Plug"), (StatusConnectionIcon::Circle, "Circle"),
                (StatusConnectionIcon::Controller, "PCController"), (StatusConnectionIcon::None, "None")] {
                if ui.dropdown_choice(app.status_bar.connection_icon == value, app.tr(label)).clicked() {
                    app.status_bar.connection_icon = value; changed = true; ui.close();
                }
            }
        });
        changed |= ui.checkbox(&mut app.status_bar.connection_mode, "Connection mode").changed();
        if !app.serial_port.is_empty() && ui.dropdown_choice(false, app.tr("Copy endpoint")).clicked() {
            ui.ctx().copy_text(app.serial_port.clone()); ui.close();
        }
    }
    if let Some(metric) = metric_index(item).map(|index| [StatusChartMetric::Voltage, StatusChartMetric::Current, StatusChartMetric::Temperature][index]) {
        if ui.dropdown_choice(false, app.tr("Show in chart")).clicked() {
            app.status_bar.chart_metric = metric; app.status_bar.chart = true; changed = true; ui.close();
        }
        if let Some(text) = metric_text(app.advertised_hardware().as_ref(), item)
            && ui.dropdown_choice(false, app.tr("Copy value")).clicked() { ui.ctx().copy_text(text); ui.close(); }
    }
    if item == Widget::Workspace && ui.dropdown_choice(false, app.tr("Manage workspaces...")).clicked() {
        app.show_workspace_profiles_dialog = true; ui.close();
    }
    if item == Widget::StatusRgb && let Some(led) = app.advertised_hardware().and_then(|h| h.status_led) {
        let color = format!("#{:02X}{:02X}{:02X}", led.red, led.green, led.blue);
        if ui.dropdown_choice(false, format!("{} · {color}", app.tr("Copy color"))).clicked() { ui.ctx().copy_text(color); ui.close(); }
    }
    if item == Widget::Timeline && ui.dropdown_choice(false, app.tr("Show in timeline")).clicked() {
        app.open_or_focus_tab(crate::ui::layout::PealayerTab::Timeline); ui.close();
    }
    if item == Widget::Progress {
        if ui.dropdown_choice(false, app.tr("Copy operation details")).clicked() {
            let text = active_operations(app.advertised_hardware().as_ref()).iter()
                .map(|operation| format!("{} · {}\n{}", operation.label, operation.phase, operation.detail)).collect::<Vec<_>>().join("\n");
            ui.ctx().copy_text(text); ui.close();
        }
        for operation in active_operations(app.advertised_hardware().as_ref()) {
            ui.label(format!("{} · {}",operation.label,operation.phase)); ui.weak(operation.detail);
        }
        ui.separator();
    }
    if item == Widget::Message && let Some(message) = current_status_message(app) { ui.label(message); ui.separator(); }
    if item == Widget::Message {
        if ui.dropdown_choice(false, app.tr("Copy message")).clicked() {
            if let Some(message) = current_status_message(app) { ui.ctx().copy_text(message); } ui.close();
        }
        if ui.dropdown_choice(false, app.tr("Dismiss")).clicked() { app.osd_message = None; ui.close(); }
    }
    for (end, label) in [(false, "Place at start"), (true, "Place at end")] {
        if ui.dropdown_choice(app.status_bar.at_end(item) == end, app.tr(label)).clicked() {
            app.status_bar.place_at_end(item, end); changed = true; ui.close();
        }
    }
    if item == Widget::Fps || metric_index(item).is_some() {
        let label = app.tr("Use monospace");
        changed |= ui.checkbox(&mut app.status_bar.monospace[item as usize], label).changed();
    }
    if let Some(index) = metric_index(item) {
        let label = app.tr("Semantic colors");
        changed |= ui.checkbox(&mut app.status_bar.semantic_colors, label).changed();
        ui.menu_button(app.tr("Value range"), |ui| {
            let (factor, unit) = match item { Widget::Voltage => (1000.0, "V"), Widget::Current => (1.0, "mA"), _ => (100.0, "°C") };
            for (bound, label) in [(0, "Minimum"), (1, "Maximum")] {
                ui.horizontal(|ui| {
                    let mut enabled = app.status_bar.metric_limits[index][bound].is_some();
                    if ui.checkbox(&mut enabled, app.tr(label)).changed() {
                        let initial = app.status_bar.metric_limits[index][1-bound].map(|other|
                            if bound==0 { 0.min(other) } else { 0.max(other) }).unwrap_or(0);
                        app.status_bar.metric_limits[index][bound] = enabled.then_some(initial); changed = true;
                    }
                    if let Some(raw) = app.status_bar.metric_limits[index][bound] {
                        let mut value = f64::from(raw) / factor;
                        if ui.add(egui::DragValue::new(&mut value).speed(0.1).suffix(format!(" {unit}"))).changed() {
                            let mut raw = (value * factor).round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
                            if let Some(other) = app.status_bar.metric_limits[index][1 - bound] { raw = if bound == 0 { raw.min(other) } else { raw.max(other) }; }
                            app.status_bar.metric_limits[index][bound] = Some(raw); changed = true;
                        }
                    }
                });
            }
        });
    }
    ui.separator();
    if item == Widget::Fps {
        for (mode, title) in [
            (StatusBarFpsMode::Media, "Media"),
            (StatusBarFpsMode::Ui, "Screen"),
            (StatusBarFpsMode::Auto, "Auto"),
        ] {
            if ui
                .dropdown_choice(app.status_bar.fps_mode == mode, app.tr(title))
                .clicked()
            {
                app.status_bar.fps_mode = mode;
                changed = true;
                ui.close();
            }
        }
        ui.separator();
    }
    if item == Widget::Chart {
        let pause_label = app.tr("Pause chart logging");
        changed |= ui.checkbox(&mut app.status_bar.chart_paused, pause_label).changed();
        if ui.dropdown_choice(false, app.tr("Copy chart history")).clicked() {
            let history = ui.ctx().data(|data| data.get_temp::<TelemetryLog>(log_id)).unwrap_or_default();
            let mut csv = String::from("elapsed_seconds,voltage_V,current_mA,temperature_C\n");
            if let Some(first) = history.samples.front() { for sample in &history.samples {
                csv.push_str(&format!("{:.3},{}\n", sample.time.duration_since(first.time).as_secs_f64(),
                    sample.values.iter().map(|value| value.map(|v| format!("{v:.3}")).unwrap_or_default()).collect::<Vec<_>>().join(",")));
            } }
            ui.ctx().copy_text(csv); ui.close();
        }
        for (metric, title) in [
            (StatusChartMetric::Voltage, "Voltage"),
            (StatusChartMetric::Current, "Current draw"),
            (StatusChartMetric::Temperature, "Temperature"),
        ] {
            if ui
                .dropdown_choice(app.status_bar.chart_metric == metric, app.tr(title))
                .clicked()
            {
                app.status_bar.chart_metric = metric;
                changed = true;
                ui.close();
            }
        }
        if ui
            .dropdown_choice(false, app.tr("Clear chart history"))
            .clicked()
        {
            ui.ctx()
                .data_mut(|data| data.remove::<TelemetryLog>(log_id));
            ui.close();
        }
        ui.separator();
    }
    if matches!(
        item,
        Widget::Connection
            | Widget::Voltage
            | Widget::Current
            | Widget::Temperature
            | Widget::StatusRgb
            | Widget::Warnings
            | Widget::Hot
            | Widget::Timeline
    ) {
        if ui
            .dropdown_choice(
                false,
                format!("{} {}", crate::ui::icons::GAUGE, app.tr("Hardware Monitor")),
            )
            .clicked()
        {
            app.open_or_focus_tab(crate::ui::layout::PealayerTab::HardwareMonitor);
            ui.close();
        }
    }
    let order: Vec<_> = app
        .status_bar
        .ordered()
        .into_iter()
        .filter(|value| *value != Widget::Workspace)
        .collect();
    let index = order.iter().position(|value| *value == item);
    for (earlier, title) in [(true, "Move earlier"), (false, "Move later")] {
        let target = index.and_then(|index| {
            if earlier {
                index.checked_sub(1)
            } else {
                (index + 1 < order.len()).then_some(index + 1)
            }
        });
        if ui
            .add_enabled_ui(target.is_some(), |ui| {
                ui.dropdown_choice(false, app.tr(title))
            })
            .inner
            .clicked()
            && let Some(target) = target
        {
            changed |= app.status_bar.reorder(item, order[target], !earlier);
            ui.close();
        }
    }
    ui.separator();
    if ui
        .dropdown_choice(
            false,
            format!(
                "{} {} {} {}",
                crate::ui::icons::EYE_SLASH,
                app.tr("Hide"),
                widget_icon(item),
                app.tr(widget_label(item))
            ),
        )
        .clicked()
    {
        app.status_bar.set_visible(item, false);
        changed = true;
        ui.close();
    }
    if changed {
        app.request_status_bar_save(ui.ctx());
    }
}

fn present(app: &PealayerApp, item: Widget, hardware: Option<&HardwareCapabilities>, progress: &[StatusProgress]) -> bool {
    let live = hardware.filter(|h| h.board_connected);
    match item {
        Widget::Voltage | Widget::Current | Widget::Temperature => metric_text(hardware, item).is_some(),
        Widget::StatusRgb => live.and_then(|h| h.status_led.as_ref()).is_some(),
        Widget::Hot => live.is_some_and(|h| h.telemetry.hot == Some(true)),
        Widget::Estop => app.estop_active, Widget::Warnings => hardware.is_some_and(|h| !h.warnings.is_empty()),
        Widget::Progress => !progress.is_empty(), Widget::Message => current_status_message(app).is_some(),
        Widget::Timeline => app.is_connected && app.engine_handle.prepared_timeline.try_lock().ok().is_some_and(|p|
            p.has_items() || p.play_requested || p.error.is_some() || p.deferred_reason.is_some()),
        _ => true,
    }
}
fn slot_width(item: Widget) -> f32 { match item {
    Widget::Fps => 88.0, Widget::Voltage | Widget::Current => 58.0, Widget::Temperature => 64.0,
    Widget::StatusRgb => 20.0, Widget::Workspace => WORKSPACE_WIDTH, Widget::Chart => 100.0,
    Widget::Progress => 168.0, Widget::Message => 180.0, Widget::Warnings => 150.0,
    Widget::Hot => 160.0, Widget::Estop => 130.0, _ => 148.0,
} }
fn group_items(app: &PealayerApp, end: bool, hardware: Option<&HardwareCapabilities>, progress: &[StatusProgress]) -> Vec<Widget> {
    let mut items: Vec<_> = app.status_bar.ordered().into_iter().filter(|item|
        *item != Widget::Workspace && app.status_bar.visible(*item) && app.status_bar.at_end(*item) == end
            && present(app, *item, hardware, progress)).collect();
    // Workspace remains the last item of its selected side, preserving the default end anchor.
    if app.status_bar.workspace && app.status_bar.at_end(Widget::Workspace) == end { items.push(Widget::Workspace); }
    items
}
fn group_menu(app: &mut PealayerApp, ui: &mut egui::Ui, end: bool) {
    ui.strong(app.tr("Status bar"));
    let mut changed = false;
    let label = app.tr("Show dividers"); changed |= ui.checkbox(&mut app.status_bar.dividers, label).changed();
    let label = app.tr("Semantic colors"); changed |= ui.checkbox(&mut app.status_bar.semantic_colors, label).changed();
    ui.separator();
    for item in app.status_bar.ordered() {
        let here = app.status_bar.visible(item) && app.status_bar.at_end(item) == end;
        if ui.dropdown_choice(here, format!("{} {}", widget_icon(item), app.tr(widget_label(item)))).clicked() {
            app.status_bar.set_visible(item, !here);
            if !here { app.status_bar.place_at_end(item, end); }
            changed = true;
        }
    }
    ui.separator();
    if ui.dropdown_choice(false, app.tr("Fonts…")).clicked() {
        app.preferences_tab = 0; crate::ui::preferences::open(app, ui.ctx()); ui.close();
    }
    if ui.dropdown_choice(false, app.tr("Reset widget order")).clicked() {
        let defaults = crate::config::StatusBarConfig::default();
        app.status_bar.order = defaults.order; app.status_bar.trailing = defaults.trailing;
        changed = true; ui.close();
    }
    if changed { app.request_status_bar_save(ui.ctx()); }
}
fn click_item(app: &mut PealayerApp, ui: &mut egui::Ui, item: Widget, progress: &[StatusProgress]) {
    match item {
        Widget::Fps => { app.status_bar.fps_mode = app.status_bar.fps_mode.next(); app.request_status_bar_save(ui.ctx()); }
        Widget::Estop => app.request_emergency_stop_change(false),
        Widget::Progress if progress.first().is_some_and(|p| p.source == "download") => crate::downloads::open(ui.ctx()),
        Widget::Timeline => app.open_or_focus_tab(crate::ui::layout::PealayerTab::Timeline),
        Widget::Connection | Widget::Voltage | Widget::Current | Widget::Temperature | Widget::StatusRgb
            | Widget::Warnings | Widget::Hot =>
                app.open_or_focus_tab(crate::ui::layout::PealayerTab::HardwareMonitor),
        _ => {},
    }
}
fn draw_group(app: &mut PealayerApp, ui: &mut egui::Ui, end: bool, hardware: Option<&HardwareCapabilities>,
    log: &TelemetryLog, progress: &[StatusProgress], log_id: egui::Id, rects: &mut Vec<(Widget, egui::Rect)>) {
    let background = ui.interact(ui.max_rect(), ui.id().with(("status-side", end)), egui::Sense::click());
    let items = group_items(app, end, hardware, progress);
    egui::ScrollArea::horizontal().id_salt(("status-side-scroll", end))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL).auto_shrink([false, true])
        .show(ui, |ui| {
            ui.with_layout(crate::ui::i18n::layout(app.rtl, egui::Align::Center), |ui| {
                for (index, item) in items.into_iter().enumerate() {
                    if index > 0 && app.status_bar.dividers {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, ITEM_HEIGHT), egui::Sense::hover());
                        ui.painter().line_segment([rect.center_top()+egui::vec2(0.0,5.0), rect.center_bottom()-egui::vec2(0.0,5.0)],
                            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color));
                    }
                    let id = ui.make_persistent_id(("status-widget", item));
                    let offset_id = id.with("offset");
                    if let Some(source) = ui.ctx().data(|d| d.get_temp::<egui::Rect>(id.with("source"))) {
                        crate::ui::layout::remember_drag_offset_on_press(ui.ctx(), source, offset_id);
                    }
                    let dragging = egui::DragAndDrop::payload::<Widget>(ui.ctx()).is_some_and(|p| *p == item)
                        && ui.input(|i| i.pointer.primary_down());
                    let layer = if dragging { egui::LayerId::new(egui::Order::Tooltip, id) } else { ui.layer_id() };
                    let result = ui.scope_builder(egui::UiBuilder::new().layer_id(layer), |ui| {
                        if dragging { ui.set_clip_rect(ui.ctx().content_rect()); }
                        let bg = ui.painter().add(egui::Shape::Noop);
                        let response = if item == Widget::Workspace { Some((workspace_selector(app, ui), String::new())) }
                            else { render_widget(app, ui, item, hardware, log, progress) };
                        if let Some((response, _)) = &response {
                            let alpha = crate::ui::motion::hover_alpha(ui, id.with("hover"), response.hovered() || response.has_focus());
                            ui.painter().set(bg, egui::Shape::rect_filled(response.rect, 3.0,
                                ui.visuals().widgets.hovered.weak_bg_fill.gamma_multiply(alpha)));
                            if response.has_focus() { ui.painter().rect_stroke(response.rect.shrink(0.5), 3.0,
                                egui::Stroke::new(1.0, ui.visuals().selection.bg_fill), egui::StrokeKind::Inside); }
                        }
                        response
                    }).inner;
                    let Some((response, _)) = result else { continue };
                    rects.push((item, response.rect));
                    if !dragging { ui.ctx().data_mut(|d| d.insert_temp(id.with("source"), response.rect)); }
                    crate::ui::layout::remember_drag_offset_on_press(ui.ctx(), response.rect, offset_id);
                    response.dnd_set_drag_payload(item);
                    if dragging && let Some(pointer) = ui.ctx().pointer_interact_pos() {
                        let offset = ui.ctx().data(|d| d.get_temp::<egui::Vec2>(offset_id)).unwrap_or(response.rect.size()*0.5);
                        ui.ctx().transform_layer_shapes(layer, egui::emath::TSTransform::from_translation(pointer-offset-response.rect.min));
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                    if response.clicked() { click_item(app, ui, item, progress); }
                    if response.dnd_hover_payload::<Widget>().is_some_and(|p| *p != item) {
                        let after = ui.ctx().pointer_interact_pos().is_some_and(|p| (p.x > response.rect.center().x) != app.rtl);
                        let x = if after != app.rtl { response.rect.right() } else { response.rect.left() };
                        ui.painter().line_segment([egui::pos2(x,response.rect.top()),egui::pos2(x,response.rect.bottom())],
                            egui::Stroke::new(2.0,ui.visuals().selection.bg_fill));
                    }
                    if let Some(moved) = response.dnd_release_payload::<Widget>() {
                        let after = ui.ctx().pointer_interact_pos().is_some_and(|p| (p.x > response.rect.center().x) != app.rtl);
                        app.status_bar.place_at_end(*moved,end); app.status_bar.set_visible(*moved,true);
                        app.status_bar.reorder(*moved,item,after); app.request_status_bar_save(ui.ctx());
                    }
                    if matches!(item, Widget::Chart | Widget::Progress | Widget::Message) {
                        egui::Popup::menu(&response).show(|ui| item_menu(app,ui,item,log_id));
                    }
                    response.context_menu(|ui| item_menu(app,ui,item,log_id));
                }
            });
        });
    let over_item = ui.ctx().pointer_interact_pos().is_some_and(|p| rects.iter().any(|(_,r)|r.contains(p)));
    if !over_item {
        if background.dnd_hover_payload::<Widget>().is_some() {
            ui.painter().rect_stroke(background.rect.shrink(1.0),3.0,egui::Stroke::new(1.0,ui.visuals().selection.bg_fill),egui::StrokeKind::Inside);
        }
        if let Some(moved) = background.dnd_release_payload::<Widget>() {
            app.status_bar.place_at_end(*moved,end); app.status_bar.set_visible(*moved,true); app.request_status_bar_save(ui.ctx());
        }
        background.context_menu(|ui| group_menu(app,ui,end));
    }
}
pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let _timing = crate::performance::span(crate::performance::Phase::Status);
    if ui.input(|i| i.viewport().fullscreen.unwrap_or(false)) { return; }
    let hardware = app.advertised_hardware();
    let progress = active_operations(hardware.as_ref());
    let log_id = egui::Id::new("status-bar-telemetry-log");
    let now = Instant::now();
    let log = ui.ctx().data_mut(|data| { let log=data.get_temp_mut_or_default::<TelemetryLog>(log_id);
        if !app.status_bar.chart_paused { log.sample(now,hardware.as_ref()); } log.clone() });
    let payload = egui::DragAndDrop::payload::<Widget>(ui.ctx());
    let mut rects = Vec::new();
    let panel = egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
        ui.spacing_mut().item_spacing.x=3.0;
        ui.with_layout(crate::ui::i18n::layout(app.rtl,egui::Align::Center), |ui| {
            let items=group_items(app,true,hardware.as_ref(),&progress);
            let separators=if app.status_bar.dividers { items.len().saturating_sub(1) as f32*4.0 } else { 0.0 };
            let end_width=(items.iter().map(|item| if *item == Widget::Connection && app.status_bar.connection_mode { 252.0 } else { slot_width(*item) }).sum::<f32>() + items.len().saturating_sub(1) as f32*3.0 + separators)
                .max(24.0).min((ui.available_width()-48.0).max(24.0));
            let start_width=(ui.available_width()-end_width-3.0).max(1.0);
            ui.allocate_ui_with_layout(egui::vec2(start_width,ITEM_HEIGHT),crate::ui::i18n::layout(app.rtl,egui::Align::Center),
                |ui|draw_group(app,ui,false,hardware.as_ref(),&log,&progress,log_id,&mut rects));
            ui.allocate_ui_with_layout(egui::vec2(end_width,ITEM_HEIGHT),crate::ui::i18n::layout(app.rtl,egui::Align::Center),
                |ui|draw_group(app,ui,true,hardware.as_ref(),&log,&progress,log_id,&mut rects));
        });
    });
    #[cfg(test)] ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("test-status-widget-rects"),rects));
    if let Some(moved)=payload && let Some(pointer)=ui.ctx().pointer_interact_pos()
        && !panel.response.rect.expand(8.0).contains(pointer) {
        let painter=ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip,egui::Id::new("status-remove-hint")));
        painter.text(pointer+egui::vec2(12.0,20.0),egui::Align2::LEFT_TOP,
            format!("{} {} {}",crate::ui::icons::EYE_SLASH,app.tr("Hide"),app.tr(widget_label(*moved))),
            egui::FontId::proportional(12.0),ui.visuals().error_fg_color);
        if ui.input(|i|i.pointer.any_released()) { app.status_bar.set_visible(*moved,false); app.request_status_bar_save(ui.ctx()); }
    }
    if app.status_bar.fps_mode != StatusBarFpsMode::Media || app.status_bar.chart || !progress.is_empty() {
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programming_latch_status_is_painted_without_a_disconnected_claim() {
        let _lock=crate::app::tests::lock_app_tests();
        let context=egui::Context::default();
        let mut app=PealayerApp::default();
        app.is_connected=true;
        app.engine_handle.is_connected.store(true,std::sync::atomic::Ordering::Relaxed);
        install_hardware_snapshot_for_test(&app);
        let mut output=context.run_ui(egui::RawInput {
            screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1600.0,420.0))),..Default::default()
        },|ui| draw(&mut app,ui));
        assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Programming latch"))));
        assert!(!output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Disconnected") || text.galley.text().contains("not connected"))));
        output.textures_delta.clear();
    }

    fn install_hardware_snapshot_for_test(app:&PealayerApp) {
        crate::four_d::engine::install_hardware_snapshot(&app.engine_handle.hardware_capabilities,&app.engine_handle.hardware_revision,
            Some(HardwareCapabilities {board_connected:true,
                programming:Some(crate::four_d::programming::HardwareProgrammingState{latch_known:true,programming_latch:true,revision:1,..Default::default()}),..Default::default()}));
    }

    #[test]
    fn regular_metrics_are_centered_muted_and_color_only_with_real_limits_or_hot() {
        let context=egui::Context::default();
        crate::ui::i18n::configure_ui_fonts(&context,false);
        let mut config=crate::config::StatusBarConfig::default();
        let mut hardware=HardwareCapabilities {board_connected:true,..Default::default()};
        hardware.telemetry.bus_mv=Some(12000);
        let mut output=context.run_ui(egui::RawInput::default(),|ui|{
            config.semantic_colors=true; config.metric_limits[0]=[Some(11000),Some(13000)];
            assert_eq!(metric_color(ui,&config,Some(&hardware),Widget::Voltage),ui.visuals().weak_text_color());
            hardware.telemetry.bus_mv=Some(14000);
            assert_eq!(metric_color(ui,&config,Some(&hardware),Widget::Voltage),ui.visuals().warn_fg_color);
            config.semantic_colors=false;
            assert_eq!(metric_color(ui,&config,Some(&hardware),Widget::Voltage),ui.visuals().weak_text_color());
            for mono in [false,true] {
                let response=text_widget(ui,"12.00 V",58.0,mono,ui.visuals().weak_text_color(),false,egui::Align::Max);
                ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new(("metric-center",mono)),response.rect));
            }
        });
        output.textures_delta.clear();
        for (index,text) in output.shapes.iter().filter_map(|s|if let egui::Shape::Text(t)=&s.shape {Some(t)}else{None}).enumerate() {
            let mono=index==1;
            let rect=context.data(|d|d.get_temp::<egui::Rect>(egui::Id::new(("metric-center",mono)))).unwrap();
            assert!((text.pos.y+text.galley.size().y*0.5-rect.center().y).abs()<0.01);
            assert_eq!(text.galley.job.sections[0].format.font_id.family,if mono {egui::FontFamily::Monospace}else{egui::FontFamily::Proportional});
        }
    }

    #[test]
    fn widget_side_and_typeface_survive_persistence_and_do_not_change_other_widgets() {
        let mut config=crate::config::StatusBarConfig::default();
        config.place_at_end(Widget::Voltage,true); config.monospace[Widget::Voltage as usize]=false;
        config.set_visible(Widget::Voltage,false); config.set_visible(Widget::Voltage,true);
        let restored:crate::config::StatusBarConfig=serde_json::from_value(serde_json::to_value(config).unwrap()).unwrap();
        assert!(restored.at_end(Widget::Voltage)); assert!(restored.at_end(Widget::Workspace));
        assert!(!restored.at_end(Widget::Current)); assert!(!restored.uses_monospace(Widget::Voltage));
        assert!(restored.uses_monospace(Widget::Current)); assert_eq!(restored.metric_limits,[[None;2];3]);
    }

    #[test]
    fn drag_uses_one_original_render_and_crosses_both_anchored_groups() {
        let _guard=crate::app::tests::lock_app_tests();
        let mut app=PealayerApp::default(); app.engine_handle=crate::four_d::engine::memory_only_test_engine();
        for item in Widget::ALL { app.status_bar.set_visible(item,matches!(item,Widget::Fps|Widget::Connection|Widget::Workspace)); }
        app.status_bar.fps_mode=StatusBarFpsMode::Media;
        let context=egui::Context::default();
        crate::ui::motion::configure(&context,crate::config::UiAnimationMode::Disabled);
        let mut time=0.0;
        let mut frame=|app:&mut PealayerApp,events| { time+=0.1; let mut output=context.run_ui(egui::RawInput {
            screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(700.0,420.0))),
            time:Some(time),events,..Default::default()
        },|ui|draw(app,ui)); output.textures_delta.clear(); output };
        let rect=|item|context.data(|d|d.get_temp::<Vec<(Widget,egui::Rect)>>(egui::Id::new("test-status-widget-rects"))).unwrap()
            .into_iter().find(|(id,_)|*id==item).unwrap().1;
        let button=|pos,pressed|egui::Event::PointerButton {pos,pressed,button:egui::PointerButton::Primary,modifiers:egui::Modifiers::NONE};
        frame(&mut app,vec![]); frame(&mut app,vec![]);
        for (target,end) in [(Widget::Workspace,true),(Widget::Connection,false)] {
            let from=rect(Widget::Fps).center()-egui::vec2(7.0,0.0);
            let to=rect(target).center();
            frame(&mut app,vec![egui::Event::PointerMoved(from),button(from,true)]);
            frame(&mut app,vec![egui::Event::PointerMoved(to)]);
            let output=frame(&mut app,vec![]);
            let fps:Vec<_>=output.shapes.iter().filter_map(|s|if let egui::Shape::Text(t)=&s.shape {
                t.galley.text().ends_with("fps").then_some(t)
            }else{None}).collect();
            assert_eq!(fps.len(),1,"drag must move existing render, not draw a second copy");
            assert!(fps[0].pos.x>from.x.min(to.x)-100.0);
            frame(&mut app,vec![button(to,false)]); frame(&mut app,vec![]);
            assert_eq!(app.status_bar.at_end(Widget::Fps),end);
        }
    }

    #[test]
    fn workspace_selector_keeps_icon_name_and_chevron_in_a_fixed_slot() {
        for rtl in [false, true] {
            for dark in [false, true] {
                for scale in [1.0, 1.25, 1.5, 2.0] {
                    let context = egui::Context::default();
                    context.set_pixels_per_point(scale);
                    context.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                    crate::ui::i18n::configure_ui_fonts(&context, false);
                    for name in ["Watch films", "My very long renamed authoring workspace"] {
                        let mut slot = egui::Rect::NOTHING;
                        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                            slot = workspace_trigger(ui, name, crate::ui::icons::FILE_VIDEO, rtl, "Switch workspace").rect;
                        });
                        output.textures_delta.clear();
                        assert_eq!(slot.size(), egui::vec2(WORKSPACE_WIDTH, ITEM_HEIGHT));
                        let icon = output.shapes.iter().find_map(|shape| match &shape.shape {
                            egui::epaint::Shape::Text(text) if text.galley.job.text == crate::ui::icons::FILE_VIDEO => Some(text),
                            _ => None,
                        }).expect("Icon must be painted separately from the bold caption");
                        assert_eq!(icon.galley.job.sections[0].format.font_id.family, egui::FontFamily::Proportional);
                        let icon_rect = egui::Rect::from_min_size(icon.pos, icon.galley.size());
                        assert!(slot.contains_rect(icon_rect));
                        assert!(if rtl { icon_rect.center().x > slot.center().x }
                            else { icon_rect.center().x < slot.center().x });
                        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                            egui::epaint::Shape::Path(path) if path.points.len() == 3)), "Open chevron must be painted");
                    }
                }
            }
        }
    }

    #[test]
    fn workspace_dropdown_click_and_keyboard_return_stable_profile_ids() {
        let profiles = vec![
            ("simple-id".into(), crate::config::WorkspaceProfile { name: "Watch films".into(), icon: "video".into(), ..Default::default() }),
            ("author-id".into(), crate::config::WorkspaceProfile { name: "My authoring desk".into(), icon: "effects".into(), ..Default::default() }),
        ];
        for opening_key in [None, Some(egui::Key::ArrowDown), Some(egui::Key::Enter)] {
            let context = egui::Context::default();
            let mut time = 0.0;
            let mut render = |events| {
                let mut result = None;
                let mut output = context.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 360.0))),
                    events, time: Some(time), ..Default::default()
                }, |ui| {
                    ui.add_space(260.0);
                    result = Some(workspace_dropdown(ui, &profiles, Some("simple-id"), "Watch films",
                        crate::ui::icons::FILE_VIDEO, false, "Switch workspace", "Manage workspaces..."));
                });
                time += 0.1;
                output.textures_delta.clear();
                (result.unwrap(), output)
            };
            let ((trigger, choice), _) = render(vec![]);
            assert_eq!(choice, None);
            let popup_id = trigger.id.with("workspace-menu");
            let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary,
                pressed, modifiers: egui::Modifiers::NONE };
            let key = |key, pressed| egui::Event::Key { key, physical_key: Some(key), pressed, repeat: false,
                modifiers: egui::Modifiers::NONE };
            if let Some(opening_key) = opening_key {
                context.memory_mut(|memory| memory.request_focus(trigger.id));
                render(vec![key(opening_key, true)]);
                render(vec![key(opening_key, false)]);
            } else {
                render(vec![egui::Event::PointerMoved(trigger.rect.center()), button(trigger.rect.center(), true)]);
                render(vec![button(trigger.rect.center(), false)]);
            }
            assert!(egui::Popup::is_id_open(&context, popup_id), "Primary click/arrows must open the selector");
            let (_, output) = render(vec![]);
            let row_text = format!("{}  My authoring desk", crate::ui::icons::SPARKLE);
            let row = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == row_text =>
                    Some(egui::Rect::from_min_size(text.pos, text.galley.size())),
                _ => None,
            }).expect("Renamed workspace must be present in the open menu");
            assert!(row.bottom() < trigger.rect.top(), "Status menu must fit above its trigger");
            let choice = if opening_key.is_some() {
                render(vec![key(egui::Key::ArrowDown, true)]);
                render(vec![key(egui::Key::ArrowDown, false)]);
                render(vec![key(egui::Key::Enter, true)]).0.1
            } else {
                render(vec![egui::Event::PointerMoved(row.center()), button(row.center(), true)]);
                render(vec![button(row.center(), false)]).0.1
            };
            assert_eq!(choice, Some(WorkspaceChoice::Restore("author-id".into())));
            assert!(!egui::Popup::is_id_open(&context, popup_id));
            assert_eq!(profiles[0].1.name, "Watch films", "Menu drawing cannot mutate profiles");
        }
    }
    #[test]
    fn physical_status_led_uses_authoritative_rgb_without_double_dimming() {
        assert_eq!(
            status_led_display_color(&HardwareStatusLed {
                red: 0,
                green: 255,
                blue: 0,
                brightness: 32,
                effect: 0,
                condition: 0
            }),
            egui::Color32::GREEN
        );
    }
    #[test]
    fn status_bar_message_uses_the_same_lifetime_as_osd() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.osd_timeout_seconds = 4.0;
        app.set_osd("Remote command accepted".into());
        assert_eq!(
            current_status_message(&app).as_deref(),
            Some("Remote command accepted")
        );
    }
    #[test]
    fn chart_is_bounded_and_disconnect_does_not_become_zero() {
        let mut log = TelemetryLog::default();
        let now = Instant::now();
        for index in 0..300 {
            log.sample(now + Duration::from_secs(index), None);
        }
        assert_eq!(log.samples.len(), LOG_LIMIT);
        assert!(log.samples.iter().all(|sample| sample.values == [None; 3]));
    }
    #[test]
    fn widget_reorder_is_unique_and_visibility_is_independent() {
        let mut config = crate::config::StatusBarConfig::default();
        assert!(!config.chart);
        assert!(config.reorder(Widget::Temperature, Widget::Fps, false));
        assert_eq!(config.order[0], Widget::Temperature);
        assert_eq!(config.ordered().len(), Widget::ALL.len());
        config.set_visible(Widget::Voltage, false);
        assert!(config.visible(Widget::Current) && !config.visible(Widget::Voltage));
        config.set_visible(Widget::Connection, false);
        assert!(config.visible(Widget::Current));
    }
    #[test]
    fn fps_auto_uses_play_pause_and_unknown_rate_stays_visible() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.status_bar.fps_mode = StatusBarFpsMode::Auto;
        app.current_video_path = Some("fixture.mp4".into());
        app.media_fps = 24.0;
        app.is_paused = false;
        assert!(!app.current_fps_display(Instant::now()).unwrap().is_ui_rate);
        app.is_paused = true;
        assert!(app.current_fps_display(Instant::now()).unwrap().is_ui_rate);
        app.status_bar.fps_mode = StatusBarFpsMode::Media;
        app.media_fps = 0.0;
        assert_eq!(
            app.current_fps_display(Instant::now()).unwrap().formatted,
            "— fps"
        );
        assert_eq!(
            StatusBarFpsMode::Media.next().next().next(),
            StatusBarFpsMode::Media
        );
    }
    #[test]
    fn reactive_idle_screen_rate_uses_real_intervals_without_continuous_repaint() {
        let mut tracker = crate::app::FrameRateTracker::default();
        let start = Instant::now();
        tracker.record_frame(start);
        assert_eq!(tracker.live_fps(), None);
        tracker.record_frame(start + Duration::from_millis(500));
        assert_eq!(tracker.live_fps(), Some(2.0));
        tracker.record_frame(start + Duration::from_millis(1000));
        assert_eq!(tracker.live_fps(), Some(2.0));
    }
    #[test]
    fn paused_status_rgb_paints_each_installed_peer_frame_without_input() {
        let _guard = crate::app::tests::lock_app_tests();
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        app.is_paused = true;
        app.is_connected = true;
        app.status_bar.status_rgb = true;
        assert!(app.advertised_hardware().is_none());
        for (red, green, blue) in [(0, 128, 40), (0, 0, 0), (255, 0, 64)] {
            crate::four_d::engine::install_hardware_snapshot(
                &app.engine_handle.hardware_capabilities,
                &app.engine_handle.hardware_revision,
                Some(HardwareCapabilities {
                    board_connected: true,
                    status_led: Some(HardwareStatusLed {
                        red, green, blue, brightness: 128, effect: 1, condition: 8,
                    }),
                    ..Default::default()
                }),
            );
            let mut output = context.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 420.0))),
                ..Default::default()
            }, |ui| draw(&mut app, ui));
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::epaint::Shape::Circle(circle) if circle.radius == 6.0
                    && circle.fill == egui::Color32::from_rgb(red, green, blue)
            )), "paused RGB dot did not paint the installed peer frame");
            output.textures_delta.clear();
        }
    }

    #[test]
    fn updating_values_keep_slots_and_workspace_stays_at_the_logical_end() {
        let _guard = crate::app::tests::lock_app_tests();
        for (width, rtl) in [(1000.0, false), (420.0, false), (1000.0, true)] {
            let context = egui::Context::default();
            let mut app = PealayerApp::default();
            app.rtl = rtl;
            app.is_connected = true;
            app.status_bar = crate::config::StatusBarConfig::default();
            app.status_bar.fps_mode = StatusBarFpsMode::Media;
            app.current_video_path = Some("fixture.mp4".into());
            // Even an old saved order cannot drag the workspace into the group.
            app.status_bar
                .reorder(Widget::Workspace, Widget::Fps, false);
            let mut previous: Option<Vec<(Widget, egui::Rect)>> = None;
            let mut previous_fps_end: Option<f32> = None;
            for index in 0..5 {
                app.media_fps = if index % 2 == 0 { 9.99 } else { 144.12 };
                let mut hardware = HardwareCapabilities::default();
                hardware.board_connected = true;
                hardware.board_name = "PCController".into();
                hardware.telemetry.bus_mv = Some(if index % 2 == 0 { 999 } else { 12021 });
                hardware.telemetry.current_ma = Some(if index % 2 == 0 { 9 } else { 1001 });
                hardware.telemetry.led_temperature_centi_c =
                    Some(if index % 2 == 0 { -501 } else { 3201 });
                app.update_hardware_capabilities(Some(hardware));
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 420.0),
                        )),
                        ..Default::default()
                    },
                    |ui| draw(&mut app, ui),
                );
                let fps_end = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text) if text.galley.text().ends_with("fps") => {
                            Some(text.pos.x + text.galley.size().x)
                        }
                        _ => None,
                    })
                    .expect("numeric FPS caption");
                if let Some(previous) = previous_fps_end {
                    assert!(
                        (previous - fps_end).abs() < 1.0,
                        "FPS unit shifted with digit count"
                    );
                }
                previous_fps_end = Some(fps_end);
                let texts: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| {
                        if let egui::epaint::Shape::Text(text) = &shape.shape {
                            Some(text.galley.text())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(!texts.iter().any(|text| text.contains("Auto")
                    || text.contains("Media ·")
                    || text.contains("Screen ·")));
                output.textures_delta.clear();
                let rects = context
                    .data(|data| {
                        data.get_temp::<Vec<(Widget, egui::Rect)>>(egui::Id::new(
                            "test-status-widget-rects",
                        ))
                    })
                    .unwrap();
                if index > 1 {
                    assert_eq!(
                        previous.as_ref().unwrap(),
                        &rects,
                        "values changed geometry at {width}, RTL={rtl}"
                    );
                }
                let workspace = rects
                    .iter()
                    .find(|(widget, _)| *widget == Widget::Workspace)
                    .unwrap()
                    .1;
                assert!(
                    (if rtl {
                        workspace.left() - 8.0
                    } else {
                        workspace.right() - (width - 8.0)
                    })
                    .abs()
                        < 1.0,
                    "workspace lost its trailing anchor: {workspace:?}"
                );
                let fps = rects
                    .iter()
                    .find(|(widget, _)| *widget == Widget::Fps)
                    .unwrap()
                    .1;
                assert_eq!(fps.width(), 88.0);
                previous = Some(rects);
            }
        }
    }

    #[test]
    fn real_status_bar_click_menu_and_drag_do_not_lose_widget_ownership() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.status_bar = crate::config::StatusBarConfig::default();
        for widget in Widget::ALL {
            app.status_bar.set_visible(
                widget,
                matches!(widget, Widget::Fps | Widget::Connection | Widget::Workspace),
            );
        }
        app.status_bar.fps_mode = StatusBarFpsMode::Media;
        let context = egui::Context::default();
        let mut time = 0.0;
        let mut frame = |app: &mut PealayerApp, events: Vec<egui::Event>| {
            time += 0.05;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(700.0, 420.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| draw(app, ui),
            );
            output.textures_delta.clear();
            output
        };
        let find_text = |output: &egui::FullOutput, needle: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::epaint::Shape::Text(text) = &shape.shape {
                        text.galley
                            .text()
                            .contains(needle)
                            .then(|| egui::Rect::from_min_size(text.pos, text.galley.size()))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| panic!("missing status text {needle}"))
        };
        let button = |pos, which, pressed| egui::Event::PointerButton {
            pos,
            button: which,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        find_text(&output, "fps");
        let widget_rect = |widget| {
            context
                .data(|data| {
                    data.get_temp::<Vec<(Widget, egui::Rect)>>(egui::Id::new(
                        "test-status-widget-rects",
                    ))
                })
                .unwrap()
                .into_iter()
                .find(|(item, _)| *item == widget)
                .unwrap()
                .1
        };
        let point = widget_rect(Widget::Fps).center();
        frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(point),
                button(point, egui::PointerButton::Primary, true),
            ],
        );
        frame(
            &mut app,
            vec![button(point, egui::PointerButton::Primary, false)],
        );
        assert_eq!(app.status_bar.fps_mode, StatusBarFpsMode::Ui);
        let output = frame(&mut app, vec![]);
        find_text(&output, "fps");
        let source = widget_rect(Widget::Fps).center();
        let target = widget_rect(Widget::Connection).right_center() - egui::vec2(2.0, 0.0);
        frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(source),
                button(source, egui::PointerButton::Primary, true),
            ],
        );
        frame(
            &mut app,
            vec![egui::Event::PointerMoved(source + egui::vec2(15.0, 0.0))],
        );
        frame(&mut app, vec![egui::Event::PointerMoved(target)]);
        frame(
            &mut app,
            vec![button(target, egui::PointerButton::Primary, false)],
        );
        assert!(
            app.status_bar
                .order
                .iter()
                .position(|item| *item == Widget::Fps)
                > app
                    .status_bar
                    .order
                    .iter()
                    .position(|item| *item == Widget::Connection)
        );
        let output = frame(&mut app, vec![]);
        find_text(&output, "fps");
        let point = widget_rect(Widget::Fps).center();
        frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(point),
                button(point, egui::PointerButton::Secondary, true),
            ],
        );
        frame(
            &mut app,
            vec![button(point, egui::PointerButton::Secondary, false)],
        );
        let output = frame(&mut app, vec![]);
        let hide = find_text(&output, "Hide");
        find_text(&output, "Frame rate");
        frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(hide.center()),
                button(hide.center(), egui::PointerButton::Primary, true),
            ],
        );
        frame(
            &mut app,
            vec![button(hide.center(), egui::PointerButton::Primary, false)],
        );
        assert!(!app.status_bar.media_rate);
        assert!(
            app.status_bar.workspace,
            "item menu must not toggle the background's other widgets"
        );
    }
}
