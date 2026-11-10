//! Human updater presentation, separate from transport and restart ownership.
//! PortableShell-inspired semantic colors without replacing the user's terminal palette.

use super::{UpdateProgress, UpdateStatus};
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutputOptions {
    pub json: bool,
    pub color: ColorChoice,
}

fn use_color(choice: ColorChoice, terminal: bool, no_color: bool, dumb: bool) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => terminal && !no_color && !dumb,
    }
}

pub struct Reporter<W: Write, E: Write> {
    output: W,
    errors: E,
    options: OutputOptions,
    color: bool,
    error_color: bool,
    interactive: bool,
    width: usize,
    started: Instant,
    last_phase: String,
    last_print: Duration,
    transfer: Option<(Duration, u64)>,
    last_bytes: Option<u64>,
    live_line: bool,
    _terminal: TerminalGuard,
}

impl Reporter<std::io::Stdout, std::io::Stderr> {
    pub fn open(title: &str, target: &str, options: OutputOptions) -> Self {
        // GUI-subsystem Windows executables may have no handles until attached.
        // Never allocate a console or replace an inherited pipe (e.g. Tee).
        let terminal = TerminalGuard::prepare();
        let output = std::io::stdout();
        let is_terminal = output.is_terminal();
        let color = use_color(
            options.color,
            is_terminal,
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var("TERM").is_ok_and(|value| value == "dumb"),
        );
        let mut reporter = Self::new(
            output,
            std::io::stderr(),
            options,
            color,
            is_terminal && color,
            terminal_width(),
        );
        reporter.error_color = use_color(
            options.color,
            std::io::stderr().is_terminal(),
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var("TERM").is_ok_and(|value| value == "dumb"),
        );
        reporter._terminal = terminal;
        reporter.heading(title, target);
        reporter
    }
}

impl<W: Write, E: Write> Reporter<W, E> {
    fn new(
        output: W,
        errors: E,
        options: OutputOptions,
        color: bool,
        interactive: bool,
        width: usize,
    ) -> Self {
        Self {
            output,
            errors,
            options,
            color,
            error_color: color,
            interactive,
            width: width.clamp(24, 100),
            started: Instant::now(),
            last_phase: String::new(),
            last_print: Duration::ZERO,
            transfer: None,
            last_bytes: None,
            live_line: false,
            _terminal: TerminalGuard::default(),
        }
    }

    fn paint(&self, text: &str, ink: u8) -> String {
        if self.color {
            format!("\x1b[38;5;{ink}m{text}\x1b[0m")
        } else {
            text.into()
        }
    }

    fn end_live_line(&mut self) {
        if self.live_line {
            let _ = writeln!(self.output);
            self.live_line = false;
        }
    }

    fn line(&mut self, text: &str, ink: u8, live: bool) {
        let text = fit(&clean(text), self.width.saturating_sub(1));
        let text = self.paint(&text, ink);
        if live && self.interactive {
            let _ = write!(self.output, "\r\x1b[2K{text}");
            self.live_line = true;
        } else {
            self.end_live_line();
            let _ = writeln!(self.output, "{text}");
        }
        let _ = self.output.flush();
    }

    fn heading(&mut self, title: &str, target: &str) {
        if self.options.json {
            return;
        }
        self.line(&format!("Pealayer  /  {title}"), 80, false);
        self.line(&"-".repeat(self.width - 1), 243, false);
        self.line(&format!("Peer    {}", clean(target)), 247, false);
        self.line("", 247, false);
    }

    pub fn progress(&mut self, progress: UpdateProgress<'_>) {
        self.progress_at(progress, self.started.elapsed());
    }

