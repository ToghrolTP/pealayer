//! Local, bounded crash evidence for GUI builds without an attached console.
//! Never uploads reports, intercepts native exceptions, or resumes after a panic.

use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_REPORT_BYTES: usize = 256 * 1024;

/// Bounded local evidence for short-lived Explorer launches. Do not record
/// media URLs, command payloads, configuration contents or credentials.
pub fn record_shell_action(phase: &str, action: &str) {
    let path = crate::server::thumbnails::get_thumbnail_cache_dir()
        .with_file_name("diagnostics").join("shell-actions.jsonl");
    let event = serde_json::json!({
        "unix_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_millis(),
        "pid": std::process::id(), "phase": phase, "action": action,
        "commit": env!("PEALAYER_GIT_COMMIT"),
    });
    let _ = append_shell_event(&path, &event);
}

fn append_shell_event(path: &Path, event: &serde_json::Value) -> std::io::Result<()> {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let full = std::fs::metadata(path).is_ok_and(|metadata| metadata.len() >= 64 * 1024);
    let mut file = std::fs::OpenOptions::new().create(true).write(true)
        .append(!full).truncate(full).open(path)?;
    writeln!(file, "{event}")
}

pub fn panic_report_path() -> PathBuf {
    crate::config::AppConfig::get_config_path()
        .with_file_name("diagnostics")
        .join("latest-panic.log")
}

fn write_report(path: &Path, report: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut end = report.len().min(MAX_REPORT_BYTES);
    while !report.is_char_boundary(end) {
        end -= 1;
    }
    // One latest report, not an unbounded collection of crash backups.
    let mut file = std::fs::File::create(path)?;
    file.write_all(&report.as_bytes()[..end])?;
    file.sync_all()
}

pub fn install_panic_reporter() {
    // Resolve configuration before entering a panic hook; no config parsing or
    // application mutex acquisition is safe once the failing thread unwinds.
    let path = panic_report_path();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let thread = std::thread::current();
        let report = format!(
            "Pealayer panic report\nversion={}\ncommit={}\nos={}\narch={}\npid={}\nunix_time={}\nthread={}\n{}\n{}\n",
            env!("CARGO_PKG_VERSION"),
            env!("PEALAYER_GIT_COMMIT"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::process::id(),
            timestamp,
            thread.name().unwrap_or("unnamed"),
            info,
            std::backtrace::Backtrace::force_capture(),
        );
        let path=if crate::peer::active(){crate::server::thumbnails::get_thumbnail_cache_dir().join("peer-latest-panic.log")}else{path.clone()};
        let _ = write_report(&path, &report);
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_action_evidence_is_bounded_and_valid_jsonl() {
        let directory = std::env::temp_dir().join(format!("pealayer-shell-evidence-{}", uuid::Uuid::new_v4()));
        let path = directory.join("shell-actions.jsonl");
        write_report(&path, &"x".repeat(64 * 1024)).unwrap();
        let event = serde_json::json!({"phase":"forward_accepted","action":"preferences"});
        append_shell_event(&path, &event).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(saved.trim()).unwrap(), event);
        append_shell_event(&path, &event).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn panic_report_is_bounded_utf8_and_replaces_previous_report() {
        let directory =
            std::env::temp_dir().join(format!("pealayer-panic-report-{}", uuid::Uuid::new_v4()));
        let path = directory.join("diagnostics/latest-panic.log");
        write_report(&path, &"ی".repeat(MAX_REPORT_BYTES)).unwrap();
        let report = std::fs::read_to_string(&path).unwrap();
        assert!(report.len() <= MAX_REPORT_BYTES);
        assert!(report.chars().all(|character| character == 'ی'));
        write_report(&path, "replacement").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "replacement");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory.join("diagnostics")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
