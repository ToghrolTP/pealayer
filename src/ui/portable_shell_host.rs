//! Pealayer's explicit host for the pinned, allocation-free PortableShell core.
//! One owner calls this object; callbacks only collect bytes or typed intents.
//! No callback executes an application action, process, socket or filesystem IO.
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr::NonNull;

const OUTPUT_LIMIT: usize = 256 * 1024;
const ACTION_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Status,
    Play,
    Pause,
    Stop,
    Seek(f64),
    Volume(f64),
    Mute(bool),
    Preferences,
    Configure,
    Get(Option<String>),
    Set {
        key: String,
        value: String,
        expected: Option<String>,
    },
}

#[repr(C)]
struct Session {
    _opaque: [u8; 0],
}

type CommandFn =
    unsafe extern "C" fn(*mut Session, c_int, *const *const c_char, *mut c_void) -> c_int;
type CompleteFn = unsafe extern "C" fn(
    *mut Session,
    c_int,
    *const *const c_char,
    *const c_char,
    *mut *const c_char,
    c_uint,
    *mut c_void,
) -> c_uint;

#[repr(C)]
struct Config {
    write: Option<unsafe extern "C" fn(*mut c_void, *const u8, c_uint)>,
    context: *mut c_void,
    columns: c_uint,
    rows: c_uint,
    flags: c_uint,
}

#[repr(C)]
struct Command {
    name: *const c_char,
    usage: *const c_char,
    category: *const c_char,
    run: Option<CommandFn>,
    complete: Option<CompleteFn>,
    context: *mut c_void,
}
#[repr(C)]
struct PromptPart {
    text: *const c_char,
    style: c_uint,
}

unsafe extern "C" {
    fn ps_sizeof_session() -> c_uint;
    fn ps_alignof_session() -> c_uint;
    fn ps_init(memory: *mut c_void, bytes: c_uint, config: *const Config) -> *mut Session;
    fn ps_register(session: *mut Session, command: *const Command) -> c_int;
    fn ps_set_prompt_parts(session: *mut Session, parts: *const PromptPart, count: c_uint)
    -> c_int;
    fn ps_set_application_title(session: *mut Session, title: *const c_char);
    fn ps_print_help(session: *mut Session, name: *const c_char, layout: c_uint) -> c_int;
    fn ps_panel_refresh(session: *mut Session);
    fn ps_panel_active(session: *const Session) -> c_uint;
    fn ph_editor_size() -> c_uint;
    fn ph_editor_align() -> c_uint;
    fn ph_editor_init(
        editor: *mut c_void,
        write: unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *const c_char,
            *const c_char,
        ) -> c_int,
        context: *mut c_void,
    );
    fn ph_editor_value(
        editor: *mut c_void,
        name: *const c_char,
        value: *const c_char,
        kind: c_uint,
    ) -> c_int;
    fn ph_editor_open(editor: *mut c_void, session: *mut Session) -> c_int;
    fn ph_help_setup(session: *mut Session);
    fn ps_start(session: *mut Session);
    fn ps_feed(session: *mut Session, bytes: *const c_void, count: c_uint);
    fn ps_tick(session: *mut Session, monotonic_ms: c_uint);
    fn ps_resize(session: *mut Session, columns: c_uint, rows: c_uint);
    fn ps_destroy_session(session: *mut Session);
    fn ps_exited(session: *const Session) -> c_int;
    fn ps_request_exit(session: *mut Session);
    fn ps_clear(session: *mut Session);
    fn ps_write(session: *mut Session, text: *const c_char);
    fn ps_newline(session: *mut Session);
    fn ps_message(session: *mut Session, style: c_uint, text: *const c_char);
}

struct Buffers {
    output: Vec<u8>,
    output_overflow: bool,
    actions: Vec<Action>,
    completion_keys: Vec<CString>,
}

impl Buffers {
    fn new() -> Self {
        Self {
            output: Vec::with_capacity(OUTPUT_LIMIT),
            output_overflow: false,
            actions: Vec::with_capacity(ACTION_LIMIT),
            completion_keys: super::terminal_settings::KEYS
                .iter()
                .filter_map(|key| CString::new(*key).ok())
                .collect(),
        }
    }
}

