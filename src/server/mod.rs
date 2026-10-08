pub mod fs_api;
mod media_stream;
pub mod thumbnails;
pub mod web_assets;

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use include_dir::{Dir, include_dir};

const MAX_HTTP_REQUEST_BYTES: usize = 1024 * 1024;
static EMBEDDED_WEB_UI: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/web_ui/dist");

fn web_dist_root() -> std::path::PathBuf {
    if let Some(override_root) = std::env::var_os("PEALAYER_WEB_ROOT")
        .map(std::path::PathBuf::from)
        .filter(|root| root.join("index.html").is_file())
    {
        return override_root;
    }
    // The peer updater replaces the executable, not an adjacent asset tree.
    // Release assets must match the Rust contract embedded in that executable.
    // Custom Web deployments remain explicitly selectable via PEALAYER_WEB_ROOT.
    if !cfg!(debug_assertions) {
        return std::path::PathBuf::new();
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(binary_directory) = executable.parent() {
            let packaged = binary_directory.join("web_ui/dist");
            if packaged.join("index.html").is_file() {
                return packaged;
            }
        }
    }
    let working_tree = std::path::PathBuf::from("web_ui/dist");
    if working_tree.join("index.html").is_file() {
        return working_tree;
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(binary_directory) = executable.parent() {
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
    pub app_icon_path: String,
    pub locale: String,
    pub direction: String,
    pub theme: String,
    pub accent_color: String,
}

impl WebRuntimeConfig {
    pub fn production(
        app_name: String,
        locale: String,
        direction: String,
        theme: String,
        accent_rgb: [u8; 3],
    ) -> Self {
        Self {
            app_name,
            version: env!("CARGO_PKG_VERSION").to_string(),
            websocket_path: "/ws".to_string(),
            app_icon_path: "/api/runtime/app-icon".to_string(),
            locale,
            direction,
            theme,
            accent_color: format!(
                "#{:02x}{:02x}{:02x}",
                accent_rgb[0], accent_rgb[1], accent_rgb[2]
            ),
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
            [0, 120, 212],
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
    spawn_control_server_on_addresses(
        vec![std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)],
        port,
        egui_ctx,
        runtime_config,
        command_tx,
        application_identity,
    )
}

pub fn spawn_control_server_for_config(
    config: &crate::config::AppConfig,
    egui_ctx: eframe::egui::Context,
    runtime_config: WebRuntimeConfig,
    command_tx: Sender<crate::platform::interop::InteropCommand>,
    application_identity: String,
) -> Sender<String> {
    let (state_tx, state_rx) = channel::<String>();
    if !crate::config::resolved_web_enabled(config) {
        // Keep a receiver alive so the application's status publisher remains
        // non-blocking even when every network surface is disabled.
        thread::spawn(move || while state_rx.recv().is_ok() {});
        return state_tx;
    }
    let addresses = match crate::config::resolved_web_bind_addresses(config) {
        Ok(addresses) => addresses,
        Err(error) => {
            log::error!("Could not resolve Pealayer Web listener addresses: {error}");
            vec![std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)]
        }
    };
    let port = crate::peer::client().map(|client|client.local_port).unwrap_or_else(||crate::config::runtime_port("PEALAYER_PORT", config.web_port));
    spawn_control_server_on_addresses_with_channel(
        addresses,
        port,
        egui_ctx,
        runtime_config,
        command_tx,
        application_identity,
        state_tx,
        state_rx,
    )
}

fn spawn_control_server_on_addresses(
    addresses: Vec<std::net::IpAddr>,
    port: u16,
    egui_ctx: eframe::egui::Context,
    runtime_config: WebRuntimeConfig,
    command_tx: Sender<crate::platform::interop::InteropCommand>,
    application_identity: String,
) -> Sender<String> {
    let (state_tx, state_rx) = channel::<String>();
    spawn_control_server_on_addresses_with_channel(
        addresses,
        port,
        egui_ctx,
        runtime_config,
        command_tx,
        application_identity,
        state_tx,
        state_rx,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_control_server_on_addresses_with_channel(
    addresses: Vec<std::net::IpAddr>,
    port: u16,
    egui_ctx: eframe::egui::Context,
    runtime_config: WebRuntimeConfig,
    command_tx: Sender<crate::platform::interop::InteropCommand>,
    application_identity: String,
    state_tx: Sender<String>,
    state_rx: Receiver<String>,
) -> Sender<String> {
    crate::update::manager().register_gui_context(egui_ctx.clone());
    let latest_status = Arc::new(Mutex::new(None));
    let websocket_clients: Arc<Mutex<Vec<Sender<String>>>> = Arc::new(Mutex::new(Vec::new()));

    let latest_status_updates = latest_status.clone();
    let websocket_updates = websocket_clients.clone();
    thread::spawn(move || {
        while let Ok(state_json) = state_rx.recv() {
            if let Ok(mut status_guard) = latest_status_updates.lock() {
                *status_guard = Some(state_json.clone());
            }
            if crate::platform::interop::get_live_config().web_sync_state
                && let Ok(mut clients) = websocket_updates.lock()
            {
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
    let mut listening = 0_usize;
    for ip in addresses {
        let address = std::net::SocketAddr::new(ip, port);
        let listener = match TcpListener::bind(address) {
            Ok(listener) => listener,
            Err(error) => {
                log::error!("Could not bind unified Pealayer control port {address}: {error}");
                continue;
            }
        };
        listening += 1;
        log::info!("Pealayer Web UI and APIs listening on http://{address}/");
        let listener_state = state.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let connection_state = listener_state.clone();
                thread::spawn(move || handle_connection(stream, connection_state));
            }
        });
    }
    if listening == 0 {
        log::error!("Pealayer Web UI could not start any configured listener");
    }

    state_tx
}

/// Native peers have no Origin header. Browser callers must be same-origin,
/// or explicitly trusted with PEALAYER_WEB_ALLOWED_ORIGINS (comma-separated).
/// Host validation prevents a rebinding hostname from authorizing itself.
fn browser_origin_allowed(headers: &[(String, String)], local: Option<std::net::SocketAddr>) -> bool {
    let hosts = headers.iter().filter(|(name, _)| name.eq_ignore_ascii_case("host")).collect::<Vec<_>>();
    if hosts.len() != 1 { return false; }
    let authority = &hosts[0].1;
    let Ok(host) = url::Url::parse(&format!("http://{authority}")) else { return false; };
    if !host.username().is_empty() || host.password().is_some() || host.path() != "/" || host.query().is_some() || host.fragment().is_some() { return false; }
    let Some(local) = local else { return false; };
    let Some(name) = host.host_str() else { return false; };
    let name = name.trim_matches(['[', ']']).trim_end_matches('.').to_ascii_lowercase();
    let local_name = name.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.to_canonical() == local.ip().to_canonical())
        || (name == "localhost" && local.ip().is_loopback())
        || [std::env::var("COMPUTERNAME").ok(), std::env::var("HOSTNAME").ok()].into_iter().flatten().any(|own| {
        let own = own.to_ascii_lowercase();
        name == own || name == format!("{own}.local")
    });
    let allowed = std::env::var("PEALAYER_WEB_ALLOWED_ORIGINS").unwrap_or_default()
        .split(',').filter_map(|value| url::Url::parse(value.trim()).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .collect::<Vec<_>>();
    let configured_name = allowed.iter().any(|url| {
        url::Url::parse(&format!("{}://{authority}", url.scheme())).is_ok_and(|host| host.origin() == url.origin())
    });
    // Same-origin browser GETs often omit Origin. They still need Host checks.
    if !local_name && !configured_name { return false; }
    let origins = headers.iter().filter(|(name, _)| name.eq_ignore_ascii_case("origin")).collect::<Vec<_>>();
    if origins.is_empty() { return true; }
    if origins.len() != 1 { return false; }
    let origin = &origins[0].1;
    let Ok(url) = url::Url::parse(origin) else { return false; };
    if !matches!(url.scheme(), "http" | "https") || url.origin().ascii_serialization() != *origin { return false; }
    allowed.iter().any(|allowed| allowed.origin() == url.origin())
        || url::Url::parse(&format!("{}://{authority}", url.scheme())).is_ok_and(|host| host.origin() == url.origin())
}

fn cors_headers(headers: &[(String, String)]) -> String {
    // Called only after browser_origin_allowed has accepted the request.
    media_stream::header(headers, "origin").map(|origin| format!(
        "Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, Range\r\nAccess-Control-Allow-Methods: GET, HEAD, POST, DELETE, OPTIONS\r\nAccess-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges, ETag, Last-Modified\r\n"
    )).unwrap_or_default()
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
            Ok(request) if !browser_origin_allowed(&request.headers, stream.local_addr().ok()) => {
                HttpResponse::text(403, "Forbidden", "Browser origin is not permitted")
            }
            Ok(request) if crate::peer::active() && peer_relay_route(&request.target) && !local_process_payload(&request.body) => {
                let _ = proxy_http(&request, &mut stream);
                return;
            }
            Ok(request) if matches!(request.method.as_str(), "GET" | "HEAD") && request.target.split('?').next() == Some("/api/fs/file") => {
                let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
                let _ = media_stream::serve(&request, &mut stream);
                return;
            }
            Ok(mut request) => {
                let head=request.method=="HEAD";
                if head{request.method="GET".into();}
                let cors = cors_headers(&request.headers);
                let mut response=route_http(request,&state);
                response.cors = cors;
                let _=write_http_response_headers(&mut stream,response,head);
                return;
            },
            Err(error) => HttpResponse::text(400, "Bad Request", error),
        };
        let _ = write_http_response(&mut stream, response);
    }
}

fn peer_relay_route(target: &str) -> bool {
    let path = target.split('?').next().unwrap_or(target);
    // Session commands operate the authority. Updating this executable and
    // inspecting this consumer are process-local, never updates of its server.
    path.starts_with("/api/") && path != "/api/client/status"
        && !path.starts_with("/api/update/")
        && !path.starts_with("/api/process/")
}

fn local_process_payload(body: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else { return false; };
    // Invalid parameters still belong to this process and must receive its
    // validation error, not be sent to the authority by the gateway.
    if let Some(method) = value.get("method").and_then(serde_json::Value::as_str) {
        return matches!(method, "pealayer.process.status" | "pealayer.process.connect" | "pealayer.process.quit");
    }
    serde_json::from_value::<crate::platform::interop::InteropCommand>(value)
        .is_ok_and(|command| command.is_process_local())
}

fn handle_websocket(stream: TcpStream, state: ControlState) {
    let local = stream.local_addr().ok();
    let mut websocket = match tungstenite::accept_hdr(stream, |request: &tungstenite::handshake::server::Request, response: tungstenite::handshake::server::Response| {
        let headers = request.headers().iter().filter_map(|(key, value)| value.to_str().ok().map(|value| (key.to_string(), value.to_string()))).collect::<Vec<_>>();
        if browser_origin_allowed(&headers, local) { Ok(response) }
        else { Err(tungstenite::http::Response::builder().status(403).body(Some("Browser origin is not permitted".to_string())).expect("valid rejection response")) }
    }) {
        Ok(websocket) => websocket,
        Err(error) => {
            log::warn!("Reject Pealayer WebSocket upgrade: {error}");
            return;
        }
    };
    let (client_tx, client_rx) = channel::<String>();
    if let Ok(mut clients) = state.websocket_clients.lock() {
        clients.push(client_tx);
    }
    if crate::platform::interop::get_live_config().web_sync_state
        && let Some(status) = state
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
    if local_process_payload(text.as_bytes()) {
        if !crate::platform::interop::get_live_config().web_allow_control {
            return Some(crate::platform::interop::format_interop_error(None, -32003, "Web control permission is disabled"));
        }
        return Some(dispatch_ipc_payload(state, text));
    }
    if let Some(client) = crate::peer::client() {
        let value = match serde_json::from_str::<serde_json::Value>(text) {
            Ok(value) => value,
            Err(_) => return Some(crate::platform::interop::format_interop_error(None, -32700, "Invalid peer command JSON")),
        };
        let path = if value.get("method").is_some() { "/api/rpc" } else { "/api/player/command" };
        return Some(match client.post(path, &value) {
            Ok(response) => response.to_string(),
            Err(error) => crate::platform::interop::format_interop_error(value.get("id").cloned(), -32000, &error),
        });
    }
    if let Ok(command) = serde_json::from_str::<crate::platform::interop::InteropCommand>(text) {
        if let Err(error) = command.validate() {
            return Some(crate::platform::interop::format_interop_error(None, -32602, &error));
        }
        if let Some(error) = remote_command_permission(&command) { return Some(crate::platform::interop::format_interop_error(None, -32003, error)); }
        if !crate::platform::interop::get_live_config().web_allow_control {
            return Some(crate::platform::interop::format_interop_error(
                None,
                -32003,
                "Web control permission is disabled",
            ));
        }
        if state.command_tx.send(command).is_ok() {
            state.egui_ctx.request_repaint();
        }
        return None;
    }
    let request = serde_json::from_str::<crate::platform::interop::JsonRpcRequest>(text).ok()?;
    Some(
        if matches!(
            request.method.as_str(),
            "config.get" | "pealayer.config.get"
        ) {
            if crate::platform::interop::get_live_config().web_allow_configuration {
                crate::platform::interop::json_rpc_result(
                    &request.id,
                    serde_json::to_value(crate::platform::interop::get_live_config())
                        .unwrap_or_else(|_| serde_json::json!({})),
                )
            } else {
                crate::platform::interop::json_rpc_error(
                    &request.id,
                    -32003,
                    "Web configuration permission is disabled",
                )
            }
        } else {
            match crate::platform::interop::command_from_json_rpc(&request) {
                Ok(Some(command)) => {
                    if let Some(error) = remote_command_permission(&command) { return Some(crate::platform::interop::json_rpc_error(&request.id, -32003, error)); }
                    if !crate::platform::interop::get_live_config().web_allow_control {
                        return Some(crate::platform::interop::json_rpc_error(
                            &request.id,
                            -32003,
                            "Web control permission is disabled",
                        ));
                    }
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
            }
        },
    )
}

struct HttpRequest {
    method: String,
    target: String,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
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
    let request_headers = parsed.headers.iter().map(|header| (header.name.to_string(), String::from_utf8_lossy(header.value).trim().to_string())).collect();
    if parsed.headers.iter().any(|header|header.name.eq_ignore_ascii_case("transfer-encoding")){
        return Err("Transfer-Encoding is not accepted; send an explicit Content-Length".into());
    }
    let lengths=parsed.headers.iter().filter(|header|header.name.eq_ignore_ascii_case("content-length")).collect::<Vec<_>>();
    if lengths.len()>1{return Err("Duplicate Content-Length is not accepted".into())}
    let content_length = parsed
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("content-length"))
        .and_then(|header| std::str::from_utf8(header.value).ok())
        .map(|value|value.trim().parse::<usize>().map_err(|_|"Invalid Content-Length".to_string()))
        .transpose()?.unwrap_or(0);
    if content_length>MAX_HTTP_REQUEST_BYTES.saturating_sub(parsed_len) {
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
        headers: request_headers,
    })
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    cache_control: &'static str,
    cors: String,
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
            cache_control: "no-store",
            cors: String::new(),
        }
    }

    fn with_cache_control(mut self, value: &'static str) -> Self {
        self.cache_control = value;
        self
    }
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    write_http_response_headers(stream,response,false)
}
fn write_http_response_headers(stream:&mut TcpStream,response:HttpResponse,head:bool)->std::io::Result<()>{
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: SAMEORIGIN\r\nReferrer-Policy: no-referrer\r\nPermissions-Policy: fullscreen=(self), screen-wake-lock=(self)\r\nContent-Security-Policy: default-src 'self'; connect-src 'self' ws: wss: http: https:; img-src 'self' data: blob: http: https:; media-src 'self' blob: http: https:; style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self' data:\r\n{}\r\n",
        response.status,
        response.reason,
        response.content_type,
        response.body.len(),
        response.cache_control,
        response.cors,
    )?;
    if !head{stream.write_all(&response.body)?;}
    stream.flush()
}

