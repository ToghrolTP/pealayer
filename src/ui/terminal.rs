//! Optional native terminal workspace. Visibility never opens a connection.
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalTarget {
    #[default]
    PcController,
    SystemShell,
    PortableShell,
}
impl TerminalTarget {
    fn label(self) -> &'static str {
        match self {
            Self::PcController => "PCController",
            Self::SystemShell => "System shell",
            Self::PortableShell => "PortableShell",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalPreferences {
    pub enabled: bool,
    pub target: TerminalTarget,
    pub controller_endpoint: String,
    pub shell_program: String,
    pub shell_arguments: String,
    pub profile_id: String,
    pub font_size: f32,
    pub scrollback_lines: usize,
}
impl Default for TerminalPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            target: TerminalTarget::PcController,
            controller_endpoint: String::new(),
            shell_program: String::new(),
            shell_arguments: String::new(),
            profile_id: String::new(),
            font_size: 13.0,
            scrollback_lines: 2000,
        }
    }
}
impl TerminalPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.font_size.is_finite()
            || !(9.0..=28.0).contains(&self.font_size)
            || !(100..=10000).contains(&self.scrollback_lines)
        {
            return Err(
                "Terminal dimensions, font size or scrollback are outside their supported range"
                    .into(),
            );
        }
        for value in [
            &self.controller_endpoint,
            &self.shell_program,
            &self.shell_arguments,
            &self.profile_id,
        ] {
            if value.len() > 8192 || value.contains(['\0', '\r', '\n']) {
                return Err(
                    "Terminal settings must be single-line values of at most 8192 bytes".into(),
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct TerminalState {
    pub preferences: TerminalPreferences,
    confirm_exit: bool,
    closing: bool,
    quit_after_close: bool,
    last_visibility_intent: Option<bool>,
    #[cfg(feature = "workspace-terminal")]
    retired: Vec<super::terminal_backend::Session>,
    #[cfg(feature = "workspace-terminal")]
    views: Vec<View>,
    #[cfg(feature = "workspace-terminal")]
    active: usize,
    #[cfg(feature = "workspace-terminal")]
    next_id: u64,
    #[cfg(feature = "workspace-terminal")]
    profiles: Vec<super::terminal_profiles::Profile>,
    #[cfg(feature = "workspace-terminal")]
    profile_scan:
        Option<std::sync::mpsc::Receiver<Result<super::terminal_profiles::Discovery, String>>>,
    #[cfg(feature = "workspace-terminal")]
    profile_warnings: Vec<String>,
}

/// A normal application close must not silently terminate a live shell.
pub fn intercept_close(app: &mut crate::app::PealayerApp) -> bool {
    if app.terminal_state.busy() {
        app.terminal_state.confirm_exit = true;
        true
    } else {
        false
    }
}

pub fn draw_exit_confirmation(app: &mut crate::app::PealayerApp, ctx: &egui::Context) {
    if !app.terminal_state.confirm_exit {
        return;
    }
    let mut close = false;
    let mut cancel = false;
    let closing = app.terminal_state.closing;
    let quit_after_close = app.terminal_state.quit_after_close;
    let dialog = super::dialog::confirmation_modal(ctx, egui::Id::unique("terminal-confirm-exit"),
        &app.tr("Close active terminal sessions?"), super::icons::TERMINAL_WINDOW,
        &app.tr("Closing Pealayer ends its terminal sessions and local shell processes. A controller command already sent may still finish."),
        &app.tr("Keep Pealayer open"), |ui| {
            if closing { ui.label(app.tr("Closing terminal sessions…")); }
            if !quit_after_close && ui.button(format!("{}  {}",super::icons::X,app.tr("Close sessions and quit"))).clicked() { close=true; }
            if ui.button(app.tr("Keep Pealayer open")).clicked() { cancel=true; }
            close || cancel
        });
    if cancel || dialog.dismissed {
        app.terminal_state.cancel_exit();
    } else if close {
        app.terminal_state.begin_exit();
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

impl TerminalState {
    fn cancel_exit(&mut self) {
        self.confirm_exit = false;
        self.quit_after_close = false;
        // Cancellation cannot undo a command or child close already requested.
        // Keep tracking that cleanup even after dismissing the quit dialog.
    }
    fn begin_exit(&mut self) {
        self.closing = true;
        self.quit_after_close = true;
        #[cfg(feature = "workspace-terminal")]
        {
            for view in &mut self.views {
                view.begin_close();
            }
            for session in &self.retired {
                session.request_close();
            }
        }
    }
    #[cfg(feature = "workspace-terminal")]
    fn collect_closed(&mut self) -> bool {
        for view in &mut self.views {
            if view.phase == "closing"
                && view
                    .session
                    .as_ref()
                    .is_some_and(|session| session.is_closed())
            {
                view.session = None;
                view.phase = "disconnected";
            }
        }
        self.retired.retain(|session| !session.is_closed());
        if self.closing && !self.busy() {
            self.closing = false;
            self.confirm_exit = false;
            return std::mem::take(&mut self.quit_after_close);
        }
        false
    }
    #[cfg(feature = "workspace-terminal")]
    fn cleanup_pending(&self) -> bool {
        self.closing
            || !self.retired.is_empty()
            || self
                .views
                .iter()
                .any(|view| view.phase == "closing" && view.session.is_some())
    }
    #[cfg(feature = "workspace-terminal")]
    fn retire_view(&mut self, index: usize) {
        let mut view = self.views.remove(index);
        if let Some(session) = view.session.take() {
            session.request_close();
            self.retired.push(session);
        }
        self.active = self.active.min(self.views.len().saturating_sub(1));
    }
    pub fn with_preferences(preferences: TerminalPreferences) -> Self {
        Self {
            preferences,
            ..Default::default()
        }
    }
    pub fn busy(&self) -> bool {
        #[cfg(feature = "workspace-terminal")]
        {
            return !self.retired.is_empty()
                || self.views.iter().any(|view| view.session.is_some());
        }
        #[cfg(not(feature = "workspace-terminal"))]
        {
            false
        }
    }
    pub fn status(&self) -> serde_json::Value {
        #[cfg(feature = "workspace-terminal")]
        let sessions: Vec<_> = self
            .views
            .iter()
            .map(|view| {
                serde_json::json!({
                    "id":view.id,"target":view.target,"phase":view.phase,
                    "rows":view.size.0,"cols":view.size.1,"connected":view.session.is_some(),
                })
            })
            .collect();
        #[cfg(not(feature = "workspace-terminal"))]
        let sessions: Vec<serde_json::Value> = Vec::new();
        #[cfg(feature = "workspace-terminal")]
        let profiles = serde_json::json!({"count":self.profiles.len(),"refreshing":self.profile_scan.is_some(),"warnings":self.profile_warnings.len()});
        #[cfg(not(feature = "workspace-terminal"))]
        let profiles = serde_json::json!({"count":0,"refreshing":false,"warnings":0});
        serde_json::json!({"available":cfg!(feature="workspace-terminal"),
            "visible":cfg!(feature="workspace-terminal") && self.preferences.enabled,"sessions":sessions,"profiles":profiles})
    }
}

pub fn toggle(app: &mut crate::app::PealayerApp) {
    if cfg!(feature = "workspace-terminal") && !app.dock_state.locked {
        app.terminal_state.preferences.enabled = !app.terminal_state.preferences.enabled;
        app.deferred_config_save = true;
    }
}

pub fn owns_keyboard(ctx: &egui::Context) -> bool {
    ctx.data(|data| {
        data.get_temp::<bool>(egui::Id::unique("terminal-keyboard-owner"))
            .unwrap_or(false)
    })
}

/// Native local IPC uses the same explicit session actions as the UI. No
/// terminal content/history is published in the shared status snapshot.
pub fn control(
    app: &mut crate::app::PealayerApp,
    ctx: &egui::Context,
    operation: crate::workspace_tools::TerminalOperation,
) -> Result<(), String> {
    #[cfg(feature = "workspace-terminal")]
    {
        use crate::workspace_tools::TerminalOperation;
        let selected = if crate::peer::active() {
            String::new()
        } else {
            app.serial_port.clone()
        };
        let state = &mut app.terminal_state;
        if matches!(operation, TerminalOperation::RefreshProfiles) {
            if state.profile_scan.is_some() {
                return Err("A profile refresh is already running".into());
            }
            state.profile_scan = Some(start_profile_scan(ctx)?);
            return Ok(());
        }
        if state.closing && matches!(operation, TerminalOperation::Connect) {
            return Err("Wait until closing terminal sessions have finished".into());
        }
        if state.views.is_empty() && matches!(operation, TerminalOperation::Connect) {
            state.next_id += 1;
            state
                .views
                .push(View::new(state.next_id, &state.preferences));
            state.active = 0;
        }
        let view = state
            .views
            .get_mut(state.active)
            .ok_or("No terminal session is selected")?;
        match operation {
            TerminalOperation::RefreshProfiles => {
                unreachable!("handled before selecting a session")
            }
            TerminalOperation::Connect => {
                if view.session.is_some() {
                    return Err("This terminal is already connecting or connected".into());
                }
                view.target = state.preferences.target;
                let profile = state
                    .profiles
                    .iter()
                    .find(|profile| profile.stable_id == state.preferences.profile_id);
                connect(view, &state.preferences, profile, &selected, ctx);
                if let Some(error) = &view.error {
                    return Err(error.clone());
                }
            }
            TerminalOperation::Disconnect => {
                view.begin_close();
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            TerminalOperation::Clear => {
                view.clear(state.preferences.scrollback_lines);
            }
            TerminalOperation::Write { data } => {
                if view.phase != "ready" {
                    return Err("Wait until the terminal session is ready".into());
                }
                let input = if view.target == TerminalTarget::PcController {
                    super::terminal_backend::Input::Command(data)
                } else {
                    super::terminal_backend::Input::Bytes(data.into_bytes())
                };
                view.session
                    .as_ref()
                    .ok_or("Terminal session has ended")?
                    .send(input)?;
            }
            TerminalOperation::Resize { rows, cols } => {
                if view.target == TerminalTarget::PcController {
                    return Err("PCController host commands do not have a PTY size".into());
                }
                view.session
                    .as_ref()
                    .ok_or("Terminal is not connected")?
                    .send(super::terminal_backend::Input::Resize { rows, cols })?;
                view.size = (rows, cols);
                view.parser.set_size(rows, cols);
            }
        }
        Ok(())
    }
    #[cfg(not(feature = "workspace-terminal"))]
    {
        let _ = (app, ctx, operation);
        Err("Terminal is not included in this build".into())
    }
}

#[cfg(feature = "workspace-terminal")]
struct View {
    id: u64,
    target: TerminalTarget,
    parser: vt100::Parser,
    screen: super::terminal_screen::ScreenViewState,
    session: Option<super::terminal_backend::Session>,
    phase: &'static str,
    error: Option<String>,
    label: String,
    input: String,
    history: Vec<String>,
    history_cursor: usize,
    history_draft: String,
    catalog: Vec<serde_json::Value>,
    size: (u16, u16),
    queries: QueryState,
    actions: Vec<super::portable_shell_host::Action>,
}

#[cfg(feature = "workspace-terminal")]
impl View {
    fn clear(&mut self, scrollback_lines: usize) {
        // Keep the embedded editor's cursor/prompt in agreement with a cleared
        // view. Ctrl-L redraws without submitting or discarding the draft.
        if self.target == TerminalTarget::PortableShell && self.phase == "ready" {
            self.send(super::terminal_backend::Input::Bytes(vec![12]));
        }
        self.parser = vt100::Parser::new(self.size.0, self.size.1, scrollback_lines);
        self.screen = Default::default();
        self.queries = QueryState::default();
    }
    fn begin_close(&mut self) {
        self.actions.clear();
        if let Some(session) = &self.session {
            session.request_close();
            self.phase = "closing";
        }
    }
    fn new(id: u64, preferences: &TerminalPreferences) -> Self {
        Self {
            id,
            target: preferences.target,
            parser: vt100::Parser::new(24, 80, preferences.scrollback_lines.clamp(100, 10000)),
            screen: Default::default(),
            session: None,
            phase: "disconnected",
            error: None,
            label: String::new(),
            input: String::new(),
            history: Vec::new(),
            history_cursor: 0,
            history_draft: String::new(),
            catalog: Vec::new(),
            size: (24, 80),
            queries: QueryState::default(),
            actions: Vec::new(),
        }
    }
    fn send(&mut self, input: super::terminal_backend::Input) -> bool {
        let result = self
            .session
            .as_ref()
            .ok_or_else(|| "This terminal session is not connected".to_string())
            .and_then(|session| session.send(input));
        match result {
            Ok(()) => true,
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }
    fn output(&mut self, bytes: &[u8]) {
        if self.target != TerminalTarget::SystemShell {
            self.parser.process(bytes);
            return;
        }
        // Only an explicitly opened PTY receives terminal replies. Recognize
        // real CSI queries, excluding OSC/DCS strings, and query the cursor at
        // that byte boundary rather than after the rest of the output chunk.
        let mut consumed = 0;
        let mut replies = Vec::new();
        for (index, byte) in bytes.iter().copied().enumerate() {
            if let Some(query) = self.queries.push(byte) {
                self.parser.process(&bytes[consumed..=index]);
                consumed = index + 1;
                match query {
                    StatusQuery::Status => replies.extend_from_slice(b"\x1b[0n"),
                    StatusQuery::Cursor => {
                        let (row, col) = self.parser.screen().cursor_position();
                        replies
                            .extend_from_slice(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes());
                    }
                }
            }
        }
        self.parser.process(&bytes[consumed..]);
        if self.phase == "closing" {
            return;
        }
        for reply in replies.chunks(32 * 1024) {
            if !self.send(super::terminal_backend::Input::Bytes(reply.to_vec())) {
                break;
            }
        }
    }
    fn tick(&mut self) -> bool {
        use super::terminal_backend::Event;
        for _ in 0..32 {
            let Some(event) = self.session.as_ref().and_then(|session| session.try_recv()) else {
                return false;
            };
            match event {
                Event::Ready { label } => {
                    self.label = label;
                    if self.phase != "closing" {
                        self.phase = "ready";
                    }
                    self.error = None;
                }
                Event::Catalog(catalog) => self.catalog = catalog,
                Event::Output(bytes) => self.output(&bytes),
                Event::Error(error) => self.error = Some(error),
                Event::Application(action) => {
                    if self.phase != "closing" {
                        self.actions.push(action);
                    }
                }
                Event::Exited { code } => {
                    // Exited is an ordered stream marker, not proof that the
                    // control/reader/child guard have all released ownership.
                    self.phase = "closing";
                    self.label = code
                        .map(|code| format!("Exit {code}"))
                        .unwrap_or_else(|| "Session closed".into());
                    return false;
                }
            }
        }
        // The bounded UI drain may leave output even after the producer has
        // stopped. Arrange one follow-up frame without installing an idle timer.
        self.phase != "closing"
    }
    fn submit(&mut self) {
        let line = self.input.trim().to_string();
        if line.is_empty() {
            return;
        }
        if !self.send(super::terminal_backend::Input::Command(line.clone())) {
            return;
        }
        self.error = None;
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.history_cursor = self.history.len();
        self.history_draft.clear();
        // Display explicit user input locally. History is never persisted.
        self.parser
            .process(format!("\r\n\x1b[36mpc›\x1b[0m {line}\r\n").as_bytes());
        self.input.clear();
    }

    fn history_step(&mut self, previous: bool) {
        if previous && self.history_cursor > 0 {
            if self.history_cursor == self.history.len() {
                self.history_draft = self.input.clone();
            }
            self.history_cursor -= 1;
            self.input = self.history[self.history_cursor].clone();
        } else if !previous && self.history_cursor < self.history.len() {
            self.history_cursor += 1;
            self.input = self
                .history
                .get(self.history_cursor)
                .cloned()
                .unwrap_or_else(|| self.history_draft.clone());
        }
    }

    fn complete_command(&mut self) {
        if self.input.contains(char::is_whitespace) {
            return;
        }
        let prefix = self.input.to_lowercase();
        let mut matches = std::collections::BTreeSet::new();
        for row in &self.catalog {
            let names = row
                .get("name")
                .and_then(serde_json::Value::as_str)
                .into_iter()
                .chain(
                    row.get("aliases")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str),
                );
            for name in names {
                if name.to_lowercase().starts_with(&prefix) {
                    matches.insert(name);
                }
            }
        }
        if matches.len() == 1 {
            self.input = format!("{} ", matches.first().unwrap());
        }
    }
}

#[cfg(feature = "workspace-terminal")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusQuery {
    Status,
    Cursor,
}

#[cfg(feature = "workspace-terminal")]
#[derive(Default)]
enum QueryState {
    #[default]
    Ground,
    Escape,
    Csi(Vec<u8>),
    IgnoreCsi,
    String {
        escaped: bool,
        bell_terminated: bool,
    },
}

#[cfg(feature = "workspace-terminal")]
impl QueryState {
    fn push(&mut self, byte: u8) -> Option<StatusQuery> {
        let mut query = None;
        *self = match std::mem::take(self) {
            Self::Ground => {
                if byte == 0x1b {
                    Self::Escape
                } else {
                    Self::Ground
                }
            }
            Self::Escape => match byte {
                b'[' => Self::Csi(Vec::new()),
                b']' => Self::String {
                    escaped: false,
                    bell_terminated: true,
                },
                b'P' | b'X' | b'^' | b'_' => Self::String {
                    escaped: false,
                    bell_terminated: false,
                },
                0x1b => Self::Escape,
                _ => Self::Ground,
            },
            Self::Csi(mut parameters) => {
                if byte == 0x1b {
                    Self::Escape
                } else if (0x40..=0x7e).contains(&byte) {
                    if byte == b'n' {
                        query = match parameters.as_slice() {
                            b"5" => Some(StatusQuery::Status),
                            b"6" => Some(StatusQuery::Cursor),
                            _ => None,
                        };
                    }
                    Self::Ground
                } else if parameters.len() < 16 {
                    parameters.push(byte);
                    Self::Csi(parameters)
                } else {
                    Self::IgnoreCsi
                }
            }
            Self::IgnoreCsi => {
                if byte == 0x1b {
                    Self::Escape
                } else if (0x40..=0x7e).contains(&byte) {
                    Self::Ground
                } else {
                    Self::IgnoreCsi
                }
            }
            Self::String {
                escaped,
                bell_terminated,
            } => {
                if (bell_terminated && byte == 7) || (escaped && byte == b'\\') {
                    Self::Ground
                } else {
                    Self::String {
                        escaped: byte == 0x1b,
                        bell_terminated,
                    }
                }
            }
        };
        query
    }
}

/// Parse literal argv, with single/double quotes grouping whitespace. Inside a
/// quoted argument a doubled matching quote is literal. Backslashes are always
/// literal, preserving UNC paths and quoted Windows paths ending in a slash.
/// This syntax performs no shell expansion, escaping or evaluation.
pub fn arguments(value: &str) -> Result<Vec<String>, String> {
    if value.len() > 8192 {
        return Err("Arguments must be at most 8192 bytes".into());
    }
    let mut args = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\0' || ch == '\r' || ch == '\n' {
            return Err("Arguments must be a single line".into());
        }
        if let Some(expected) = quote {
            if ch == expected && chars.peek() == Some(&expected) {
                word.push(ch);
                chars.next();
            } else if ch == expected {
                quote = None;
            } else {
                word.push(ch);
            }
            started = true;
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
            started = true;
        } else if ch.is_whitespace() {
            if started {
                args.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(ch);
            started = true;
        }
    }
    if quote.is_some() {
        return Err("An argument has an unclosed quote".into());
    }
    if started {
        args.push(word);
    }
    if args.len() > 128 {
        return Err("Use at most 128 shell arguments".into());
    }
    Ok(args)
}

#[cfg(feature = "workspace-terminal")]
fn connect(
    view: &mut View,
    preferences: &TerminalPreferences,
    profile: Option<&super::terminal_profiles::Profile>,
    selected: &str,
    ctx: &egui::Context,
) {
    use super::terminal_backend::{Session, Target};
    if view.session.is_some() {
        return;
    }
    let target = match view.target {
        TerminalTarget::PcController => {
            let endpoint = if preferences.controller_endpoint.trim().is_empty() {
                selected
            } else {
                preferences.controller_endpoint.trim()
            };
            if !endpoint.starts_with("pccontroller://") && !endpoint.starts_with("tcp://") {
                view.error =
                    Some("Choose a PCController host endpoint in Terminal preferences".into());
                return;
            }
            Target::Controller {
                endpoint: endpoint.to_string(),
            }
        }
        TerminalTarget::PortableShell => Target::Embedded,
        TerminalTarget::SystemShell if !preferences.profile_id.is_empty() => {
            let Some(profile) = profile else {
                view.error = Some(
                    "Refresh terminal profiles before connecting to the selected profile".into(),
                );
                return;
            };
            if let Some(reason) = profile.blocked_reason() {
                view.error = Some(reason.into());
                return;
            }
            Target::Shell {
                program: profile.program.clone().unwrap(),
                args: profile.args.clone(),
                cwd: if profile.inherit_cwd {
                    None
                } else {
                    profile.cwd.clone()
                },
                environment: profile
                    .env
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
                clear_environment: profile.clear_env,
            }
        }
        TerminalTarget::SystemShell => {
            let (program, args) = {
                let default = if cfg!(windows) {
                    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
                } else {
                    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
                };
                (
                    if preferences.shell_program.trim().is_empty() {
                        default
                    } else {
                        preferences.shell_program.clone()
                    },
                    &preferences.shell_arguments,
                )
            };
            let args = match arguments(args) {
                Ok(args) => args,
                Err(error) => {
                    view.error = Some(error);
                    return;
                }
            };
            Target::Shell {
                program,
                args,
                cwd: None,
                environment: Vec::new(),
                clear_environment: false,
            }
        }
    };
    match Session::spawn(target, view.size.0, view.size.1, ctx.clone()) {
        Ok(session) => {
            // A new endpoint/process owns a new VT state. Reusing modes from a
            // previous TUI (alternate screen, application keys, paste) corrupts
            // the next session even when its text happens to look unchanged.
            view.parser =
                vt100::Parser::new(view.size.0, view.size.1, preferences.scrollback_lines);
            view.screen = Default::default();
            view.session = Some(session);
            view.phase = "opening";
            view.error = None;
            view.catalog.clear();
            view.queries = QueryState::default();
        }
        Err(error) => view.error = Some(error),
    }
}

#[cfg(feature = "workspace-terminal")]
fn panel_contents(
    app: &mut crate::app::PealayerApp,
    ui: &mut egui::Ui,
    state: &mut TerminalState,
    open_preferences: &mut bool,
) {
    if state.views.is_empty() {
        state.next_id += 1;
        state
            .views
            .push(View::new(state.next_id, &state.preferences));
    }
    let mut add = false;
    let mut remove = None;
    let mut focus_input = false;
    ui.horizontal(|ui| {
        for (index, view) in state.views.iter().enumerate() {
            if ui
                .add(
                    egui::Button::new(format!(
                        "{}  {}",
                        super::icons::TERMINAL_WINDOW,
                        view.target.label()
                    ))
                    .selected(state.active == index)
                    .frame(false),
                )
                .on_hover_text(format!("{} · {}", app.tr("Session"), view.id))
                .clicked()
            {
                state.active = index;
            }
        }
        if ui
            .add_enabled(
                state.views.len() < 4,
                egui::Button::new(super::icons::PLUS).frame(false),
            )
            .on_hover_text(app.tr("New session"))
            .clicked()
        {
            add = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button(super::icons::DOTS_THREE, |ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        super::icons::GEAR,
                        app.tr("Terminal preferences")
                    ))
                    .clicked()
                {
                    *open_preferences = true;
                    ui.close();
                }
                if ui.button(app.tr("Close session")).clicked() {
                    remove = Some(state.active);
                    ui.close();
                }
            });
        });
    });
    if add {
        state.next_id += 1;
        state
            .views
            .push(View::new(state.next_id, &state.preferences));
        state.active = state.views.len() - 1;
    }
    if let Some(index) = remove {
        state.retire_view(index);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
    let Some(view) = state.views.get_mut(state.active) else {
        return;
    };
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let before = view.target;
        ui.add_enabled_ui(view.session.is_none(), |ui| {
            super::dropdown::ComboBox::from_id_salt(("terminal-target", view.id))
                .selected_text(view.target.label())
                .width(145.0)
                .show_ui(ui, |ui| {
                    for target in [
                        TerminalTarget::PcController,
                        TerminalTarget::SystemShell,
                        TerminalTarget::PortableShell,
                    ] {
                        ui.selectable_value(&mut view.target, target, target.label());
                    }
                });
        });
        if before != view.target {
            state.preferences.target = view.target;
            app.deferred_config_save = true;
        }
        if view.target == TerminalTarget::SystemShell {
            ui.add_enabled_ui(view.session.is_none(), |ui| {
                let selected = state
                    .profiles
                    .iter()
                    .find(|profile| profile.stable_id == state.preferences.profile_id)
                    .map(|profile| profile.name.as_str())
                    .unwrap_or(if state.preferences.profile_id.is_empty() {
                        "Custom / system default"
                    } else {
                        "Refresh to load saved profile"
                    });
                let old = state.preferences.profile_id.clone();
                super::dropdown::ComboBox::from_id_salt(("terminal-profile", view.id))
                    .selected_text(selected)
                    .width(190.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.preferences.profile_id,
                            String::new(),
                            app.tr("Custom / system default"),
                        );
                        for profile in state.profiles.iter().filter(|profile| !profile.hidden) {
                            ui.add_enabled_ui(profile.ready(), |ui| {
                                ui.selectable_value(
                                    &mut state.preferences.profile_id,
                                    profile.stable_id.clone(),
                                    &profile.name,
                                )
                                .on_hover_text(
                                    profile.blocked_reason().unwrap_or(&profile.provenance),
                                );
                            });
                        }
                    });
                if state.preferences.profile_id != old {
                    app.deferred_config_save = true;
                }
            });
            if ui
                .add_enabled(
                    state.profile_scan.is_none(),
                    egui::Button::new(super::icons::ARROW_CLOCKWISE).frame(false),
                )
                .on_hover_text(
                    app.tr("Refresh terminal profiles from Windows Terminal and VS Code settings"),
                )
                .clicked()
            {
                match start_profile_scan(ui.ctx()) {
                    Ok(receiver) => state.profile_scan = Some(receiver),
                    Err(error) => state.profile_warnings = vec![error],
                }
            }
            if state.profile_scan.is_some() {
                ui.spinner();
            }
            if !state.profile_warnings.is_empty() {
                ui.label(super::icons::INFO)
                    .on_hover_text(state.profile_warnings.join("\n"));
            }
        }
        let color = if view.phase == "ready" {
            egui::Color32::from_rgb(80, 185, 130)
        } else {
            ui.visuals().weak_text_color()
        };
        ui.label(egui::RichText::new(app.tr(view.phase)).small().color(color));
        if view.phase == "opening" {
            ui.spinner();
        }
        if view.session.is_none() {
            if ui
                .add_enabled(
                    !state.closing,
                    egui::Button::new(format!("{} {}", super::icons::PLUG, app.tr("Connect"))),
                )
                .clicked()
            {
                let endpoint = if crate::peer::active() {
                    ""
                } else {
                    &app.serial_port
                };
                let profile = state
                    .profiles
                    .iter()
                    .find(|profile| profile.stable_id == state.preferences.profile_id);
                connect(view, &state.preferences, profile, endpoint, ui.ctx());
                focus_input = true;
            }
        } else if ui
            .add_enabled(
                view.phase != "closing",
                egui::Button::new(format!("{} {}", super::icons::X, app.tr("Disconnect"))),
            )
            .clicked()
        {
            view.begin_close();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        if ui
            .button(super::icons::TRASH)
            .on_hover_text(app.tr("Clear terminal"))
            .clicked()
        {
            view.clear(state.preferences.scrollback_lines);
        }
        if !view.label.is_empty() {
            ui.add(egui::Label::new(egui::RichText::new(&view.label).small().weak()).truncate())
                .on_hover_text(&view.label);
        }
    });
    if let Some(error) = &view.error {
        ui.horizontal(|ui| {
            ui.colored_label(
                ui.visuals().error_fg_color,
                app.tr("Terminal operation failed"),
            );
            ui.label(super::icons::INFO).on_hover_text(error);
        });
    }
    if view.session.is_none()
        && view.phase == "disconnected"
        && view.parser.screen().contents().trim().is_empty()
    {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(super::icons::TERMINAL_WINDOW)
                    .size(28.0)
                    .weak(),
            );
            ui.label(app.tr("Choose a terminal and connect"));
            ui.label(
                egui::RichText::new(app.tr("Sessions start only when you connect."))
                    .small()
                    .weak(),
            );
        });
        return;
    }
    let line_mode = view.target == TerminalTarget::PcController;
    let footer = if line_mode {
        super::control_density::height(ui) + 8.0
    } else {
        0.0
    };
    let height = (ui.available_height() - footer).max(40.0);
    let output = ui
        .allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
            ui.set_height(height);
            super::terminal_screen::show(
                ui,
                &mut view.parser,
                &mut view.screen,
                egui::Id::unique(("terminal-screen", view.id)),
                !line_mode && view.phase == "ready",
                state.preferences.font_size,
            )
        })
        .inner;
    let size = (output.rows, output.cols);
    if size != view.size {
        view.size = size;
        if !line_mode && view.phase == "ready" {
            view.send(super::terminal_backend::Input::Resize {
                rows: size.0,
                cols: size.1,
            });
        }
    }
    if !line_mode {
        for input in output.input {
            view.send(super::terminal_backend::Input::Bytes(input));
        }
    }
    let mut keyboard = output.focused;
    if line_mode {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("pc›")
                    .monospace()
                    .color(ui.visuals().selection.stroke.color),
            );
            let edit_id = egui::Id::unique(("terminal-command", view.id));
            // Consume terminal-specific keys before TextEdit handles Enter
            // (which otherwise relinquishes focus) or Tab (focus traversal).
            let accepts_commands = view.phase == "ready"
                && ui.is_enabled()
                && !egui::Popup::is_any_open(ui.ctx())
                && ui
                    .ctx()
                    .memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
            let keys = command_keys(ui.ctx(), edit_id, accepts_commands);
            if keys.0 {
                view.submit();
            }
            if keys.1 {
                view.history_step(true);
            }
            if keys.2 {
                view.history_step(false);
            }
            if keys.3 {
                view.complete_command();
            }
            let response = ui.add_enabled(
                view.phase == "ready",
                egui::TextEdit::singleline(&mut view.input)
                    .id(edit_id)
                    .font(egui::TextStyle::Monospace)
                    .hint_text(app.tr("Enter a controller command"))
                    .desired_width(f32::INFINITY),
            );
            if view.phase == "ready" && (focus_input || output.clicked || keys.0) {
                response.request_focus();
            }
            if response.changed() {
                view.history_cursor = view.history.len();
                view.history_draft.clear();
            }
            if response.has_focus() {
                keyboard = true;
            }
        });
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::unique("terminal-keyboard-owner"), keyboard));
}

