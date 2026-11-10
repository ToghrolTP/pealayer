//! Native consumer of the same paired host API as Web. No shell/session owner.
use eframe::egui;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;
#[derive(Default)]
pub struct State {
    key: String,
    viewer: Option<Viewer>,
    snapshot: Value,
    error: Option<String>,
    parser: Option<vt100::Parser>,
    screen: super::terminal_screen::ScreenViewState,
    input: String,
}
struct Viewer {
    commands: mpsc::SyncSender<crate::terminal_access::Operation>,
    latest: Arc<Mutex<Option<Result<Value, String>>>>,
    cancelled: Arc<AtomicBool>,
}
impl Drop for Viewer {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
fn response(response: reqwest::blocking::Response) -> Result<Value, String> {
    use std::io::Read;
    let status = response.status();
    let mut bytes = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Terminal response failed".to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Terminal snapshot exceeded its limit".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid terminal snapshot".to_string())?;
    if !status.is_success() {
        return Err(value["error"]
            .as_str()
            .unwrap_or("Terminal request was denied")
            .into());
    }
    Ok(value)
}
impl Viewer {
    fn connect(origin: url::Url, key: String, ctx: egui::Context) -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(18))
            .build()
            .map_err(|e| e.to_string())?;
        let latest = Arc::new(Mutex::new(None));
        let cancelled = Arc::new(AtomicBool::new(false));
        let owner = Arc::new(Mutex::new(String::new()));
        let (tx, rx) = mpsc::sync_channel::<crate::terminal_access::Operation>(64);
        let slot = latest.clone();
        let cancel = cancelled.clone();
        let observed_owner = owner.clone();
        let events = origin
            .join("/api/terminal/events")
            .map_err(|e| e.to_string())?;
        let controls = origin
            .join("/api/terminal/control")
            .map_err(|e| e.to_string())?;
        let client = http.clone();
        let token = key.clone();
        let repaint = ctx.clone();
        std::thread::Builder::new()
            .name("terminal-host-events".into())
            .spawn(move || {
                let mut revision = 0u64;
                while !cancel.load(Ordering::Acquire) {
                    let result = client
                        .get(events.clone())
                        .query(&[("after", revision)])
                        .bearer_auth(&token)
                        .send()
                        .map_err(|_| "Terminal host is unavailable".to_string())
                        .and_then(response)
                        .and_then(|value| {
                            revision = value["revision"]
                                .as_u64()
                                .ok_or("Invalid terminal revision")?;
                            let state = value["state"].clone();
                            if state.is_null() {
                                return Ok(state);
                            }
                            let id = state["owner_id"]
                                .as_str()
                                .ok_or("Terminal owner identity missing")?;
                            let mut owner =
                                observed_owner.lock().unwrap_or_else(|e| e.into_inner());
                            if !owner.is_empty() && owner.as_str() != id {
                                return Err("Terminal owner changed; pair again".into());
                            }
                            *owner = id.into();
                            Ok(state)
                        });
                    if cancel.load(Ordering::Acquire) {
                        break;
                    }
                    let failed = result.is_err();
                    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
                    repaint.request_repaint();
                    if failed {
                        cancel.store(true, Ordering::Release);
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let slot = latest.clone();
        let cancel = cancelled.clone();
        std::thread::Builder::new()
            .name("terminal-host-input".into())
            .spawn(move || {
                while !cancel.load(Ordering::Acquire) {
                    let operation = match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(op) => op,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    if cancel.load(Ordering::Acquire) {
                        break;
                    }
                    // Serialized input; no retry/replay after a lost acknowledgment.
                    let result = http
                        .post(controls.clone())
                        .bearer_auth(&key)
                        .json(&operation)
                        .send()
                        .map_err(|_| {
                            "Terminal command acknowledgment was lost; not replayed".to_string()
                        })
                        .and_then(response);
                    if let Err(error) = result {
                        *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(Err(error));
                        ctx.request_repaint();
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            commands: tx,
            latest,
            cancelled,
        })
    }
}
fn request(state: &mut State, operation: crate::terminal_access::Operation) {
    if let Some(viewer) = &state.viewer
        && viewer.commands.try_send(operation).is_err()
    {
        state.error = Some("Terminal input queue is full; nothing was sent".into());
    }
}
fn install(state: &mut State, snapshot: Value) -> Result<(), String> {
    if snapshot.is_null() {
        return Ok(());
    }
    let mut bytes = Vec::new();
    let screen = &snapshot["screen"];
    if !screen.is_null() {
        let id = screen["id"].as_u64().ok_or("Missing terminal session ID")?;
        let session = snapshot["sessions"]
            .as_array()
            .and_then(|sessions| sessions.iter().find(|s| s["id"] == id))
            .ok_or("Missing terminal session")?;
        let rows = session["rows"]
            .as_u64()
            .filter(|n| (1..=80).contains(n))
            .ok_or("Invalid terminal rows")? as u16;
        let cols = session["cols"]
            .as_u64()
            .filter(|n| (1..=240).contains(n))
            .ok_or("Invalid terminal columns")? as u16;
        let lines = screen["lines"]
            .as_array()
            .filter(|l| l.len() == usize::from(rows))
            .ok_or("Invalid terminal grid")?;
        bytes.extend_from_slice(b"\x1b[2J\x1b[H");
        if screen["alternate"] == true {
            bytes.extend_from_slice(b"\x1b[?1049h");
        }
        if screen["mouse"] == true {
            bytes.extend_from_slice(b"\x1b[?1002h\x1b[?1006h");
        }
        for (row, runs) in lines.iter().enumerate() {
            bytes.extend_from_slice(format!("\x1b[{};1H", row + 1).as_bytes());
            for run in runs.as_array().ok_or("Invalid terminal row")? {
                bytes.extend_from_slice(b"\x1b[0m");
                for (name, code) in [("fg", 38), ("bg", 48)] {
                    if let Some(color) = run[name].as_str() {
                        let color = u32::from_str_radix(
                            color.strip_prefix('#').ok_or("Invalid terminal color")?,
                            16,
                        )
                        .map_err(|_| "Invalid terminal color")?;
                        bytes.extend_from_slice(
                            format!(
                                "\x1b[{code};2;{};{};{}m",
                                (color >> 16) & 255,
                                (color >> 8) & 255,
                                color & 255
                            )
                            .as_bytes(),
                        );
                    }
                }
                for (name, code) in [("bold", 1), ("inverse", 7), ("underline", 4)] {
                    if run[name] == true {
                        bytes.extend_from_slice(format!("\x1b[{code}m").as_bytes());
                    }
                }
                let text = run["text"].as_str().ok_or("Invalid terminal text")?;
                if text.chars().any(|c| c.is_control()) {
                    return Err("Terminal grid contained control characters".into());
                }
                bytes.extend_from_slice(text.as_bytes());
            }
        }
        let cursor = screen["cursor"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or("Missing terminal cursor")?;
        let row = cursor[0]
            .as_u64()
            .filter(|n| *n < u64::from(rows))
            .ok_or("Invalid terminal cursor")?;
        let col = cursor[1]
            .as_u64()
            .filter(|n| *n < u64::from(cols))
            .ok_or("Invalid terminal cursor")?;
        bytes.extend_from_slice(
            format!(
                "\x1b[{};{}H\x1b[?25{}\x1b[?2004{}",
                row + 1,
                col + 1,
                if screen["hide_cursor"] == true {
                    "l"
                } else {
                    "h"
                },
                if screen["bracketed_paste"] == true {
                    "h"
                } else {
                    "l"
                }
            )
            .as_bytes(),
        );
        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(&bytes);
        state.parser = Some(parser);
    } else {
        state.parser = None;
    }
    state.snapshot = snapshot;
    Ok(())
}
pub fn draw(
    ui: &mut egui::Ui,
    state: &mut State,
    origin: url::Url,
    preferences: &super::terminal::TerminalPreferences,
) -> bool {
    let mut open_preferences = false;
    use crate::terminal_access::Operation;
    if let Some(viewer) = &state.viewer {
        let value = viewer
            .latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(value) = value {
            match value.and_then(|value| install(state, value)) {
                Ok(()) => state.error = None,
                Err(error) => state.error = Some(error),
            }
        }
    }
    if state.viewer.is_none() {
        ui.heading(format!("{}  Host terminal", super::icons::TERMINAL_WINDOW));
        ui.label("Pair using an access key from the authority's terminal menu. This opens its existing sessions, not a shell on this client.");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.key)
                    .password(true)
                    .hint_text("Access key"),
            );
            if ui
                .add_enabled(state.key.len() == 64, egui::Button::new("Connect to host"))
                .clicked()
            {
                match Viewer::connect(origin, std::mem::take(&mut state.key), ui.ctx().clone()) {
                    Ok(viewer) => state.viewer = Some(viewer),
                    Err(error) => state.error = Some(error),
                }
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            if let Some(sessions) = state.snapshot["sessions"].as_array().cloned() {
                for session in sessions {
                    if let Some(id) = session["id"].as_u64() {
                        let selected = state.snapshot["screen"]["id"] == id;
                        if let Ok(target) = serde_json::from_value(session["target"].clone()) {
                            let (select, close) = super::terminal::session_tab(
                                ui,
                                id,
                                target,
                                selected,
                                session["phase"].as_str().unwrap_or("disconnected"),
                                1.0,
                            );
                            if select {
                                request(state, Operation::Select { id });
                            }
                            if close {
                                request(state, Operation::Close { id });
                            }
                        }
                    }
                }
            }
            if ui
                .button(format!("{}  Session", super::icons::PLUS))
                .clicked()
            {
                request(
                    state,
                    Operation::Create {
                        target: super::terminal::TerminalTarget::PortableShell,
                    },
                );
            }
            if ui.button("Disconnect viewer").clicked() {
                state.viewer = None;
                state.snapshot = Value::Null;
                state.parser = None;
            }
        });
        ui.separator();
        let id = state.snapshot["screen"]["id"].as_u64();
        let line_mode = state.snapshot["sessions"]
            .as_array()
            .and_then(|sessions| sessions.iter().find(|s| s["id"].as_u64() == id))
            .is_some_and(|s| s["target"] == "pc_controller");
        if let Some(parser) = &mut state.parser {
            let mut style = preferences.clone();
            if let Ok(value) = serde_json::from_value(state.snapshot["cursor_style"].clone()) {
                style.cursor_style = value;
            }
            style.cursor_blink = state.snapshot["cursor_blink"] == true;
            style.accent_commands = state.snapshot["accent_commands"] == true;
            style.font_size = state.snapshot["font_size"]
                .as_f64()
                .filter(|v| v.is_finite() && (9.0..=28.0).contains(v))
                .unwrap_or(13.0) as f32;
            let output = super::terminal_screen::show_shared(
                ui,
                parser,
                &mut state.screen,
                egui::Id::unique("remote-host-terminal"),
                !line_mode,
                &style,
            );
            open_preferences = output.preferences;
            if output.clear
                && let Some(id) = id
            {
                request(state, Operation::Clear { id });
            }
            if let Some(id) = id {
                for bytes in output.input {
                    if let Ok(data) = String::from_utf8(bytes) {
                        request(state, Operation::Input { id, data });
                    }
                }
            }
        }
        if line_mode {
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.input).hint_text("PCController command"),
            );
            if response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter))
                && let Some(id) = id
            {
                let data = std::mem::take(&mut state.input);
                request(state, Operation::Input { id, data });
            }
        }
    }
    if let Some(error) = &state.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    open_preferences
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn remote_grid_never_executes_escaped_text() {
        let mut state = State::default();
        assert!(install(&mut state,json!({"screen":{"id":1,"lines":[[{"text":"\u{1b}[2J"}]]},"sessions":[{"id":1,"rows":1,"cols":20}]})).is_err());
        assert!(state.parser.is_none());
    }
}