/// Intentionally not Send/Sync: create and destroy on the owning worker.
pub struct Host {
    session: NonNull<Session>,
    allocation: NonNull<u8>,
    layout: Layout,
    // Stable address even when Host moves. It outlives ps_destroy_session.
    buffers: Box<Buffers>,
    editor: NonNull<u8>,
    editor_layout: Layout,
    branding: CString,
}

impl Host {
    pub fn new(rows: u16, columns: u16) -> Result<Self, String> {
        // Query actual linked ABI, not PS_DEFAULT_SESSION_STORAGE or guessed alignment.
        let bytes = unsafe { ps_sizeof_session() };
        let alignment = unsafe { ps_alignof_session() };
        if bytes == 0 || bytes > 4 * 1024 * 1024 {
            return Err("PortableShell reported an invalid session size".into());
        }
        let layout = Layout::from_size_align(bytes as usize, alignment as usize)
            .map_err(|_| "PortableShell reported an invalid session alignment")?;
        let allocation = NonNull::new(unsafe { alloc_zeroed(layout) })
            .ok_or("Could not allocate the embedded terminal")?;
        let mut buffers = Box::new(Buffers::new());
        let context = (&mut *buffers as *mut Buffers).cast::<c_void>();
        let config = Config {
            write: Some(collect_output),
            context,
            columns: u32::from(columns.clamp(8, 240)),
            rows: u32::from(rows.clamp(4, 80)),
            // Interactive, color, Unicode, truecolor, case-insensitive, no builtins,
            // extended keys, no OSC (terminal title/clipboard are application-owned).
            flags: 1 | 2 | 4 | 16 | 32 | 64 | 1024 | 4096,
        };
        let Some(session) =
            NonNull::new(unsafe { ps_init(allocation.as_ptr().cast(), bytes, &config) })
        else {
            unsafe { dealloc(allocation.as_ptr(), layout) };
            return Err("Could not initialize the embedded terminal".into());
        };
        let editor_layout = Layout::from_size_align(unsafe { ph_editor_size() } as usize, unsafe {
            ph_editor_align()
        }
            as usize)
        .map_err(|_| "Invalid settings editor ABI")?;
        let editor = NonNull::new(unsafe { alloc_zeroed(editor_layout) })
            .ok_or("Could not allocate settings editor")?;
        unsafe {
            ph_editor_init(editor.as_ptr().cast(), queue_setting, context);
        }
        let host = Self {
            session,
            allocation,
            layout,
            buffers,
            editor,
            editor_layout,
            branding: CString::new("Pealayer").unwrap(),
        };
        for (name, usage) in COMMANDS {
            let command = Command {
                name: name.as_ptr(),
                usage: usage.as_ptr(),
                category: c"Pealayer".as_ptr(),
                run: Some(dispatch_command),
                complete: Some(complete_setting),
                context,
            };
            if unsafe { ps_register(session.as_ptr(), &command) } < 0 {
                return Err("Could not register Pealayer terminal commands".into());
            }
        }
        unsafe {
            ph_help_setup(session.as_ptr());
            ps_set_application_title(session.as_ptr(), c"Pealayer · Command workspace".as_ptr());
            let parts = [
                PromptPart {
                    text: c"╭─ ".as_ptr(),
                    style: 6,
                },
                PromptPart {
                    text: c"Pealayer".as_ptr(),
                    style: 7,
                },
                PromptPart {
                    text: c" · host\n╰─❯ ".as_ptr(),
                    style: 6,
                },
            ];
            ps_set_prompt_parts(session.as_ptr(), parts.as_ptr(), parts.len() as u32);
            ps_start(session.as_ptr());
        }
        Ok(host)
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        if !self.exited() {
            for chunk in bytes.chunks(4096) {
                unsafe {
                    ps_feed(
                        self.session.as_ptr(),
                        chunk.as_ptr().cast(),
                        chunk.len() as u32,
                    )
                };
                if self.exited() {
                    break;
                }
            }
        }
    }

    pub fn tick(&mut self, monotonic_ms: u32) {
        if !self.exited() {
            unsafe { ps_tick(self.session.as_ptr(), monotonic_ms) };
        }
    }

