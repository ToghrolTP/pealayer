pub mod fs_api;
pub mod thumbnails;
pub mod web_assets;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn spawn_web_server(port: u16) -> (Sender<String>, Receiver<crate::platform::interop::InteropCommand>) {
    let (_cmd_tx, cmd_rx) = channel::<crate::platform::interop::InteropCommand>();
    let (state_tx, state_rx) = channel::<String>();

    let clients: Arc<Mutex<Vec<Sender<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let clients_clone = clients.clone();

    // Broadcast state updates from main thread to WebSocket clients
    thread::spawn(move || {
        while let Ok(state_json) = state_rx.recv() {
            let mut list = clients_clone.lock().unwrap();
            list.retain(|tx| tx.send(state_json.clone()).is_ok());
        }
    });

    thread::spawn(move || {
        let server_addr = format!("0.0.0.0:{}", port);
        if let Ok(server) = tiny_http::Server::http(&server_addr) {
            for mut request in server.incoming_requests() {
                let url = request.url().to_string();

                if url.starts_with("/api/fs/browse") {
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
                } else if url.starts_with("/ws/control") {
                    // WebSocket Upgrade Connection
                    let clients_ref = clients.clone();
                    let (ws_tx, ws_rx) = channel::<String>();

                    clients_ref.lock().unwrap().push(ws_tx);

                    // Accept WebSocket framing via HTTP response
                    let response = tiny_http::Response::from_string(web_assets::INDEX_HTML)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap());
                    let _ = request.respond(response);

                    thread::spawn(move || {
                        while let Ok(_msg) = ws_rx.recv() {
                            // WS message push
                        }
                    });
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