    fn progress_at(&mut self, progress: UpdateProgress<'_>, elapsed: Duration) {
        if self.options.json {
            return;
        }
        let (phase, status) = match progress {
            UpdateProgress::Phase(phase) => (phase, None),
            UpdateProgress::Status(status) => (status.phase_label(), Some(status)),
        };
        let changed = phase != self.last_phase;
        if changed {
            self.end_live_line();
            self.line(&format!("> {phase}"), 183, false);
            self.last_phase = phase.into();
        }
        if let Some(status) =
            status.filter(|value| matches!(value.state.as_str(), "receiving" | "downloading"))
        {
            let (began, initial_bytes) = *self.transfer.get_or_insert((elapsed, status.bytes_done));
            let interval = Duration::from_millis(if self.interactive { 100 } else { 1000 });
            let complete = self.last_bytes != Some(status.bytes_done)
                && status
                    .bytes_total
                    .is_some_and(|total| total > 0 && status.bytes_done >= total);
            if changed || elapsed.saturating_sub(self.last_print) >= interval || complete {
                let seconds = elapsed.saturating_sub(began).as_secs_f64();
                let speed = (seconds >= 0.25 && status.bytes_done > initial_bytes)
                    .then(|| (status.bytes_done - initial_bytes) as f64 / seconds);
                self.line(&transfer_line(status, speed, self.width), 215, true);
                self.last_print = elapsed;
                self.last_bytes = Some(status.bytes_done);
            }
        }
        // No transfer percentage in verification/restart: 100% is not completion.
    }

    pub fn finish(&mut self, result: &Result<UpdateStatus, String>) {
        if self.options.json {
            match result {
                Ok(status) => {
                    let _ = serde_json::to_writer_pretty(&mut self.output, status);
                    let _ = writeln!(self.output);
                }
                Err(error) => {
                    let _ = writeln!(
                        self.errors,
                        "{}",
                        serde_json::json!({"state":"failed", "error":clean(error)})
                    );
                }
            }
            let _ = self.output.flush();
            let _ = self.errors.flush();
            return;
        }
        self.end_live_line();
        match result {
            Ok(status) => {
                self.progress(UpdateProgress::Status(status));
                self.line("", 247, false);
                let success = matches!(status.state.as_str(), "current" | "completed");
                let label = match status.state.as_str() {
                    "completed" => "OK  Updated build acknowledged by peer",
                    "current" => "OK  Peer already has this build",
                    "failed" => "ERROR  Update failed",
                    _ => "Status received (not a completion receipt)",
                };
                self.line(
                    label,
                    if success {
                        114
                    } else if status.state == "failed" {
                        203
                    } else {
                        221
                    },
                    false,
                );
                if let Some(version) = &status.version {
                    self.line(&format!("Version  {version}"), 150, false);
                }
                self.line(
                    status.error.as_deref().unwrap_or(&status.message),
                    247,
                    false,
                );
                if success {
                    self.line(
                        &format!("Elapsed  {:.1}s", self.started.elapsed().as_secs_f64()),
                        215,
                        false,
                    );
                }
            }
            Err(error) => {
                // Stderr color policy is independent: that stream may be redirected.
                for text in [
                    "ERROR  Update did not complete",
                    &clean(error),
                    "Inspect --update-status before retrying.",
                ] {
                    for line in wrap(text, self.width - 1) {
                        if self.error_color {
                            let _ = writeln!(self.errors, "\x1b[38;5;203m{line}\x1b[0m");
                        } else {
                            let _ = writeln!(self.errors, "{line}");
                        }
                    }
                }
                let _ = self.errors.flush();
            }
        }
    }
}

fn bytes(value: f64) -> String {
    super::format_bytes(value.max(0.0) as u64)
}

fn transfer_line(status: &UpdateStatus, speed: Option<f64>, width: usize) -> String {
    let mut text = if let Some(percent) = status.progress_percent() {
        if width < 34 {
            format!("{percent:.1}%  {}", bytes(status.bytes_done as f64))
        } else if width < 60 {
            format!(
                "{percent:.1}%  {} / {}",
                bytes(status.bytes_done as f64),
                bytes(status.bytes_total.unwrap_or(0) as f64)
            )
        } else {
            let count = if width >= 70 { 16 } else { 8 };
            let filled = ((percent / 100.0 * count as f64) as usize).min(count);
            format!(
                "  [{}{}] {:5.1}%  {} / {}",
                "=".repeat(filled),
                "-".repeat(count - filled),
                percent,
                bytes(status.bytes_done as f64),
                bytes(status.bytes_total.unwrap_or(0) as f64)
            )
        }
    } else {
        format!("  {} transferred", bytes(status.bytes_done as f64))
    };
    if let Some(speed) = speed.filter(|value| value.is_finite() && *value > 0.0) {
        let rate = format!("  {}/s", bytes(speed));
        if text.len() + rate.len() < width {
            text.push_str(&rate);
        }
        if let Some(total) = status.bytes_total {
            let eta = format!(
                "  ETA {:.0}s",
                total.saturating_sub(status.bytes_done) as f64 / speed
            );
            if text.len() + eta.len() < width {
                text.push_str(&eta);
            }
        }
    }
    text
}