    pub fn resize(&mut self, rows: u16, columns: u16) {
        unsafe {
            ps_resize(
                self.session.as_ptr(),
                u32::from(columns.clamp(8, 240)),
                u32::from(rows.clamp(4, 80)),
            );
        }
    }

    pub fn take_output(&mut self) -> Vec<u8> {
        // Retain callback capacity across idle ticks instead of allocating a
        // quarter-megabyte replacement every time the worker checks output.
        let mut output = self.buffers.output.clone();
        self.buffers.output.clear();
        if std::mem::take(&mut self.buffers.output_overflow) {
            output.extend_from_slice(b"\x18\x1b[0m\r\n[Terminal output limit reached; some display bytes were omitted.]\r\n");
        }
        output
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        let actions = self.buffers.actions.clone();
        self.buffers.actions.clear();
        actions
    }

    pub fn exited(&self) -> bool {
        unsafe { ps_exited(self.session.as_ptr()) != 0 }
    }
    pub fn table_active(&self) -> bool {
        unsafe { ps_panel_active(self.session.as_ptr()) != 0 }
    }
    pub fn clear_notice(&mut self) {
        if self.table_active() {
            unsafe {
                ps_message(self.session.as_ptr(), 0, c"".as_ptr());
            }
        }
    }
    pub fn branding(&mut self, name: &str) {
        let name: String = name.chars().filter(|c| !c.is_control()).take(40).collect();
        let Ok(name) = CString::new(name) else {
            return;
        };
        if name == self.branding {
            return;
        }
        self.branding = name;
        let parts = [
            PromptPart {
                text: c"╭─ ".as_ptr(),
                style: 6,
            },
            PromptPart {
                text: self.branding.as_ptr(),
                style: 7,
            },
            PromptPart {
                text: c" · host\n╰─❯ ".as_ptr(),
                style: 6,
            },
        ];
        unsafe {
            ps_set_application_title(self.session.as_ptr(), self.branding.as_ptr());
            ps_set_prompt_parts(self.session.as_ptr(), parts.as_ptr(), parts.len() as u32);
        }
    }
    pub fn settings(&mut self, values: &[(String, String)], open: bool) {
        for (key, value) in values {
            let kind = if matches!(value.as_str(), "true" | "false") {
                0
            } else if key == "ui.terminal.scrollback_lines" {
                1
            } else {
                4
            };
            if let (Ok(key), Ok(value)) = (CString::new(key.as_str()), CString::new(value.as_str()))
            {
                unsafe {
                    ph_editor_value(
                        self.editor.as_ptr().cast(),
                        key.as_ptr(),
                        value.as_ptr(),
                        kind,
                    );
                }
            }
        }
        unsafe {
            if open {
                ph_editor_open(self.editor.as_ptr().cast(), self.session.as_ptr());
            } else {
                ps_panel_refresh(self.session.as_ptr());
            }
        }
    }

