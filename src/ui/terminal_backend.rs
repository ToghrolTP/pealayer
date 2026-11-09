//! Explicitly opened, bounded terminal sessions. The UI never performs child,
//! transport, PTY read/write, resize, wait or kill operations.
use eframe::egui;
use portable_pty::{CommandBuilder, PtySize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
};

const INPUT_CAPACITY: usize = 64;
const OUTPUT_CAPACITY: usize = 64;
const OUTPUT_CHUNK: usize = 8 * 1024;
const MAX_INPUT_BYTES: usize = 32 * 1024;
const MAX_COMMAND_BYTES: usize = 8 * 1024;
const MAX_CATALOG_BYTES: usize = 1024 * 1024;
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;

#[derive(Clone, Debug)]
pub enum Target {
    Controller {
        endpoint: String,
    },
    Shell {
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
        environment: Vec<(String, Option<String>)>,
        clear_environment: bool,
    },
    Embedded,
}

#[derive(Debug)]
pub enum Input {
    Bytes(Vec<u8>),
    Command(String),
    Resize { rows: u16, cols: u16 },
    Response { text: String, error: bool },
}

#[derive(Debug)]
pub enum Event {
    Ready { label: String },
    Catalog(Vec<Value>),
    Output(Vec<u8>),
    Exited { code: Option<u32> },
    Error(String),
    Application(super::portable_shell_host::Action),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Controller,
    Shell,
    Embedded,
}

#[derive(Default)]
struct State {
    inputs: VecDeque<Input>,
    cancelled: bool,
    finished: bool,
    child_exited: bool,
    output_done: bool,
    resize: Option<(u16, u16)>,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
    generation: AtomicU64,
    repaint_pending: AtomicBool,
    exit_reported: AtomicBool,
}

impl Shared {
    fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
            generation: AtomicU64::new(1),
            repaint_pending: AtomicBool::new(false),
            exit_reported: AtomicBool::new(false),
        }
    }

    fn active(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }

    fn next_input(&self) -> Option<Input> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        loop {
            if state.cancelled || state.finished {
                return None;
            }
            if let Some(input) = state.inputs.pop_front() {
                return Some(input);
            }
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    fn finish(&self) {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .finished = true;
        self.wake.notify_all();
    }

    fn cancel(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.cancelled = true;
        self.generation.fetch_add(1, Ordering::AcqRel);
        drop(state);
        self.wake.notify_all();
    }
}

#[derive(Clone)]
struct Events {
    sender: SyncSender<Event>,
    shared: Arc<Shared>,
    ctx: egui::Context,
    generation: u64,
}

impl Events {
    fn send(&self, mut event: Event) -> bool {
        // Backpressure is bounded to 64 events (each output chunk <=8 KiB).
        // A close request must also release producers while the UI retains its
        // receiver to observe that every owned worker has actually finished.
        loop {
            if !self.shared.active(self.generation) {
                return false;
            }
            match self.sender.try_send(event) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => return false,
                Err(mpsc::TrySendError::Full(pending)) => {
                    event = pending;
                    let state = self
                        .shared
                        .state
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if state.cancelled {
                        return false;
                    }
                    drop(
                        self.shared
                            .wake
                            .wait_timeout(state, std::time::Duration::from_millis(50)),
                    );
                }
            }
        }
        if !self.shared.repaint_pending.swap(true, Ordering::AcqRel) {
            self.ctx.request_repaint();
        }
        true
    }

    fn output(&self, bytes: &[u8]) -> bool {
        // Split byte streams without decoding: the terminal parser owns UTF-8
        // sequences that cross read boundaries and all VT escape sequences.
        bytes
            .chunks(OUTPUT_CHUNK)
            .all(|chunk| self.send(Event::Output(chunk.to_vec())))
    }

    fn exited(&self, code: Option<u32>) {
        if !self.shared.exit_reported.swap(true, Ordering::AcqRel) {
            self.send(Event::Exited { code });
        }
    }
}

pub struct Session {
    kind: Kind,
    shared: Arc<Shared>,
    events: Receiver<Event>,
}

