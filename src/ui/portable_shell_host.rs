//! Pealayer's explicit host for the pinned, allocation-free PortableShell core.
//! One owner calls this object; callbacks only collect bytes or typed intents.
//! No callback executes an application action, process, socket or filesystem IO.
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr::NonNull;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    EditSetting(String),
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
#[repr(C)]
struct ReportRow {
    label: *const c_char,
    annotation: *const c_char,
    value: *const c_char,
    flags: c_uint,
}
#[repr(C)]
struct EditorDefinition {
    key: *const c_char,
    label: *const c_char,
    page: *const c_char,
    group: *const c_char,
    choices: *const *const c_char,
    choice_labels: *const *const c_char,
    choice_count: c_uint,
    kind: c_uint,
    read_only: c_uint,
}
struct OwnedSetting {
    key: CString,
    label: CString,
    page: CString,
    group: CString,
    _values: Vec<CString>,
    _labels: Vec<CString>,
    values: Vec<*const c_char>,
    labels: Vec<*const c_char>,
    kind: u32,
    read_only: bool,
}
impl OwnedSetting {
    fn matches(&self, setting: &super::terminal_settings::Setting) -> bool {
        self.key.as_bytes() == setting.key.as_bytes()
            && self.label.as_bytes() == setting.label.as_bytes()
            && self.page.as_bytes() == setting.page.as_bytes()
            && self.group.as_bytes() == setting.group.as_bytes()
            && self.kind == setting.kind
            && self.read_only == setting.read_only
            && self._values.len() == setting.choices.len()
            && self
                ._values
                .iter()
                .zip(&self._labels)
                .zip(&setting.choices)
                .all(|((value, label), (v, l))| {
                    value.as_bytes() == v.as_bytes() && label.as_bytes() == l.as_bytes()
                })
    }
    fn new(setting: &super::terminal_settings::Setting) -> Result<Self, String> {
        let string =
            |value: &str| CString::new(value).map_err(|_| "Setting contains NUL".to_string());
        let values = setting
            .choices
            .iter()
            .map(|(value, _)| string(value))
            .collect::<Result<Vec<_>, _>>()?;
        let labels = setting
            .choices
            .iter()
            .map(|(_, label)| string(label))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            key: string(&setting.key)?,
            label: string(&setting.label)?,
            page: string(&setting.page)?,
            group: string(&setting.group)?,
            values: values.iter().map(|value| value.as_ptr()).collect(),
            labels: labels.iter().map(|value| value.as_ptr()).collect(),
            _values: values,
            _labels: labels,
            kind: setting.kind,
            read_only: setting.read_only,
        })
    }
    fn definition(&self) -> EditorDefinition {
        EditorDefinition {
            key: self.key.as_ptr(),
            label: self.label.as_ptr(),
            page: self.page.as_ptr(),
            group: self.group.as_ptr(),
            choices: if self.kind == 3 {
                self.values.as_ptr()
            } else {
                std::ptr::null()
            },
            choice_labels: if self.kind == 3 {
                self.labels.as_ptr()
            } else {
                std::ptr::null()
            },
            choice_count: if self.kind == 3 {
                self.values.len() as u32
            } else {
                0
            },
            kind: self.kind,
            read_only: u32::from(self.read_only),
        }
    }
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
        open: unsafe extern "C" fn(*mut c_void, *const c_char),
        context: *mut c_void,
    );
    fn ph_editor_value(editor: *mut c_void, name: *const c_char, value: *const c_char) -> c_int;
    fn ph_editor_define(editor: *mut c_void, definition: *const EditorDefinition) -> c_int;
    fn ps_panel_attach(session: *mut Session, panel: *const c_void) -> c_int;
    fn ph_editor_open(editor: *mut c_void, session: *mut Session) -> c_int;
    fn ph_help_setup(session: *mut Session);
    fn ph_shell_style(session: *mut Session);
    fn ps_start(session: *mut Session);
    fn ps_feed(session: *mut Session, bytes: *const c_void, count: c_uint);
    fn ps_tick(session: *mut Session, monotonic_ms: c_uint);
    fn ps_resize(session: *mut Session, columns: c_uint, rows: c_uint);
    fn ps_destroy_session(session: *mut Session);
    fn ps_exited(session: *const Session) -> c_int;
    fn ps_request_exit(session: *mut Session);
    fn ps_clear(session: *mut Session);
    fn ps_message(session: *mut Session, style: c_uint, text: *const c_char);
    fn ps_message_block(
        session: *mut Session,
        style: c_uint,
        text: *const c_char,
        bytes: c_uint,
    ) -> c_uint;
    fn ps_message_report(
        session: *mut Session,
        rows: *const ReportRow,
        count: c_uint,
        style: c_uint,
    ) -> c_uint;
    fn ps_set_status(session: *mut Session, status: c_int);
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
            completion_keys: crate::preferences_contract::preference_controls(
                &crate::config::AppConfig::default(),
            )
            .iter()
            .filter_map(|entry| CString::new(entry.key).ok())
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
    prompt_status: String,
    columns: u16,
    catalog: Vec<OwnedSetting>,
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
            // extended keys. Allow the explicit semantic OSC marker; the VT
            // renderer ignores title/clipboard OSC and has no OS side effects.
            flags: 1 | 2 | 4 | 16 | 32 | 64 | 1024,
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
            ph_editor_init(
                editor.as_ptr().cast(),
                queue_setting,
                queue_open_setting,
                context,
            );
        }
        let mut host = Self {
            session,
            allocation,
            layout,
            buffers,
            editor,
            editor_layout,
            branding: CString::new("Pealayer Shell").unwrap(),
            prompt_status: "■ Idle".into(),
            columns: config.columns as u16,
            catalog: Vec::new(),
        };
        for (name, usage) in COMMANDS {
            let command = Command {
                name: name.as_ptr(),
                usage: usage.as_ptr(),
                category: match name.to_bytes() {
                    b"play" | b"pause" | b"stop" | b"seek" => c"Playback".as_ptr(),
                    b"volume" | b"mute" => c"Audio".as_ptr(),
                    b"get" | b"set" | b"configure" | b"preferences" => c"Preferences".as_ptr(),
                    _ => c"Session".as_ptr(),
                },
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
            ph_shell_style(session.as_ptr());
        }
        host.update_prompt();
        unsafe {
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
        let columns = columns.clamp(8, 240);
        let previous = self.columns;
        // Prepare shorter prompt lines before the core repaints a smaller grid.
        if columns < previous {
            self.columns = columns;
            self.update_prompt();
        }
        unsafe {
            ps_resize(
                self.session.as_ptr(),
                u32::from(columns),
                u32::from(rows.clamp(4, 80)),
            );
        }
        if columns > previous {
            self.columns = columns;
            self.update_prompt();
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
        let Ok(name) = CString::new(format!("{name} Shell")) else {
            return;
        };
        if name == self.branding {
            return;
        }
        self.branding = name;
        self.update_prompt();
    }
    pub fn prompt_status(&mut self, text: &str) {
        let text: String = text.chars()
            .filter(|c| !c.is_control() || *c == '\n').take(200).collect();
        if text == self.prompt_status {
            return;
        }
        self.prompt_status = text;
        self.update_prompt();
    }
    fn update_prompt(&mut self) {
        let (summary, activity) = self.prompt_status.split_once('\n')
            .unwrap_or((&self.prompt_status, ""));
        let brand = self.branding.to_str().unwrap_or("Pealayer Shell");
        // Keep each physical row below both the renderer's wrap margin and the
        // core's three-cell reserve. The core only tracks explicit newlines.
        let width = usize::from(self.columns).saturating_sub(7);
        let inline = brand.width() + 3 + summary.width() <= width;
        // The upstream prompt buffer is 256 UTF-8 bytes. Leave room for
        // styled prefixes, separators and the input marker.
        let brand = bounded_prompt_part(brand, width, 88);
        let summary = bounded_prompt_part(summary, width, 88);
        let activity = bounded_prompt_part(activity, width, 48);
        let mut parts = vec![
            prompt_part(c"╭─ ", 6),
            prompt_part(&brand, 7),
            prompt_part(if inline { c" · " } else { c"\n├─ " }, 6),
            prompt_part(&summary, 0),
        ];
        if !activity.to_bytes().is_empty() {
            parts.extend([prompt_part(c"\n├─ ", 6), prompt_part(&activity, 3)]);
        }
        parts.extend([prompt_part(c"\n╰─", 6), prompt_part(c"❯ ", 9)]);
        unsafe {
            ps_set_application_title(self.session.as_ptr(), self.branding.as_ptr());
            ps_set_prompt_parts(self.session.as_ptr(), parts.as_ptr(), parts.len() as u32);
        }
    }
    pub fn settings(
        &mut self,
        values: &[super::terminal_settings::Setting],
        open: bool,
    ) -> Result<(), String> {
        let same = self.catalog.len() == values.len()
            && self
                .catalog
                .iter()
                .zip(values)
                .all(|(owned, value)| owned.matches(value));
        let rebuild = open || (!self.table_active() && !same);
        if rebuild {
            let catalog = values
                .iter()
                .map(OwnedSetting::new)
                .collect::<Result<Vec<_>, _>>()?;
            // Detach before changing borrowed metadata or registry membership.
            unsafe {
                ps_panel_attach(self.session.as_ptr(), std::ptr::null());
            }
            self.catalog = catalog;
            unsafe {
                ph_editor_init(
                    self.editor.as_ptr().cast(),
                    queue_setting,
                    queue_open_setting,
                    (&mut *self.buffers as *mut Buffers).cast(),
                );
            }
            for entry in &self.catalog {
                if unsafe { ph_editor_define(self.editor.as_ptr().cast(), &entry.definition()) }
                    == 0
                {
                    return Err("Settings catalog exceeded the supported page/field bounds".into());
                }
            }
            self.buffers.completion_keys =
                self.catalog.iter().map(|entry| entry.key.clone()).collect();
        }
        for entry in values {
            let key = CString::new(entry.key.as_str()).map_err(|_| "Invalid setting key")?;
            let value = CString::new(entry.value.as_str()).map_err(|_| "Invalid setting value")?;
            if unsafe { ph_editor_value(self.editor.as_ptr().cast(), key.as_ptr(), value.as_ptr()) }
                == 0
                && rebuild
            {
                return Err(format!("Setting could not be represented: {}", entry.key));
            }
        }
        if open {
            if unsafe { ph_editor_open(self.editor.as_ptr().cast(), self.session.as_ptr()) } == 0 {
                return Err("Settings editor could not open".into());
            }
        } else {
            unsafe {
                ps_panel_refresh(self.session.as_ptr());
            }
        }
        Ok(())
    }
    #[cfg(test)]
    fn settings_pairs(&mut self, pairs: &[(String, String)], open: bool) {
        let mut values = super::terminal_settings::snapshot(&crate::config::AppConfig::default());
        values.retain(|entry| pairs.iter().any(|(key, _)| *key == entry.key));
        for entry in &mut values {
            entry.value = pairs
                .iter()
                .find(|(key, _)| *key == entry.key)
                .unwrap()
                .1
                .clone();
        }
        self.settings(&values, open).unwrap();
    }

    /// Owner-loop completion/readback. Safe output strips controls and preserves input.
    pub fn response(&mut self, text: &str, is_error: bool) {
        unsafe {
            ps_set_status(self.session.as_ptr(), c_int::from(is_error));
        }
        let mut bounded: String = text.chars().take(8192).filter(|ch| *ch != '\0').collect();
        if text.chars().count() > 8192 {
            bounded.push_str(" … [response truncated]");
        }
        // One whole reply owns one suspend/redraw, including all logical lines.
        if let Ok(text) = CString::new(format!("{} {bounded}", if is_error { "✕" } else { "✓" }))
        {
            unsafe {
                ps_message_block(
                    self.session.as_ptr(),
                    if is_error { 4 } else { 2 },
                    text.as_ptr(),
                    text.as_bytes().len() as u32,
                );
            }
        }
    }
    pub fn status_report(&mut self, status: &super::shell_status::Status) {
        const SECTION: u32 = 1;
        const NUMBER: u32 = 4;
        const DIVIDER: u32 = 8;
        const SUCCESS: u32 = 16;
        const WARNING: u32 = 32;
        const ERROR: u32 = 64;
        const INACTIVE: u32 = 128;
        let positions = CString::new(if status.loaded {
            format!(
                "{} / {}",
                super::shell_status::precise_time(status.position),
                super::shell_status::precise_time(status.duration)
            )
        } else {
            "—".into()
        })
        .unwrap();
        let audio = CString::new(status.audio()).unwrap();
        let state = CString::new(status.state()).unwrap();
        let update = CString::new(status.update_line()).unwrap();
        let row = |label: &CStr, value: &CStr, flags| ReportRow {
            label: label.as_ptr(),
            annotation: std::ptr::null(),
            value: value.as_ptr(),
            flags,
        };
        let rows = [
            row(c"Playback", c"", SECTION),
            row(
                c"State",
                &state,
                if status.playing { SUCCESS } else { INACTIVE },
            ),
            row(
                c"Position",
                &positions,
                if status.loaded { NUMBER } else { INACTIVE },
            ),
            row(c"Audio", &audio, NUMBER),
            row(c"Hardware", c"", SECTION | DIVIDER),
            row(
                c"Controller",
                if status.controller {
                    c"● Connected"
                } else {
                    c"○ Disconnected"
                },
                if status.controller { SUCCESS } else { INACTIVE },
            ),
            row(c"Application", c"", SECTION | DIVIDER),
            row(
                c"Update",
                &update,
                match status.update.state.as_str() {
                    "completed" | "current" => SUCCESS,
                    "failed" => ERROR,
                    _ if status.update.active() => WARNING,
                    _ => INACTIVE,
                },
            ),
        ];
        unsafe {
            ps_message_report(
                self.session.as_ptr(),
                rows.as_ptr(),
                rows.len() as u32,
                1 | 4,
            );
        }
    }
    pub fn pending(&mut self, text: &str) {
        let text: String = text.chars().filter(|c| !c.is_control()).take(200).collect();
        if let Ok(text) = CString::new(format!("… {text}")) {
            unsafe {
                ps_message(self.session.as_ptr(), 3, text.as_ptr());
            }
        }
    }
}

fn prompt_part(text: &CStr, style: u32) -> PromptPart {
    PromptPart { text: text.as_ptr(), style }
}

fn bounded_prompt_part(text: &str, width: usize, max_bytes: usize) -> CString {
    if text.width() <= width && text.len() <= max_bytes {
        return CString::new(text).unwrap();
    }
    let mut used = 0;
    let mut clipped = String::new();
    for ch in text.chars() {
        let next = used + ch.width().unwrap_or(0);
        if next >= width || clipped.len() + ch.len_utf8() + '…'.len_utf8() > max_bytes {
            break;
        }
        clipped.push(ch);
        used = next;
    }
    clipped.push('…');
    CString::new(clipped).unwrap()
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
    (c"status", c"status — inspect playback, hardware and app update"),
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
    (c"volume", c"volume PERCENT — set volume from 0 to 130"),
    (c"mute", c"mute on|off — explicitly set mute"),
    (
        c"preferences",
        c"preferences [KEY] — open Preferences or a setting's native editor",
    ),
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

unsafe extern "C" fn queue_open_setting(context: *mut c_void, key: *const c_char) {
    if context.is_null() || key.is_null() {
        return;
    }
    let buffers = unsafe { &mut *context.cast::<Buffers>() };
    if buffers.actions.len() >= ACTION_LIMIT {
        return;
    }
    if let Ok(key) = unsafe { CStr::from_ptr(key) }.to_str() {
        buffers.actions.push(Action::EditSetting(key.into()));
    }
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
    if command != c"get" && command != c"set" && command != c"preferences" {
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
                unsafe {
                    ps_message(session, 2, c"✓ Shell session closed".as_ptr());
                    ps_request_exit(session)
                };
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
            // Release the queue borrow before printing can call the byte sink.
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
                    ps_message(
                        session,
                        4,
                        c"✕ Command queue is full; wait for pending commands, then retry.".as_ptr(),
                    );
                }
                return 1;
            }
            0
        }
        Err(error) => {
            if let Ok(message) = CString::new(format!("✕ {}", error.to_string_lossy())) {
                unsafe {
                    ps_message(session, 4, message.as_ptr());
                }
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
        ["preferences", key] => Ok(Action::EditSetting((*key).into())),
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
            .filter(|n| *n <= 130.0)
            .map(Action::Volume)
            .ok_or(c"Volume must be a number between 0 and 130."),
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
        host.settings_pairs(&[("ui.terminal.font_size".into(), "13".into())], true);
        host.tick(100);
        parser.process(&host.take_output());
        assert!(parser.screen().alternate_screen());
        assert!(parser.screen().contents().contains("Font size"));
        host.feed(b"\r\x1514\r");
        assert_eq!(
            host.take_actions(),
            vec![Action::Set {
                key: "ui.terminal.font_size".into(),
                value: "14".into(),
                expected: Some("13".into())
            }]
        );
        host.settings_pairs(&[("ui.terminal.font_size".into(), "14".into())], false);
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
        host.feed(b"powershell\rvolume 131\rseek NaN\rplay extra\rmute toggle\r");
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
        host.settings_pairs(&[("ui.terminal.cursor_blink".into(), "true".into())], true);
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
        host.settings_pairs(
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
    fn settings_use_friendly_labels_and_switch_to_distinct_stable_keys() {
        let mut host = Host::new(30, 110).unwrap();
        let mut parser = vt100::Parser::new(30, 110, 0);
        parser.process(&host.take_output());
        host.settings_pairs(&[("color_palette".into(), "studio".into())], true);
        host.tick(100);
        parser.process(&host.take_output());
        assert!(parser.screen().contents().contains("Color palette"));
        assert!(!parser.screen().contents().contains("color_palette"));
        host.feed(b"\x1b[17~");
        host.tick(300);
        parser.process(&host.take_output());
        assert!(parser.screen().contents().contains("color_palette"));
        host.feed(b"\x1b[17~");
        host.tick(500);
        parser.process(&host.take_output());
        assert!(parser.screen().contents().contains("Color palette"));
        assert!(
            host.take_actions().is_empty(),
            "presentation must never write a setting"
        );
    }

    #[test]
    fn complete_preferences_catalog_opens_and_routes_every_section_page() {
        let values =
            super::super::terminal_settings::snapshot(&crate::config::AppConfig::default());
        let mut firsts = Vec::new();
        for entry in &values {
            if firsts
                .last()
                .is_none_or(|previous: &&super::super::terminal_settings::Setting| {
                    previous.page != entry.page
                })
            {
                firsts.push(entry);
            }
        }
        let mut host = Host::new(45, 160).unwrap();
        let mut parser = vt100::Parser::new(45, 160, 0);
        parser.process(&host.take_output());
        host.settings(&values, true).unwrap();
        for (index, entry) in firsts.iter().enumerate() {
            host.tick((index as u32 + 1) * 200);
            parser.process(&host.take_output());
            let contents = parser.screen().contents();
            assert!(
                contents.contains(&entry.label.chars().take(32).collect::<String>()),
                "missing {} page first control {}: {contents}",
                entry.page,
                entry.label
            );
            host.feed(b"\t");
        }
        assert!(
            host.take_actions().is_empty(),
            "page navigation must not change outputs/configuration"
        );
    }

    #[test]
    fn choice_popup_cancels_or_commits_canonical_values_with_owner_cas() {
        let mut host = Host::new(30, 100).unwrap();
        host.take_output();
        host.settings_pairs(&[("theme".into(), "system".into())], true);
        host.tick(100);
        host.feed(b"\x1bOQ");
        host.tick(300); // F2: real choice popup.
        host.feed(b"\x1b[B\x1b");
        host.tick(500);
        assert!(
            host.take_actions().is_empty(),
            "cancel must leave owner unchanged"
        );
        host.feed(b"\x1bOQ\x1b[B\r");
        host.tick(700);
        assert_eq!(
            host.take_actions(),
            vec![Action::Set {
                key: "theme".into(),
                value: "light".into(),
                expected: Some("system".into())
            }]
        );
    }

    #[test]
    fn prompt_emits_explicit_semantic_accent_with_standard_fallback() {
        let mut host = Host::new(24, 100).unwrap();
        let output = host.take_output();
        assert!(
            output
                .windows(b"\x1b]777;ps;ink;accent\x1b\\".len())
                .any(|bytes| bytes == b"\x1b]777;ps;ink;accent\x1b\\")
        );
        let mut parser = super::super::terminal_parser::Parser::semantic(24, 100, 0);
        parser.process(&output);
        assert!((0..24).any(|row| (0..100).any(|col| parser.accent(row, col))));
    }

    #[test]
    fn structured_status_is_one_aligned_block_and_does_not_duplicate_the_prompt() {
        let mut host = Host::new(30, 100).unwrap();
        let mut parser = vt100::Parser::new(30, 100, 100);
        parser.process(&host.take_output());
        host.feed(b"vol");
        host.status_report(&super::super::shell_status::Status {
            loaded: true,
            position: 61.25,
            duration: 3600.0,
            volume: 57.0,
            muted: true,
            ..Default::default()
        });
        parser.process(&host.take_output());
        let text = parser.screen().contents();
        assert_eq!(text.matches("╭─").count(), 1, "{text}");
        assert!(text.contains("00:01:01.250 / 01:00:00.000"), "{text}");
        assert!(text.contains("○ Disconnected"));
        assert!(text.lines().any(|line| line.contains("Update") && line.ends_with("Idle")));
        assert!(text.contains("─"));
        let positions: Vec<_> = [
            "Ⅱ Paused",
            "00:01:01.250",
            "⊘ Muted (57%)",
            "○ Disconnected",
        ]
        .iter()
        .map(|value| {
            text.lines()
                .find_map(|line| line.find(value))
                .expect("status value must be visible")
        })
        .collect();
        assert!(
            positions.iter().all(|column| *column == positions[0]),
            "aligned value starts: {positions:?}"
        );
        host.prompt_status("Ⅱ Paused 00:01:01 / 01:00:00 · ⊘ Muted (57%) · ○ Hardware");
        host.feed(b"ume 25\r");
        assert_eq!(host.take_actions(), [Action::Volume(25.0)]);
    }

    #[test]
    fn narrow_live_prompt_shows_update_without_duplicate_lines_or_losing_input() {
        let mut host = Host::new(21, 63).unwrap();
        let mut parser = vt100::Parser::new(21, 63, 100);
        host.branding("Pealayer Terminal Review");
        host.feed(b"sta");
        parser.process(&host.take_output());
        let summary = "■ Idle · ♪ 100% · ○ Hardware";
        host.prompt_status(&format!("{summary}\n↻ Uploading package 0%"));
        parser.process(&host.take_output());
        let active = parser.screen().contents();
        assert_eq!(active.matches("╭─").count(), 1, "{active}");
        assert!(active.contains("↻ Uploading package 0%"), "{active}");
        assert!(active.contains("╰─❯ sta"), "{active}");
        host.prompt_status(summary);
        parser.process(&host.take_output());
        let settled = parser.screen().contents();
        assert_eq!(settled.matches("╭─").count(), 1, "{settled}");
        assert!(!settled.contains("Uploading package"), "{settled}");
        host.feed(b"tus\r");
        assert_eq!(host.take_actions(), [Action::Status]);
    }

    #[test]
    fn f4_reuses_the_native_editor_without_writing_a_value() {
        let mut host = Host::new(30, 100).unwrap();
        host.settings_pairs(&[("ui.terminal.font_size".into(), "13".into())], true);
        host.tick(100);
        host.feed(b"\x1bOS");
        assert_eq!(
            host.take_actions(),
            [Action::EditSetting("ui.terminal.font_size".into())]
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
            text.lines().any(|line| line.trim() == "✓ Playback: idle"),
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
        assert_eq!(
            text.matches("╭─").count(),
            1,
            "one response must restore the prompt only once: {text}"
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