    /// Owner-loop completion/readback. Safe output strips controls and preserves input.
    pub fn response(&mut self, text: &str, is_error: bool) {
        let mut bounded: String = text.chars().take(8192).filter(|ch| *ch != '\0').collect();
        if text.chars().count() > 8192 {
            bounded.push_str(" … [response truncated]");
        }
        // ps_message deliberately strips embedded control characters. Submit
        // logical lines separately instead of losing line boundaries, while
        // letting the core suspend/redraw the current unfinished prompt.
        for line in bounded.lines() {
            if let Ok(text) = CString::new(line) {
                unsafe {
                    ps_message(
                        self.session.as_ptr(),
                        if is_error { 4 } else { 0 },
                        text.as_ptr(),
                    )
                };
            }
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // Restore terminal modes and scrub history/drafts, without ps_finish:
        // disconnect must never execute the user's unterminated command.
        unsafe {
            ps_destroy_session(self.session.as_ptr());
            dealloc(self.allocation.as_ptr(), self.layout);
            dealloc(self.editor.as_ptr(), self.editor_layout);
        }
    }
}

const COMMANDS: &[(&CStr, &CStr)] = &[
    (c"help", c"help — show Pealayer commands"),
    (c"status", c"status — inspect the current media state"),
    (
        c"configure",
        c"configure — edit live Pealayer settings in a table",
    ),
    (c"get", c"get [KEY] — inspect editable settings"),
    (
        c"set",
        c"set KEY VALUE — validate, apply and save a setting",
    ),
    (c"play", c"play — resume playback"),
    (c"pause", c"pause — pause playback"),
    (c"stop", c"stop — stop playback"),
    (
        c"seek",
        c"seek SECONDS — seek to an absolute nonnegative position",
    ),
    (c"volume", c"volume PERCENT — set volume from 0 to 100"),
    (c"mute", c"mute on|off — explicitly set mute"),
    (c"preferences", c"preferences — open Pealayer Preferences"),
    (c"clear", c"clear — clear the terminal display"),
    (c"exit", c"exit — close this terminal session, not Pealayer"),
];

unsafe extern "C" fn collect_output(context: *mut c_void, bytes: *const u8, count: c_uint) {
    if context.is_null() || bytes.is_null() {
        return;
    }
    let buffers = unsafe { &mut *context.cast::<Buffers>() };
    let bytes = unsafe { std::slice::from_raw_parts(bytes, count as usize) };
    let accepted = bytes.len().min(OUTPUT_LIMIT - buffers.output.len());
    // Capacity is preallocated; no allocation or reentry across the C boundary.
    buffers.output.extend_from_slice(&bytes[..accepted]);
    buffers.output_overflow |= accepted != bytes.len();
}

unsafe extern "C" fn queue_setting(
    context: *mut c_void,
    key: *const c_char,
    value: *const c_char,
    expected: *const c_char,
) -> c_int {
    if context.is_null() || key.is_null() || value.is_null() || expected.is_null() {
        return 0;
    }
    let buffers = unsafe { &mut *context.cast::<Buffers>() };
    if buffers.actions.len() == ACTION_LIMIT {
        return 0;
    }
    let strings = unsafe {
        (
            CStr::from_ptr(key).to_str(),
            CStr::from_ptr(value).to_str(),
            CStr::from_ptr(expected).to_str(),
        )
    };
    let (Ok(key), Ok(value), Ok(expected)) = strings else {
        return 0;
    };
    buffers.actions.push(Action::Set {
        key: key.into(),
        value: value.into(),
        expected: Some(expected.into()),
    });
    1
}
unsafe extern "C" fn complete_setting(
    _session: *mut Session,
    argc: c_int,
    argv: *const *const c_char,
    prefix: *const c_char,
    out: *mut *const c_char,
    capacity: c_uint,
    context: *mut c_void,
) -> c_uint {
    if context.is_null() || argv.is_null() || prefix.is_null() || out.is_null() || argc < 1 {
        return 0;
    }
    let command = unsafe { CStr::from_ptr(*argv) };
    if command != c"get" && command != c"set" {
        return 0;
    }
    let prefix = unsafe { CStr::from_ptr(prefix) }.to_bytes();
    let buffers = unsafe { &*context.cast::<Buffers>() };
    let mut count = 0;
    for key in &buffers.completion_keys {
        if count == capacity {
            break;
        }
        if key.as_bytes().starts_with(prefix) {
            unsafe {
                *out.add(count as usize) = key.as_ptr();
            }
            count += 1;
        }
    }
    count
}

unsafe extern "C" fn dispatch_command(
    session: *mut Session,
    argc: c_int,
    argv: *const *const c_char,
    context: *mut c_void,
) -> c_int {
    if context.is_null() || argv.is_null() || !(1..=32).contains(&argc) {
        return 1;
    }
    let pointers = unsafe { std::slice::from_raw_parts(argv, argc as usize) };
    let mut args = [""; 32];
    for (slot, pointer) in args.iter_mut().zip(pointers) {
        if pointer.is_null() {
            return 1;
        }
        let Ok(text) = unsafe { CStr::from_ptr(*pointer) }.to_str() else {
            return 1;
        };
        *slot = text;
    }
    let args = &args[..argc as usize];
    if args.len() == 1 {
        match args[0] {
            "help" => {
                unsafe {
                    ps_print_help(session, std::ptr::null(), 0);
                }
                return 0;
            }
            "clear" => {
                unsafe { ps_clear(session) };
                return 0;
            }
            "exit" => {
                unsafe { ps_request_exit(session) };
                return 0;
            }
            _ => {}
        }
    }
    if let ["help", name] = args {
        if let Ok(name) = CString::new(*name) {
            unsafe {
                ps_print_help(session, name.as_ptr(), 0);
            }
        }
        return 0;
    }
    let action = parse_action(args);
    match action {
        Ok(action) => {
            // Release queue borrow before ps_write can synchronously call the sink.
            let queued = {
                let buffers = unsafe { &mut *context.cast::<Buffers>() };
                if buffers.actions.len() == ACTION_LIMIT {
                    false
                } else {
                    buffers.actions.push(action);
                    true
                }
            };
            if !queued {
                unsafe {
                    ps_write(
                        session,
                        c"Command queue is full; wait for pending commands, then retry.".as_ptr(),
                    );
                    ps_newline(session);
                }
                return 1;
            }
            0
        }
        Err(error) => {
            unsafe {
                ps_write(session, error.as_ptr());
                ps_newline(session);
            }
            1
        }
    }
}

fn parse_action(args: &[&str]) -> Result<Action, &'static CStr> {
    let number = |text: &str| {
        text.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n >= 0.0)
    };
    match args {
        ["status"] => Ok(Action::Status),
        ["play"] => Ok(Action::Play),
        ["pause"] => Ok(Action::Pause),
        ["stop"] => Ok(Action::Stop),
        ["preferences"] => Ok(Action::Preferences),
        ["configure"] => Ok(Action::Configure),
        ["get"] => Ok(Action::Get(None)),
        ["get", key] => Ok(Action::Get(Some((*key).into()))),
        ["set", key, value] => Ok(Action::Set {
            key: (*key).into(),
            value: (*value).into(),
            expected: None,
        }),
        ["seek", seconds] => number(seconds)
            .map(Action::Seek)
            .ok_or(c"Seek needs a finite, nonnegative number of seconds."),
        ["volume", percent] => number(percent)
            .filter(|n| *n <= 100.0)
            .map(Action::Volume)
            .ok_or(c"Volume must be a number between 0 and 100."),
        ["mute", "on"] => Ok(Action::Mute(true)),
        ["mute", "off"] => Ok(Action::Mute(false)),
        _ => Err(c"Invalid command arguments. Type help for accepted commands."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rich_help_multiline_prompt_and_settings_table_use_the_pinned_core() {
        let mut host = Host::new(30, 100).unwrap();
        let mut parser = vt100::Parser::new(30, 100, 100);
        parser.process(&host.take_output());
        assert!(parser.screen().contents().contains("╭─"));
        host.feed(b"help set\r");
        parser.process(&host.take_output());
        assert!(parser.screen().contents().contains("VALUE"));
        host.settings(&[("ui.terminal.font_size".into(), "13".into())], true);
        host.tick(100);
        parser.process(&host.take_output());
        assert!(parser.screen().alternate_screen());
        assert!(parser.screen().contents().contains("ui.terminal.font_size"));
        host.feed(b"\r\x1514\r");
        assert_eq!(
            host.take_actions(),
            vec![Action::Set {
                key: "ui.terminal.font_size".into(),
                value: "14".into(),
                expected: Some("13".into())
            }]
        );
        host.settings(&[("ui.terminal.font_size".into(), "14".into())], false);
        host.tick(200);
        parser.process(&host.take_output());
        assert!(parser.screen().contents().contains("14"));
    }

    #[test]
    fn startup_is_pure_and_commands_are_typed_not_host_executed() {
        let mut host = Host::new(24, 80).unwrap();
        assert!(host.take_actions().is_empty());
        assert!(String::from_utf8_lossy(&host.take_output()).contains("Pealayer"));
        host.feed(b"play\rpause\rseek 12.5\rvolume 42\rmute on\rpreferences\rstatus\r");
        assert_eq!(
            host.take_actions(),
            vec![
                Action::Play,
                Action::Pause,
                Action::Seek(12.5),
                Action::Volume(42.0),
                Action::Mute(true),
                Action::Preferences,
                Action::Status
            ]
        );
        host.feed(b"powershell\rvolume 101\rseek NaN\rplay extra\rmute toggle\r");
        assert!(host.take_actions().is_empty());
    }

    #[test]
    fn unterminated_input_cancellation_and_sessions_are_isolated() {
        let mut host = Host::new(24, 80).unwrap();
        host.feed(b"play");
        host.tick(100);
        assert!(host.take_actions().is_empty());
        drop(host); // Must not execute a final unterminated command.
        let mut next = Host::new(24, 80).unwrap();
        next.feed(b"\r");
        assert!(next.take_actions().is_empty());
        next.feed(b"exit\rplay\r");
        assert!(next.exited());
        assert!(next.take_actions().is_empty());
    }
    #[test]
    fn bracketed_paste_cannot_submit_playback_commands() {
        let mut host = Host::new(24, 80).unwrap();
        host.take_output();
        host.feed(b"\x1b[200~play\r\npause\n\x1b[201~");
        assert!(host.take_actions().is_empty());
        host.feed(&[3]);
        assert!(host.take_actions().is_empty());
    }
    #[test]
    fn settings_toggles_wait_for_owner_readback_without_duplicate_writes() {
        let mut host = Host::new(30, 100).unwrap();
        host.take_output();
        host.settings(&[("ui.terminal.cursor_blink".into(), "true".into())], true);
        host.tick(100);
        host.feed(b" ");
        assert_eq!(
            host.take_actions(),
            vec![Action::Set {
                key: "ui.terminal.cursor_blink".into(),
                value: "false".into(),
                expected: Some("true".into())
            }]
        );
        host.feed(b" ");
        assert!(host.take_actions().is_empty());
        host.settings(
            &[("ui.terminal.cursor_blink".into(), "false".into())],
            false,
        );
        host.tick(300);
        host.feed(b" ");
        assert_eq!(
            host.take_actions(),
            vec![Action::Set {
                key: "ui.terminal.cursor_blink".into(),
                value: "true".into(),
                expected: Some("false".into())
            }]
        );
    }

    #[test]
    fn callbacks_bound_actions_and_output_and_report_overflow() {
        let mut host = Host::new(24, 80).unwrap();
        for _ in 0..100 {
            host.feed(b"play\r");
        }
        assert_eq!(host.take_actions().len(), ACTION_LIMIT);
        assert!(String::from_utf8_lossy(&host.take_output()).contains("queue is full"));
        for _ in 0..1500 {
            host.feed(b"help\r");
        }
        assert!(host.buffers.output.len() <= OUTPUT_LIMIT);
        let output = host.take_output();
        assert!(String::from_utf8_lossy(&output).contains("output limit reached"));
        assert!(host.take_actions().is_empty());
    }

    #[test]
    fn replies_preserve_pending_input_and_resize_is_bounded() {
        let mut host = Host::new(0, u16::MAX).unwrap();
        host.feed(b"vol");
        host.branding("Pealayer review");
        host.feed(&[12]); // Clear/redraw must preserve the unfinished prompt.
        host.response("Current volume: 35", false);
        host.resize(u16::MAX, 0);
        host.feed(b"ume 25\r");
        assert_eq!(host.take_actions(), vec![Action::Volume(25.0)]);
    }

    #[test]
    fn multiline_status_keeps_lines_and_unfinished_prompt_separate() {
        let mut host = Host::new(24, 80).unwrap();
        let mut parser = vt100::Parser::new(24, 80, 100);
        parser.process(&host.take_output());
        host.feed(b"vol");
        host.response("Playback: idle\nPosition: 0.000 s\nVolume: 100%", false);
        parser.process(&host.take_output());
        let text = parser.screen().contents();
        assert!(
            text.lines().any(|line| line.trim() == "Playback: idle"),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line.trim() == "Position: 0.000 s"),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line.trim() == "Volume: 100%"),
            "{text}"
        );
        host.feed(b"ume 25\r");
        assert_eq!(host.take_actions(), [Action::Volume(25.0)]);
    }

    #[test]
    fn core_canonicalizes_case_insensitive_registered_command_names() {
        let mut host = Host::new(24, 80).unwrap();
        host.take_output();
        host.feed(b"HELP\rPlAy\rPAUSE\rVoLuMe 30\r");
        assert_eq!(
            host.take_actions(),
            vec![Action::Play, Action::Pause, Action::Volume(30.0)]
        );
        assert!(String::from_utf8_lossy(&host.take_output()).contains("seek SECONDS"));
    }
}