impl Session {
    pub fn spawn(target: Target, rows: u16, cols: u16, ctx: egui::Context) -> Result<Self, String> {
        validate_size(rows, cols)?;
        let kind = match &target {
            Target::Embedded => Kind::Embedded,
            Target::Controller { endpoint } => {
                if endpoint.trim().is_empty()
                    || endpoint.len() > 4096
                    || endpoint.contains(['\r', '\n', '\0'])
                {
                    return Err("Choose a valid PCController host endpoint".into());
                }
                Kind::Controller
            }
            Target::Shell {
                program,
                args,
                cwd,
                environment,
                ..
            } => {
                if program.trim().is_empty()
                    || program.len() > 4096
                    || program.contains(['\r', '\n', '\0'])
                {
                    return Err("Choose an executable name or path".into());
                }
                if args.len() > 128
                    || args.iter().any(|arg| arg.contains('\0'))
                    || args.iter().map(String::len).sum::<usize>() > 64 * 1024
                {
                    return Err("Terminal arguments are invalid or too large".into());
                }
                if cwd
                    .as_ref()
                    .is_some_and(|path| path.len() > 8192 || path.contains('\0'))
                    || environment.len() > 64
                    || environment.iter().any(|(key, value)| {
                        key.is_empty()
                            || key.contains(['=', '\0'])
                            || key.len() > 4096
                            || value
                                .as_ref()
                                .is_some_and(|value| value.len() > 65536 || value.contains('\0'))
                    })
                {
                    return Err("Terminal working directory or environment is invalid".into());
                }
                Kind::Shell
            }
        };
        let shared = Arc::new(Shared::new());
        let (sender, events) = mpsc::sync_channel(OUTPUT_CAPACITY);
        let output = Events {
            sender,
            shared: shared.clone(),
            ctx,
            generation: 1,
        };
        std::thread::Builder::new()
            .name("terminal-session".into())
            .spawn(move || {
                let result = match target {
                    Target::Embedded => run_embedded(rows, cols, &output),
                    Target::Controller { endpoint } => run_controller(&endpoint, &output),
                    Target::Shell {
                        program,
                        args,
                        cwd,
                        environment,
                        clear_environment,
                    } => run_shell(
                        &program,
                        &args,
                        cwd.as_deref(),
                        &environment,
                        clear_environment,
                        rows,
                        cols,
                        &output,
                    ),
                };
                if let Err(error) = result {
                    output.shared.finish();
                    output.send(Event::Error(error));
                    output.exited(None);
                }
            })
            .map_err(|error| format!("Could not start terminal worker: {error}"))?;
        Ok(Self {
            kind,
            shared,
            events,
        })
    }

    /// Acceptance means queued once, never replayed. An in-flight host command
    /// cannot be rolled back by closing a console; queued commands are cancelled.
    pub fn send(&self, input: Input) -> Result<(), String> {
        validate_input(self.kind, &input)?;
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.cancelled || state.finished {
            return Err("This terminal session has ended".into());
        }
        if state.inputs.len() >= INPUT_CAPACITY {
            return Err(
                "Terminal input queue is full; wait for the current input to finish".into(),
            );
        }
        state.inputs.push_back(input);
        drop(state);
        self.shared.wake.notify_all();
        Ok(())
    }

    pub fn try_recv(&self) -> Option<Event> {
        // Clear before observing the queue so a racing producer always schedules
        // a repaint after the consumer's final empty read. No timer is required.
        self.shared.repaint_pending.store(false, Ordering::Release);
        let event = self.events.try_recv().ok();
        if event.is_some() {
            self.shared.wake.notify_all();
        }
        event
    }

    /// Begin cancellation without blocking the UI or discarding the cleanup
    /// receipt. In-flight controller calls retain their bounded RPC timeout.
    pub fn request_close(&self) {
        self.shared.cancel();
    }

    /// Every worker and the owned-child guard retains Shared until its final
    /// cleanup. The Session's sole remaining reference proves all have exited.
    pub fn is_closed(&self) -> bool {
        Arc::strong_count(&self.shared) == 1
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.shared.cancel();
    }
}

/// Reuse the application's argv-safe executable resolution. Call this from a
/// validation worker when displaying the resolved path; spawn resolves off-UI.
pub fn resolve_program(program: &str) -> Result<PathBuf, String> {
    crate::mpv::external::resolve_executable(program)
}

fn validate_size(rows: u16, cols: u16) -> Result<(), String> {
    if !(1..=256).contains(&rows) || !(1..=512).contains(&cols) {
        Err("Terminal dimensions must be 1–256 rows and 1–512 columns".into())
    } else {
        Ok(())
    }
}