#[cfg(feature = "workspace-terminal")]
fn command_keys(ctx: &egui::Context, id: egui::Id, enabled: bool) -> (bool, bool, bool, bool) {
    if !enabled || !ctx.memory(|memory| memory.has_focus(id)) {
        return (false, false, false, false);
    }
    ctx.input_mut(|input| {
        (
            input.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            input.consume_key(egui::Modifiers::NONE, egui::Key::Tab),
        )
    })
}

#[cfg(feature = "workspace-terminal")]
fn start_profile_scan(
    ctx: &egui::Context,
) -> Result<std::sync::mpsc::Receiver<Result<super::terminal_profiles::Discovery, String>>, String>
{
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("terminal-profiles".into())
        .spawn(move || {
            let _ = sender.send(super::terminal_profiles::discover_with_diagnostics());
            ctx.request_repaint();
        })
        .map_err(|error| format!("Could not refresh terminal profiles: {error}"))?;
    Ok(receiver)
}

pub fn tick(app: &mut crate::app::PealayerApp, ctx: &egui::Context) {
    // Dock bodies paint after global shortcut dispatch. Prime ownership from
    // last frame's stable focused widget, not a stale output flag; typing into
    // a terminal must never run playback or hardware shortcuts first.
    #[cfg(feature = "workspace-terminal")]
    let claimed = app.terminal_state.preferences.enabled
        && ctx.memory(|memory| {
            app.terminal_state.views.iter().any(|view| {
                memory.has_focus(egui::Id::unique(("terminal-screen", view.id)))
                    || memory.has_focus(egui::Id::unique(("terminal-command", view.id)))
            })
        });
    #[cfg(not(feature = "workspace-terminal"))]
    let claimed = false;
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("terminal-keyboard-owner"), claimed));
    #[cfg(feature = "workspace-terminal")]
    for view in &mut app.terminal_state.views {
        if view.tick() {
            ctx.request_repaint();
        }
    }
    #[cfg(feature = "workspace-terminal")]
    {
        if let Some(receiver) = &app.terminal_state.profile_scan {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Profile discovery ended unexpectedly".into()))
                }
            };
            if let Some(result) = result {
                app.terminal_state.profile_scan = None;
                match result {
                    Ok(discovery) => {
                        app.terminal_state.profiles = discovery.profiles;
                        app.terminal_state.profile_warnings = discovery.warnings;
                    }
                    Err(error) => app.terminal_state.profile_warnings = vec![error],
                }
            }
        }
        let actions: Vec<_> = app
            .terminal_state
            .views
            .iter_mut()
            .flat_map(|view| {
                std::mem::take(&mut view.actions)
                    .into_iter()
                    .map(move |action| (view.id, action))
            })
            .collect();
        for (id, action) in actions {
            if !app
                .terminal_state
                .views
                .iter()
                .any(|view| view.id == id && view.phase == "ready")
            {
                continue;
            }
            use super::portable_shell_host::Action;
            use crate::platform::interop::InteropCommand;
            let command = match action {
                Action::Status => None,
                Action::Play => Some(InteropCommand::Play),
                Action::Pause => Some(InteropCommand::Pause),
                Action::Stop => Some(InteropCommand::Stop),
                Action::Seek(seconds) => Some(InteropCommand::SeekTo { seconds }),
                Action::Volume(value) => Some(InteropCommand::SetVolume { value }),
                Action::Mute(muted) => Some(InteropCommand::SetMute { muted }),
                Action::Preferences => Some(InteropCommand::OpenPreferences),
            };
            let result = if let Some(command) = command {
                match command.validate() {
                    Ok(()) => {
                        app.apply_interop_command(ctx, command, "Pealayer shell");
                        Ok("Request sent to Pealayer. Use status to inspect playback.".to_string())
                    }
                    Err(error) => Err(error),
                }
            } else {
                let status = crate::platform::interop::get_live_status();
                Ok(format!(
                    "Playback: {}\nPosition: {:.3} / {:.3} s\nVolume: {:.0}%{}\nController: {}",
                    if status.playing { "playing" } else { "paused" },
                    status.playback_time,
                    status.duration,
                    status.volume,
                    if status.muted { " (muted)" } else { "" },
                    if status.controller_connected {
                        "connected"
                    } else {
                        "disconnected"
                    }
                ))
            };
            if let Some(view) = app
                .terminal_state
                .views
                .iter_mut()
                .find(|view| view.id == id)
                && view.phase != "closing"
            {
                let (text, error) = match result {
                    Ok(text) => (text, false),
                    Err(text) => (text, true),
                };
                view.send(super::terminal_backend::Input::Response { text, error });
            }
        }
        if app.terminal_state.collect_closed() {
            app.request_quit(ctx);
        } else if app.terminal_state.cleanup_pending() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    crate::workspace_tools::publish(app);
}

