pub mod fs_api;
pub mod thumbnails;
pub mod web_assets;

use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

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
    pub ws_port: u16,
    pub locale: String,
    pub direction: String,
    pub theme: String,
}

impl WebRuntimeConfig {
    pub fn production(
        app_name: String,
        ws_port: u16,
        locale: String,
        direction: String,
        theme: String,
    ) -> Self {
        Self {
            app_name,
            version: env!("CARGO_PKG_VERSION").to_string(),
            ws_port,
            locale,
            direction,
            theme,
        }
    }
}

pub fn spawn_web_server(
    http_port: u16,
    ws_port: u16,
    egui_ctx: eframe::egui::Context,
) -> (
    Sender<String>,
    Receiver<crate::platform::interop::InteropCommand>,
) {
    spawn_web_server_configured(
        http_port,
        ws_port,
        egui_ctx,
        WebRuntimeConfig::production(
            "Pealayer".to_string(),
            ws_port,
            "en".to_string(),
            "ltr".to_string(),
            "system".to_string(),
        ),
    )
}

pub fn spawn_web_server_configured(
    http_port: u16,
    ws_port: u16,
    egui_ctx: eframe::egui::Context,
    runtime_config: WebRuntimeConfig,
) -> (
    Sender<String>,
    Receiver<crate::platform::interop::InteropCommand>,
) {
    let (cmd_tx, cmd_rx) = channel::<crate::platform::interop::InteropCommand>();
    let (state_tx, state_rx) = channel::<String>();

    // No player state is authoritative until the UI thread publishes its first
    // snapshot. Keeping this as None prevents new clients from observing an
    // invented idle/volume state during startup.
    let latest_status: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let latest_status_clone = latest_status.clone();

    let ws_clients: Arc<Mutex<Vec<Sender<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let ws_clients_clone = ws_clients.clone();
    let bind_address = web_bind_address();

    // 1. Broadcast state updates from main thread to WebSocket clients and cache latest status
    thread::spawn(move || {
        while let Ok(state_json) = state_rx.recv() {
            if let Ok(mut status_guard) = latest_status_clone.lock() {
                *status_guard = Some(state_json.clone());
            }
            let mut list = ws_clients_clone.lock().unwrap();
            list.retain(|tx| tx.send(state_json.clone()).is_ok());
        }
    });

    // 2. Dedicated WebSocket Server Thread on ws_port (8081)
    let cmd_tx_ws = cmd_tx.clone();
    let ws_clients_register = ws_clients.clone();
    let latest_status_ws = latest_status.clone();
    let egui_ctx_ws = egui_ctx.clone();
    thread::spawn(move || {
        let ws_addr = std::net::SocketAddr::new(bind_address, ws_port);
        if let Ok(listener) = std::net::TcpListener::bind(&ws_addr) {
            for stream in listener.incoming().flatten() {
                let cmd_tx_conn = cmd_tx_ws.clone();
                let ws_clients_conn = ws_clients_register.clone();
                let latest_status_conn = latest_status_ws.clone();
                let egui_ctx_conn = egui_ctx_ws.clone();

                thread::spawn(move || {
                    if let Ok(mut websocket) = tungstenite::accept(stream) {
                        let (client_tx, client_rx) = channel::<String>();
                        ws_clients_conn.lock().unwrap().push(client_tx);

                        websocket.get_mut().set_nonblocking(true).ok();

                        loop {
                            // Check for outgoing state broadcasts to send to WS client
                            if let Ok(msg_text) = client_rx.try_recv() {
                                if websocket
                                    .send(tungstenite::Message::Text(msg_text.into()))
                                    .is_err()
                                {
                                    break;
                                }
                            }

                            // Read incoming WebSocket frames from browser client
                            match websocket.read() {
                                Ok(tungstenite::Message::Text(text)) => {
                                    if let Ok(cmd) = serde_json::from_str::<
                                        crate::platform::interop::InteropCommand,
                                    >(&text)
                                    {
                                        let _ = cmd_tx_conn.send(cmd);
                                        egui_ctx_conn.request_repaint();
                                    } else if let Ok(request) =
                                        serde_json::from_str::<
                                            crate::platform::interop::JsonRpcRequest,
                                        >(&text)
                                    {
                                        let response =
                                            match crate::platform::interop::command_from_json_rpc(
                                                &request,
                                            ) {
                                                Ok(Some(command)) => {
                                                    let _ = cmd_tx_conn.send(command);
                                                    egui_ctx_conn.request_repaint();
                                                    crate::platform::interop::json_rpc_result(
                                                        &request.id,
                                                        serde_json::json!({"accepted":true}),
                                                    )
                                                }
                                                Ok(None) => {
                                                    let status =
                                                        latest_status_conn.lock().unwrap().clone();
                                                    let value = status
                                                    .as_deref()
                                                    .and_then(|status| serde_json::from_str(status).ok())
                                                    .unwrap_or_else(|| serde_json::json!({"status":"initializing"}));
                                                    crate::platform::interop::json_rpc_result(
                                                        &request.id,
                                                        value,
                                                    )
                                                }
                                                Err(error) => {
                                                    crate::platform::interop::json_rpc_error(
                                                        &request.id,
                                                        -32601,
                                                        &error,
                                                    )
                                                }
                                            };
                                        let _ = websocket
                                            .send(tungstenite::Message::Text(response.into()));
                                    }
                                }
                                Ok(tungstenite::Message::Close(_)) => break,
                                Err(tungstenite::Error::Io(ref e))
                                    if e.kind() == std::io::ErrorKind::WouldBlock =>
                                {
                                    thread::sleep(std::time::Duration::from_millis(15));
                                }
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    }
                });
            }
        }
    });

    // 3. HTTP Web & REST Server Thread on http_port (8080)
    let latest_status_http = latest_status.clone();
    let cmd_tx_http = cmd_tx.clone();
    let egui_ctx_http = egui_ctx.clone();
    let web_dist_http = web_dist_root();
    let runtime_config_json =
        serde_json::to_string(&runtime_config).expect("web runtime configuration must serialize");
    thread::spawn(move || {
        let server_addr = std::net::SocketAddr::new(bind_address, http_port);
        if let Ok(server) = tiny_http::Server::http(&server_addr) {
            for mut request in server.incoming_requests() {
                let url = request.url().to_string();

                if url == "/api/runtime/config" {
                    let response = tiny_http::Response::from_string(runtime_config_json.clone())
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/json"[..],
                            )
                            .unwrap(),
                        );
                    let _ = request.respond(response);
                } else if url == "/api/config" && request.method() == &tiny_http::Method::Get {
                    let body =
                        serde_json::to_string_pretty(&crate::platform::interop::get_live_config())
                            .unwrap_or_else(|_| "{}".to_string());
                    let response = tiny_http::Response::from_string(body).with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    );
                    let _ = request.respond(response);
                } else if url == "/api/config" && request.method() == &tiny_http::Method::Post {
                    let mut body = String::new();
                    let parsed = request
                        .as_reader()
                        .take(1024 * 1024)
                        .read_to_string(&mut body)
                        .map_err(|error| format!("read configuration update: {error}"))
                        .and_then(|_| {
                            serde_json::from_str::<serde_json::Value>(&body)
                                .map_err(|error| format!("invalid configuration JSON: {error}"))
                        })
                        .and_then(|values| {
                            crate::config::AppConfig::validate_patch_shape(&values)?;
                            crate::platform::interop::get_live_config().apply_patch(&values)?;
                            Ok(values)
                        });
                    match parsed {
                        Ok(values) => {
                            let _ = cmd_tx_http.send(
                                crate::platform::interop::InteropCommand::UpdateConfig { values },
                            );
                            egui_ctx_http.request_repaint();
                            let _ = request.respond(
                                tiny_http::Response::from_string("{\"accepted\":true}")
                                    .with_status_code(202)
                                    .with_header(
                                        tiny_http::Header::from_bytes(
                                            &b"Content-Type"[..],
                                            &b"application/json"[..],
                                        )
                                        .unwrap(),
                                    ),
                            );
                        }
                        Err(error) => {
                            let _ = request.respond(
                                tiny_http::Response::from_string(
                                    serde_json::json!({"error": error}).to_string(),
                                )
                                .with_status_code(400)
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/json"[..],
                                    )
                                    .unwrap(),
                                ),
                            );
                        }
                    }
                } else if url == "/healthz" {
                    let response = tiny_http::Response::from_string(
                        "{\"status\":\"ok\",\"service\":\"pealayer\",\"rpc\":\"2.0\"}",
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    );
                    let _ = request.respond(response);
                } else if url.starts_with("/api/player/status") {
                    let json = latest_status_http.lock().unwrap().clone();
                    let (body, status_code) = json
                        .map(|body| (body, 200))
                        .unwrap_or_else(|| ("{\"status\":\"initializing\"}".to_string(), 503));
                    let response = tiny_http::Response::from_string(body)
                        .with_status_code(status_code)
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/json"[..],
                            )
                            .unwrap(),
                        );
                    let _ = request.respond(response);
                } else if url.starts_with("/api/rpc")
                    && request.method() == &tiny_http::Method::Post
                {
                    let mut body = String::new();
                    let _ = request
                        .as_reader()
                        .take(1024 * 1024)
                        .read_to_string(&mut body);
                    let response_body = match serde_json::from_str::<
                        crate::platform::interop::JsonRpcRequest,
                    >(&body)
                    {
                        Ok(rpc)
                            if matches!(
                                rpc.method.as_str(),
                                "config.get" | "pealayer.config.get"
                            ) =>
                        {
                            crate::platform::interop::json_rpc_result(
                                &rpc.id,
                                serde_json::to_value(crate::platform::interop::get_live_config())
                                    .unwrap_or_else(|_| serde_json::json!({})),
                            )
                        }
                        Ok(rpc) => match crate::platform::interop::command_from_json_rpc(&rpc) {
                            Ok(Some(command)) => {
                                let _ = cmd_tx_http.send(command);
                                egui_ctx_http.request_repaint();
                                crate::platform::interop::json_rpc_result(
                                    &rpc.id,
                                    serde_json::json!({"accepted":true}),
                                )
                            }
                            Ok(None) => {
                                let status = latest_status_http.lock().unwrap().clone();
                                let value = status
                                    .as_deref()
                                    .and_then(|status| serde_json::from_str(status).ok())
                                    .unwrap_or_else(
                                        || serde_json::json!({"status":"initializing"}),
                                    );
                                crate::platform::interop::json_rpc_result(&rpc.id, value)
                            }
                            Err(error) => {
                                crate::platform::interop::json_rpc_error(&rpc.id, -32601, &error)
                            }
                        },
                        Err(error) => crate::platform::interop::json_rpc_error(
                            &serde_json::Value::Null,
                            -32700,
                            &format!("invalid JSON-RPC request: {error}"),
                        ),
                    };
                    let response = tiny_http::Response::from_string(response_body).with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    );
                    let _ = request.respond(response);
                } else if url.starts_with("/api/player/command")
                    && request.method() == &tiny_http::Method::Post
                {
                    let mut body = String::new();
                    let reader = request.as_reader();
                    if reader.read_to_string(&mut body).is_ok() {
                        if let Ok(cmd) =
                            serde_json::from_str::<crate::platform::interop::InteropCommand>(&body)
                        {
                            let _ = cmd_tx_http.send(cmd);
                            egui_ctx_http.request_repaint();
                            let response = tiny_http::Response::from_string("{\"status\":\"ok\"}")
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/json"[..],
                                    )
                                    .unwrap(),
                                );
                            let _ = request.respond(response);
                            continue;
                        }
                    }
                    let _ = request.respond(
                        tiny_http::Response::from_string("Bad Command").with_status_code(400),
                    );
                } else if url.starts_with("/api/player/frame") {
                    let mut path_opt = None;
                    if let Some(p) = url.split("path=").nth(1) {
                        let path_clean = p.split('&').next().unwrap_or(p);
                        let decoded = urlencoding_decode(path_clean);
                        if !decoded.is_empty() {
                            path_opt = Some(std::path::PathBuf::from(decoded));
                        }
                    }
                    if path_opt.is_none() {
                        if let Some(status) = latest_status_http.lock().unwrap().as_deref() {
                            if let Ok(json) = serde_json::from_str::<
                                crate::platform::interop::PlayerStatusResponse,
                            >(status)
                            {
                                if let Some(v_path) = json.current_video {
                                    path_opt = Some(std::path::PathBuf::from(v_path));
                                }
                            }
                        }
                    }
                    if let Some(ref path) = path_opt {
                        if let Some(thumb_path) = thumbnails::get_or_generate_thumbnail(path) {
                            if let Ok(data) = std::fs::read(&thumb_path) {
                                let response = tiny_http::Response::from_data(data).with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"image/jpeg"[..],
                                    )
                                    .unwrap(),
                                );
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let response =
                        tiny_http::Response::from_string("Frame Not Found").with_status_code(404);
                    let _ = request.respond(response);
                } else if url.starts_with("/api/fs/browse") {
                    let path_param = url.split("path=").nth(1).map(|p| p.to_string());
                    let decoded_path = path_param.as_deref().map(|p| urlencoding_decode(p));

                    if let Ok(res) = fs_api::browse_directory(decoded_path.as_deref()) {
                        let json = serde_json::to_string(&res).unwrap_or_default();
                        let response = tiny_http::Response::from_string(json).with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/json"[..],
                            )
                            .unwrap(),
                        );
                        let _ = request.respond(response);
                    }
                } else if url.starts_with("/api/fs/thumbnail") {
                    if let Some(path_param) = url.split("path=").nth(1) {
                        let decoded = urlencoding_decode(path_param);
                        let path = std::path::PathBuf::from(&decoded);
                        if let Some(thumb_path) = thumbnails::get_or_generate_thumbnail(&path) {
                            if let Ok(data) = std::fs::read(&thumb_path) {
                                let response = tiny_http::Response::from_data(data).with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"image/jpeg"[..],
                                    )
                                    .unwrap(),
                                );
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let response = tiny_http::Response::from_string("Thumbnail Not Found")
                        .with_status_code(404);
                    let _ = request.respond(response);
                } else if url.starts_with("/api/fs/rename")
                    && request.method() == &tiny_http::Method::Post
                {
                    let mut body = String::new();
                    let reader = request.as_reader();
                    if reader.read_to_string(&mut body).is_ok() {
                        if let Ok(req) = serde_json::from_str::<fs_api::RenameRequest>(&body) {
                            if let Ok(new_path) = fs_api::rename_file(&req.old_path, &req.new_name)
                            {
                                let response = tiny_http::Response::from_string(format!(
                                    "{{\"status\":\"ok\",\"new_path\":\"{}\"}}",
                                    new_path
                                ))
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/json"[..],
                                    )
                                    .unwrap(),
                                );
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let _ = request
                        .respond(tiny_http::Response::from_string("Error").with_status_code(400));
                } else if url.starts_with("/api/fs/trash")
                    && request.method() == &tiny_http::Method::Post
                {
                    let mut body = String::new();
                    let reader = request.as_reader();
                    if reader.read_to_string(&mut body).is_ok() {
                        if let Ok(req) = serde_json::from_str::<fs_api::TrashRequest>(&body) {
                            if fs_api::trash_file(&req.target_path).is_ok() {
                                let response =
                                    tiny_http::Response::from_string("{\"status\":\"ok\"}")
                                        .with_header(
                                            tiny_http::Header::from_bytes(
                                                &b"Content-Type"[..],
                                                &b"application/json"[..],
                                            )
                                            .unwrap(),
                                        );
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let _ = request
                        .respond(tiny_http::Response::from_string("Error").with_status_code(400));
                } else {
                    // Serve Web UI single page app (Ant Design React interface from web_ui/dist)
                    let req_path = url.split('?').next().unwrap_or(&url);
                    let target_file = web_asset_path(&web_dist_http, req_path)
                        .unwrap_or_else(|| web_dist_http.join("__invalid_request_path__"));

                    if target_file.exists() && target_file.is_file() {
                        if let Ok(data) = std::fs::read(&target_file) {
                            let mime = match target_file.extension().and_then(|e| e.to_str()) {
                                Some("html") => "text/html; charset=utf-8",
                                Some("js") => "application/javascript; charset=utf-8",
                                Some("css") => "text/css; charset=utf-8",
                                Some("svg") => "image/svg+xml",
                                Some("png") => "image/png",
                                Some("jpg") | Some("jpeg") => "image/jpeg",
                                Some("json") => "application/json",
                                Some("woff2") => "font/woff2",
                                _ => "application/octet-stream",
                            };
                            let response = tiny_http::Response::from_data(data).with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    mime.as_bytes(),
                                )
                                .unwrap(),
                            );
                            let _ = request.respond(response);
                            continue;
                        }
                    }

                    // SPA fallback: serve web_ui/dist/index.html if available, or fallback to web_assets::INDEX_HTML
                    let fallback_html = std::fs::read_to_string(web_dist_http.join("index.html"))
                        .unwrap_or_else(|_| web_assets::INDEX_HTML.to_string());
                    let response = tiny_http::Response::from_string(fallback_html).with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"text/html; charset=utf-8"[..],
                        )
                        .unwrap(),
                    );
                    let _ = request.respond(response);
                }
            }
        }
    });

    (state_tx, cmd_rx)
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let mut hex = String::new();
            if let Some(h1) = chars.next() {
                hex.push(h1);
            }
            if let Some(h2) = chars.next() {
                hex.push(h2);
            }
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }
    result
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
        let raw = "hello%20world%2Ftest";
        assert_eq!(urlencoding_decode(raw), "hello world/test");
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
