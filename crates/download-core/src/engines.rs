//! Queue engine runners share state, storage ownership and public telemetry.
use crate::{Manager, State, persist};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

impl Manager {
    pub(crate) fn transfer_ffmpeg(&self, id: &str) -> Result<(), String> {
        let (input, directory, filename, operation) = {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or("Job removed")?;
            let processing = job
                .processing
                .as_ref()
                .ok_or("Missing media-processing operation")?;
            let source = inner
                .jobs
                .iter()
                .find(|j| j.id == processing.source_id && j.state == State::Complete)
                .ok_or("Source media is unavailable")?;
            let input = std::path::PathBuf::from(
                source
                    .output
                    .as_deref()
                    .ok_or("Source media is unavailable")?,
            )
            .canonicalize()
            .map_err(|_| "Source media is unavailable")?;
            if !input.starts_with(
                inner
                    .root
                    .join(&source.id)
                    .canonicalize()
                    .map_err(|_| "Source cache is unavailable")?,
            ) {
                return Err("Source media escaped its cache".into());
            }
            (
                input,
                inner.root.join(id),
                job.filename.clone(),
                processing.operation,
            )
        };
        fs::create_dir_all(&directory).map_err(|_| "Cannot create processing storage")?;
        let extension = std::path::Path::new(&filename)
            .extension()
            .and_then(|p| p.to_str())
            .unwrap_or("mp4");
        let working = directory.join(format!("working.{extension}"));
        if working.exists() {
            fs::remove_file(&working)
                .map_err(|_| "Cannot replace interrupted processing output")?;
        }
        let executable = crate::tools::find("ffmpeg").ok_or("FFmpeg is unavailable")?;
        let command = if matches!(operation, crate::MediaOperation::ExtractAudio) {
            crate::tools::extract_audio(&executable, &input, &working)?
        } else {
            crate::tools::remux(&executable, &input, &working)?
        };
        crate::tools::run(
            command,
            || matches!(self.job_state(id), Ok(State::Paused | State::Cancelled)),
            |line| {
                if let Some(("total_size", count)) = crate::tools::ffmpeg_progress(line) {
                    if let Ok(size) = count.parse::<u64>() {
                        self.engine_progress(id, size, None, 0)?;
                    }
                }
                Ok(())
            },
        )?;
        if matches!(self.job_state(id)?, State::Paused | State::Cancelled) {
            return Ok(());
        }
        let output = directory.join(filename);
        fs::hard_link(&working, &output)
            .map_err(|_| "Cannot commit processed media without overwriting")?;
        fs::remove_file(working).map_err(|_| "Cannot clean processing stage")?;
        self.complete_output(id, &output)
    }
    pub(crate) fn transfer_aria2(&self, id: &str) -> Result<(), String> {
        let (settings, request, directory, filename, old_gid) = {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or("Download removed")?;
            (
                inner.engines.clone(),
                job.request.clone(),
                inner.root.join(id),
                job.filename.clone(),
                job.engine_id.clone(),
            )
        };
        let client = crate::aria2::Client::new(
            settings
                .aria2_endpoint
                .as_deref()
                .ok_or("aria2 is unconfigured")?,
            settings.aria2_secret,
        )?;
        fs::create_dir_all(&directory).map_err(|_| "Cannot create engine storage")?;
        let limit = self.worker_limit();
        let gid = match old_gid {
            Some(gid) => gid,
            None => {
                let gid = client.add(
                    &request,
                    directory.to_str().ok_or("Invalid engine storage path")?,
                    &filename,
                    limit,
                )?;
                let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
                inner
                    .jobs
                    .iter_mut()
                    .find(|j| j.id == id)
                    .ok_or("Download removed")?
                    .engine_id = Some(gid.clone());
                if let Err(error) = persist(&inner) {
                    let _ = client.action(&gid, "pause");
                    return Err(error);
                }
                gid
            }
        };
        let mut previous_limit = limit;
        struct RemoteGuard<'a> {
            client: &'a crate::aria2::Client,
            gid: &'a str,
        }
        impl Drop for RemoteGuard<'_> {
            fn drop(&mut self) {
                let _ = self.client.action(self.gid, "pause");
            }
        }
        let _guard = RemoteGuard {
            client: &client,
            gid: &gid,
        };
        let mut activated = false;
        loop {
            let state = self.job_state(id)?;
            if state == State::Cancelled {
                client.action(&gid, "cancel")?;
                return Ok(());
            }
            if state == State::Paused {
                client.action(&gid, "pause")?;
                return Ok(());
            }
            let status = match client.status(&gid) {
                Ok(status) => status,
                Err(error) => {
                    let paused = client.action(&gid, "pause").is_ok();
                    return Err(if paused {
                        error
                    } else {
                        "aria2 control connection lost; remote transfer may still run. Reconnect to reclaim it.".into()
                    });
                }
            };
            if status.gid != gid {
                return Err("aria2 returned a different transfer identity".into());
            }
            match status.status.as_str() {
                "complete" => {
                    let output = directory.join(filename);
                    let expected = status
                        .total_length
                        .parse::<u64>()
                        .map_err(|_| "Invalid aria2 file length")?;
                    if !output.is_file()
                        || fs::metadata(&output)
                            .map_err(|_| "Cannot inspect aria2 output")?
                            .len()
                            != expected
                    {
                        return Err(
                            "aria2 output is not available in this host's owned cache".into()
                        );
                    }
                    self.complete_output(id, &output)?;
                    return Ok(());
                }
                "error" => {
                    return Err(format!(
                        "aria2 transfer failed (code {})",
                        status.error_code
                    ));
                }
                "removed" => return Err("aria2 transfer was removed outside Pealayer".into()),
                "paused" if !activated => {
                    client.action(&gid, "resume")?;
                    activated = true;
                }
                "paused" => return Err("aria2 transfer was paused outside Pealayer".into()),
                "active" | "waiting" => activated = true,
                _ => return Err("Unknown aria2 transfer state".into()),
            }
            let limit = self.worker_limit();
            if limit != previous_limit {
                client.set_speed_limit(&gid, limit)?;
                previous_limit = limit;
            }
            let downloaded = status
                .completed_length
                .parse::<u64>()
                .map_err(|_| "Invalid aria2 byte count")?;
            let total = status
                .total_length
                .parse::<u64>()
                .map_err(|_| "Invalid aria2 size")?;
            let speed = status
                .download_speed
                .parse::<u64>()
                .map_err(|_| "Invalid aria2 speed")?;
            self.engine_progress(id, downloaded, (total > 0).then_some(total), speed)?;
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    pub(crate) fn worker_limit(&self) -> u64 {
        let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if inner.bytes_per_second == 0 {
            0
        } else {
            (inner.bytes_per_second / inner.max_concurrent as u64).max(1)
        }
    }
    pub(crate) fn job_state(&self, id: &str) -> Result<State, String> {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .jobs
            .iter()
            .find(|j| j.id == id)
            .map(|j| j.state)
            .ok_or_else(|| "Download removed".into())
    }
    pub(crate) fn engine_progress(
        &self,
        id: &str,
        downloaded: u64,
        total: Option<u64>,
        speed: u64,
    ) -> Result<(), String> {
        if total.is_some_and(|total| downloaded > total) {
            return Err("Engine byte count exceeds its file size".into());
        }
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let job = inner
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or("Download removed")?;
        if !matches!(job.state, State::Paused | State::Cancelled) {
            job.state = State::Downloading;
        }
        job.downloaded = downloaded;
        job.total = total;
        job.speed = speed;
        job.eta_seconds =
            total.and_then(|n| (speed > 0).then(|| n.saturating_sub(downloaded) / speed));
        if job
            .last_sample
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(250))
        {
            if job.samples.len() >= 120 {
                job.samples.pop_front();
            }
            job.samples.push_back(speed);
            job.last_sample = Some(Instant::now());
            inner.revision += 1;
        }
        Ok(())
    }
    pub(crate) fn complete_output(&self, id: &str, output: &Path) -> Result<(), String> {
        let canonical = output
            .canonicalize()
            .map_err(|_| "Engine output is unavailable")?;
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let owned = inner
            .root
            .join(id)
            .canonicalize()
            .map_err(|_| "Engine cache is unavailable")?;
        if !canonical.starts_with(&owned) || !canonical.is_file() {
            return Err("Engine output escaped its owned cache".into());
        }
        let job = inner
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or("Download removed")?;
        if matches!(job.state, State::Paused | State::Cancelled) {
            return Ok(());
        }
        job.downloaded = fs::metadata(&canonical)
            .map_err(|_| "Engine output metadata unavailable")?
            .len();
        job.total = Some(job.downloaded);
        job.filename = crate::safe_filename(
            canonical
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or("Invalid finalized filename")?,
        )?;
        job.output = Some(canonical.to_string_lossy().into());
        job.state = State::Complete;
        Ok(())
    }

    pub(crate) fn transfer_media(&self, id: &str) -> Result<(), String> {
        let (request, directory) = {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or("Download removed")?;
            (job.request.clone(), inner.root.join(id))
        };
        fs::create_dir_all(&directory).map_err(|_| "Cannot create media download storage")?;
        let executable = crate::tools::find("yt-dlp").ok_or("yt-dlp is unavailable")?;
        let command =
            crate::tools::media_download(&executable, &request, &directory, self.worker_limit())?;
        let mut output = None;
        crate::tools::run(
            command,
            || matches!(self.job_state(id), Ok(State::Paused | State::Cancelled)),
            |line| {
                if let Some(progress) = crate::tools::yt_dlp_progress(line) {
                    let downloaded = progress
                        .get("downloaded_bytes")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let total = progress
                        .get("total_bytes")
                        .and_then(serde_json::Value::as_u64);
                    let speed = progress
                        .get("speed")
                        .and_then(serde_json::Value::as_f64)
                        .filter(|n| n.is_finite() && *n >= 0.0)
                        .unwrap_or(0.0) as u64;
                    self.engine_progress(id, downloaded, total, speed)?;
                }
                if let Some(path) = line.strip_prefix("done:") {
                    output = Some(std::path::PathBuf::from(path));
                }
                Ok(())
            },
        )?;
        if matches!(self.job_state(id)?, State::Paused | State::Cancelled) {
            return Ok(());
        }
        self.complete_output(
            id,
            &output.ok_or("Extraction did not report a finalized media file")?,
        )
    }
}