/// Apply a changed visibility intent once. Native pane close/profile actions
/// write back the same preference, so hidden sessions never reopen themselves.
pub fn sync_visibility(app: &mut crate::app::PealayerApp) {
    #[cfg(feature = "workspace-terminal")]
    {
        let wanted = app.terminal_state.preferences.enabled;
        if app.terminal_state.last_visibility_intent == Some(wanted) {
            return;
        }
        let tab = super::layout::PealayerTab::Terminal;
        let visible = app.dock_state.layout.contains(&tab);
        if wanted != visible {
            if wanted && !app.dock_state.locked {
                app.show_four_d_editor = true;
                if super::layout::reveal_and_focus_tab(&mut app.dock_state, tab) {
                    app.pending_tab_reveals.retain(|queued| *queued != tab);
                    app.pending_tab_reveals.push(tab);
                }
            } else {
                let _ = app.dock_state.set_visible(
                    tab,
                    false,
                    Some(super::layout::PealayerTab::Timeline),
                    |_| craft_ui::docking::Permissions::default(),
                );
            }
        }
        app.sync_terminal_visibility_from_workspace();
        app.terminal_state.last_visibility_intent = Some(app.terminal_state.preferences.enabled);
    }
    #[cfg(not(feature = "workspace-terminal"))]
    let _ = app;
}

