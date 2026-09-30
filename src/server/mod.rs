pub mod fs_api;
pub mod thumbnails;
pub mod web_assets;

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_HTTP_REQUEST_BYTES: usize = 1024 * 1024;

fn resolve_web_bind_address(value: Option<&str>) -> std::net::IpAddr {
    value
        .and_then(|candidate| candidate.trim().parse().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
}

fn web_bind_address() -> std::net::IpAddr {
    resolve_web_bind_address(std::env::var("PEALAYER_WEB_BIND").ok().as_deref())
}

fn web_dist_root() -> std::path::PathBuf {
    let working_tree = std::path::PathBuf::from("web_ui/dist");
    if working_tree.join("index.html").is_file() {
        return working_tree;
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(binary_directory) = executable.parent() {
            let packaged = binary_directory.join("web_ui/dist");
            if packaged.join("index.html").is_file() {
                return packaged;
            }
            let canonical_source = binary_directory.join("../source/Pealayer/web_ui/dist");
            if canonical_source.join("index.html").is_file() {
                return canonical_source;
            }
        }
    }
    working_tree
}

fn web_asset_path(root: &std::path::Path, request_path: &str) -> Option<std::path::PathBuf> {
    let mut result = root.to_path_buf();
    let relative = request_path.trim_start_matches('/');
    if relative.is_empty() {
        return Some(result.join("index.html"));
    }
    for component in std::path::Path::new(relative).components() {
        match component {
            std::path::Component::Normal(value) => result.push(value),
            _ => return None,
        }
    }
    Some(result)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebRuntimeConfig {
    pub app_name: String,
    pub version: String,
    pub websocket_path: String,
    pub locale: String,
    pub direction: String,
    pub theme: String,
}

impl WebRuntimeConfig {
    pub fn production(app_name: String, locale: String, direction: String, theme: String) -> Self {
        Self {
            app_name,
            version: env!("CARGO_PKG_VERSION").to_string(),
            websocket_path: "/ws".to_string(),
            locale,
            direction,
            theme,
        }
    }
}

#[derive(Default)]
struct LaunchReceiptCache {
    operation_ids: VecDeque<String>,
}

impl LaunchReceiptCache {
    const CAPACITY: usize = 1024;

    fn claim(&mut self, operation_id: &str) -> bool {
        if self.operation_ids.iter().any(|known| known == operation_id) {
            return false;
        }
        if self.operation_ids.len() == Self::CAPACITY {
            self.operation_ids.pop_front();
        }
        self.operation_ids.push_back(operation_id.to_string());
        true
    }

    fn release(&mut self, operation_id: &str) {
        self.operation_ids.retain(|known| known != operation_id);
    }
}

#[derive(Clone)]
struct ControlState {
    command_tx: Sender<crate::platform::interop::InteropCommand>,
    latest_status: Arc<Mutex<Option<String>>>,
    websocket_clients: Arc<Mutex<Vec<Sender<String>>>>,
    egui_ctx: eframe::egui::Context,
    runtime_config_json: Arc<str>,
    web_dist_root: std::path::PathBuf,
    launch_receipts: Arc<Mutex<LaunchReceiptCache>>,
    application_identity: Arc<str>,
    expected_session_id: Option<u32>,
}

pub fn spawn_web_server(
    port: u16,
    egui_ctx: eframe::egui::Context,
) -> (
    Sender<String>,
    Receiver<crate::platform::interop::InteropCommand>,
) {
    spawn_web_server_configured(
        port,
        egui_ctx,
        WebRuntimeConfig::production(
            "Pealayer".to_string(),
            "en".to_string(),
            "ltr".to_string(),
            "system".to_string(),
        ),
    )
}

pub fn spawn_web_server_configured(
    port: u16,
    egui_ctx: eframe::egui::Context,
    runtime_config: WebRuntimeConfig,
) -> (
    Sender<String>,
    Receiver<crate::platform::interop::InteropCommand>,
) {
    let (command_tx, command_rx) = channel();
    let state_tx = spawn_control_server_configured(
        port,
        egui_ctx,
        runtime_config.clone(),
        command_tx,
        runtime_config.app_name.clone(),
    );
    (state_tx, command_rx)
}

/// Starts every TCP-facing control protocol on one listener. HTTP and REST use
/// their normal paths, WebSocket upgrades use `/ws`, and CLI/native automation
/// posts newline-compatible JSON to `/api/ipc`.
pub fn spawn_control_server_configured(
    port: u16,
    egui_ctx: eframe::egui::Context,
    runtime_config: WebRuntimeConfig,
    command_tx: Sender<crate::platform::interop::InteropCommand>,
    application_identity: String,
) -> Sender<String> {
    let (state_tx, state_rx) = channel::<String>();
    let latest_status = Arc::new(Mutex::new(None));
    let websocket_clients: Arc<Mutex<Vec<Sender<String>>>> = Arc::new(Mutex::new(Vec::new()));

    let latest_status_updates = latest_status.clone();
    let websocket_updates = websocket_clients.clone();
    thread::spawn(move || {
        while let Ok(state_json) = state_rx.recv() {
            if let Ok(mut status_guard) = latest_status_updates.lock() {
                *status_guard = Some(state_json.clone());
            }
            if let Ok(mut clients) = websocket_updates.lock() {
                clients.retain(|client| client.send(state_json.clone()).is_ok());
            }
        }
    });

    #[cfg(target_os = "windows")]
    let expected_session_id = crate::platform::windows::current_session_id().ok();
    #[cfg(not(target_os = "windows"))]
    let expected_session_id = None;

    let state = ControlState {
        command_tx,
        latest_status,
        websocket_clients,
        egui_ctx,
        runtime_config_json: Arc::from(
            serde_json::to_string(&runtime_config)
                .expect("web runtime configuration must serialize"),
        ),
        web_dist_root: web_dist_root(),
        launch_receipts: Arc::new(Mutex::new(LaunchReceiptCache::default())),
        application_identity: Arc::from(application_identity),
        expected_session_id,
    };
    let address = std::net::SocketAddr::new(web_bind_address(), port);

    thread::spawn(move || {
        let listener = match TcpListener::bind(address) {
            Ok(listener) => listener,
            Err(error) => {
                log::error!("Could not bind unified Pealayer control port {address}: {error}");
                return;
            }
        };
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let connection_state = state.clone();
            thread::spawn(move || handle_connection(stream, connection_state));
        }
    });

    state_tx
}

fn handle_connection(mut stream: TcpStream, state: ControlState) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let mut preview = [0_u8; 16 * 1024];
    let preview_started = std::time::Instant::now();
    let preview_len = loop {
        let length = match stream.peek(&mut preview) {
            Ok(0) | Err(_) => return,
            Ok(length) => length,
        };
        if preview[..length]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
            || length == preview.len()
        {
            break length;
        }
        if preview_started.elapsed() >= Duration::from_secs(3) {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    };
    let preview_text = String::from_utf8_lossy(&preview[..preview_len]).to_ascii_lowercase();
    let websocket_upgrade = preview_text.starts_with("get /ws ")
        && preview_text.contains("upgrade: websocket")
        && preview_text.contains("connection:");
    if websocket_upgrade {
        handle_websocket(stream, state);
    } else {
        let response = match read_http_request(&mut stream) {
            Ok(request) => route_http(request, &state),
            Err(error) => HttpResponse::text(400, "Bad Request", error),
        };
        let _ = write_http_response(&mut stream, response);
    }
}

fn handle_websocket(stream: TcpStream, state: ControlState) {
    let Ok(mut websocket) = tungstenite::accept(stream) else {
        return;
    };
    let (client_tx, client_rx) = channel::<String>();
    if let Ok(mut clients) = state.websocket_clients.lock() {
        clients.push(client_tx);
    }
    if let Some(status) = state
        .latest_status
        .lock()
        .ok()
        .and_then(|value| value.clone())
    {
        if websocket
            .send(tungstenite::Message::Text(status.into()))
            .is_err()
        {
            return;
        }
    }
    let _ = websocket.get_mut().set_nonblocking(true);
    loop {
        if let Ok(text) = client_rx.try_recv() {
            if websocket
                .send(tungstenite::Message::Text(text.into()))
                .is_err()
            {
                break;
            }
        }
        match websocket.read() {
            Ok(tungstenite::Message::Text(text)) => {
                if let Some(response) = handle_websocket_text(&state, &text) {
                    if websocket
                        .send(tungstenite::Message::Text(response.into()))
                        .is_err()
                    {
                        break;
                    }
                }
            }
            Ok(tungstenite::Message::Close(_)) => break,
            Err(tungstenite::Error::Io(ref error))
                if error.kind() == std::io::ErrorKind::WouldBlock =>
            {
                thread::sleep(Duration::from_millis(15));
            }
            Err(_) => break,
            _ => {}
        }
    }
}

fn handle_websocket_text(state: &ControlState, text: &str) -> Option<String> {
    if let Ok(command) = serde_json::from_str::<crate::platform::interop::InteropCommand>(text) {
        if state.command_tx.send(command).is_ok() {
            state.egui_ctx.request_repaint();
        }
        return None;
    }
    let request = serde_json::from_str::<crate::platform::interop::JsonRpcRequest>(text).ok()?;
    Some(
        match crate::platform::interop::command_from_json_rpc(&request) {
            Ok(Some(command)) => {
                let accepted = state.command_tx.send(command).is_ok();
                if accepted {
                    state.egui_ctx.request_repaint();
                }
                crate::platform::interop::json_rpc_result(
                    &request.id,
                    serde_json::json!({"accepted":accepted}),
                )
            }
            Ok(None) => {
                let value = state
                    .latest_status
                    .lock()
                    .ok()
                    .and_then(|status| status.clone())
                    .as_deref()
                    .and_then(|status| serde_json::from_str(status).ok())
                    .unwrap_or_else(|| serde_json::json!({"status":"initializing"}));
                crate::platform::interop::json_rpc_result(&request.id, value)
            }
            Err(error) => crate::platform::interop::json_rpc_error(&request.id, -32601, &error),
        },
    )
}

struct HttpRequest {
    method: String,
    target: String,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut bytes = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        let count = stream
            .read(&mut chunk)
            .map_err(|error| format!("read request: {error}"))?;
        if count == 0 {
            return Err("connection closed before request headers".to_string());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > MAX_HTTP_REQUEST_BYTES {
            return Err("request exceeds 1 MiB limit".to_string());
        }
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut parsed = httparse::Request::new(&mut headers);
    let parsed_len = match parsed
        .parse(&bytes)
        .map_err(|error| format!("parse request: {error}"))?
    {
        httparse::Status::Complete(length) => length,
        httparse::Status::Partial => return Err("incomplete request headers".to_string()),
    };
    debug_assert_eq!(header_end, parsed_len);
    let method = parsed.method.unwrap_or_default().to_string();
    let target = parsed.path.unwrap_or("/").to_string();
    let content_length = parsed
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("content-length"))
        .and_then(|header| std::str::from_utf8(header.value).ok())
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if parsed_len + content_length > MAX_HTTP_REQUEST_BYTES {
        return Err("request exceeds 1 MiB limit".to_string());
    }
    while bytes.len() < parsed_len + content_length {
        let count = stream
            .read(&mut chunk)
            .map_err(|error| format!("read request body: {error}"))?;
        if count == 0 {
            return Err("connection closed before request body".to_string());
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(HttpRequest {
        method,
        target,
        body: bytes[parsed_len..parsed_len + content_length].to_vec(),
    })
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl HttpResponse {
    fn json(status: u16, reason: &'static str, body: impl Into<String>) -> Self {
        Self::bytes(status, reason, "application/json", body.into().into_bytes())
    }

    fn text(status: u16, reason: &'static str, body: impl Into<String>) -> Self {
        Self::bytes(
            status,
            reason,
            "text/plain; charset=utf-8",
            body.into().into_bytes(),
        )
    }

    fn bytes(status: u16, reason: &'static str, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            reason,
            content_type,
            body,
        }
    }
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        response.status,
        response.reason,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn route_http(request: HttpRequest, state: &ControlState) -> HttpResponse {
    let path = request.target.split('?').next().unwrap_or("/");
    match (request.method.as_str(), path) {
        ("GET", "/healthz") => HttpResponse::json(
            200,
            "OK",
            r#"{"status":"ok","service":"pealayer","rpc":"2.0","transport":"unified"}"#,
        ),
        ("GET", "/api/runtime/config") => {
            HttpResponse::json(200, "OK", state.runtime_config_json.to_string())
        }
        ("GET", "/api/player/status") => {
            match state
                .latest_status
                .lock()
                .ok()
                .and_then(|value| value.clone())
            {
                Some(body) => HttpResponse::json(200, "OK", body),
                None => {
                    HttpResponse::json(503, "Service Unavailable", r#"{"status":"initializing"}"#)
                }
            }
        }
        ("POST", "/api/rpc") => json_rpc_response(&request.body, state),
        ("POST", "/api/player/command") => player_command_response(&request.body, state),
        ("POST", "/api/ipc") => HttpResponse::json(
            200,
            "OK",
            dispatch_ipc_payload(state, String::from_utf8_lossy(&request.body).trim()),
        ),
        ("GET", "/api/player/frame") => player_frame_response(&request.target, state),
        ("GET", "/api/fs/browse") => browse_response(&request.target),
        ("GET", "/api/fs/thumbnail") => thumbnail_response(&request.target),
        ("POST", "/api/fs/rename") => rename_response(&request.body),
        ("POST", "/api/fs/trash") => trash_response(&request.body),
        ("GET", _) => static_response(path, state),
        _ => HttpResponse::text(404, "Not Found", "Not Found"),
    }
}

fn json_rpc_response(body: &[u8], state: &ControlState) -> HttpResponse {
    let response = match serde_json::from_slice::<crate::platform::interop::JsonRpcRequest>(body) {
        Ok(request) => match crate::platform::interop::command_from_json_rpc(&request) {
            Ok(Some(command)) => {
                let accepted = state.command_tx.send(command).is_ok();
                if accepted {
                    state.egui_ctx.request_repaint();
                }
                crate::platform::interop::json_rpc_result(
                    &request.id,
                    serde_json::json!({"accepted":accepted}),
                )
            }
            Ok(None) => {
                let value = state
                    .latest_status
                    .lock()
                    .ok()
                    .and_then(|status| status.clone())
                    .as_deref()
                    .and_then(|status| serde_json::from_str(status).ok())
                    .unwrap_or_else(|| serde_json::json!({"status":"initializing"}));
                crate::platform::interop::json_rpc_result(&request.id, value)
            }
            Err(error) => crate::platform::interop::json_rpc_error(&request.id, -32601, &error),
        },
        Err(error) => crate::platform::interop::json_rpc_error(
            &serde_json::Value::Null,
            -32700,
            &format!("invalid JSON-RPC request: {error}"),
        ),
    };
    HttpResponse::json(200, "OK", response)
}

fn player_command_response(body: &[u8], state: &ControlState) -> HttpResponse {
    match serde_json::from_slice::<crate::platform::interop::InteropCommand>(body) {
        Ok(command) => match state.command_tx.send(command) {
            Ok(()) => {
                state.egui_ctx.request_repaint();
                HttpResponse::json(200, "OK", r#"{"status":"ok"}"#)
            }
            Err(_) => HttpResponse::text(503, "Service Unavailable", "Dispatcher unavailable"),
        },
        _ => HttpResponse::text(400, "Bad Request", "Bad Command"),
    }
}

fn dispatch_ipc_payload(state: &ControlState, payload: &str) -> String {
    use crate::platform::interop::InteropCommand;

    let (id, command) = match crate::platform::interop::parse_interop_request(payload) {
        Ok(parsed) => parsed,
        Err(error) => return crate::platform::interop::format_interop_error(None, -32600, &error),
    };
    if matches!(command, InteropCommand::GetStatus) {
        let value = serde_json::to_value(crate::platform::interop::get_live_status())
            .unwrap_or_else(|_| serde_json::json!({"status":"initializing"}));
        return crate::platform::interop::format_interop_response(id, &value);
    }

    let operation_id = match &command {
        InteropCommand::Launch { request } => {
            if let Err(error) = validate_launch_destination(
                request,
                &state.application_identity,
                state.expected_session_id,
            ) {
                return crate::platform::interop::format_interop_error(id, -32600, &error);
            }
            Some(request.operation_id.clone())
        }
        _ => None,
    };

    if let Some(operation_id) = operation_id {
        let mut receipts = match state.launch_receipts.lock() {
            Ok(receipts) => receipts,
            Err(_) => {
                return crate::platform::interop::format_interop_error(
                    id,
                    -32000,
                    "launch receipt cache is unavailable",
                );
            }
        };
        if !receipts.claim(&operation_id) {
            return crate::platform::interop::format_interop_response(
                id,
                &serde_json::json!({"status":"accepted","duplicate":true}),
            );
        }
        if state.command_tx.send(command).is_err() {
            receipts.release(&operation_id);
            return crate::platform::interop::format_interop_error(
                id,
                -32000,
                "application dispatcher is unavailable",
            );
        }
    } else if state.command_tx.send(command).is_err() {
        return crate::platform::interop::format_interop_error(
            id,
            -32000,
            "application dispatcher is unavailable",
        );
    }
    state.egui_ctx.request_repaint();
    crate::platform::interop::format_interop_response(id, &serde_json::json!({"status":"accepted"}))
}

fn validate_launch_destination(
    request: &crate::platform::interop::LaunchRequest,
    expected_identity: &str,
    _expected_session_id: Option<u32>,
) -> Result<(), String> {
    request.validate()?;
    if request.application_identity.trim().to_lowercase() != expected_identity.trim().to_lowercase()
    {
        return Err("launch request targets a different application identity".to_string());
    }
    #[cfg(target_os = "windows")]
    if request.sender_session_id != _expected_session_id {
        return Err("launch request targets a different Windows session".to_string());
    }
    Ok(())
}

fn player_frame_response(target: &str, state: &ControlState) -> HttpResponse {
    let path = query_value(target, "path")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            state
                .latest_status
                .lock()
                .ok()
                .and_then(|status| status.clone())
                .as_deref()
                .and_then(|status| {
                    serde_json::from_str::<crate::platform::interop::PlayerStatusResponse>(status)
                        .ok()
                })
                .and_then(|status| status.current_video)
                .map(std::path::PathBuf::from)
        });
    image_response(path.as_deref(), "Frame Not Found")
}

fn browse_response(target: &str) -> HttpResponse {
    let path = query_value(target, "path");
    match fs_api::browse_directory(path.as_deref()) {
        Ok(result) => HttpResponse::json(
            200,
            "OK",
            serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(error) => HttpResponse::text(400, "Bad Request", error),
    }
}

fn thumbnail_response(target: &str) -> HttpResponse {
    let path = query_value(target, "path").map(std::path::PathBuf::from);
    image_response(path.as_deref(), "Thumbnail Not Found")
}

fn image_response(path: Option<&std::path::Path>, not_found: &str) -> HttpResponse {
    if let Some(path) = path
        && let Some(thumbnail) = thumbnails::get_or_generate_thumbnail(path)
        && let Ok(data) = std::fs::read(thumbnail)
    {
        return HttpResponse::bytes(200, "OK", "image/jpeg", data);
    }
    HttpResponse::text(404, "Not Found", not_found)
}

fn rename_response(body: &[u8]) -> HttpResponse {
    match serde_json::from_slice::<fs_api::RenameRequest>(body)
        .map_err(|error| error.to_string())
        .and_then(|request| fs_api::rename_file(&request.old_path, &request.new_name))
    {
        Ok(new_path) => HttpResponse::json(
            200,
            "OK",
            serde_json::json!({"status":"ok","new_path":new_path}).to_string(),
        ),
        Err(error) => HttpResponse::text(400, "Bad Request", error),
    }
}

fn trash_response(body: &[u8]) -> HttpResponse {
    match serde_json::from_slice::<fs_api::TrashRequest>(body)
        .map_err(|error| error.to_string())
        .and_then(|request| fs_api::trash_file(&request.target_path))
    {
        Ok(()) => HttpResponse::json(200, "OK", r#"{"status":"ok"}"#),
        Err(error) => HttpResponse::text(400, "Bad Request", error),
    }
}

fn static_response(path: &str, state: &ControlState) -> HttpResponse {
    let target = web_asset_path(&state.web_dist_root, path)
        .unwrap_or_else(|| state.web_dist_root.join("__invalid_request_path__"));
    if target.is_file()
        && let Ok(data) = std::fs::read(&target)
    {
        return HttpResponse::bytes(200, "OK", mime_for_path(&target), data);
    }
    let fallback = std::fs::read(state.web_dist_root.join("index.html"))
        .unwrap_or_else(|_| web_assets::INDEX_HTML.as_bytes().to_vec());
    HttpResponse::bytes(200, "OK", "text/html; charset=utf-8", fallback)
}

fn query_value(target: &str, name: &str) -> Option<String> {
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        key.eq_ignore_ascii_case(name)
            .then(|| urlencoding_decode(value))
    })
}

fn urlencoding_decode(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '%' {
            let hex = format!(
                "{}{}",
                chars.next().unwrap_or_default(),
                chars.next().unwrap_or_default()
            );
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if character == '+' {
            result.push(' ');
        } else {
            result.push(character);
        }
    }
    result
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("json") => "application/json",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn web_server_is_loopback_only_unless_explicitly_exposed() {
        assert_eq!(
            resolve_web_bind_address(None),
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
        assert_eq!(
            resolve_web_bind_address(Some("invalid-hostname")),
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
        assert_eq!(
            resolve_web_bind_address(Some("0.0.0.0")),
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        );
        assert_eq!(
            resolve_web_bind_address(Some("::")),
            IpAddr::V6(Ipv6Addr::UNSPECIFIED)
        );
    }

    #[test]
    fn test_url_decoding() {
        assert_eq!(
            urlencoding_decode("hello%20world%2Ftest"),
            "hello world/test"
        );
    }

    #[test]
    fn web_assets_never_escape_distribution_root() {
        let root = std::path::Path::new("C:/Pealayer/web_ui/dist");
        assert_eq!(
            web_asset_path(root, "/assets/app.js").unwrap(),
            root.join("assets/app.js")
        );
        assert!(web_asset_path(root, "/../Cargo.toml").is_none());
        #[cfg(windows)]
        assert!(web_asset_path(root, "C:/Windows/win.ini").is_none());
    }
}