/// Remove terminal controls and URL credentials/query secrets in human/error output.
fn clean(value: &str) -> String {
    let value: String = value.chars().map(|ch| if ch.is_control() || matches!(ch, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') { ' ' } else { ch }).collect();
    value
        .split(' ')
        .map(|word| {
            if let Some(index) = word.find("http://").or_else(|| word.find("https://")) {
                let (prefix, raw) = word.split_at(index);
                if let Ok(mut url) =
                    url::Url::parse(raw.trim_end_matches([')', ']', ',', '\'', '"']))
                {
                    let _ = url.set_username("");
                    let _ = url.set_password(None);
                    url.set_query(None);
                    url.set_fragment(None);
                    return format!("{prefix}{url}");
                }
                return format!("{prefix}[redacted URL]");
            }
            word.to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fit(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_owned()
    } else {
        format!(
            "{}...",
            value
                .chars()
                .take(width.saturating_sub(3))
                .collect::<String>()
        )
    }
}

fn wrap(value: &str, width: usize) -> Vec<String> {
    let chars: Vec<char> = value.chars().collect();
    chars
        .chunks(width.max(1))
        .map(|chunk| chunk.iter().collect())
        .collect()
}

#[derive(Default)]
struct TerminalGuard {
    #[cfg(windows)]
    mode: Option<(
        windows::Win32::Foundation::HANDLE,
        windows::Win32::System::Console::CONSOLE_MODE,
    )>,
}

impl TerminalGuard {
    fn prepare() -> Self {
        #[allow(unused_mut)]
        let mut guard = Self::default();
        #[cfg(windows)]
        unsafe {
            use windows::Win32::System::Console::*;
            let stdout = GetStdHandle(STD_OUTPUT_HANDLE)
                .ok()
                .filter(|handle| !handle.is_invalid());
            let stderr = GetStdHandle(STD_ERROR_HANDLE)
                .ok()
                .filter(|handle| !handle.is_invalid());
            if (stdout.is_none() || stderr.is_none())
                && AttachConsole(ATTACH_PARENT_PROCESS).is_ok()
            {
                if let Some(handle) = stdout {
                    let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
                }
                if let Some(handle) = stderr {
                    let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
                }
            }
            if let Ok(handle) = GetStdHandle(STD_OUTPUT_HANDLE) {
                let mut mode = CONSOLE_MODE::default();
                if GetConsoleMode(handle, &mut mode).is_ok()
                    && SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING).is_ok()
                {
                    guard.mode = Some((handle, mode));
                }
            }
        }
        guard
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some((handle, mode)) = self.mode {
            unsafe {
                let _ = windows::Win32::System::Console::SetConsoleMode(handle, mode);
            }
        }
    }
}

fn terminal_width() -> usize {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::*;
        let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
        if let Ok(handle) = GetStdHandle(STD_OUTPUT_HANDLE)
            && GetConsoleScreenBufferInfo(handle, &mut info).is_ok()
        {
            return (info.srWindow.Right - info.srWindow.Left + 1).max(24) as usize;
        }
    }
    #[cfg(unix)]
    unsafe {
        let mut size: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) == 0 && size.ws_col > 0 {
            return size.ws_col as usize;
        }
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(88)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reporter(json: bool, color: bool, width: usize) -> Reporter<Vec<u8>, Vec<u8>> {
        Reporter::new(
            Vec::new(),
            Vec::new(),
            OutputOptions {
                json,
                ..Default::default()
            },
            color,
            color,
            width,
        )
    }
    fn status(state: &str, done: u64) -> UpdateStatus {
        UpdateStatus {
            state: state.into(),
            bytes_done: done,
            bytes_total: Some(10485760),
            ..Default::default()
        }
    }
    #[test]
    fn pipe_output_is_bounded_and_verification_has_no_stale_percent() {
        let mut report = reporter(false, false, 88);
        report.heading("Peer update", "http://peer.invalid:8080");
        for index in 0..=100 {
            report.progress_at(
                UpdateProgress::Status(&status("receiving", index * 104857)),
                Duration::from_millis(index * 10),
            );
        }
        report.progress_at(
            UpdateProgress::Status(&status("verifying", 10485760)),
            Duration::from_secs(2),
        );
        report.progress_at(
            UpdateProgress::Phase("Waiting for peer build acknowledgement"),
            Duration::from_secs(3),
        );
        let output = String::from_utf8(report.output).unwrap();
        assert!(!output.contains(['\x1b', '\r']));
        assert!(output.lines().count() < 12, "{output}");
        assert!(
            !output
                .split("Verifying package")
                .nth(1)
                .unwrap()
                .contains('%')
        );
    }
    #[test]
    fn speed_eta_and_unknown_totals_are_factual() {
        let status = status("receiving", 5242880);
        let line = transfer_line(&status, Some(1048576.0), 100);
        assert!(
            line.contains("50.0%") && line.contains("1.0 MiB/s") && line.contains("ETA 5s"),
            "{line}"
        );
        let unknown = UpdateStatus {
            bytes_total: None,
            ..status
        };
        let line = transfer_line(&unknown, None, 88);
        assert!(!line.contains('%') && !line.contains("ETA"));
    }
    #[test]
    fn colors_respect_terminal_environment_and_explicit_choices() {
        for terminal in [false, true] {
            for no_color in [false, true] {
                for dumb in [false, true] {
                    assert_eq!(
                        use_color(ColorChoice::Auto, terminal, no_color, dumb),
                        terminal && !no_color && !dumb
                    );
                    assert!(use_color(ColorChoice::Always, terminal, no_color, dumb));
                    assert!(!use_color(ColorChoice::Never, terminal, no_color, dumb));
                }
            }
        }
        let mut report = reporter(false, true, 88);
        report.heading("Peer update", "peer");
        report.progress_at(
            UpdateProgress::Status(&status("receiving", 1)),
            Duration::ZERO,
        );
        report.finish(&Ok(status("completed", 10485760)));
        let output = String::from_utf8(report.output).unwrap();
        assert!(
            output.contains("\x1b[38;5;80m")
                && output.contains("\x1b[38;5;114m")
                && output.contains("\r\x1b[2K")
        );
        assert!(output.ends_with('\n'));
    }
    #[test]
    fn json_has_exactly_one_status_document_and_no_presentation() {
        let mut report = reporter(true, true, 88);
        report.heading("Peer update", "peer");
        report.progress(UpdateProgress::Status(&status("receiving", 5)));
        let status = status("completed", 10485760);
        report.finish(&Ok(status.clone()));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&report.output).unwrap(),
            serde_json::to_value(status).unwrap()
        );
        assert!(report.errors.is_empty());
        let mut report = reporter(true, true, 88);
        report.finish(&Err("unavailable".into()));
        assert!(report.output.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&report.errors).unwrap()["state"],
            "failed"
        );
    }
    #[test]
    fn secrets_and_controls_are_removed_and_narrow_output_is_bounded() {
        assert_eq!(
            clean("Peer https://user:secret@peer.invalid/file?token=private#secret"),
            "Peer https://peer.invalid/file"
        );
        assert!(
            !clean("\x1b]0;title\x07\ntext\r\u{202e}")
                .contains(['\x1b', '\x07', '\n', '\r', '\u{202e}'])
        );
        for width in [24, 40, 80, 100] {
            let mut report = reporter(false, false, width);
            report.heading("Peer update", "https://example.invalid/a/very/long/path");
            report.progress(UpdateProgress::Status(&status("receiving", 5242880)));
            report.finish(&Err("Very long diagnostic ".repeat(20)));
            let output = String::from_utf8(report.output).unwrap();
            let errors = String::from_utf8(report.errors).unwrap();
            assert!(output.lines().all(|line| line.chars().count() < width));
            assert!(errors.lines().all(|line| line.chars().count() < width));
            assert!(output.contains("50.0%"), "{output}");
        }
    }
    #[test]
    fn nonterminal_status_is_not_presented_as_success() {
        let mut report = reporter(false, false, 88);
        report.finish(&Ok(status("verifying", 10485760)));
        let output = String::from_utf8(report.output).unwrap();
        assert!(!output.contains("OK") && !output.contains("100%"));
    }
    #[test]
    fn disconnected_pipe_never_panics_or_cancels_transport() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        let mut report = Reporter::new(Closed, Closed, OutputOptions::default(), true, true, 88);
        report.heading("Peer update", "peer");
        report.progress(UpdateProgress::Status(&status("receiving", 5)));
        report.finish(&Err("disconnected".into()));
        report.options.json = true;
        report.finish(&Ok(status("completed", 5)));
    }
}
