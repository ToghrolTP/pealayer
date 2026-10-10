//! Shared remote-file/index discovery. HTML is parsed as data, never executed.
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use url::Url;

pub const USER_AGENT: &str = concat!(
    "Pealayer/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/ToghrolTP/pealayer)"
);
const MAX_HTML: u64 = 4 * 1024 * 1024;
const MAX_ENTRIES: usize = 5000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemoteEntry {
    pub name: String,
    pub url: String,
    pub is_dir: bool,
    pub playable: bool,
    pub size_bytes: Option<u64>,
    pub modified: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemoteListing {
    pub requested_url: String,
    pub url: String,
    pub host: String,
    pub server: Option<String>,
    pub content_type: Option<String>,
    pub parent_url: Option<String>,
    pub file: Option<RemoteEntry>,
    pub entries: Vec<RemoteEntry>,
    pub warning: Option<String>,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Name,
    Date,
    Size,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BrowserState {
    pub request_id: u64,
    pub previous_file: Option<String>,
    pub next_file: Option<String>,
    pub revision: u64,
    pub visible: bool,
    pub loading: bool,
    pub target: String,
    pub use_proxy: bool,
    pub auto_next: bool,
    pub thumbnails: bool,
    pub sort: SortBy,
    pub descending: bool,
    pub selected: Option<String>,
    pub error: Option<String>,
    pub route_errors: Vec<RouteFailure>,
    pub listing: Option<RemoteListing>,
    pub thumbnail_status: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardLinkBehavior {
    #[default]
    VerifyBeforeDialog,
    ShowWhileVerifying,
    ShowWithoutVerification,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteFailure {
    pub use_proxy: bool,
    pub message: String,
}

impl RouteFailure {
    pub fn label(&self) -> &'static str {
        if self.use_proxy { "Proxy" } else { "Direct" }
    }
}
#[derive(Clone)]
pub struct Playback {
    pub target: String,
    pub use_proxy: bool,
}
#[derive(Default)]
struct Browser {
    current: Option<String>,
    state: BrowserState,
    generation: u64,
    clipboard_generation: u64,
    pending_play: Option<Playback>,
    playlist: Vec<RemoteEntry>,
    playlist_proxy: bool,
    thumbnail_paths: BTreeMap<String, std::path::PathBuf>,
    thumbnail_workers: usize,
    proxy_override: Option<bool>,
}

/// Bounded parallel routing. Explicit checkbox choices bypass auto probing.
/// Return the first validated success without waiting for a failing other route.
/// The losing bounded HTTP request releases its slot when its timeout expires.
#[derive(Clone, Debug)]
pub enum RouteProbeError { Busy, Failed(Vec<RouteFailure>) }
impl RouteProbeError {
    /// Identical failures are one reason; distinct failures retain their route.
    pub fn presentation(&self) -> (Option<String>, Vec<RouteFailure>) {
        match self {
            Self::Busy => (Some("Remote inspection is busy".into()), Vec::new()),
            Self::Failed(errors) if errors.is_empty() =>
                (Some("Remote inspection did not produce a response".into()), Vec::new()),
            Self::Failed(errors) if errors.iter().all(|error| error.message == errors[0].message) =>
                (Some(errors[0].message.clone()), Vec::new()),
            Self::Failed(errors) => {
                let mut errors = errors.clone();
                errors.sort_by_key(|error| !error.use_proxy);
                (None, errors)
            }
        }
    }
}
impl std::fmt::Display for RouteProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (message, errors) = self.presentation();
        if let Some(message) = message { return formatter.write_str(&message); }
        for (index, error) in errors.iter().enumerate() {
            if index > 0 { formatter.write_str("\n")?; }
            write!(formatter, "{}: {}", error.label(), error.message)?;
        }
        Ok(())
    }
}
pub fn probe_routes<T: Send + 'static>(
    preferred: bool, automatic: bool,
    probe: impl Fn(bool) -> Result<T, String> + Send + Sync + 'static,
) -> Result<(bool, T), RouteProbeError> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static ACTIVE: AtomicUsize = AtomicUsize::new(0);
    let count = if automatic { 2 } else { 1 };
    ACTIVE.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| (n + count <= 8).then_some(n + count))
        .map_err(|_| RouteProbeError::Busy)?;
    struct Slot;
    impl Drop for Slot { fn drop(&mut self) { ACTIVE.fetch_sub(1, Ordering::AcqRel); } }
    let probe = std::sync::Arc::new(probe);
    let (tx, rx) = std::sync::mpsc::channel();
    for proxy in [preferred, !preferred].into_iter().take(count) {
        let tx = tx.clone(); let probe = probe.clone();
        std::thread::spawn(move || { let _slot = Slot; let _ = tx.send((proxy, probe(proxy))); });
    }
    drop(tx);
    let mut errors = Vec::new();
    for (proxy, result) in rx {
        match result { Ok(value) => return Ok((proxy, value)), Err(message) => errors.push(RouteFailure { use_proxy: proxy, message }) }
    }
    Err(RouteProbeError::Failed(errors))
}

