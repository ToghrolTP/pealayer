pub mod fs_api;
pub mod thumbnails;
pub mod web_assets;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn spawn_web_server(http_port: u16, ws_port: u16, egui_ctx: eframe::egui::Context) -> (Sender<String>, Receiver<crate::platform::interop::InteropCommand>) {
    let (cmd_tx, cmd_rx) = channel::<crate::platform::interop::InteropCommand>();
    let (state_tx, state_rx) = channel::<String>();

    let latest_status: Arc<Mutex<String>> = Arc::new(Mutex::new("{\"status\":\"ok\",\"playing\":false,\"volume\":100.0,\"playback_time\":0.0,\"duration\":0.0}".to_string()));
    let latest_status_clone = latest_status.clone();

    let ws_clients: Arc<Mutex<Vec<Sender<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let ws_clients_clone = ws_clients.clone();

    // 1. Broadcast state updates from main thread to WebSocket clients and cache latest status
    thread::spawn(move || {
        while let Ok(state_json) = state_rx.recv() {
            if let Ok(mut status_guard) = latest_status_clone.lock() {
                *status_guard = state_json.clone();
            }
            let mut list = ws_clients_clone.lock().unwrap();
            list.retain(|tx| tx.send(state_json.clone()).is_ok());
        }
    });

    // 2. Dedicated WebSocket Server Thread on ws_port (8081)
    let cmd_tx_ws = cmd_tx.clone();
    let ws_clients_register = ws_clients.clone();
    let ctx_ws = egui_ctx.clone();
    thread::spawn(move || {
        let ws_addr = format!("0.0.0.0:{}", ws_port);
        if let Ok(listener) = std::net::TcpListener::bind(&ws_addr) {
            for stream in listener.incoming().flatten() {
                let cmd_tx_conn = cmd_tx_ws.clone();
                let ws_clients_conn = ws_clients_register.clone();
                let ctx_conn = ctx_ws.clone();

                thread::spawn(move || {
                    if let Ok(mut websocket) = tungstenite::accept(stream) {
                        let (client_tx, client_rx) = channel::<String>();
                        ws_clients_conn.lock().unwrap().push(client_tx);

                        websocket.get_mut().set_nonblocking(true).ok();

                        loop {
                            // Check for outgoing state broadcasts to send to WS client
                            if let Ok(msg_text) = client_rx.try_recv() {
                                if websocket.send(tungstenite::Message::Text(msg_text.into())).is_err() {
                                    break;
                                }
                            }

                            // Read incoming WebSocket frames from browser client
                            match websocket.read() {
                                Ok(tungstenite::Message::Text(text)) => {
                                    if let Ok(cmd) = serde_json::from_str::<crate::platform::interop::InteropCommand>(&text) {
                                        let _ = cmd_tx_conn.send(cmd);
                                        ctx_conn.request_repaint();
                                    }
                                }
                                Ok(tungstenite::Message::Close(_)) => break,
                                Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
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
    let ctx_http = egui_ctx.clone();
    thread::spawn(move || {
        let server_addr = format!("0.0.0.0:{}", http_port);
        if let Ok(server) = tiny_http::Server::http(&server_addr) {
            for mut request in server.incoming_requests() {
                let url = request.url().to_string();

                if url.starts_with("/api/player/status") {
                    let json = latest_status_http.lock().unwrap().clone();
                    let response = tiny_http::Response::from_string(json)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    let _ = request.respond(response);
                } else if url.starts_with("/api/player/command") && request.method() == &tiny_http::Method::Post {
                    let mut body = String::new();
                    let reader = request.as_reader();
                    if reader.read_to_string(&mut body).is_ok() {
                        if let Ok(cmd) = serde_json::from_str::<crate::platform::interop::InteropCommand>(&body) {
                            let _ = cmd_tx_http.send(cmd);
                            ctx_http.request_repaint();
                            let response = tiny_http::Response::from_string("{\"status\":\"ok\"}")
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(response);
                            continue;
                        }
                    }
                    let _ = request.respond(tiny_http::Response::from_string("Bad Command").with_status_code(400));
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
                        if let Ok(json) = serde_json::from_str::<crate::platform::interop::PlayerStatusResponse>(&latest_status_http.lock().unwrap()) {
                            if let Some(v_path) = json.current_video {
                                path_opt = Some(std::path::PathBuf::from(v_path));
                            }
                        }
                    }
                    if let Some(ref path) = path_opt {
                        if let Some(thumb_path) = thumbnails::get_or_generate_thumbnail(path) {
                            if let Ok(data) = std::fs::read(&thumb_path) {
                                let response = tiny_http::Response::from_data(data)
                                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"image/jpeg"[..]).unwrap());
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let response = tiny_http::Response::from_string("Frame Not Found").with_status_code(404);
                    let _ = request.respond(response);
                } else if url.starts_with("/api/fs/browse") {
                    let path_param = url.split("path=").nth(1).map(|p| p.to_string());
                    let decoded_path = path_param.as_deref().map(|p| urlencoding_decode(p));

                    if let Ok(res) = fs_api::browse_directory(decoded_path.as_deref()) {
                        let json = serde_json::to_string(&res).unwrap_or_default();
                        let response = tiny_http::Response::from_string(json)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                        let _ = request.respond(response);
                    }
                } else if url.starts_with("/api/fs/thumbnail") {
                    if let Some(path_param) = url.split("path=").nth(1) {
                        let decoded = urlencoding_decode(path_param);
                        let path = std::path::PathBuf::from(&decoded);
                        if let Some(thumb_path) = thumbnails::get_or_generate_thumbnail(&path) {
                            if let Ok(data) = std::fs::read(&thumb_path) {
                                let response = tiny_http::Response::from_data(data)
                                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"image/jpeg"[..]).unwrap());
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let response = tiny_http::Response::from_string("Thumbnail Not Found")
                        .with_status_code(404);
                    let _ = request.respond(response);
                } else if url.starts_with("/api/fs/rename") && request.method() == &tiny_http::Method::Post {
                    let mut body = String::new();
                    let reader = request.as_reader();
                    if reader.read_to_string(&mut body).is_ok() {
                        if let Ok(req) = serde_json::from_str::<fs_api::RenameRequest>(&body) {
                            if let Ok(new_path) = fs_api::rename_file(&req.old_path, &req.new_name) {
                                let response = tiny_http::Response::from_string(format!("{{\"status\":\"ok\",\"new_path\":\"{}\"}}", new_path))
                                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let _ = request.respond(tiny_http::Response::from_string("Error").with_status_code(400));
                } else if url.starts_with("/api/fs/trash") && request.method() == &tiny_http::Method::Post {
                    let mut body = String::new();
                    let reader = request.as_reader();
                    if reader.read_to_string(&mut body).is_ok() {
                        if let Ok(req) = serde_json::from_str::<fs_api::TrashRequest>(&body) {
                            if fs_api::trash_file(&req.target_path).is_ok() {
                                let response = tiny_http::Response::from_string("{\"status\":\"ok\"}")
                                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
                    let _ = request.respond(tiny_http::Response::from_string("Error").with_status_code(400));
                } else {
                    // Serve Embedded Web UI single page app
                    let response = tiny_http::Response::from_string(web_assets::INDEX_HTML)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap());
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
            if let Some(h1) = chars.next() { hex.push(h1); }
            if let Some(h2) = chars.next() { hex.push(h2); }
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

    #[test]
    fn test_url_decoding() {
        let raw = "hello%20world%2Ftest";
        assert_eq!(urlencoding_decode(raw), "hello world/test");
    }
}