fn route_http(request: HttpRequest, state: &ControlState) -> HttpResponse {
    crate::peer::observe_consumer(&request.headers);
    let path = request.target.split('?').next().unwrap_or("/");
    let web = crate::platform::interop::get_live_config();
    if media_stream::header(&request.headers, "x-pealayer-route").is_some_and(|route| route.split(',').any(|id| id.trim() == crate::peer::instance_id()) || route.split(',').count() > 8) {
        return HttpResponse::text(508, "Loop Detected", "Pealayer peer routing loop");
    }
    if let Some(capability) = denied_web_capability(&request.method, path, &web) {
        return permission_denied(capability);
    }
    match (request.method.as_str(), path) {
        ("GET", "/api/peer/session") => {
            match crate::peer::server_session() {
                Ok(session) => HttpResponse::json(200,"OK",serde_json::to_string(&session).unwrap_or_default()),
                Err(error) => HttpResponse::text(503,"Service Unavailable",error),
            }
        }
        ("GET", "/api/client/status")=>HttpResponse::json(200,"OK",crate::peer::diagnostics().to_string()),
        ("GET", "/api/process/status")=>HttpResponse::json(200,"OK",crate::process_control::status().to_string()),
        ("POST", "/api/process/command") => {
            if !std::str::from_utf8(&request.body).ok()
                .and_then(|payload|crate::platform::interop::parse_interop_request(payload).ok())
                .is_some_and(|(_,command)|command.is_process_local()) {
                HttpResponse::text(400, "Bad Request", "A validated process-local command is required")
            } else {
                HttpResponse::json(200, "OK", dispatch_ipc_payload(state, &String::from_utf8_lossy(&request.body)))
            }
        }
        ("POST", "/api/peer/media") => peer_result(serde_json::from_slice(&request.body).map_err(|error|error.to_string()).and_then(crate::peer::apply_media_operation).map(|_|serde_json::json!({"accepted":true}))),
        ("POST", "/api/peer/controller") => {
            let result = serde_json::from_slice::<serde_json::Value>(&request.body).map_err(|error|error.to_string()).and_then(|value| {
                let method=value.get("method").and_then(serde_json::Value::as_str).ok_or("A controller method is required")?;
                crate::peer::controller_call(method.into(),value.get("params").cloned().unwrap_or_else(||serde_json::json!({})))
            });
            peer_result(result)
        }
        ("POST", "/api/peer/hardware") => peer_result(serde_json::from_slice(&request.body).map_err(|error|error.to_string()).and_then(crate::peer::hardware_command).map(|_|serde_json::json!({"accepted":true}))),
        ("POST", "/api/peer/config" | "/api/peer/timeline")=>peer_result(serde_json::from_slice::<serde_json::Value>(&request.body).map_err(|error|error.to_string()).and_then(|mut value|{
            if path=="/api/peer/config" {
                let consumer=media_stream::header(&request.headers,"x-pealayer-client").unwrap_or("");
                let object=value.as_object_mut().ok_or("Configuration request must be an object")?;
                object.insert("_consumer".into(),serde_json::Value::String(consumer.to_string()));
            }
            crate::peer::gui_request(path,value)
        })),
        ("POST", "/api/peer/files")=>peer_result(serde_json::from_slice(&request.body).map_err(|error|error.to_string()).and_then(|value|crate::peer::gui_request(path,value))),
        ("POST", "/api/peer/open")=>peer_result(serde_json::from_slice(&request.body).map_err(|error|error.to_string()).and_then(|value|crate::peer::gui_request(path,value))),
        ("OPTIONS", _) => HttpResponse::text(204, "No Content", ""),
        ("GET", "/healthz") => HttpResponse::json(
            200,
            "OK",
            r#"{"status":"ok","service":"pealayer","rpc":"2.0"}"#,
        ),
        ("GET", "/api/runtime/config") => {
            HttpResponse::json(200, "OK", state.runtime_config_json.to_string())
        }
        ("GET", "/api/runtime/app-icon") => runtime_app_icon_response(&request.target),
        ("GET", "/api/runtime/app-icon-192.png") => runtime_pwa_icon_response(&request.target, 192),
        ("GET", "/api/runtime/app-icon-512.png") => runtime_pwa_icon_response(&request.target, 512),
        ("GET", "/manifest.webmanifest") => pwa_manifest_response(state),
        ("GET", "/api/config") => HttpResponse::json(
            200,
            "OK",
            serde_json::to_string_pretty(&crate::platform::interop::get_live_config())
                .unwrap_or_else(|_| "{}".to_string()),
        ),
        ("GET", "/api/preferences") => HttpResponse::json(
            200,
            "OK",
            serde_json::to_string_pretty(&crate::preferences_contract::preferences_contract(
                &crate::platform::interop::get_live_config(),
            ))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        ("POST", "/api/config") => config_update_response(&request.body, state),
        ("GET", "/api/update/manifest") => match crate::update::current_manifest() {
            Ok(manifest) => HttpResponse::json(
                200,
                "OK",
                serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".to_string()),
            ),
            Err(error) => update_error_response(500, "Internal Server Error", error),
        },
        ("GET", "/api/update/artifact") => match crate::update::current_artifact() {
            Ok(bytes) => HttpResponse::bytes(200, "OK", "application/octet-stream", bytes),
            Err(error) => update_error_response(500, "Internal Server Error", error),
        },
        ("GET", "/api/update/status") => HttpResponse::json(
            200,
            "OK",
            serde_json::to_string(&crate::update::manager().status())
                .unwrap_or_else(|_| "{}".to_string()),
        ),
        ("POST", "/api/update/begin") => {
            match serde_json::from_slice::<crate::update::BeginUploadRequest>(&request.body)
                .map_err(|error| format!("invalid begin-update request: {error}"))
                .and_then(|request| crate::update::manager().begin_upload(request))
            {
                Ok(status) => update_status_response(202, "Accepted", status),
                Err(error) => update_error_response(400, "Bad Request", error),
            }
        }
        ("POST", "/api/update/chunk") => {
            let result = query_value(&request.target, "id")
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "update chunk requires an operation id".to_string())
                .and_then(|operation_id| {
                    query_value(&request.target, "offset")
                        .ok_or_else(|| "update chunk requires an offset".to_string())
                        .and_then(|value| {
                            value
                                .parse::<u64>()
                                .map_err(|_| "update chunk offset must be an integer".to_string())
                        })
                        .and_then(|offset| {
                            crate::update::manager().append_chunk(
                                &operation_id,
                                offset,
                                &request.body,
                            )
                        })
                });
            match result {
                Ok(status) => update_status_response(200, "OK", status),
                Err(error) => update_error_response(400, "Bad Request", error),
            }
        }
        ("POST", "/api/update/finish") => {
            match serde_json::from_slice::<crate::update::FinishUploadRequest>(&request.body)
                .map_err(|error| format!("invalid finish-update request: {error}"))
                .and_then(|request| {
                    crate::update::manager()
                        .finish_upload(&request.operation_id, state.command_tx.clone())
                }) {
                Ok(status) => update_status_response(202, "Accepted", status),
                Err(error) => update_error_response(400, "Bad Request", error),
            }
        }
        ("POST", "/api/update/abort") => {
            match serde_json::from_slice::<crate::update::FinishUploadRequest>(&request.body)
                .map_err(|error| format!("invalid abort-update request: {error}"))
                .and_then(|request| crate::update::manager().abort_upload(&request.operation_id))
            {
                Ok(status) => update_status_response(200, "OK", status),
                Err(error) => update_error_response(400, "Bad Request", error),
            }
        }
        ("POST", "/api/update/from-url") => {
            match serde_json::from_slice::<crate::update::FetchUpdateRequest>(&request.body)
                .map_err(|error| format!("invalid URL-update request: {error}"))
                .and_then(|request| {
                    crate::update::manager().fetch_and_apply(request, state.command_tx.clone())
                }) {
                Ok(status) => update_status_response(202, "Accepted", status),
                Err(error) => update_error_response(400, "Bad Request", error),
            }
        }
        ("GET", "/api/messages") => HttpResponse::json(200, "OK", serde_json::to_string(&crate::messaging::snapshot()).unwrap_or_default()),
        ("GET", "/api/remote/state") => HttpResponse::json(200, "OK", serde_json::to_string(&crate::remote_location::snapshot()).unwrap_or_default()),
        ("GET", "/api/remote/thumbnail") => {
            match query_value(&request.target, "url").ok_or_else(|| "A listed URL is required".to_string()).and_then(|url| crate::remote_location::thumbnail(&url, &state.egui_ctx)) {
                Ok(Some(path)) => match std::fs::read(path) { Ok(bytes) => HttpResponse::bytes(200,"OK","image/jpeg",bytes), Err(e) => HttpResponse::json(404,"Not Found",serde_json::json!({"error":e.to_string()}).to_string()) },
                Ok(None) => HttpResponse::json(202,"Accepted",r#"{"status":"pending"}"#),
                Err(error) => HttpResponse::json(400,"Bad Request",serde_json::json!({"error":error}).to_string()),
            }
        }
        ("POST", "/api/messages") => {
            match serde_json::from_slice::<crate::messaging::ToastRequest>(&request.body) {
                Ok(toast) => {
                    let command = crate::platform::interop::InteropCommand::PublishToast { toast };
                    player_command_response(&serde_json::to_vec(&command).unwrap_or_default(), state)
                }
                Err(error) => HttpResponse::json(400, "Bad Request", serde_json::json!({"error": error.to_string()}).to_string()),
            }
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
        ("GET", "/api/player/commands") => HttpResponse::json(
            200,
            "OK",
            crate::platform::interop::command_catalog().to_string(),
        ),
        ("POST", "/api/osd") => osd_response(&request.body, false, state),
        ("DELETE", "/api/osd") => osd_response(&request.body, true, state),
        ("POST", "/api/rpc") => json_rpc_response(&request.body, state),
        ("POST", "/api/player/command") => player_command_response(&request.body, state),
        ("POST", "/api/ipc") => HttpResponse::json(
            200,
            "OK",
            dispatch_ipc_payload(state, String::from_utf8_lossy(&request.body).trim()),
        ),
        ("GET", "/api/player/frame") => player_frame_response(&request.target, state),
        ("GET", "/api/player/taskbar-preview") => HttpResponse::json(200,"OK",
            crate::platform::taskbar_preview::diagnostics().to_string()),
        ("GET", "/api/player/taskbar-preview.png") => match crate::platform::taskbar_preview::frame_png() {
            Some(bytes) => HttpResponse::bytes(200,"OK","image/png",bytes),
            None => HttpResponse::text(404,"Not Found","No MPV taskbar frame is available"),
        },
        ("GET", "/api/player/taskbar-icons.png") => match crate::platform::windows::thumbnail_toolbar_png() {
            Some(bytes) => HttpResponse::bytes(200,"OK","image/png",bytes),
            None => HttpResponse::text(404,"Not Found","Windows thumbnail toolbar is unavailable"),
        },
        ("GET", "/api/player/seek-thumbnail") => {
            player_seek_thumbnail_response(&request.target, state)
        }
        ("GET", "/api/fs/browse") => browse_response(&request.target),
        ("GET", "/api/fs/thumbnail") => thumbnail_response(&request.target),
        ("POST", "/api/fs/rename") => rename_response(&request.body),
        ("POST", "/api/fs/trash") => trash_response(&request.body),
        ("GET", _) => static_response(&request.target, state),
        _ => HttpResponse::text(404, "Not Found", "Not Found"),
    }
}

fn denied_web_capability(
    method: &str,
    path: &str,
    web: &crate::config::AppConfig,
) -> Option<&'static str> {
    let configuration_route = matches!(
        (method, path),
        ("GET", "/api/config") | ("GET", "/api/preferences") | ("POST", "/api/config") | ("GET", "/api/peer/session") | ("POST", "/api/peer/config") | ("POST", "/api/peer/files")
    );
    let control_route = matches!(
        (method, path),
        ("POST", "/api/osd")
            | ("POST", "/api/messages")
            | ("DELETE", "/api/osd")
            | ("POST", "/api/player/command")
            | ("POST", "/api/ipc")
            | ("POST", "/api/process/command")
    );
    let file_route = path.starts_with("/api/fs/") || path.starts_with("/api/remote/") || matches!(path,"/api/peer/files"|"/api/peer/open")
        || matches!(path, "/api/player/frame" | "/api/player/seek-thumbnail"
            | "/api/player/taskbar-preview" | "/api/player/taskbar-preview.png" | "/api/player/taskbar-icons.png");
    let update_route = path.starts_with("/api/update/");
    if configuration_route && !web.web_allow_configuration {
        return Some("configuration");
    }
    if (control_route || (method=="POST" && path.starts_with("/api/peer/"))) && !web.web_allow_control {
        return Some("control");
    }
    if file_route && !web.web_allow_file_access {
        return Some("host file access");
    }
    if update_route && !web.web_allow_updates {
        return Some("application updates");
    }
    None
}

fn peer_result(result: Result<serde_json::Value,String>) -> HttpResponse {
    match result {
        Ok(value)=>HttpResponse::json(200,"OK",value.to_string()),
        Err(error)=>HttpResponse::json(400,"Bad Request",serde_json::json!({"error":error}).to_string()),
    }
}

/// API gateway for a consumer's local port. Streams through bounded buffers;
/// upstream permissions, errors, ranges and content headers remain authoritative.
fn proxy_http(request: &HttpRequest, stream:&mut TcpStream)->std::io::Result<()> {
    let Some(client)=crate::peer::client() else {return Ok(())};
    let route=media_stream::header(&request.headers,"x-pealayer-route").unwrap_or("");
    if route.split(',').any(|id|id.trim()==crate::peer::instance_id()) || route.split(',').count()>8 {
        return write_http_response_headers(stream,HttpResponse::text(508,"Loop Detected","Pealayer peer routing loop"),request.method=="HEAD");
    }
    let forwarded=if route.is_empty(){crate::peer::instance_id().to_string()}else{format!("{route},{}",crate::peer::instance_id())};
    let result=(|| -> Result<reqwest::blocking::Response,String> {
        let method=reqwest::Method::from_bytes(request.method.as_bytes()).map_err(|error|error.to_string())?;
        let streaming=request.method=="GET" && request.target.split('?').next()==Some("/api/fs/file");
        let mut upstream=client.request(method,&request.target)?.timeout(Duration::from_secs(if streaming {86400}else{10}));
        let connection_tokens=request.headers.iter().filter(|(key,_)|key.eq_ignore_ascii_case("connection")).flat_map(|(_,value)|value.split(',')).map(|token|token.trim().to_ascii_lowercase()).collect::<Vec<_>>();
        for (key,value) in &request.headers {
            let lower=key.to_ascii_lowercase();
            if !hop_by_hop_header(&lower,&connection_tokens) && !matches!(lower.as_str(),"host"|"content-length"|"x-pealayer-route"|"x-pealayer-client"|"user-agent"|"origin") {upstream=upstream.header(key,value)}
        }
        let mut upstream=upstream.body(request.body.clone()).build().map_err(|error|error.to_string())?;
        // RequestBuilder::header appends. Replace the default route instead so
        // every relay retains the complete loop-detection chain exactly once.
        upstream.headers_mut().insert("x-pealayer-route",reqwest::header::HeaderValue::from_str(&forwarded).map_err(|error|error.to_string())?);
        if let Some(consumer)=media_stream::header(&request.headers,"x-pealayer-client")
            && uuid::Uuid::parse_str(consumer).is_ok()
        {
            upstream.headers_mut().insert("x-pealayer-client",reqwest::header::HeaderValue::from_str(consumer).map_err(|error|error.to_string())?);
        }
        client.http.execute(upstream).map_err(|error|error.to_string())
    })();
    let mut upstream=match result {Ok(response)=>response,Err(error)=>return write_http_response_headers(stream,HttpResponse::text(502,"Bad Gateway",error),request.method=="HEAD")};
    write!(stream,"HTTP/1.1 {} {}\r\nConnection: close\r\n",upstream.status().as_u16(),upstream.status().canonical_reason().unwrap_or("Upstream Response"))?;
    let connection_tokens=upstream.headers().get_all("connection").iter().filter_map(|value|value.to_str().ok()).flat_map(|value|value.split(',')).map(|token|token.trim().to_ascii_lowercase()).collect::<Vec<_>>();
    for (key,value) in upstream.headers(){
        if hop_by_hop_header(key.as_str(),&connection_tokens) || key.as_str().starts_with("access-control-"){continue}
        if let Ok(value)=value.to_str(){write!(stream,"{key}: {value}\r\n")?}
    }
    write!(stream,"{}\r\n",cors_headers(&request.headers))?;
    let _=stream.set_write_timeout(Some(Duration::from_secs(30)));
    if request.method!="HEAD"{std::io::copy(&mut upstream,stream)?;}
    stream.flush()
}

fn hop_by_hop_header(name:&str, connection_tokens:&[String])->bool {
    matches!(name,"connection"|"keep-alive"|"proxy-authenticate"|"proxy-authorization"|"te"|"trailer"|"transfer-encoding"|"upgrade") || connection_tokens.iter().any(|token|token==name)
}

fn permission_denied(capability: &str) -> HttpResponse {
    HttpResponse::json(
        403,
        "Forbidden",
        serde_json::json!({
            "error": format!("Web {capability} permission is disabled"),
            "permission": capability,
        })
        .to_string(),
    )
}

fn remote_command_permission(command: &crate::platform::interop::InteropCommand) -> Option<&'static str> {
    use crate::platform::interop::InteropCommand;
    if crate::platform::interop::get_live_config().web_allow_file_access { return None; }
    match command {
        InteropCommand::BrowseRemote { .. } | InteropCommand::SelectRemote { .. } | InteropCommand::Open { .. } => Some("Web host file access permission is disabled"),
        InteropCommand::Launch { request } => if request.target.is_some() || request.commands.iter().any(|c| remote_command_permission(c).is_some()) { Some("Web host file access permission is disabled") } else { None },
        _ => None,
    }
}