/// The renderer owns pane geometry, title, menus and close button. This body
/// never creates a competing fixed panel or overwrites remembered dock bounds.
pub fn draw_docked(app: &mut crate::app::PealayerApp, ui: &mut egui::Ui) {
    #[cfg(feature = "workspace-terminal")]
    if app.terminal_state.preferences.enabled {
        let mut state = std::mem::take(&mut app.terminal_state);
        let mut open_preferences = false;
        ui.set_min_height(ui.available_height());
        panel_contents(app, ui, &mut state, &mut open_preferences);
        app.terminal_state = state;
        // Preferences snapshots must see the restored state, not mem::take's
        // temporary default. Opening a helper inside panel() loses that state.
        if open_preferences {
            app.apply_interop_command(
                ui.ctx(),
                crate::platform::interop::InteropCommand::NavigatePreferences {
                    navigation: crate::preferences_contract::PreferencesNavigation {
                        section: Some("workspace_tools".into()),
                        scroll_y: 0.0,
                    },
                },
                "terminal",
            );
        }
    }
    #[cfg(not(feature = "workspace-terminal"))]
    let _ = (app, ui);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_terminal_never_allocates_a_connection() {
        let state = TerminalState::default();
        assert!(!state.preferences.enabled);
        assert!(!state.busy());
        assert_eq!(state.status()["sessions"], serde_json::json!([]));
        assert_eq!(
            serde_json::from_value::<TerminalPreferences>(serde_json::json!({})).unwrap(),
            state.preferences
        );
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn docked_body_never_overwrites_workspace_geometry_or_preference_height() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = crate::app::PealayerApp::default();
        app.terminal_state.preferences.enabled = true;
        app.terminal_state.preferences.font_size = 13.0;
        let workspace = app.dock_state.clone();
        let ctx = egui::Context::default();
        for height in [280.0, 280.0, 280.0, 360.0, 360.0] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, height),
                    )),
                    ..Default::default()
                },
                |ui| draw_docked(&mut app, ui),
            );
            output.textures_delta.clear();
            assert_eq!(app.terminal_state.preferences.font_size, 13.0);
            assert!(!app.terminal_state.busy());
            assert_eq!(app.dock_state, workspace);
        }
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn visibility_intent_reconciles_once_and_close_never_reopens_sessions() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = crate::app::PealayerApp::default();
        let tab = super::super::layout::PealayerTab::Terminal;
        app.terminal_state.preferences.enabled = true;
        sync_visibility(&mut app);
        assert!(app.dock_state.layout.contains(&tab));
        let remembered = app.dock_state.clone();
        sync_visibility(&mut app);
        assert_eq!(remembered, app.dock_state);
        app.toggle_tab(tab);
        assert!(!app.terminal_state.preferences.enabled);
        sync_visibility(&mut app);
        sync_visibility(&mut app);
        assert!(!app.dock_state.layout.contains(&tab));
        app.dock_state.locked = true;
        app.terminal_state.preferences.enabled = true;
        sync_visibility(&mut app);
        assert!(!app.dock_state.layout.contains(&tab));
        assert!(!app.terminal_state.preferences.enabled);
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn focused_terminal_preclaims_keyboard_before_dock_paint() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = crate::app::PealayerApp::default();
        app.terminal_state.preferences.enabled = true;
        app.terminal_state
            .views
            .push(View::new(7, &app.terminal_state.preferences));
        let ctx = egui::Context::default();
        ctx.memory_mut(|memory| {
            memory.request_focus(egui::Id::unique(("terminal-command", 7_u64)))
        });
        tick(&mut app, &ctx);
        assert!(owns_keyboard(&ctx));
        app.terminal_state.preferences.enabled = false;
        tick(&mut app, &ctx);
        assert!(!owns_keyboard(&ctx));
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn cancelling_quit_retains_cleanup_but_never_quits_afterward() {
        let mut state = TerminalState::default();
        state.views.push(View::new(1, &state.preferences));
        state.views[0].input = "unfinished command".into();
        state.views[0].parser.process(b"preserved terminal output");
        state.confirm_exit = true;
        state.begin_exit();
        state.cancel_exit();
        assert!(state.closing, "Dismissal must not abandon pending cleanup");
        assert!(!state.confirm_exit);
        assert!(!state.quit_after_close);
        assert!(
            !state.collect_closed(),
            "Canceled quit must never be retriggered"
        );
        assert!(!state.closing);
        assert_eq!(state.views[0].input, "unfinished command");
        assert!(
            state.views[0]
                .parser
                .screen()
                .contents()
                .contains("preserved terminal output")
        );
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn completed_exit_is_issued_once_and_empty_tabs_have_no_cleanup_work() {
        let mut state = TerminalState::default();
        state.views.push(View::new(1, &state.preferences));
        state.views.push(View::new(2, &state.preferences));
        state.active = 1;
        state.views[1].input = "keep draft".into();
        state.retire_view(0);
        assert_eq!(state.active, 0);
        assert_eq!(state.views[0].input, "keep draft");
        assert!(!state.busy());
        assert!(!state.cleanup_pending());
        state.begin_exit();
        assert!(state.collect_closed());
        assert!(!state.collect_closed());
        assert!(!state.cleanup_pending());
    }
    #[test]
    fn arguments_are_literal_argv_without_shell_expansion() {
        assert_eq!(
            arguments(r#"--name "two words" 'three words' C:\Tools\file.exe "$HOME" "#).unwrap(),
            vec![
                "--name",
                "two words",
                "three words",
                r"C:\Tools\file.exe",
                "$HOME"
            ]
        );
        assert!(arguments("'unclosed").is_err());
        assert!(arguments("arg\nother").is_err());
    }
    #[test]
    fn arguments_preserve_unc_paths_empty_values_and_literal_quotes() {
        assert_eq!(
            arguments(r#"\\server\share "\\server\folder name\" "" '' "a""b" 'c''d' ; & $(noop)"#)
                .unwrap(),
            vec![
                r"\\server\share",
                r"\\server\folder name\",
                "",
                "",
                "a\"b",
                "c'd",
                ";",
                "&",
                "$(noop)"
            ]
        );
        assert!(arguments(&"x".repeat(8193)).is_err());
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn rejected_submission_preserves_input_history_and_screen() {
        let mut view = View::new(1, &TerminalPreferences::default());
        view.input = "relay 1 on".into();
        let before = view.parser.screen().contents();
        view.submit();
        assert_eq!(view.input, "relay 1 on");
        assert!(view.history.is_empty());
        assert_eq!(view.parser.screen().contents(), before);
        assert!(view.error.is_some());
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn command_history_restores_draft_and_completes_advertised_aliases() {
        let mut view = View::new(1, &TerminalPreferences::default());
        view.history = vec!["status".into(), "help".into()];
        view.history_cursor = 2;
        view.input = "draft".into();
        view.history_step(true);
        assert_eq!(view.input, "help");
        view.history_step(true);
        assert_eq!(view.input, "status");
        view.history_step(false);
        view.history_step(false);
        assert_eq!(view.input, "draft");
        view.catalog = vec![
            serde_json::json!({"name":"status","aliases":["info"]}),
            serde_json::json!({"name":"start"}),
        ];
        view.input = "IN".into();
        view.complete_command();
        assert_eq!(view.input, "info ");
        view.input = "st".into();
        view.complete_command();
        assert_eq!(view.input, "st");
        view.input = "status ar".into();
        view.complete_command();
        assert_eq!(view.input, "status ar");
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn status_queries_ignore_control_strings_and_bound_csi_parameters() {
        let mut state = QueryState::default();
        for byte in b"\x1b]title \x1b[6n\x07\x1bPdata\x07\x1b[5n\x1b\\\x1b[?6n" {
            assert_eq!(state.push(*byte), None);
        }
        for byte in b"\x1b[1234567890123456789012345678906n" {
            assert_eq!(state.push(*byte), None);
        }
        for byte in b"\x1b[6" {
            assert_eq!(state.push(*byte), None);
        }
        assert_eq!(state.push(b'n'), Some(StatusQuery::Cursor));
        for byte in b"\x1b[5" {
            assert_eq!(state.push(*byte), None);
        }
        assert_eq!(state.push(b'n'), Some(StatusQuery::Status));
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn command_enter_is_captured_before_text_edit_relinquishes_focus() {
        let ctx = egui::Context::default();
        let id = egui::Id::unique("terminal-command-test");
        let mut value = "status".to_string();
        let mut first = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut value).id(id))
                .request_focus();
        });
        first.textures_delta.clear();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut second = ctx.run_ui(input, |ui| {
            assert_eq!(
                command_keys(ui.ctx(), id, true),
                (true, false, false, false)
            );
            let response = ui.add(egui::TextEdit::singleline(&mut value).id(id));
            assert!(response.has_focus());
        });
        second.textures_delta.clear();
        assert_eq!(value, "status");
    }
    #[test]
    fn terminal_preferences_bound_memory_and_reject_multiline_programs() {
        let mut preferences = TerminalPreferences::default();
        assert!(preferences.validate().is_ok());
        preferences.scrollback_lines = usize::MAX;
        assert!(preferences.validate().is_err());
        preferences = TerminalPreferences::default();
        preferences.shell_program = "cmd.exe\r\necho other".into();
        assert!(preferences.validate().is_err());
    }
}
