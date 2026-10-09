//! Read-through media buffering: cached contiguous bytes first, validated origin ranges for gaps.
use crate::{Engine, Manager, State, USER_AGENT, parse_content_range};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    time::Duration,
};

struct OriginReader {
    response: reqwest::Response,
    pending: Vec<u8>,
    offset: usize,
    runtime: tokio::runtime::Runtime,
}
impl Read for OriginReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.offset == self.pending.len() {
            self.pending = match self
                .runtime
                .block_on(self.response.chunk())
                .map_err(|_| std::io::Error::other("Streaming origin interrupted"))?
            {
                Some(bytes) => bytes.to_vec(),
                None => return Ok(0),
            };
            self.offset = 0;
        }
        let length = output.len().min(self.pending.len() - self.offset);
        output[..length].copy_from_slice(&self.pending[self.offset..self.offset + length]);
        self.offset += length;
        Ok(length)
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Info {
    pub filename: String,
    pub total: u64,
    pub buffered: u64,
    pub complete: bool,
}
impl Manager {
    pub fn cache_info(&self, id: &str) -> Result<Info, String> {
        let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let job = inner
            .jobs
            .iter()
            .find(|j| j.id == id)
            .ok_or("Download does not exist")?;
        let complete = job.state == State::Complete && job.output.is_some();
        if !complete
            && (job.request.engine != Engine::Native
                || job.validator.is_none()
                || job.downloaded == 0)
        {
            return Err("This job is not ready for progressive streaming".into());
        }
        Ok(Info {
            filename: job.filename.clone(),
            total: job.total.ok_or("Streaming requires a known file size")?,
            buffered: job.downloaded,
            complete,
        })
    }

    /// Returns exactly one requested range. Never exposes a sparse file as downloaded media.
    /// Missing bytes use an origin range with the cached source validator, not a fabricated zero-fill.
    pub fn read_range(
        &self,
        id: &str,
        start: u64,
        end: u64,
    ) -> Result<Box<dyn Read + Send>, String> {
        let info = self.cache_info(id)?;
        if start > end || end >= info.total {
            return Err("Media range is outside the source".into());
        }
        let (request, path, validator, directory) = {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or("Download removed")?;
            (
                job.request.clone(),
                job.output
                    .as_ref()
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| inner.root.join(id).join("payload.part")),
                job.validator.clone(),
                inner.root.join(id),
            )
        };
        let path = path
            .canonicalize()
            .map_err(|_| "Cached media is unavailable")?;
        if !path.starts_with(
            directory
                .canonicalize()
                .map_err(|_| "Cache directory is unavailable")?,
        ) {
            return Err("Cached media escaped its owned directory".into());
        }
        let mut file = File::open(&path).map_err(|_| "Cannot read cached media")?;
        let cached_end = info
            .buffered
            .min(
                file.metadata()
                    .map_err(|_| "Cannot inspect cached media")?
                    .len(),
            )
            .min(end + 1);
        let cached_length = cached_end.saturating_sub(start);
        file.seek(SeekFrom::Start(start))
            .map_err(|_| "Cannot seek cached media")?;
        if cached_length == end - start + 1 {
            return Ok(Box::new(file.take(cached_length)));
        }
        if info.complete {
            return Err("Completed cache entry is truncated".into());
        }
        let origin_start = start + cached_length;
        let validator = validator.ok_or("Progressive streaming requires a source validator")?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "Cannot create streaming runtime")?;
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30));
        if !request.use_proxy {
            builder = builder.no_proxy();
        } else if let Some(proxy) = &request.proxy_url {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|_| "Invalid proxy")?);
        }
        let pending = builder
            .build()
            .map_err(|_| "Cannot create streaming client")?
            .get(&request.url)
            .header("Accept-Encoding", "identity")
            .header("Range", format!("bytes={origin_start}-{end}"))
            .header("If-Range", &validator);
        let response = runtime
            .block_on(async { pending.send().await })
            .map_err(|_| "Streaming origin is unreachable")?;
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(
                "Origin rejected the validated streaming range; finish downloading before playback"
                    .into(),
            );
        }
        let range = response
            .headers()
            .get("Content-Range")
            .and_then(|v| v.to_str().ok())
            .ok_or("Origin omitted Content-Range")?;
        let (actual_start, actual_end, total) = parse_content_range(range)?;
        if (actual_start, actual_end, total) != (origin_start, end, info.total)
            || response
                .content_length()
                .is_some_and(|n| n != end - origin_start + 1)
        {
            return Err("Origin returned inconsistent streaming bounds".into());
        }
        let current = response
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.starts_with("W/"))
            .or_else(|| {
                response
                    .headers()
                    .get("Last-Modified")
                    .and_then(|v| v.to_str().ok())
            });
        if current != Some(&validator) {
            return Err("Streaming source changed; cached and origin bytes cannot be mixed".into());
        }
        Ok(Box::new(
            file.take(cached_length).chain(
                OriginReader {
                    response,
                    pending: Vec::new(),
                    offset: 0,
                    runtime,
                }
                .take(end - origin_start + 1),
            ),
        ))
    }
}