fn update_status_response(
    status: u16,
    reason: &'static str,
    update: crate::update::UpdateStatus,
) -> HttpResponse {
    HttpResponse::json(
        status,
        reason,
        serde_json::to_string(&update).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn update_error_response(status: u16, reason: &'static str, error: impl ToString) -> HttpResponse {
    HttpResponse::json(
        status,
        reason,
        serde_json::json!({"error": error.to_string()}).to_string(),
    )
}

fn requested_icon_state(target: &str) -> crate::branding::PlaybackIconState {
    query_value(target, "state")
        .as_deref()
        .and_then(crate::branding::PlaybackIconState::parse)
        .unwrap_or_else(crate::branding::current_state)
}

fn runtime_app_icon_response(target: &str) -> HttpResponse {
    let config = crate::platform::interop::get_live_config();
    if let Some((path, bytes)) = crate::branding::icon_bytes(&config, requested_icon_state(target))
    {
        return HttpResponse::bytes(200, "OK", crate::branding::icon_mime(&path), bytes)
            .with_cache_control("no-cache");
    }
    HttpResponse::bytes(
        200,
        "OK",
        "image/png",
        include_bytes!("../../assets/pealayer-icon.png").to_vec(),
    )
    .with_cache_control("no-cache")
}

fn runtime_pwa_icon_response(target: &str, size: u32) -> HttpResponse {
    let config = crate::platform::interop::get_live_config();
    pwa_icon_response(&config, requested_icon_state(target), size)
}

fn pwa_icon_response(config: &crate::config::AppConfig, state: crate::branding::PlaybackIconState, size: u32) -> HttpResponse {
    let configured = crate::branding::icon_image(config, state);
    let source = configured
        .or_else(|| image::load_from_memory(include_bytes!("../../assets/pealayer-icon.png")).ok());
    let Some(source) = source else {
        return HttpResponse::text(500, "Internal Server Error", "Application icon unavailable");
    };
    let resized = source.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let mut png = std::io::Cursor::new(Vec::new());
    if resized.write_to(&mut png, image::ImageFormat::Png).is_err() {
        return HttpResponse::text(500, "Internal Server Error", "Application icon unavailable");
    }
    HttpResponse::bytes(200, "OK", "image/png", png.into_inner()).with_cache_control("no-cache")
}

fn pwa_manifest_response(state: &ControlState) -> HttpResponse {
    let runtime = serde_json::from_str::<serde_json::Value>(&state.runtime_config_json)
        .unwrap_or_else(|_| serde_json::json!({}));
    let app_name = runtime
        .get("appName")
        .and_then(serde_json::Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Pealayer");
    let theme_color = runtime
        .get("accentColor")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("#0078d4");
    HttpResponse::bytes(
        200,
        "OK",
        "application/manifest+json; charset=utf-8",
        serde_json::json!({
            "name": app_name,
            "short_name": app_name,
            "id": "/",
            "description": "Media, timeline, effects, and live PCController hardware workspace",
            "start_url": "/#/player",
            "scope": "/",
            "display": "standalone",
            "display_override": ["window-controls-overlay", "standalone", "minimal-ui"],
            "background_color": "#080a0e",
            "theme_color": theme_color,
            "orientation": "any",
            "categories": ["entertainment", "multimedia", "productivity", "utilities"],
            "prefer_related_applications": false,
            "launch_handler": {"client_mode": ["navigate-existing", "auto"]},
            "icons": [
                {
                    "src": "/api/runtime/app-icon-192.png?state=stopped",
                    "sizes": "192x192",
                    "type": "image/png",
                    "purpose": "any maskable"
                },
                {
                    "src": "/api/runtime/app-icon-512.png?state=stopped",
                    "sizes": "512x512",
                    "type": "image/png",
                    "purpose": "any maskable"
                }
            ],
            "shortcuts": [
                {"name": "Player", "url": "/#/player"},
                {"name": "Timeline", "url": "/#/timeline"},
                {"name": "Effects", "url": "/#/effects"},
                {"name": "Hardware", "url": "/#/hardware"}
            ]
        })
        .to_string()
        .into_bytes(),
    )
    .with_cache_control("no-cache")
}

fn json_rpc_response(body: &[u8], state: &ControlState) -> HttpResponse {
    if local_process_payload(body) {
        if !crate::platform::interop::get_live_config().web_allow_control { return permission_denied("control"); }
        return HttpResponse::json(200, "OK", dispatch_ipc_payload(state, &String::from_utf8_lossy(body)));
    }
    let response = match serde_json::from_slice::<crate::platform::interop::JsonRpcRequest>(body) {
        Ok(request)
            if matches!(
                request.method.as_str(),
                "config.get" | "pealayer.config.get"
            ) =>
        {
            if crate::platform::interop::get_live_config().web_allow_configuration {
                crate::platform::interop::json_rpc_result(
                    &request.id,
                    serde_json::to_value(crate::platform::interop::get_live_config())
                        .unwrap_or_else(|_| serde_json::json!({})),
                )
            } else {
                crate::platform::interop::json_rpc_error(
                    &request.id,
                    -32003,
                    "Web configuration permission is disabled",
                )
            }
        }
        Ok(request) => match crate::platform::interop::command_from_json_rpc(&request) {
            Ok(Some(command)) => {
                if let Some(error) = remote_command_permission(&command) { return HttpResponse::json(403,"Forbidden",crate::platform::interop::json_rpc_error(&request.id,-32003,error)); }
                if !crate::platform::interop::get_live_config().web_allow_control {
                    return HttpResponse::json(
                        403,
                        "Forbidden",
                        crate::platform::interop::json_rpc_error(
                            &request.id,
                            -32003,
                            "Web control permission is disabled",
                        ),
                    );
                }
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

fn config_update_response(body: &[u8], state: &ControlState) -> HttpResponse {
    let parsed = serde_json::from_slice::<serde_json::Value>(body)
        .map_err(|error| format!("invalid configuration JSON: {error}"))
        .and_then(|values| {
            crate::config::AppConfig::validate_patch_shape(&values)?;
            crate::platform::interop::get_live_config().apply_patch(&values)?;
            Ok(values)
        });
    match parsed {
        Ok(values) => {
            let accepted = state
                .command_tx
                .send(crate::platform::interop::InteropCommand::UpdateConfig { values })
                .is_ok();
            if accepted {
                state.egui_ctx.request_repaint();
                HttpResponse::json(202, "Accepted", r#"{"accepted":true}"#)
            } else {
                HttpResponse::text(503, "Service Unavailable", "Dispatcher unavailable")
            }
        }
        Err(error) => HttpResponse::json(
            400,
            "Bad Request",
            serde_json::json!({"error": error}).to_string(),
        ),
    }
}

fn player_command_response(body: &[u8], state: &ControlState) -> HttpResponse {
    if local_process_payload(body) {
        if !crate::platform::interop::get_live_config().web_allow_control { return permission_denied("control"); }
        return HttpResponse::json(200, "OK", dispatch_ipc_payload(state, &String::from_utf8_lossy(body)));
    }
    match parse_player_command(body) {
        Ok(command) if remote_command_permission(&command).is_some() => permission_denied("host file access"),
        Ok(command) => match command.validate() {
            Err(error) => HttpResponse::json(
                400,
                "Bad Request",
                serde_json::json!({"error": error}).to_string(),
            ),
            Ok(()) => match state.command_tx.send(command) {
                Ok(()) => {
                    state.egui_ctx.request_repaint();
                    HttpResponse::json(200, "OK", r#"{"status":"ok"}"#)
                }
                Err(_) => HttpResponse::text(503, "Service Unavailable", "Dispatcher unavailable"),
            },
        },
        Err(error) => HttpResponse::json(
            400,
            "Bad Request",
            serde_json::json!({"error": format!("invalid command: {error}")}).to_string(),
        ),
    }
}

fn osd_response(body: &[u8], force_hide: bool, state: &ControlState) -> HttpResponse {
    use crate::platform::interop::{InteropCommand, OsdOptions};

    let command = if force_hide || body.iter().all(u8::is_ascii_whitespace) {
        Ok(InteropCommand::HideOsd)
    } else {
        serde_json::from_slice::<serde_json::Value>(body)
            .map_err(|error| format!("invalid OSD JSON: {error}"))
            .and_then(|value| match value {
                serde_json::Value::String(message) => Ok(InteropCommand::ShowMessage { message }),
                serde_json::Value::Null => Ok(InteropCommand::HideOsd),
                serde_json::Value::Object(values) => {
                    let message = values
                        .get("message")
                        .or_else(|| values.get("text"))
                        .or_else(|| values.get("value"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let options_value = values
                        .get("options")
                        .or_else(|| values.get("style"))
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::Object(values));
                    let options = serde_json::from_value::<OsdOptions>(options_value)
                        .map_err(|error| format!("invalid OSD options: {error}"))?;
                    Ok(InteropCommand::ShowOsd { message, options })
                }
                _ => Err("OSD body must be a JSON object, string, or null".to_string()),
            })
    };

    match command.and_then(|command| {
        command.validate()?;
        state
            .command_tx
            .send(command)
            .map_err(|_| "dispatcher unavailable".to_string())
    }) {
        Ok(()) => {
            state.egui_ctx.request_repaint();
            HttpResponse::json(202, "Accepted", r#"{"accepted":true}"#)
        }
        Err(error) => HttpResponse::json(
            400,
            "Bad Request",
            serde_json::json!({"error": error}).to_string(),
        ),
    }
}

fn parse_player_command(body: &[u8]) -> Result<crate::platform::interop::InteropCommand, String> {
    let value = serde_json::from_slice::<serde_json::Value>(body)
        .map_err(|error| format!("invalid command JSON: {error}"))?;
    if let Ok(command) = serde_json::from_value(value.clone()) {
        return Ok(command);
    }

    // The SPA uses the exact same dotted method names over WebSocket and the
    // HTTP fallback. Accept those names here as JSON-RPC-shaped commands so a
    // transient WebSocket outage cannot silently remove hardware/effect
    // capabilities from the web surface.
    let mut params = value
        .as_object()
        .cloned()
        .ok_or_else(|| "command body must be a JSON object".to_string())?;
    let method = params
        .remove("command")
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or_else(|| "command body is missing a command name".to_string())?;
    let request = crate::platform::interop::JsonRpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: serde_json::Value::Null,
        method,
        params: serde_json::Value::Object(params),
    };
    crate::platform::interop::command_from_json_rpc(&request)?
        .ok_or_else(|| "status queries must use the status endpoint".to_string())
}

fn dispatch_ipc_payload(state: &ControlState, payload: &str) -> String {
    use crate::platform::interop::InteropCommand;

    let (id, command) = match crate::platform::interop::parse_interop_request(payload) {
        Ok(parsed) => parsed,
        Err(error) => return crate::platform::interop::format_interop_error(None, -32600, &error),
    };
    if let Some(error) = remote_command_permission(&command) { return crate::platform::interop::format_interop_error(id, -32003, error); }
    if let Some(value) = command.query_result() {
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
    crate::platform::interop::wake_command_dispatcher(&state.egui_ctx);
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

fn current_media_target(state: &ControlState) -> Option<String> {
    state
        .latest_status
        .lock()
        .ok()
        .and_then(|status| status.clone())
        .as_deref()
        .and_then(|status| {
            serde_json::from_str::<crate::platform::interop::PlayerStatusResponse>(status).ok()
        })
        .and_then(|status| status.current_video)
}

fn player_seek_thumbnail_response(target: &str, state: &ControlState) -> HttpResponse {
    let Some(seconds) = query_value(target, "seconds").and_then(|value| value.parse::<f64>().ok())
    else {
        return HttpResponse::text(400, "Bad Request", "A numeric seconds value is required");
    };
    if !seconds.is_finite() || seconds < 0.0 {
        return HttpResponse::text(
            400,
            "Bad Request",
            "Seek-preview time must be finite and non-negative",
        );
    }
    let Some(media_target) = current_media_target(state) else {
        return HttpResponse::text(404, "Not Found", "No media is currently loaded");
    };
    let config = crate::config::AppConfig::load();
    match thumbnails::get_or_generate_seek_thumbnail(
        &media_target,
        seconds,
        config.open_url_use_proxy,
        config.open_url_proxy_url.as_deref(),
    ) {
        Ok(path) => match std::fs::read(path) {
            Ok(data) => HttpResponse::bytes(200, "OK", "image/jpeg", data),
            Err(error) => HttpResponse::text(
                500,
                "Internal Server Error",
                format!("Could not read the seek preview: {error}"),
            ),
        },
        Err(error) => HttpResponse::text(404, "Not Found", error),
    }
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

fn static_response(target: &str, state: &ControlState) -> HttpResponse {
    let path = target.split('?').next().unwrap_or("/");
    // Filenames are stable across builds. Never apply immutable HTTP caching;
    // the PWA uses exact versioned URLs and generation-specific caches.
    let cache_control = "no-cache";
    if path.starts_with("/assets/") && let Some(requested) = query_value(target, "v") {
        let build = std::fs::read(state.web_dist_root.join("pwa-build.json")).ok()
            .or_else(|| EMBEDDED_WEB_UI.get_file("pwa-build.json").map(|file| file.contents().to_vec()));
        if let Some(build) = build.and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            && build["version"].as_str().is_some_and(|current| current != requested)
        {
            return HttpResponse::text(409, "Conflict", "Web UI updated; reload to use the current version")
                .with_cache_control("no-cache");
        }
    }
    if !state.web_dist_root.as_os_str().is_empty() {
        let target = web_asset_path(&state.web_dist_root, path)
            .unwrap_or_else(|| state.web_dist_root.join("__invalid_request_path__"));
        if target.is_file()
            && let Ok(data) = std::fs::read(&target)
        {
            return HttpResponse::bytes(200, "OK", mime_for_path(&target), data)
                .with_cache_control(cache_control);
        }
    }
    let relative = path.trim_start_matches('/');
    let embedded_path = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    if let Some(file) = EMBEDDED_WEB_UI.get_file(embedded_path) {
        return HttpResponse::bytes(
            200,
            "OK",
            mime_for_path(std::path::Path::new(embedded_path)),
            file.contents().to_vec(),
        )
        .with_cache_control(cache_control);
    }
    if let Some(index) = EMBEDDED_WEB_UI.get_file("index.html") {
        return HttpResponse::bytes(
            200,
            "OK",
            "text/html; charset=utf-8",
            index.contents().to_vec(),
        )
        .with_cache_control("no-cache");
    }
    HttpResponse::bytes(
        200,
        "OK",
        "text/html; charset=utf-8",
        web_assets::INDEX_HTML.as_bytes().to_vec(),
    )
    .with_cache_control("no-cache")
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
    crate::remote_location::decoded(&value.replace('+', " "))
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("json") => "application/json",
        Some("webmanifest") => "application/manifest+json",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn remote_consumer_updates_stay_local_while_session_commands_relay() {
        for target in ["/api/update/manifest", "/api/update/begin", "/api/update/chunk?id=one", "/api/update/finish", "/api/update/status", "/api/update/from-url", "/api/client/status"] {
            assert!(!super::peer_relay_route(target), "{target}");
        }
        for target in ["/api/config", "/api/player/command", "/api/rpc", "/api/peer/session", "/api/fs/file?path=media"] {
            assert!(super::peer_relay_route(target), "{target}");
        }
        for target in ["/api/process/status", "/api/process/command"] {
            assert!(!super::peer_relay_route(target));
        }
    }
    use super::*;

    #[test]
    fn process_lifecycle_uses_one_contract_across_rpc_ipc_http_and_websocket() {
        use crate::platform::interop::InteropCommand;
        let (tx, rx) = channel();
        let state = ControlState {
            command_tx:tx, latest_status:Arc::new(Mutex::new(None)),
            websocket_clients:Arc::new(Mutex::new(vec![])), egui_ctx:eframe::egui::Context::default(),
            runtime_config_json:"{}".into(), web_dist_root:std::path::PathBuf::new(),
            launch_receipts:Arc::new(Mutex::new(LaunchReceiptCache::default())),
            application_identity:"Pealayer".into(), expected_session_id:None,
        };
        let query=r#"{"jsonrpc":"2.0","id":1,"method":"pealayer.process.status"}"#;
        assert!(local_process_payload(query.as_bytes()));
        let response:serde_json::Value=serde_json::from_str(&dispatch_ipc_payload(&state,query)).unwrap();
        assert_eq!(response["result"]["process_id"],std::process::id());
        assert!(rx.try_recv().is_err());
        assert!(handle_websocket_text(&state,query).unwrap().contains("process_id"));
        for response in [json_rpc_response(query.as_bytes(),&state),player_command_response(query.as_bytes(),&state)] {
            assert_eq!(response.status,200);
            let value:serde_json::Value=serde_json::from_slice(&response.body).unwrap();
            assert_eq!(value["result"]["process_id"],std::process::id());
        }
        let quit=r#"{"jsonrpc":"2.0","id":2,"method":"pealayer.process.quit"}"#;
        assert!(dispatch_ipc_payload(&state,quit).contains("accepted"));
        assert_eq!(rx.try_recv().unwrap(),InteropCommand::QuitLocal);
        let connect=serde_json::json!({"jsonrpc":"2.0","id":3,"method":"pealayer.process.connect", "params":{
            "operation_id":uuid::Uuid::new_v4().to_string(),"endpoint":"pealayer://publisher.example:8080","client_port":8081
        }}).to_string();
        assert!(local_process_payload(connect.as_bytes()));
        assert!(dispatch_ipc_payload(&state,&connect).contains("accepted"));
        assert!(matches!(rx.try_recv().unwrap(),InteropCommand::ConnectPeer{..}));
        assert!(!local_process_payload(br#"{"command":"quit"}"#));
        assert!(!local_process_payload(br#"{"command":"play"}"#));
        let invalid=r#"{"jsonrpc":"2.0","id":4,"method":"pealayer.process.connect","params":{}}"#;
        assert!(local_process_payload(invalid.as_bytes()));
        assert!(dispatch_ipc_payload(&state,invalid).contains("error"));
        assert!(rx.try_recv().is_err());
        let restricted=crate::config::AppConfig{web_allow_control:false,..Default::default()};
        assert_eq!(denied_web_capability("POST","/api/process/command",&restricted),Some("control"));
        assert_eq!(denied_web_capability("GET","/api/process/status",&restricted),None);
    }

    #[test]
    fn browser_origins_must_match_listener_host_and_cannot_rebind() {
        let local = Some("127.0.0.1:8080".parse().unwrap());
        let headers = |origin: &str, host: &str| vec![("Origin".to_string(), origin.to_string()), ("Host".to_string(), host.to_string())];
        assert!(browser_origin_allowed(&headers("http://127.0.0.1:8080", "127.0.0.1:8080"), local));
        assert!(!browser_origin_allowed(&headers("http://127.0.0.1:8081", "127.0.0.1:8080"), local));
        assert!(!browser_origin_allowed(&headers("http://rebind.invalid:8080", "rebind.invalid:8080"), local));
        assert!(!browser_origin_allowed(&headers("null", "127.0.0.1:8080"), local));
        assert!(browser_origin_allowed(&[("Host".into(), "127.0.0.1:8080".into())], local)); // Native RPC peers.
        assert!(!browser_origin_allowed(&[("Host".into(), "rebind.invalid:8080".into())], local));
        assert!(!browser_origin_allowed(&[], local));
        let mut duplicate = headers("http://127.0.0.1:8080", "127.0.0.1:8080");
        duplicate.push(("origin".into(), "http://127.0.0.1:8080".into()));
        assert!(!browser_origin_allowed(&duplicate, local));
    }

    #[test]
    fn remote_query_urls_preserve_unicode_and_encoded_separators() {
        let url = "https://files.invalid/فارسی +%20.mkv";
        let query = url::form_urlencoded::Serializer::new(String::new()).append_pair("url", url).finish();
        assert_eq!(query_value(&format!("/api/remote/thumbnail?{query}"), "url").as_deref(), Some(url));
        let config = crate::config::AppConfig { web_allow_file_access: false, ..Default::default() };
        assert_eq!(denied_web_capability("GET", "/api/remote/thumbnail", &config), Some("host file access"));
        assert_eq!(denied_web_capability("GET", "/api/remote/state", &config), Some("host file access"));
    }

    #[test]
    fn web_server_defaults_are_explicit_loopback_settings() {
        let config = crate::config::AppConfig::default();
        assert!(config.web_enabled);
        assert_eq!(config.web_listen_addresses, ["127.0.0.1"]);
        assert_eq!(config.web_port, 8080);
    }

    #[test]
    fn web_permissions_gate_only_their_owned_api_surfaces() {
        let restricted = crate::config::AppConfig {
            web_allow_control: false,
            web_allow_configuration: false,
            web_allow_file_access: false,
            web_allow_updates: false,
            ..crate::config::AppConfig::default()
        };
        assert_eq!(
            denied_web_capability("POST", "/api/player/command", &restricted),
            Some("control")
        );
        assert_eq!(
            denied_web_capability("GET", "/api/preferences", &restricted),
            Some("configuration")
        );
        assert_eq!(
            denied_web_capability("GET", "/api/fs/browse", &restricted),
            Some("host file access")
        );
        assert_eq!(
            denied_web_capability("POST", "/api/update/begin", &restricted),
            Some("application updates")
        );
        assert_eq!(
            denied_web_capability("GET", "/api/player/status", &restricted),
            None
        );
        assert_eq!(denied_web_capability("GET", "/", &restricted), None);
        assert_eq!(denied_web_capability("POST", "/api/messages", &restricted), Some("control"));
        assert_eq!(denied_web_capability("GET", "/api/messages", &restricted), None);
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

    #[test]
    fn runtime_icon_mime_supports_native_windows_icons() {
        assert_eq!(
            mime_for_path(std::path::Path::new("brand.ico")),
            "image/x-icon"
        );
        assert_eq!(
            mime_for_path(std::path::Path::new("brand.svg")),
            "image/svg+xml"
        );
        assert_eq!(
            mime_for_path(std::path::Path::new("brand.png")),
            "image/png"
        );
    }

    #[test]
    fn runtime_contract_carries_the_resolved_desktop_accent() {
        let runtime = WebRuntimeConfig::production(
            "Pealayer".to_string(),
            "en".to_string(),
            "ltr".to_string(),
            "system".to_string(),
            [56, 210, 122],
        );
        let value = serde_json::to_value(runtime).unwrap();
        assert_eq!(value["accentColor"], "#38d27a");
    }

    #[test]
    fn static_mime_table_recognizes_installable_web_manifests() {
        assert_eq!(
            mime_for_path(std::path::Path::new("manifest.webmanifest")),
            "application/manifest+json"
        );
    }

    #[test]
    fn pwa_icons_are_served_as_real_square_png_sizes() {
        for size in [192, 512] {
            let response = runtime_pwa_icon_response("/api/runtime/app-icon?state=stopped", size);
            assert_eq!(response.status, 200);
            assert_eq!(response.content_type, "image/png");
            let icon = image::load_from_memory(&response.body).unwrap();
            assert_eq!((icon.width(), icon.height()), (size, size));
        }
    }

    #[test]
    fn pwa_icon_response_respects_the_bundled_preset() {
        use crate::{branding::PlaybackIconState, config::AppIconPreset};
        let mut config = crate::config::AppConfig::default();
        let mut previous = None;
        for preset in [AppIconPreset::Current, AppIconPreset::Classic] {
            config.app_icon_preset = preset;
            let response = pwa_icon_response(&config, PlaybackIconState::Stopped, 192);
            assert_eq!(response.status, 200);
            assert_eq!(response.content_type, "image/png");
            let icon = image::load_from_memory(&response.body).unwrap();
            assert_eq!((icon.width(), icon.height()), (192, 192));
            if let Some(previous) = previous {
                assert_ne!(response.body, previous);
            }
            previous = Some(response.body);
        }
    }

    #[test]
    fn http_fallback_accepts_the_same_dotted_hardware_method_as_websocket() {
        let command = parse_player_command(
            br#"{"command":"hardware.action.invoke","action_id":"relay.5.on"}"#,
        )
        .unwrap();
        assert_eq!(
            command,
            crate::platform::interop::InteropCommand::InvokeHardwareAction {
                action_id: "relay.5.on".to_string()
            }
        );
    }

    #[test]
    fn http_fallback_preserves_native_command_encoding() {
        let command = parse_player_command(br#"{"command":"set_volume","value":42}"#).unwrap();
        assert_eq!(
            command,
            crate::platform::interop::InteropCommand::SetVolume { value: 42.0 }
        );
    }
}
