//! Bounded, seekable file delivery. Never buffers a movie in an HTTP response.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::time::UNIX_EPOCH;

pub(crate) fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// A server may ignore unsupported multiple ranges; never misreport them as a
/// single partial response. Empty files and unsatisfiable ranges produce 416.
pub(crate) fn byte_range(value: &str, size: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(value) = value.strip_prefix("bytes=") else {
        return Ok(None);
    };
    if value.contains(',') {
        return Ok(None);
    }
    let (start, end) = value.trim().split_once('-').ok_or(())?;
    if size == 0 {
        return Err(());
    }
    if start.is_empty() {
        let suffix = end.parse::<u64>().map_err(|_| ())?;
        if suffix == 0 {
            return Err(());
        }
        return Ok(Some((size.saturating_sub(suffix), size - 1)));
    }
    let start = start.parse::<u64>().map_err(|_| ())?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().map_err(|_| ())?.min(size - 1)
    };
    if start >= size || start > end {
        return Err(());
    }
    Ok(Some((start, end)))
}

pub(crate) fn serve(request: &super::HttpRequest, stream: &mut TcpStream) -> std::io::Result<()> {
    let error = |stream: &mut TcpStream, response| {
        super::write_http_response_headers(stream, response, request.method == "HEAD")
    };
    let config = crate::platform::interop::get_live_config();
    if !config.web_allow_file_access {
        return error(stream, super::permission_denied("host file access"));
    }
    let requested = super::query_value(&request.target, "path");
    let Some(requested) = requested.filter(|value| !value.trim().is_empty()) else {
        return error(
            stream,
            super::HttpResponse::text(400, "Bad Request", "A server file path is required"),
        );
    };
    // File::open followed by metadata prevents directory enumeration/read via
    // this route. No URL is fetched here (no arbitrary-network SSRF gateway).
    let path = match std::path::Path::new(&requested).canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return error(
                stream,
                super::HttpResponse::text(404, "Not Found", "Server file is unavailable"),
            );
        }
    };
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            return error(
                stream,
                super::HttpResponse::text(403, "Forbidden", "Server file cannot be read"),
            );
        }
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return error(
            stream,
            super::HttpResponse::text(
                400,
                "Bad Request",
                "The requested path is not a regular file",
            ),
        );
    }
    let size = metadata.len();
    let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
    let mtime = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Metadata isn't a content digest. Mark this validator weak honestly.
    let etag = format!("W/\"{size:x}-{mtime:x}\"");
    let last_modified = httpdate::fmt_http_date(modified);
    let modified_seconds = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let not_modified = header(&request.headers, "if-none-match")
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim() == etag || part.trim() == "*")
        })
        .unwrap_or_else(|| {
            header(&request.headers, "if-modified-since")
                .and_then(|value| httpdate::parse_http_date(value).ok())
                .is_some_and(|date| {
                    modified_seconds
                        <= date
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                })
        });
    if not_modified {
        write!(
            stream,
            "HTTP/1.1 304 Not Modified\r\nETag: {etag}\r\nConnection: close\r\n\r\n"
        )?;
        return stream.flush();
    }
    let range_allowed = header(&request.headers, "if-range").is_none_or(|value| {
        // Weak entity tags cannot satisfy If-Range. Dates can.
        httpdate::parse_http_date(value).is_ok_and(|date| {
            modified_seconds
                <= date
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
        })
    });
    let range = if request.method == "GET" && range_allowed {
        header(&request.headers, "range")
            .map(|value| byte_range(value, size))
            .transpose()
    } else {
        Ok(None)
    };
    let range = match range {
        Ok(range) => range.flatten(),
        Err(()) => {
            write!(
                stream,
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{size}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )?;
            return stream.flush();
        }
    };
    let (start, length) = range
        .map(|(start, end)| (start, end - start + 1))
        .unwrap_or((0, size));
    let mime = match path
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
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "vtt" => "text/vtt",
        _ => "application/octet-stream",
    };
    let disposition = if mime == "application/octet-stream"
        || super::query_value(&request.target, "download").as_deref() == Some("1")
    {
        "attachment"
    } else {
        "inline"
    };
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let encoded: String = url::form_urlencoded::byte_serialize(name.as_bytes()).collect();
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {mime}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nETag: {etag}\r\nLast-Modified: {last_modified}\r\nCache-Control: private, no-cache\r\nContent-Disposition: {disposition}; filename*=UTF-8''{encoded}\r\nX-Content-Type-Options: nosniff\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges, ETag, Last-Modified\r\nConnection: close\r\n",
        if range.is_some() {
            "206 Partial Content"
        } else {
            "200 OK"
        }
    )?;
    if let Some((start, end)) = range {
        write!(stream, "Content-Range: bytes {start}-{end}/{size}\r\n")?;
    }
    write!(stream, "\r\n")?;
    if request.method != "HEAD" {
        file.seek(SeekFrom::Start(start))?;
        let mut remaining = file.take(length);
        std::io::copy(&mut remaining, stream)?;
    }
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges() {
        assert_eq!(byte_range("bytes=2-5", 10), Ok(Some((2, 5))));
        assert_eq!(byte_range("bytes=8-", 10), Ok(Some((8, 9))));
        assert_eq!(byte_range("bytes=-3", 10), Ok(Some((7, 9))));
        assert_eq!(byte_range("bytes=-30", 10), Ok(Some((0, 9))));
        assert_eq!(byte_range("bytes=0-99", 10), Ok(Some((0, 9))));
        assert_eq!(byte_range("bytes=10-", 10), Err(()));
        assert_eq!(byte_range("bytes=-0", 10), Err(()));
        assert_eq!(byte_range("bytes=0-1", 0), Err(()));
        assert_eq!(byte_range("bytes=0-1,4-5", 10), Ok(None));
    }
}
