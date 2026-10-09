//! Download-ID media delivery; existing origin/peer/permission gates still apply.
use std::{io::Write, net::TcpStream};
pub(super) fn serve(request: &super::HttpRequest, stream: &mut TcpStream) -> std::io::Result<()> {
    let error = |stream: &mut TcpStream, response| {
        super::write_http_response_headers(stream, response, request.method == "HEAD")
    };
    if !crate::platform::interop::get_live_config().web_allow_file_access {
        return error(stream, super::permission_denied("host file access"));
    }
    let Some(id) =
        super::query_value(&request.target, "id").filter(|id| uuid::Uuid::parse_str(id).is_ok())
    else {
        return error(
            stream,
            super::HttpResponse::text(400, "Bad Request", "A valid download ID is required"),
        );
    };
    let manager = match crate::downloads::manager() {
        Ok(manager) => manager,
        Err(message) => {
            return error(
                stream,
                super::HttpResponse::text(503, "Service Unavailable", message),
            );
        }
    };
    let info = match manager.cache_info(&id) {
        Ok(info) => info,
        Err(message) => return error(stream, super::HttpResponse::text(409, "Conflict", message)),
    };
    let range = super::media_stream::header(&request.headers, "range")
        .map(|value| super::media_stream::byte_range(value, info.total))
        .transpose();
    let range = match range {
        Ok(value) => value.flatten(),
        Err(()) => {
            write!(
                stream,
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                info.total
            )?;
            return stream.flush();
        }
    };
    let (start, end) = range.unwrap_or((0, info.total.saturating_sub(1)));
    let mut reader = if request.method == "HEAD" {
        None
    } else {
        match manager.read_range(&id, start, end) {
            Ok(reader) => Some(reader),
            Err(message) => {
                return error(stream, super::HttpResponse::text(409, "Conflict", message));
            }
        }
    };
    let mime = match std::path::Path::new(&info.filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    };
    let length = if info.total == 0 { 0 } else { end - start + 1 };
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {mime}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nCache-Control: private, no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n{}",
        if range.is_some() {
            "206 Partial Content"
        } else {
            "200 OK"
        },
        super::cors_headers(&request.headers)
    )?;
    if range.is_some() {
        write!(
            stream,
            "Content-Range: bytes {start}-{end}/{}\r\n",
            info.total
        )?;
    }
    write!(stream, "\r\n")?;
    if let Some(reader) = &mut reader {
        std::io::copy(reader, stream)?;
    }
    stream.flush()
}
