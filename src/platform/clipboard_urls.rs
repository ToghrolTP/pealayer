//! Native text-only clipboard observation. Never inject paste into a focused
//! widget, log clipboard contents, open media automatically, or read images.
use std::sync::{Mutex, OnceLock};
static PENDING: Mutex<Option<String>> = Mutex::new(None);

pub fn take(ctx: &eframe::egui::Context) -> Option<String> {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut clipboard = None;
            let mut last = None;
            let mut seen = std::collections::VecDeque::new();
            #[cfg(windows)]
            let mut sequence = None;
            loop {
                // Initial contents are a baseline, not a newly copied link.
                if crate::platform::interop::get_live_config().clipboard_url_detection {
                    #[cfg(windows)]
                    {
                        let current = unsafe {
                            windows::Win32::System::DataExchange::GetClipboardSequenceNumber()
                        };
                        if sequence == Some(current) {
                            std::thread::sleep(std::time::Duration::from_millis(400));
                            continue;
                        }
                        // Retry a busy clipboard without losing this sequence.
                        if clipboard.is_none() {
                            clipboard = arboard::Clipboard::new().ok();
                        }
                        if let Some(text) = clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                            sequence = Some(current);
                            observe(text, &mut last, &mut seen, &ctx);
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        if clipboard.is_none() {
                            clipboard = arboard::Clipboard::new().ok();
                        }
                        if let Some(text) = clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                            observe(text, &mut last, &mut seen, &ctx);
                        }
                    }
                } else {
                    clipboard = None;
                    last = None;
                    #[cfg(windows)]
                    {
                        sequence = None;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        });
    });
    PENDING
        .lock()
        .ok()?
        .take()
        .filter(|_| crate::platform::interop::get_live_config().clipboard_url_detection)
}

fn observe(
    text: String,
    last: &mut Option<u64>,
    seen: &mut std::collections::VecDeque<String>,
    ctx: &eframe::egui::Context,
) {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hash);
    let fingerprint = hash.finish();
    if *last != Some(fingerprint) && last.is_some() {
        if let Some(url) = candidate(&text) {
            if !seen.contains(&url) {
                seen.push_back(url.clone());
                if seen.len() > 128 {
                    seen.pop_front();
                }
                if let Ok(mut pending) = PENDING.lock() {
                    *pending = Some(url);
                }
                ctx.request_repaint();
            }
        }
    }
    *last = Some(fingerprint);
}

pub fn candidate(text: &str) -> Option<String> {
    let text = text.trim();
    if text.contains(['\n', '\r']) {
        return None;
    }
    crate::remote_location::normalize(text)
        .ok()
        .map(|u| u.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_a_complete_single_safe_http_link_is_observed() {
        assert_eq!(
            super::candidate(" https://example.test/folder/#top ").as_deref(),
            Some("https://example.test/folder/")
        );
        for invalid in [
            "ordinary text",
            "file:///C:/private",
            "https://user:secret@example.test/",
            "https://example.test/\nhttps://other.test/",
            "ftp://example.test/",
        ] {
            assert!(super::candidate(invalid).is_none());
        }
        assert!(crate::config::AppConfig::default().clipboard_url_detection);
        assert!(crate::config::AppConfig::default().open_url_auto_proxy);
    }
}