pub fn accept_listing(mut listing: RemoteListing, use_proxy: bool, ctx: &eframe::egui::Context) {
    if crate::peer::active() { let _ = request(&listing.requested_url, Some(use_proxy), false, ctx); return; }
    if let Ok(mut b) = browser().lock() {
        b.generation += 1;
        b.state.request_id = b.generation;
        b.state.visible = true;
        b.state.loading = false;
        b.state.target = listing.requested_url.clone();
        b.state.use_proxy = use_proxy;
        b.state.error = None;
        b.state.route_errors.clear();
        sort_entries(&mut listing.entries, b.state.sort, b.state.descending);
        b.state.selected = listing.file.as_ref().map(|e| e.url.clone());
        b.state.listing = Some(listing);
        b.state.revision += 1;
    }
    ctx.request_repaint();
}
fn browser() -> &'static Mutex<Browser> {
    static BROWSER: OnceLock<Mutex<Browser>> = OnceLock::new();
    BROWSER.get_or_init(Default::default)
}
pub fn snapshot() -> BrowserState {
    if let Some(snapshot)=crate::peer::client().and_then(|client|client.snapshot()) {
        return serde_json::from_value(snapshot.session.status.get("remote_browser").cloned().unwrap_or_default()).unwrap_or_default();
    }
    browser()
        .lock()
        .map(|b| b.state.clone())
        .unwrap_or_default()
}
pub fn revision() -> u64 {
    if crate::peer::active(){return snapshot().revision;}
    browser()
        .lock()
        .map(|b| b.state.revision)
        .unwrap_or_default()
}
fn update_neighbors(browser: &mut Browser) {
    let files: Vec<_> = browser
        .playlist
        .iter()
        .filter(|entry| entry.playable && !entry.is_dir)
        .collect();
    let index = browser
        .current
        .as_ref()
        .and_then(|current| files.iter().position(|entry| &entry.url == current));
    browser.state.previous_file = index
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| files.get(i))
        .map(|e| e.url.clone());
    browser.state.next_file = index.and_then(|i| files.get(i + 1)).map(|e| e.url.clone());
}
pub fn set_current(target: Option<&str>) {
    if crate::peer::active(){return;}
    if let Ok(mut b) = browser().lock() {
        if b.current.as_deref() == target {
            return;
        }
        let normalized = target
            .and_then(|target| normalize(target).ok())
            .map(|url| url.to_string());
        if b.current != normalized {
            b.current = normalized;
            update_neighbors(&mut b);
            b.state.revision += 1;
        }
    }
}
static CONTEXT: OnceLock<eframe::egui::Context> = OnceLock::new();
pub fn install_context(ctx: &eframe::egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}
pub fn context() -> Option<&'static eframe::egui::Context> {
    CONTEXT.get()
}
pub fn normalize(target: &str) -> Result<Url, String> {
    if target.len() > 8192 || target.trim().is_empty() {
        return Err("Enter an HTTP or HTTPS location (up to 8192 characters).".into());
    }
    let mut url = Url::parse(target.trim())
        .map_err(|_| "Enter a complete remote URL, including https://.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Remote folders require HTTP or HTTPS.".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Embedded credentials are not supported in remote-folder URLs.".into());
    }
    url.set_fragment(None);
    Ok(url)
}
pub fn decoded(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((a * 16 + b) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
pub fn playable(url: &Url) -> bool {
    let path = decoded(url.path()).to_ascii_lowercase();
    [
        "mkv", "mp4", "m4v", "webm", "avi", "mov", "mpeg", "mpg", "ts", "m2ts", "ogv", "wmv",
        "flv", "mp3", "flac", "ogg", "opus", "wav", "m4a", "aac", "m3u8", "m3u", "pls",
    ]
    .iter()
    .any(|ext| path.ends_with(&format!(".{ext}")))
}
fn directory_base(url: &Url) -> Url {
    let mut physical = url.clone();
    if let Some((_, dir)) = url.query_pairs().find(|(key, _)| key == "dir") {
        physical.set_query(None);
        physical.set_path(&format!("/{}/", dir.trim_matches('/')));
    }
    physical
}
pub fn parent(url: &Url) -> Option<String> {
    if let Some((_, dir)) = url.query_pairs().find(|(key, _)| key == "dir") {
        let trimmed = dir.trim_matches('/');
        if trimmed.is_empty() {
            return None;
        }
        let mut result = url.clone();
        let pairs: Vec<_> = url
            .query_pairs()
            .filter(|(k, _)| k != "dir")
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        result.set_query(None);
        result.query_pairs_mut().extend_pairs(pairs).append_pair(
            "dir",
            trimmed.rsplit_once('/').map(|(p, _)| p).unwrap_or(""),
        );
        return Some(result.to_string());
    }
    if url.path() == "/" {
        None
    } else {
        url.join("../").ok().map(|u| u.to_string())
    }
}
fn size(text: &str) -> Option<u64> {
    let text = text.trim().replace(',', "");
    let split = text
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(text.len());
    let value: f64 = text[..split].parse().ok()?;
    let unit = text[split..].trim().to_ascii_uppercase();
    let factor = match unit.as_str() {
        "" | "B" => 1.0,
        "K" | "KB" | "KIB" => 1024.0,
        "M" | "MB" | "MIB" => 1024.0f64.powi(2),
        "G" | "GB" | "GIB" => 1024.0f64.powi(3),
        "T" | "TB" | "TIB" => 1024.0f64.powi(4),
        _ => return None,
    };
    (value.is_finite() && value >= 0.0 && value * factor <= u64::MAX as f64)
        .then_some((value * factor).round() as u64)
}
fn text(element: ElementRef<'_>) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("constant selector")
}

pub fn parse_index(base: &Url, html: &str) -> Result<Vec<RemoteEntry>, String> {
    let document = Html::parse_document(html);
    let physical = directory_base(base);
    let recognized = document
        .select(&selector("title,h1"))
        .any(|e| text(e).to_lowercase().contains("index of"))
        || document
            .select(&selector("table[summary],li[data-type],a.is-file,a.is-dir"))
            .next()
            .is_some();
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for anchor in document.select(&selector("a[href]")) {
        let href = anchor.value().attr("href").unwrap_or_default().trim();
        if href.is_empty()
            || href.starts_with('#')
            || href.starts_with('?') && !href.contains("dir=")
        {
            continue;
        }
        let Ok(mut url) = base.join(href) else {
            continue;
        };
        url.set_fragment(None);
        if url.origin() != base.origin()
            || !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        let entry_physical = directory_base(&url);
        if !entry_physical.path().starts_with(physical.path()) {
            continue;
        }
        let relative = &entry_physical.path()[physical.path().len()..];
        if relative.trim_matches('/').is_empty() || relative.trim_end_matches('/').contains('/') {
            continue;
        }
        let row = anchor
            .ancestors()
            .filter_map(ElementRef::wrap)
            .find(|e| matches!(e.value().name(), "tr" | "li"));
        let mut modified = None;
        let mut bytes = None;
        if let Some(row) = row {
            modified = row
                .select(&selector(".m,.file-modified,.modified,time"))
                .next()
                .map(text)
                .filter(|s| !s.is_empty() && s != "-");
            bytes = row
                .select(&selector(".s,.file-size,.size"))
                .next()
                .and_then(|e| size(&text(e)));
            let cells: Vec<_> = row.select(&selector("td")).collect();
            let link_cell = anchor
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|e| e.value().name() == "td");
            let index = link_cell
                .and_then(|cell| cells.iter().position(|e| e.id() == cell.id()))
                .unwrap_or(0);
            if cells.len() > index + 2 {
                let date = text(cells[index + 1]);
                if modified.is_none() {
                    modified = (!date.is_empty() && date != "-").then_some(date);
                }
                if bytes.is_none() {
                    bytes = size(&text(cells[index + 2]));
                }
            }
        } else if anchor
            .ancestors()
            .filter_map(ElementRef::wrap)
            .any(|e| e.value().name() == "pre")
        {
            let trailing = anchor
                .next_siblings()
                .take_while(|n| !n.value().is_element())
                .filter_map(|n| n.value().as_text())
                .map(|t| t.text.as_ref())
                .collect::<String>();
            let line = trailing.lines().next().unwrap_or_default();
            let mut tokens: Vec<_> = line.split_whitespace().collect();
            if let Some(last) = tokens.pop() {
                bytes = size(last);
            }
            if !tokens.is_empty() {
                modified = Some(tokens.join(" "));
            }
        }
        if !recognized && (row.is_none() || modified.is_none() && bytes.is_none()) {
            continue;
        }
        let is_dir = entry_physical.path().ends_with('/');
        let name = decoded(relative.trim_end_matches('/'));
        if seen.insert(url.to_string()) {
            entries.push(RemoteEntry {
                name,
                playable: !is_dir && playable(&url),
                url: url.to_string(),
                is_dir,
                size_bytes: bytes,
                modified,
            });
        }
        if entries.len() > MAX_ENTRIES {
            return Err(
                "This directory exceeds the 5000-entry limit; choose a smaller folder.".into(),
            );
        }
    }
    if !recognized && entries.is_empty() {
        return Err("This page does not expose a supported directory index. It may require authentication or JavaScript.".into());
    }
    sort_entries(&mut entries, SortBy::Name, false);
    Ok(entries)
}
fn episode(name: &str) -> Option<(u64, u64)> {
    let b = name.as_bytes();
    for i in 0..b.len() {
        if b[i].eq_ignore_ascii_case(&b's') {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && j < b.len() && b[j].eq_ignore_ascii_case(&b'e') {
                let start = j + 1;
                j = start;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                if j > start {
                    return Some((
                        name[i + 1..start - 1].parse().ok()?,
                        name[start..j].parse().ok()?,
                    ));
                }
            }
        }
    }
    None
}
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    let mut left = a.chars().peekable();
    let mut right = b.chars().peekable();
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut x = String::new();
                let mut y = String::new();
                while left.peek().is_some_and(|c| c.is_ascii_digit()) {
                    x.push(left.next().unwrap());
                }
                while right.peek().is_some_and(|c| c.is_ascii_digit()) {
                    y.push(right.next().unwrap());
                }
                let x = x.trim_start_matches('0');
                let y = y.trim_start_matches('0');
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if !order.is_eq() {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                left.next();
                right.next();
                let order = x.cmp(&y);
                if !order.is_eq() {
                    return order;
                }
            }
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
        }
    }
}
pub fn sort_entries(entries: &mut [RemoteEntry], by: SortBy, descending: bool) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| {
                let order = match by {
                    SortBy::Name => match (episode(&a.name), episode(&b.name)) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    }
                    .then_with(|| natural_cmp(&a.name, &b.name)),
                    SortBy::Size => a
                        .size_bytes
                        .cmp(&b.size_bytes)
                        .then_with(|| natural_cmp(&a.name, &b.name)),
                    SortBy::Date => date_key(a.modified.as_deref())
                        .cmp(&date_key(b.modified.as_deref()))
                        .then_with(|| natural_cmp(&a.name, &b.name)),
                };
                if descending { order.reverse() } else { order }
            })
            .then_with(|| a.url.cmp(&b.url))
    });
}
fn date_key(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    // ISO table dates and nginx/Apache dd-Mon-yyyy date formats, no invented timezone.
    let tokens: Vec<_> = value.split_whitespace().collect();
    let date = tokens.first().copied().unwrap_or_default();
    let pieces: Vec<_> = date.split('-').collect();
    if pieces.len() == 3 && pieces[0].len() <= 2 {
        let month = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ]
        .iter()
        .position(|m| pieces[1].eq_ignore_ascii_case(m))
        .map(|i| i + 1)
        .unwrap_or(0);
        return format!(
            "{}-{month:02}-{:02} {}",
            pieces[2],
            pieces[0].parse::<u32>().unwrap_or(0),
            tokens.get(1).unwrap_or(&"")
        );
    }
    value.into()
}
pub fn client(
    use_proxy: bool,
    custom_proxy: Option<&str>,
) -> Result<reqwest::blocking::Client, String> {
    let mut builder = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::limited(8));
    if !use_proxy {
        builder = builder.no_proxy();
    } else if let Some(proxy) = custom_proxy.filter(|s| !s.trim().is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|_| "Configured proxy is invalid".to_string())?);
    }
    builder
        .build()
        .map_err(|e| format!("Cannot initialize remote browser: {e}"))
}
fn html_response(response: reqwest::blocking::Response) -> Result<(Url, String), String> {
    let response = response
        .error_for_status()
        .map_err(|e| format!("Directory request failed: {e}"))?;
    if response.content_length().is_some_and(|n| n > MAX_HTML) {
        return Err("Directory page exceeds the 4 MiB limit.".into());
    }
    let url = response.url().clone();
    let mut data = Vec::new();
    response
        .take(MAX_HTML + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_HTML {
        return Err("Directory page exceeds the 4 MiB limit.".into());
    }
    Ok((url, String::from_utf8_lossy(&data).into_owned()))
}
pub fn discover(
    target: &str,
    use_proxy: bool,
    custom_proxy: Option<&str>,
) -> Result<RemoteListing, String> {
    let requested = normalize(target)?;
    let client = client(use_proxy, custom_proxy)?;
    let mut response = client
        .head(requested.clone())
        .send()
        .map_err(|e| format!("Host request failed: {e}"))?;
    if matches!(response.status().as_u16(), 403 | 405 | 501) {
        response = client
            .get(requested.clone())
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .map_err(|e| e.to_string())?;
    }
    response = response
        .error_for_status()
        .map_err(|e| format!("Remote host rejected the location: {e}"))?;
    let final_url = response.url().clone();
    let header = |key| {
        response
            .headers()
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let server = header(reqwest::header::SERVER);
    let content_type = header(reqwest::header::CONTENT_TYPE);
    let is_html = content_type
        .as_deref()
        .is_some_and(|v| v.contains("text/html") || v.contains("application/xhtml"));
    let looks_dir =
        final_url.path().ends_with('/') || final_url.query_pairs().any(|(k, _)| k == "dir");
    if is_html || looks_dir && !playable(&final_url) {
        let (url, html) = html_response(
            client
                .get(final_url)
                .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
                .send()
                .map_err(|e| e.to_string())?,
        )?;
        let entries = parse_index(&url, &html)?;
        Ok(RemoteListing {
            requested_url: requested.to_string(),
            host: url.host_str().unwrap_or_default().into(),
            parent_url: parent(&url),
            url: url.to_string(),
            server,
            content_type,
            file: None,
            entries,
            warning: None,
        })
    } else {
        let media_type = content_type.as_deref().unwrap_or_default().split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
        if !playable(&final_url) && !media_type.starts_with("video/") && !media_type.starts_with("audio/")
            && !matches!(media_type.as_str(), "application/vnd.apple.mpegurl" | "application/x-mpegurl" | "application/dash+xml" | "application/ogg") {
            return Err("This link does not expose a supported media file or directory index.".into());
        }
        let total = header(reqwest::header::CONTENT_RANGE)
            .and_then(|s| s.rsplit_once('/').and_then(|(_, s)| s.parse().ok()))
            .or_else(|| header(reqwest::header::CONTENT_LENGTH).and_then(|s| s.parse().ok()));
        let file = RemoteEntry {
            name: decoded(
                final_url
                    .path_segments()
                    .and_then(|s| s.last())
                    .unwrap_or("Remote media"),
            ),
            url: final_url.to_string(),
            is_dir: false,
            playable: true,
            size_bytes: total,
            modified: header(reqwest::header::LAST_MODIFIED),
        };
        let folder = final_url.join(".").map_err(|e| e.to_string())?;
        let sibling_result = client
            .get(folder.clone())
            .timeout(Duration::from_secs(2))
            .header(reqwest::header::ACCEPT, "text/html")
            .send()
            .map_err(|e| e.to_string())
            .and_then(html_response)
            .and_then(|(base, html)| parse_index(&base, &html));
        let (mut entries, warning) = match sibling_result {
            Ok(entries) => (entries, None),
            Err(error) => (
                Vec::new(),
                Some(format!(
                    "File available; sibling discovery unavailable: {error}"
                )),
            ),
        };
        if !entries.iter().any(|e| e.url == file.url) {
            entries.push(file.clone());
            sort_entries(&mut entries, SortBy::Name, false);
        }
        Ok(RemoteListing {
            requested_url: requested.to_string(),
            url: final_url.to_string(),
            host: final_url.host_str().unwrap_or_default().into(),
            server,
            content_type,
            parent_url: Some(folder.to_string()),
            file: Some(file),
            entries,
            warning,
        })
    }
}

pub fn request(
    target: &str,
    use_proxy: Option<bool>,
    play_files: bool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    if let Some(client)=crate::peer::client(){
        if play_files {return client.queue("/api/peer/open",serde_json::json!({"target":target,"use_proxy":use_proxy}));}
        return client.queue("/api/player/command",serde_json::json!({"command":"browse_remote","target":target,"use_proxy":use_proxy}));
    }
    request_inner(target, use_proxy, play_files, RequestMode::Browse, ctx)
}
pub fn request_copied_link(target: &str, ctx: &eframe::egui::Context) -> Result<(), String> {
    if let Some(client) = crate::peer::client() {
        return client.queue("/api/player/command", serde_json::json!({"command":"browse_remote","target":target,"clipboard":true}));
    }
    normalize(target)?;
    let config = crate::platform::interop::get_live_config();
    if !config.clipboard_url_detection { return Ok(()); }
    request_inner(target, None, false, RequestMode::Clipboard(config.clipboard_link_behavior), ctx)
}
pub fn prefetch(target: &str, use_proxy: bool, ctx: &eframe::egui::Context) -> Result<(), String> {
    if crate::peer::active(){return Ok(());}
    request_inner(target, Some(use_proxy), false, RequestMode::Background, ctx)
}

#[derive(Clone, Copy)]
enum RequestMode { Browse, Background, Clipboard(ClipboardLinkBehavior) }
impl RequestMode {
    fn hidden_verification(self) -> bool {
        matches!(self, Self::Clipboard(ClipboardLinkBehavior::VerifyBeforeDialog))
    }
    fn verifies(self) -> bool {
        !matches!(self, Self::Clipboard(ClipboardLinkBehavior::ShowWithoutVerification))
    }
}

fn prepare_state(browser: &mut Browser, target: &str, use_proxy: bool, mode: RequestMode, config: &crate::config::AppConfig) {
    browser.state.request_id = browser.generation;
    browser.state.visible = !matches!(mode, RequestMode::Background);
    browser.state.target = target.into();
    browser.state.use_proxy = use_proxy;
    browser.state.auto_next = config.remote_folder_auto_next;
    browser.state.thumbnails = config.remote_folder_thumbnails;
    browser.state.loading = mode.verifies() && !target.is_empty();
    browser.state.error = None;
    browser.state.route_errors.clear();
    browser.state.selected = None;
    browser.state.thumbnail_status.clear();
    browser.pending_play = None;
    if !mode.verifies() {
        // Showing without verification must not retain another link's files.
        browser.state.listing = None;
    }
    browser.state.revision += 1;
}

fn finish_request(
    browser: &mut Browser, generation: u64, clipboard_generation: u64, target: &str,
    mode: RequestMode, play_files: bool, config: &crate::config::AppConfig,
    result: Result<(bool, RemoteListing), RouteProbeError>,
) -> bool {
    if browser.generation != generation || matches!(mode, RequestMode::Clipboard(_)) && browser.clipboard_generation != clipboard_generation {
        return false;
    }
    if mode.hidden_verification() {
        if result.is_err() { return false; }
        browser.generation += 1;
        prepare_state(browser, target, result.as_ref().unwrap().0, mode, config);
    }
    browser.state.loading = false;
    match result {
        Ok((use_proxy, mut listing)) => {
            browser.state.use_proxy = use_proxy;
            sort_entries(&mut listing.entries, browser.state.sort, browser.state.descending);
            browser.state.selected = listing.file.as_ref().map(|file| file.url.clone());
            if matches!(mode, RequestMode::Background) && listing.file.is_some() || play_files && listing.file.is_some() {
                browser.playlist = listing.entries.clone();
                browser.playlist_proxy = use_proxy;
                update_neighbors(browser);
            }
            if play_files {
                if let Some(file) = &listing.file {
                    browser.pending_play = Some(Playback { target: file.url.clone(), use_proxy });
                    browser.state.visible = false;
                }
            }
            browser.state.listing = Some(listing);
        }
        Err(error) => {
            (browser.state.error, browser.state.route_errors) = error.presentation();
        }
    }
    browser.state.revision += 1;
    true
}

fn request_inner(
    target: &str,
    use_proxy: Option<bool>,
    play_files: bool,
    mode: RequestMode,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    if !target.trim().is_empty() {
        normalize(target)?;
    }
    let config = crate::platform::interop::get_live_config();
    let mut browser = browser()
        .lock()
        .map_err(|_| "Remote browser unavailable".to_string())?;
    let background = matches!(mode, RequestMode::Background);
    let manual = use_proxy.or_else(|| if browser.state.visible && !background { browser.proxy_override } else { None });
    let automatic = manual.is_none() && config.open_url_auto_proxy;
    let use_proxy = manual.unwrap_or(config.open_url_use_proxy);
    if !background { browser.proxy_override = manual; }
    browser.clipboard_generation += 1;
    if !mode.hidden_verification() { browser.generation += 1; }
    let generation = browser.generation;
    let clipboard_generation = browser.clipboard_generation;
    // Keep the last successful listing mounted during refresh; it cannot be
    // acted on while loading. Fast replies no longer flash an empty table.
    if !mode.hidden_verification() { prepare_state(&mut browser, target.trim(), use_proxy, mode, &config); }
    if background {
        browser.playlist.clear();
        browser.playlist_proxy = use_proxy;
        update_neighbors(&mut browser);
    }
    if target.trim().is_empty() || !mode.verifies() {
        ctx.request_repaint();
        return Ok(());
    }
    let target = target.trim().to_owned();
    let ctx = ctx.clone();
    drop(browser);
    std::thread::spawn(move || {
        let result = loop {
            if self::browser().lock().is_ok_and(|b| b.generation != generation || matches!(mode, RequestMode::Clipboard(_)) && b.clipboard_generation != clipboard_generation) { return; }
            let target = target.clone(); let custom = config.open_url_proxy_url.clone();
            match probe_routes(use_proxy, automatic, move |proxy| discover(&target, proxy, custom.as_deref())) {
                Err(RouteProbeError::Busy) => std::thread::sleep(Duration::from_millis(250)),
                result => break result,
            }
        };
        if let Ok(mut b) = self::browser().lock() {
            finish_request(&mut b, generation, clipboard_generation, &target, mode, play_files, &config, result);
        }
        ctx.request_repaint();
    });
    Ok(())
}
pub fn close() {
    if let Some(client)=crate::peer::client(){let _=client.queue("/api/player/command",serde_json::json!({"command":"close_remote_browser"}));return;}
    if let Ok(mut b) = browser().lock() {
        b.state.visible = false;
        b.generation += 1;
        b.state.loading = false;
        b.pending_play = None;
        b.proxy_override = None;
        b.state.revision += 1;
    }
}
pub fn take_playback() -> Option<Playback> {
    if crate::peer::active(){return None;}
    browser().lock().ok()?.pending_play.take()
}
pub fn select(target: &str, play: bool) -> Result<(), String> {
    if let Some(client)=crate::peer::client(){return client.queue("/api/player/command",serde_json::json!({"command":"select_remote","target":target,"play":play}));}
    let mut b = browser()
        .lock()
        .map_err(|_| "Remote browser unavailable".to_string())?;
    if b.state.loading { return Err("Wait for the current location to finish loading.".into()); }
    let listing = b
        .state
        .listing
        .as_ref()
        .ok_or("No remote listing is loaded")?;
    if !b.state.target.is_empty() && listing.requested_url != normalize(&b.state.target)?.as_str() {
        return Err("This listing belongs to the previous location; reload before selecting a file.".into());
    }
    let entry = listing
        .entries
        .iter()
        .find(|e| e.url == target)
        .cloned()
        .ok_or("File is no longer in this listing")?;
    if play {
        if !entry.playable {
            return Err("Choose a playable media file.".into());
        }
        b.playlist = listing.entries.clone();
        b.playlist_proxy = b.state.use_proxy;
        update_neighbors(&mut b);
        b.pending_play = Some(Playback {
            target: entry.url.clone(),
            use_proxy: b.state.use_proxy,
        });
        b.state.visible = false;
    }
    b.state.selected = Some(entry.url);
    b.state.revision += 1;
    Ok(())
}
pub fn sort(by: SortBy, descending: bool) {
    if let Some(client)=crate::peer::client(){let _=client.queue("/api/player/command",serde_json::json!({"command":"sort_remote","by":by,"descending":descending}));return;}
    if let Ok(mut b) = browser().lock() {
        b.state.sort = by;
        b.state.descending = descending;
        if let Some(listing) = b.state.listing.as_mut() {
            sort_entries(&mut listing.entries, by, descending);
        }
        sort_entries(&mut b.playlist, by, descending);
        update_neighbors(&mut b);
        b.state.revision += 1;
    }
}
pub fn step(current: &str, direction: i32, automatic: bool) -> Option<Playback> {
    let b = browser().lock().ok()?;
    if automatic && !crate::platform::interop::get_live_config().remote_folder_auto_next {
        return None;
    }
    let files: Vec<_> = b
        .playlist
        .iter()
        .filter(|e| e.playable && !e.is_dir)
        .collect();
    let current = normalize(current).ok()?.to_string();
    let index = files.iter().position(|e| e.url == current)?;
    let next = index.checked_add_signed(direction as isize)?;
    Some(Playback {
        target: files.get(next)?.url.clone(),
        use_proxy: b.playlist_proxy,
    })
}
pub fn sync_options(auto_next: bool, thumbnails: bool) {
    if crate::peer::active(){return;}
    if let Ok(mut b) = browser().lock() {
        if b.state.auto_next != auto_next || b.state.thumbnails != thumbnails {
            b.state.auto_next = auto_next;
            b.state.thumbnails = thumbnails;
            b.state.revision += 1;
        }
    }
}
pub fn playback_proxy_for(target: &str) -> Option<bool> {
    let target = normalize(target).ok()?.to_string();
    let browser = browser().lock().ok()?;
    browser
        .playlist
        .iter()
        .any(|entry| entry.url == target)
        .then_some(browser.playlist_proxy)
}
pub fn thumbnail(
    target: &str,
    ctx: &eframe::egui::Context,
) -> Result<Option<std::path::PathBuf>, String> {
    let config = crate::platform::interop::get_live_config();
    let peer_listing=crate::peer::active().then(snapshot);
    let mut b = browser()
        .lock()
        .map_err(|_| "Remote browser unavailable".to_string())?;
    if !config.remote_folder_thumbnails {
        return Err("Folder thumbnails are disabled.".into());
    }
    if !peer_listing.as_ref().map(|state|&state.listing).unwrap_or(&b.state.listing)
        .as_ref()
        .is_some_and(|l| l.entries.iter().any(|e| e.url == target && e.playable))
    {
        return Err("Thumbnail target is not a listed media file.".into());
    }
    if let Some(path) = b.thumbnail_paths.get(target).filter(|path| path.is_file()) {
        return Ok(Some(path.clone()));
    }
    if b.state
        .thumbnail_status
        .get(target)
        .is_some_and(|status| status == "ready")
    {
        b.state.thumbnail_status.remove(target);
    }
    if let Some(status) = b.state.thumbnail_status.get(target) {
        if status.starts_with("error:") {
            return Err(status[6..].into());
        }
        return Ok(None);
    }
    if b.thumbnail_workers >= 2 {
        return Ok(None);
    }
    let target = target.to_owned();
    let use_proxy = b.state.use_proxy;
    let ctx = ctx.clone();
    b.thumbnail_workers += 1;
    b.state
        .thumbnail_status
        .insert(target.clone(), "loading".into());
    drop(b);
    std::thread::spawn(move || {
        let result = if let Some(client)=crate::peer::client(){client.folder_thumbnail(&target)}else{
            crate::server::thumbnails::get_or_generate_remote_thumbnail(&target,use_proxy,config.open_url_proxy_url.as_deref()).map(|file|file.path)
        };
        if let Ok(mut b) = browser().lock() {
            b.thumbnail_workers = b.thumbnail_workers.saturating_sub(1);
            match result {
                Ok(path) => {
                    if b.thumbnail_paths.len() >= 256 {
                        if let Some(key) = b.thumbnail_paths.keys().next().cloned() {
                            b.thumbnail_paths.remove(&key);
                        }
                    }
                    b.thumbnail_paths.insert(target.clone(), path);
                    b.state.thumbnail_status.insert(target, "ready".into());
                }
                Err(error) => {
                    b.state
                        .thumbnail_status
                        .insert(target, format!("error:{error}"));
                }
            }
            b.state.revision += 1;
        }
        ctx.request_repaint();
    });
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_probe_uses_first_valid_response_and_respects_manual_choice() {
        let both = std::sync::Arc::new(std::sync::Barrier::new(2));
        let start = std::time::Instant::now();
        let (proxy, value) = probe_routes(true, true, move |proxy| {
            both.wait();
            if proxy { std::thread::sleep(Duration::from_millis(250)); Err("Unavailable".into()) } else { Ok(42) }
        }).unwrap();
        assert!(!proxy); assert_eq!(value, 42);
        assert!(start.elapsed() < Duration::from_millis(200));
        let (proxy, _) = probe_routes(false, false, |proxy| { assert!(!proxy); Ok(()) }).unwrap();
        assert!(!proxy);
        let error = probe_routes::<()>(true, true, |_| Err("Rejected".into())).unwrap_err();
        assert_eq!(error.to_string(), "Rejected");
        assert_eq!(error.presentation(), (Some("Rejected".into()), Vec::new()));
        let error = probe_routes::<()>(true, true, |proxy| Err(if proxy {"Proxy unavailable"} else {"Authentication required"}.into())).unwrap_err();
        assert_eq!(error.to_string(), "Proxy: Proxy unavailable\nDirect: Authentication required");
        let (message, routes) = error.presentation();
        assert!(message.is_none());
        assert_eq!(routes.len(), 2);
        assert!(routes[0].use_proxy);
        assert!(!routes[1].use_proxy);
    }
    fn test_listing() -> RemoteListing {
        RemoteListing { requested_url: "https://files.invalid/show/".into(), url: "https://files.invalid/show/".into(),
            host: "files.invalid".into(), server: None, content_type: Some("text/html".into()), parent_url: None,
            file: None, entries: Vec::new(), warning: None }
    }
    #[test]
    fn copied_links_only_open_after_support_is_verified_and_ignore_stale_results() {
        let config = crate::config::AppConfig::default();
        assert_eq!(config.clipboard_link_behavior, ClipboardLinkBehavior::VerifyBeforeDialog);
        let mode = RequestMode::Clipboard(config.clipboard_link_behavior);
        let mut browser = Browser::default();
        browser.generation = 3;
        browser.clipboard_generation = 5;
        let failure = RouteProbeError::Failed(vec![RouteFailure {use_proxy: false, message: "Unsupported".into()}]);
        let original = browser.state.clone();
        assert!(!finish_request(&mut browser, 3, 5, "https://files.invalid/show/", mode, false, &config, Err(failure.clone())));
        assert_eq!(browser.state, original);
        browser.state.visible = true;
        browser.state.target = "https://previous.invalid/".into();
        browser.state.loading = true;
        let original = browser.state.clone();
        assert!(!finish_request(&mut browser, 3, 5, "https://files.invalid/show/", mode, false, &config, Err(failure)));
        assert_eq!(browser.state, original, "Unsupported copies must not replace an existing dialog or pending manual browse");
        assert!(!finish_request(&mut browser, 3, 4, "https://files.invalid/show/", mode, false, &config, Ok((false, test_listing()))));
        assert_eq!(browser.state, original);
        assert!(finish_request(&mut browser, 3, 5, "https://files.invalid/show/", mode, false, &config, Ok((false, test_listing()))));
        assert!(browser.state.visible);
        assert!(!browser.state.loading);
        assert_eq!(browser.state.target, "https://files.invalid/show/");
        assert!(!browser.state.use_proxy);
        assert!(browser.pending_play.is_none());
    }
    #[test]
    fn copied_link_dialog_timing_and_verification_are_independent_of_metadata_fetch() {
        let config = crate::config::AppConfig::default();
        let mut browser = Browser::default();
        let mode = RequestMode::Clipboard(ClipboardLinkBehavior::ShowWhileVerifying);
        prepare_state(&mut browser, "https://files.invalid/show/", true, mode, &config);
        assert!(browser.state.visible && browser.state.loading && mode.verifies());
        let failure = RouteProbeError::Failed(vec![RouteFailure {use_proxy: false, message: "Unsupported".into()}]);
        assert!(finish_request(&mut browser, 0, 0, "https://files.invalid/show/", mode, false, &config, Err(failure)));
        assert!(browser.state.visible && !browser.state.loading);
        assert_eq!(browser.state.error.as_deref(), Some("Unsupported"));
        browser.state.listing = Some(test_listing());
        let mode = RequestMode::Clipboard(ClipboardLinkBehavior::ShowWithoutVerification);
        prepare_state(&mut browser, "https://unverified.invalid/", true, mode, &config);
        assert!(browser.state.visible && !browser.state.loading && !mode.verifies());
        assert!(browser.state.listing.is_none());
        assert!(browser.state.error.is_none() && browser.state.route_errors.is_empty());
        for value in ["verify_before_dialog", "show_while_verifying", "show_without_verification"] {
            let behavior: ClipboardLinkBehavior = serde_json::from_value(serde_json::json!(value)).unwrap();
            assert_eq!(serde_json::to_value(behavior).unwrap(), value);
        }
    }
    #[test]
    fn parses_table_metadata_and_episode_order() {
        let url = normalize("https://files.invalid/show/").unwrap();
        let entries=parse_index(&url,r#"<title>Index of</title><table><tr><td><a href="../">Parent</a></td></tr><tr><td><a href="Show.S01E10.mkv">x</a></td><td>2025-04-20 10:00</td><td>2.5M</td></tr><tr><td><a href="Show.S01E2.mkv">x</a></td><td>2025-04-20 09:00</td><td>123</td></tr></table>"#).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].name.contains("E2"));
        assert_eq!(entries[1].size_bytes, Some(2621440));
    }
    #[test]
    fn parses_pre_and_decodes_unicode_once() {
        let url = normalize("https://files.invalid/folder/").unwrap();
        let entries = parse_index(
            &url,
            r#"<h1>Index of</h1><pre><a href="../">../</a>
<a href="%D9%81%D8%A7%D8%B1%D8%B3%DB%8C%20%2520+01.mkv">clip</a> 20-Apr-2025 10:18 199M
</pre>"#,
        )
        .unwrap();
        assert_eq!(entries[0].name, "فارسی %20+01.mkv");
        assert_eq!(entries[0].modified.as_deref(), Some("20-Apr-2025 10:18"));
        assert_eq!(entries[0].size_bytes, Some(199 * 1024 * 1024));
    }
    #[test]
    fn query_listings_preserve_navigation_and_reject_unrelated_links() {
        let url = normalize("https://files.invalid/?dir=shows%2FMy%20Show").unwrap();
        let entries=parse_index(&url,r#"<ul><li data-type="file"><a href="shows/My%20Show/Episode%202.mkv" class="is-file"><span class="file-size">20MB</span><span class="file-modified">2025-01-01 12:00</span></a></li></ul><a href="https://elsewhere.invalid/ad.mkv">ad</a><a href="?dir=shows">ancestor</a><a href="javascript:alert(1)">bad</a>"#).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Episode 2.mkv");
        assert!(parent(&url).unwrap().contains("dir=shows"));
    }
    #[test]
    fn not_every_html_page_is_a_listing() {
        assert!(
            parse_index(
                &normalize("https://files.invalid/").unwrap(),
                "<html><a href='/login'>Login</a></html>"
            )
            .is_err()
        );
        assert_eq!(
            parse_index(
                &normalize("https://files.invalid/empty/").unwrap(),
                "<h1>Index of /empty/</h1>"
            )
            .unwrap()
            .len(),
            0
        );
    }
    #[test]
    fn natural_sort_and_dates() {
        assert!(natural_cmp("Episode 2.mkv", "Episode 10.mkv").is_lt());
        assert!(date_key(Some("20-Apr-2025 10:18")) < date_key(Some("2026-01-01 00:00")));
        assert!(normalize("javascript:evil").is_err());
        assert!(normalize("https://user:password@files.invalid/").is_err());
    }

    #[test]
    fn apache_icon_column_and_duplicate_links_are_handled() {
        let entries = parse_index(&normalize("https://files.invalid/a/").unwrap(), r#"<title>Index of /a/</title><table><tr><td>ICON</td><td><a href="clip.mp4">clip.mp4</a></td><td>2025-02-03 11:22</td><td>1.5M</td></tr><tr><td><a href="clip.mp4">duplicate</a></td></tr><tr><td><a href="../secret.mp4">escape</a></td></tr></table>"#).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].modified.as_deref(), Some("2025-02-03 11:22"));
        assert_eq!(entries[0].size_bytes, Some(1572864));
    }
    #[test]
    fn folders_stay_first_and_episode_sort_is_total() {
        let entries = parse_index(&normalize("https://files.invalid/a/").unwrap(), r#"<h1>Index of</h1><pre><a href="child/">child/</a><a href="Show.S01E10.mkv">episode</a><a href="Show.S01E2.mkv">episode</a><a href="readme.txt">text</a></pre>"#).unwrap();
        assert!(entries[0].is_dir);
        assert!(entries[1].name.contains("E2"));
        let mut reverse = entries;
        sort_entries(&mut reverse, SortBy::Name, true);
        assert!(reverse[0].is_dir);
        assert!(!crate::config::AppConfig::default().remote_folder_auto_next);
    }
}
