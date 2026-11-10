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

unsafe extern "C" {
    fn ps_sizeof_session() -> c_uint;
    fn ps_alignof_session() -> c_uint;
    fn ps_init(memory: *mut c_void, bytes: c_uint, config: *const Config) -> *mut Session;
    fn ps_register(session: *mut Session, command: *const Command) -> c_int;
    fn ps_set_prompt(session: *mut Session, text: *const c_char, style: c_uint) -> c_int;
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
}

impl Buffers {
    fn new() -> Self {
        Self {
            output: Vec::with_capacity(OUTPUT_LIMIT),
            output_overflow: false,
            actions: Vec::with_capacity(ACTION_LIMIT),
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
        let host = Self {
            session,
            allocation,
            layout,
            buffers,
        };
        for (name, usage) in COMMANDS {
            let command = Command {
                name: name.as_ptr(),
                usage: usage.as_ptr(),
                category: c"Pealayer".as_ptr(),
                run: Some(dispatch_command),
                complete: None,
                context,
            };
            if unsafe { ps_register(session.as_ptr(), &command) } < 0 {
                return Err("Could not register Pealayer terminal commands".into());
            }
        }
        unsafe {
            ps_set_prompt(session.as_ptr(), c"Pealayer › ".as_ptr(), 1);
            ps_write(
                session.as_ptr(),
                c"Pealayer terminal · PortableShell".as_ptr(),
            );
            ps_newline(session.as_ptr());
            ps_write(
                session.as_ptr(),
                c"Type help for commands. Playback changes require an explicit command.".as_ptr(),
            );
            ps_newline(session.as_ptr());
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
        }
    }
}

const COMMANDS: &[(&CStr, &CStr)] = &[
    (c"help", c"help — show Pealayer commands"),
    (c"status", c"status — inspect the current media state"),
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
                for (_, usage) in COMMANDS {
                    unsafe {
                        ps_write(session, usage.as_ptr());
                        ps_newline(session);
                    }
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