fn validate_input(kind: Kind, input: &Input) -> Result<(), String> {
    match input {
        Input::Command(line) if kind == Kind::Controller => {
            if line.trim().is_empty()
                || line.len() > MAX_COMMAND_BYTES
                || line.contains(['\r', '\n', '\0'])
            {
                Err("Enter one command line, without line breaks or NUL, up to 8 KiB".into())
            } else {
                Ok(())
            }
        }
        Input::Bytes(bytes) if matches!(kind, Kind::Shell | Kind::Embedded) => {
            if bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
                Err("Terminal input must contain 1–32768 bytes per submission".into())
            } else {
                Ok(())
            }
        }
        Input::Resize { rows, cols } if matches!(kind, Kind::Shell | Kind::Embedded) => {
            validate_size(*rows, *cols)
        }
        Input::Response { text, .. } if kind == Kind::Embedded && text.len() <= 8192 => Ok(()),
        _ => Err("This input is not supported by the selected terminal session".into()),
    }
}

fn catalog_entries(value: Value) -> Result<Vec<Value>, String> {
    let entries = value
        .as_array()
        .ok_or("PCController returned an invalid command catalog")?;
    if entries.len() > 4096
        || serde_json::to_vec(&value)
            .map_err(|error| error.to_string())?
            .len()
            > MAX_CATALOG_BYTES
        || entries.iter().any(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|name| name.trim().is_empty() || name.len() > 256)
                || ["usage", "summary", "group"]
                    .iter()
                    .any(|key| entry.get(*key).and_then(Value::as_str).is_none())
                || entry
                    .get("aliases")
                    .and_then(Value::as_array)
                    .is_none_or(|aliases| aliases.iter().any(|alias| !alias.is_string()))
        })
    {
        return Err("PCController returned an invalid or oversized command catalog".into());
    }
    Ok(entries.clone())
}

fn run_controller(endpoint: &str, output: &Events) -> Result<(), String> {
    if !output.shared.active(output.generation) {
        return Ok(());
    }
    let mut client = crate::four_d::controller::ControllerClient::connect_preferred(endpoint)?;
    if !output.shared.active(output.generation) {
        return Ok(());
    }
    let catalog = catalog_entries(client.call("controller.command.catalog", json!({}))?)?;
    if !output.send(Event::Catalog(catalog))
        || !output.send(Event::Ready {
            label: "PCController host console".into(),
        })
    {
        return Ok(());
    }
    while let Some(Input::Command(line)) = output.shared.next_input() {
        // This session's generation is invalidated on Drop. Never dispatch a
        // previously queued command into a replacement session or retry it.
        if !output.shared.active(output.generation) {
            break;
        }
        match client.call_detailed("controller.command.execute", json!({"command": line})) {
            Ok(result) => {
                let text = result
                    .get("output")
                    .and_then(Value::as_str)
                    .ok_or("PCController command response is missing its output string")?;
                if text.len() > MAX_COMMAND_OUTPUT {
                    if !output.send(Event::Error(
                        "PCController command output exceeds the console's 1 MiB response limit"
                            .into(),
                    )) {
                        break;
                    }
                    continue;
                }
                let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
                if !output.output(text.as_bytes()) {
                    break;
                }
            }
            Err(error) if error.transport_failed => {
                return Err(format!(
                    "PCController connection failed; the command was not retried: {}",
                    error.message
                ));
            }
            Err(error) => {
                if !output.send(Event::Error(error.message)) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn pty_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// PortableShell runs in-process on one owned worker, never through an OS
/// executable or command interpreter. Only explicit submitted commands cross
/// back to the UI's typed application dispatcher.
fn run_embedded(rows: u16, cols: u16, output: &Events) -> Result<(), String> {
    let mut host = super::portable_shell_host::Host::new(rows.min(80), cols.min(240))?;
    let started = std::time::Instant::now();
    output.send(Event::Ready {
        label: "Pealayer · embedded PortableShell".into(),
    });
    let mut escape_deadline = None;
    loop {
        if !output.shared.active(output.generation) {
            break;
        }
        if !output.output(&host.take_output()) {
            break;
        }
        for action in host.take_actions() {
            if !output.send(Event::Application(action)) {
                return Ok(());
            }
        }
        if host.exited() {
            output.shared.finish();
            output.exited(Some(0));
            break;
        }
        let mut state = output
            .shared
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while !state.cancelled && !state.finished && state.inputs.is_empty() {
            if let Some(deadline) = escape_deadline {
                let now = std::time::Instant::now();
                if now >= deadline {
                    break;
                }
                state = output
                    .shared
                    .wake
                    .wait_timeout(state, deadline - now)
                    .unwrap_or_else(|error| error.into_inner())
                    .0;
            } else {
                state = output
                    .shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(|error| error.into_inner());
            }
        }
        if state.cancelled || state.finished {
            break;
        }
        let input = state.inputs.pop_front();
        drop(state);
        let now = started.elapsed().as_millis() as u32;
        match input {
            Some(Input::Bytes(bytes)) => {
                host.feed(&bytes);
                escape_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(70));
            }
            Some(Input::Resize { rows, cols }) => host.resize(rows.min(80), cols.min(240)),
            Some(Input::Response { text, error }) => host.response(&text, error),
            Some(Input::Command(_)) => unreachable!("input validated at session boundary"),
            None => {
                host.tick(now);
                escape_deadline = None;
            }
        }
    }
    // Dropping Host cancels unfinished input; it must never submit it.
    Ok(())
}

/// A spawn failure after creating a child must not orphan it. This guard lives
/// only on workers, and is disarmed only after that exact child has been reaped.
struct OwnedChild {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    reaped: bool,
    shared: Arc<Shared>,
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            if self.child.wait().is_ok() {
                self.shared
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .child_exited = true;
            }
        }
    }
}

fn run_shell(
    program: &str,
    args: &[String],
    cwd: Option<&str>,
    environment: &[(String, Option<String>)],
    clear_environment: bool,
    rows: u16,
    cols: u16,
    output: &Events,
) -> Result<(), String> {
    let program = resolve_program(program)?;
    if !output.shared.active(output.generation) {
        return Ok(());
    }
    let pair = portable_pty::native_pty_system()
        .openpty(pty_size(rows, cols))
        .map_err(|error| format!("Could not open terminal: {error}"))?;
    let mut command = CommandBuilder::new(&program);
    command.args(args);
    if clear_environment {
        command.env_clear();
    }
    if let Some(cwd) = cwd {
        command.cwd(cwd);
    }
    for (key, value) in environment {
        if let Some(value) = value {
            command.env(key, value);
        } else {
            command.env_remove(key);
        }
    }
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    if !output.shared.active(output.generation) {
        return Ok(());
    }
    let mut child = OwnedChild {
        child: pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("Could not start terminal executable: {error}"))?,
        reaped: false,
        shared: output.shared.clone(),
    };
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Could not read terminal: {error}"))?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Could not write terminal: {error}"))?;
    let master = pair.master;
    let control_output = output.clone();
    std::thread::Builder::new()
        .name("terminal-control".into())
        .spawn(move || {
            let shared = &control_output.shared;
            loop {
                let mut state = shared
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                while !state.cancelled && !state.finished && state.resize.is_none() {
                    state = shared
                        .wake
                        .wait(state)
                        .unwrap_or_else(|error| error.into_inner());
                }
                let cancelled = state.cancelled;
                let finished = state.finished;
                let resize = state.resize.take();
                drop(state);
                if cancelled || finished {
                    break;
                }
                if let Some((rows, cols)) = resize {
                    if let Err(error) = master.resize(pty_size(rows, cols)) {
                        if !control_output
                            .send(Event::Error(format!("Could not resize terminal: {error}")))
                        {
                            break;
                        }
                    }
                }
            }
            // Close this session's PTY master here, never on the UI thread.
            drop(master);
        })
        .map_err(|error| format!("Could not start terminal control worker: {error}"))?;

    // Even an executable which immediately exits must report Ready before its
    // final status. No input is dispatched until the workers below are ready.
    if !output.send(Event::Ready {
        label: program.display().to_string(),
    }) {
        output.shared.cancel();
        return Ok(());
    }

    let read_output = output.clone();
    std::thread::Builder::new()
        .name("terminal-output".into())
        .spawn(move || {
            let mut buffer = [0_u8; OUTPUT_CHUNK];
            while read_output.shared.active(read_output.generation) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if !read_output.output(&buffer[..count]) {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        // Unix PTYs conventionally return EIO when the slave closes.
                        #[cfg(unix)]
                        if error.raw_os_error() == Some(libc::EIO) {
                            break;
                        }
                        if read_output.shared.active(read_output.generation) {
                            read_output.shared.finish();
                            read_output
                                .send(Event::Error(format!("Terminal output ended: {error}")));
                        }
                        break;
                    }
                }
            }
            read_output
                .shared
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .output_done = true;
            read_output.shared.wake.notify_all();
        })
        .map_err(|error| format!("Could not start terminal output worker: {error}"))?;

    let wait_output = output.clone();
    std::thread::Builder::new()
        .name("terminal-exit".into())
        .spawn(move || {
            // Keep ownership of the actual child while checking cancellation.
            // portable-pty's Unix cloned killer sends only SIGHUP, which a
            // program can ignore. Its actual Child::kill escalates to SIGKILL.
            // Never signal a cached PID from another worker after it is reaped.
            let result = loop {
                match child.child.try_wait() {
                    Ok(Some(status)) => break Ok(status),
                    Err(error) => break Err(error),
                    Ok(None) => {}
                }
                let state = wait_output
                    .shared
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                if state.cancelled || state.finished {
                    drop(state);
                    let _ = child.child.kill();
                    break child.child.wait();
                }
                // Only an explicitly running child has this bounded waiter;
                // cancellation/transport failure wakes it immediately.
                drop(
                    wait_output
                        .shared
                        .wake
                        .wait_timeout(state, std::time::Duration::from_millis(100)),
                );
            };
            child.reaped = result.is_ok();
            {
                let mut state = wait_output
                    .shared
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                state.child_exited = child.reaped;
                state.finished = true;
            }
            wait_output.shared.wake.notify_all();
            // Closing the owned master releases the reader. Drain its queued bytes
            // before Exited so a short command's final UTF-8 text is not lost.
            let mut state = wait_output
                .shared
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            while !state.output_done && !state.cancelled {
                state = wait_output
                    .shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(|error| error.into_inner());
            }
            drop(state);
            match result {
                Ok(status) => wait_output.exited(Some(status.exit_code())),
                Err(error) => {
                    wait_output.send(Event::Error(format!(
                        "Could not wait for terminal exit: {error}"
                    )));
                    wait_output.exited(None);
                }
            }
        })
        .map_err(|error| format!("Could not start terminal exit worker: {error}"))?;
    while let Some(input) = output.shared.next_input() {
        if !output.shared.active(output.generation) {
            break;
        }
        match input {
            Input::Bytes(bytes) => {
                if let Err(error) = writer.write_all(&bytes).and_then(|_| writer.flush()) {
                    output.shared.finish();
                    output.send(Event::Error(format!(
                        "Could not send terminal input: {error}"
                    )));
                    // The child waiter owns the final exit event, after draining
                    // output, even when this write fails or is interrupted.
                    break;
                }
            }
            Input::Resize { rows, cols } => {
                output
                    .shared
                    .state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .resize = Some((rows, cols));
                output.shared.wake.notify_all();
            }
            Input::Command(_) | Input::Response { .. } => {
                unreachable!("input validated at session boundary")
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_session_is_readonly_until_submission_and_cancels_unfinished_input() {
        use super::super::portable_shell_host::Action;
        let session = Session::spawn(Target::Embedded, 24, 80, egui::Context::default()).unwrap();
        let mut output = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut ready = false;
        while !ready && std::time::Instant::now() < deadline {
            while let Some(event) = session.try_recv() {
                match event {
                    Event::Ready { .. } => ready = true,
                    Event::Output(bytes) => output.extend(bytes),
                    Event::Application(_) | Event::Catalog(_) => {
                        panic!("Opening the prompt dispatched a command")
                    }
                    Event::Error(error) => panic!("{error}"),
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(ready);
        session
            .send(Input::Bytes(b"status\rseek 1.25\r".to_vec()))
            .unwrap();
        let mut actions = Vec::new();
        while actions.len() < 2 && std::time::Instant::now() < deadline {
            while let Some(event) = session.try_recv() {
                match event {
                    Event::Application(action) => actions.push(action),
                    Event::Output(bytes) => output.extend(bytes),
                    Event::Error(error) => panic!("{error}"),
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(actions, [Action::Status, Action::Seek(1.25)]);
        assert!(String::from_utf8_lossy(&output).contains("Pealayer"));
        session.send(Input::Bytes(b"play".to_vec())).unwrap();
        session.request_close();
        while !session.is_closed() && std::time::Instant::now() < deadline {
            if let Some(Event::Application(_)) = session.try_recv() {
                panic!("Closing executed unfinished input");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            session.is_closed(),
            "Embedded worker must release ownership after cancellation"
        );
        assert!(session.send(Input::Bytes(b"\r".to_vec())).is_err());
    }

    #[test]
    fn controller_session_dispatches_only_explicit_commands_and_never_replays_failures() {
        use std::io::{BufRead, BufReader};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        // Ephemeral loopback host only: no embedded host, serial port, board or
        // user shell is involved in this protocol/lifecycle fixture.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("pccontroller://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let (observed_tx, observed_rx) = mpsc::channel();
        let (close_tx, close_rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(8);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "Console never connected to fixture"
                        );
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("Fixture accept failed: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for expected in [
                "controller.ping",
                "controller.app.instance.report",
                "controller.command.catalog",
            ] {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                let request: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(request["method"], expected);
                let result = if expected == "controller.command.catalog" {
                    json!([{"name":"status", "aliases":["info"], "usage":"status", "summary":"Fixture only", "group":"system"}])
                } else {
                    json!({"ok":true})
                };
                writeln!(
                    stream,
                    "{}",
                    json!({"jsonrpc":"2.0", "id":request["id"], "result":result})
                )
                .unwrap();
            }
            for command in ["status فارسی", "rejected", "status again", "disconnect"] {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                let request: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(request["method"], "controller.command.execute");
                assert_eq!(request["params"], json!({"command": command}));
                observed_tx.send(command.to_string()).unwrap();
                if command == "disconnect" {
                    close_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    break;
                }
                let response = if command == "rejected" {
                    json!({"jsonrpc":"2.0", "id":request["id"], "error":{"code":-32000,"message":"fixture rejection","data":{"kind":"device_rejected","retryable":false}}})
                } else {
                    json!({"jsonrpc":"2.0", "id":request["id"], "result":{"output":"فارسی café\r\nsecond\n"}})
                };
                writeln!(stream, "{response}").unwrap();
            }
            drop(reader);
            drop(stream);
            // Keep listening briefly after deliberate transport loss: a retry
            // or automatic reconnect would be observable, and is forbidden.
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                match listener.accept() {
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    other => panic!("Unexpected reconnect after failed command: {other:?}"),
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let session = Session::spawn(
            Target::Controller { endpoint },
            24,
            80,
            egui::Context::default(),
        )
        .unwrap();
        let next = || {
            session
                .events
                .recv_timeout(Duration::from_secs(8))
                .expect("Console fixture event timed out")
        };
        assert!(
            matches!(next(), Event::Catalog(catalog) if catalog.len()==1 && catalog[0]["name"]=="status")
        );
        assert!(matches!(next(), Event::Ready { .. }));
        assert!(
            matches!(
                observed_rx.recv_timeout(Duration::from_millis(100)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "Connecting executed a command without explicit input"
        );

        for command in ["status فارسی", "rejected", "status again"] {
            session.send(Input::Command(command.into())).unwrap();
            assert_eq!(
                observed_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
                command
            );
            match next() {
                Event::Error(error) if command == "rejected" => {
                    assert!(error.contains("fixture rejection"))
                }
                Event::Output(bytes) if command != "rejected" => {
                    assert_eq!(bytes, "فارسی café\r\nsecond\r\n".as_bytes())
                }
                other => panic!("Unexpected console response: {other:?}"),
            }
            assert!(
                !session.shared.state.lock().unwrap().finished,
                "Semantic rejection ended healthy session"
            );
        }
        session.send(Input::Command("disconnect".into())).unwrap();
        assert_eq!(
            observed_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            "disconnect"
        );
        // This queued command must be discarded, not replayed after reconnect.
        session
            .send(Input::Command("must not replay".into()))
            .unwrap();
        close_tx.send(()).unwrap();
        assert!(matches!(next(), Event::Error(error) if error.contains("not retried")));
        assert!(matches!(next(), Event::Exited { code: None }));
        assert!(
            session
                .send(Input::Command("must not reopen".into()))
                .is_err()
        );
        server.join().unwrap();
        assert!(observed_rx.try_recv().is_err());
    }

    #[test]
    fn host_commands_are_single_bounded_lines_and_raw_keys_belong_only_to_shells() {
        for line in ["help\nrelay on", "help\r", "\0help", "   "] {
            assert!(validate_input(Kind::Controller, &Input::Command(line.into())).is_err());
        }
        assert!(validate_input(Kind::Controller, &Input::Command("help".repeat(2049))).is_err());
        assert!(validate_input(Kind::Controller, &Input::Command("status".into())).is_ok());
        assert!(validate_input(Kind::Controller, &Input::Bytes(vec![3])).is_err());
        assert!(validate_input(Kind::Shell, &Input::Command("status".into())).is_err());
        assert!(validate_input(Kind::Shell, &Input::Bytes(vec![0, 3, 27])).is_ok());
        assert!(validate_size(0, 80).is_err());
        assert!(validate_size(24, 513).is_err());
    }

    #[test]
    fn full_input_queue_returns_an_error_and_closed_generations_never_dispatch() {
        let shared = Arc::new(Shared::new());
        let (_, events) = mpsc::sync_channel(1);
        let session = Session {
            kind: Kind::Controller,
            shared: shared.clone(),
            events,
        };
        for _ in 0..INPUT_CAPACITY {
            session.send(Input::Command("help".into())).unwrap();
        }
        assert!(
            session
                .send(Input::Command("status".into()))
                .unwrap_err()
                .contains("full")
        );
        assert_eq!(shared.state.lock().unwrap().inputs.len(), INPUT_CAPACITY);
        drop(session);
        assert!(!shared.active(1));
        assert!(shared.next_input().is_none());
    }

    #[test]
    fn malformed_catalog_is_not_accepted_as_an_empty_console() {
        assert!(catalog_entries(json!({"commands": []})).is_err());
        assert!(catalog_entries(json!([{"summary": "missing name"}])).is_err());
        assert!(catalog_entries(json!([{"name": ""}])).is_err());
        let catalog = json!([{"name": "status", "aliases": ["info"], "summary": "Host status", "group": "system", "usage": "status"}]);
        assert_eq!(
            catalog_entries(catalog.clone()).unwrap(),
            catalog.as_array().unwrap().clone()
        );
    }

    #[test]
    fn output_backpressure_preserves_bytes_and_drop_releases_the_producer() {
        let shared = Arc::new(Shared::new());
        let (sender, events) = mpsc::sync_channel(1);
        let output = Events {
            sender,
            shared: shared.clone(),
            ctx: egui::Context::default(),
            generation: 1,
        };
        assert!(output.output(&[0xf0, 0x9f]));
        assert!(matches!(events.recv().unwrap(), Event::Output(bytes) if bytes == [0xf0, 0x9f]));
        assert!(output.output(&[0x98, 0x80]));
        let producer = std::thread::spawn(move || output.output(b"blocked until receiver closes"));
        shared.cancel();
        drop(events);
        assert!(!producer.join().unwrap());
    }

    #[test]
    fn close_request_releases_full_output_queue_without_dropping_receiver() {
        let shared = Arc::new(Shared::new());
        let (sender, events) = mpsc::sync_channel(1);
        let output = Events {
            sender,
            shared: shared.clone(),
            ctx: egui::Context::default(),
            generation: 1,
        };
        let session = Session {
            kind: Kind::Shell,
            shared,
            events,
        };
        assert!(output.output(b"first queued output"));
        let producer = std::thread::spawn(move || output.output(b"blocked pending output"));
        session.request_close();
        assert!(!producer.join().unwrap());
        assert!(session.is_closed());
        assert!(
            matches!(session.try_recv(),Some(Event::Output(bytes)) if bytes==b"first queued output")
        );
    }

    // This fixture is an owned copy of the Rust test executable, never a user's
    // login shell or profile. The parent must opt in before process startup.
    #[test]
    #[ignore = "Child of the explicit portable PTY fixture test"]
    fn pty_fixture_process() {
        assert_eq!(
            std::env::var("PEALAYER_TEST_PTY_FIXTURE").as_deref(),
            Ok("1")
        );
        // Session cancellation must terminate even a child ignoring hangup.
        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGHUP, libc::SIG_IGN);
        }
        let mut stdout = std::io::stdout();
        stdout
            .write_all(b"\x1b[32mPTY-FIXTURE-READY\x1b[0m\r\n")
            .unwrap();
        stdout.flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        write!(
            stdout,
            "PTY-FIXTURE-ECHO:{}\r\n",
            line.trim_end_matches(['\r', '\n'])
        )
        .unwrap();
        stdout.flush().unwrap();
    }

    fn fixture_target() -> Target {
        assert_eq!(
            std::env::var("PEALAYER_TEST_PTY_FIXTURE").as_deref(),
            Ok("1")
        );
        Target::Shell {
            program: std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            args: vec![
                "--ignored".into(),
                "--exact".into(),
                "ui::terminal_backend::tests::pty_fixture_process".into(),
                "--nocapture".into(),
            ],
            cwd: None,
            environment: Vec::new(),
            clear_environment: false,
        }
    }

    fn fixture_capture(
        session: &Session,
        chunk: &[u8],
        bytes: &mut Vec<u8>,
        parser: &mut vt100::Parser,
        query_tail: &mut [u8; 4],
    ) {
        // portable-pty enables ConPTY's INHERIT_CURSOR option. ConPTY asks
        // the terminal for its cursor before starting the child; a backend-only
        // test must emulate the terminal reply just as the real UI does.
        // This helper consumes only the explicitly owned fixture's output.
        for byte in chunk {
            parser.process(std::slice::from_ref(byte));
            query_tail.copy_within(1..4, 0);
            query_tail[3] = *byte;
            if *query_tail == *b"\x1b[6n" {
                let (row, col) = parser.screen().cursor_position();
                session
                    .send(Input::Bytes(
                        format!("\x1b[{};{}R", row + 1, col + 1).into_bytes(),
                    ))
                    .unwrap();
            }
        }
        bytes.extend_from_slice(chunk);
        // Diagnostic output must stay bounded even if a fixture goes wrong.
        if bytes.len() > 256 * 1024 {
            bytes.drain(..bytes.len() - 256 * 1024);
        }
    }

    fn fixture_diagnostics(session: &Session, ready: bool, sent: bool, bytes: &[u8]) -> String {
        let state = session.shared.state.lock().unwrap();
        let tail = &bytes[bytes.len().saturating_sub(8192)..];
        format!(
            "ready={ready}, sent={sent}, cancelled={}, finished={}, child_exited={}, output_done={}, inputs={}, shared_refs={}, output_tail={:?}",
            state.cancelled,
            state.finished,
            state.child_exited,
            state.output_done,
            state.inputs.len(),
            Arc::strong_count(&session.shared),
            String::from_utf8_lossy(tail)
        )
    }

    #[test]
    #[ignore = "Opt-in native PTY fixture; PEALAYER_TEST_PTY_FIXTURE=1 from parent"]
    fn portable_pty_preserves_unicode_resize_exit_and_owned_child_cancellation() {
        let session = Session::spawn(fixture_target(), 24, 80, egui::Context::default()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut bytes = Vec::new();
        let mut parser = vt100::Parser::new(24, 80, 100);
        let mut query_tail = [0; 4];
        let mut ready = false;
        let mut sent = false;
        let mut exited = false;
        while std::time::Instant::now() < deadline {
            while let Some(event) = session.try_recv() {
                match event {
                    Event::Ready { .. } => ready = true,
                    Event::Output(chunk) => {
                        fixture_capture(&session, &chunk, &mut bytes, &mut parser, &mut query_tail)
                    }
                    Event::Error(error) => panic!("PTY fixture failed: {error}"),
                    Event::Exited { code } => {
                        assert!(ready);
                        assert_eq!(code, Some(0));
                        exited = true;
                    }
                    Event::Catalog(_) | Event::Application(_) => {
                        panic!("OS shell must not dispatch application or controller commands")
                    }
                }
            }
            if !sent && String::from_utf8_lossy(&bytes).contains("PTY-FIXTURE-READY") {
                parser.set_size(36, 120);
                session
                    .send(Input::Resize {
                        rows: 36,
                        cols: 120,
                    })
                    .unwrap();
                session
                    .send(Input::Bytes("فارسی café\r".as_bytes().to_vec()))
                    .unwrap();
                sent = true;
            }
            if exited {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            exited,
            "Owned fixture failed to exit: {}",
            fixture_diagnostics(&session, ready, sent, &bytes)
        );
        assert!(
            String::from_utf8_lossy(&bytes).contains("PTY-FIXTURE-ECHO:فارسی café"),
            "Fixture Unicode echo differs: {}",
            fixture_diagnostics(&session, ready, sent, &bytes)
        );
        assert!(
            session
                .send(Input::Bytes(b"must not run".to_vec()))
                .is_err()
        );
        drop(session);

        // A waiting child is cancelled by closing only its owning Session.
        let session = Session::spawn(fixture_target(), 24, 80, egui::Context::default()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut bytes = Vec::new();
        let mut parser = vt100::Parser::new(24, 80, 100);
        let mut query_tail = [0; 4];
        while std::time::Instant::now() < deadline
            && !String::from_utf8_lossy(&bytes).contains("PTY-FIXTURE-READY")
        {
            if let Some(event) = session.try_recv() {
                match event {
                    Event::Output(chunk) => {
                        fixture_capture(&session, &chunk, &mut bytes, &mut parser, &mut query_tail)
                    }
                    Event::Error(error) => panic!("Cancellation fixture startup failed: {error}"),
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            String::from_utf8_lossy(&bytes).contains("PTY-FIXTURE-READY"),
            "Cancellation fixture never became ready: {}",
            fixture_diagnostics(&session, false, false, &bytes)
        );
        session.request_close();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !session.is_closed() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            session.is_closed(),
            "PTY workers or child survived session cancellation: {}",
            fixture_diagnostics(&session, true, false, &bytes)
        );
        assert!(session.shared.state.lock().unwrap().child_exited);
    }
}
