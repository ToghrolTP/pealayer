//! Read-only source cue index for the native timeline. No playback or network I/O.
use crate::app::{MediaTrackInfo, MediaTrackType};
use eframe::egui;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const MAX_OUTPUT: usize = 12 * 1024 * 1024;
const MAX_SOURCES: usize = 16;
const SCAN_LIMIT: Duration = Duration::from_secs(35);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Span {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug)]
pub(crate) struct Spans {
    spans: Vec<Span>,
    prefix_end: Vec<f64>,
}

impl Spans {
    pub(crate) fn new(mut spans: Vec<Span>) -> Self {
        spans.sort_unstable_by(|a, b| a.start.total_cmp(&b.start).then(a.end.total_cmp(&b.end)));
        spans.dedup();
        let mut end = f64::NEG_INFINITY;
        let prefix_end = spans
            .iter()
            .map(|span| {
                end = end.max(span.end);
                end
            })
            .collect();
        Self { spans, prefix_end }
    }
    pub fn visible(&self, start: f64, end: f64) -> impl Iterator<Item = Span> + '_ {
        let hi = self.spans.partition_point(|span| span.start < end);
        let lo = self
            .prefix_end
            .partition_point(|stop| *stop <= start)
            .min(hi);
        self.spans[lo..hi]
            .iter()
            .copied()
            .filter(move |span| span.end > start)
    }
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[derive(Debug)]
pub(crate) enum Timing {
    Loading,
    Ready(Spans),
    Unavailable(&'static str),
}

#[derive(Clone, Debug)]
struct Track {
    id: i64,
    index: Option<i64>,
    codec: Option<String>,
}

type Plan = BTreeMap<(PathBuf, bool), Vec<Track>>;
type Results = BTreeMap<i64, Timing>;

struct Scan {
    key: u64,
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Results>,
}

#[derive(Default)]
pub(crate) struct Index {
    key: Option<u64>,
    pending: Option<Plan>,
    scan: Option<Scan>,
    tracks: Results,
}

impl Drop for Index {
    fn drop(&mut self) {
        if let Some(scan) = &self.scan {
            scan.cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl Index {
    pub fn get(&self, id: i64) -> Option<&Timing> {
        self.tracks.get(&id)
    }

    pub fn invalidate(&mut self) {
        self.key = None;
        self.tracks.clear();
        self.pending = None;
        if let Some(scan) = &self.scan {
            scan.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        enabled: bool,
        peer: bool,
        media: Option<&Path>,
        tracks: &[MediaTrackInfo],
    ) {
        let mut hash = DefaultHasher::new();
        enabled.hash(&mut hash);
        if enabled {
            peer.hash(&mut hash);
            media.hash(&mut hash);
            for track in tracks.iter().filter(|t| t.kind == MediaTrackType::Subtitle) {
                (
                    track.id,
                    track.external,
                    track.external_filename.as_deref(),
                    track.ffmpeg_index,
                    track.codec.as_deref(),
                )
                    .hash(&mut hash);
            }
        }
        let key = hash.finish();
        if self.key != Some(key) {
            self.invalidate();
            self.key = Some(key);
            if enabled {
                let (plan, states) = plan(media, peer, tracks);
                self.tracks = states;
                if !plan.is_empty() {
                    self.pending = Some(plan);
                }
            }
        }
        if let Some(scan) = &self.scan {
            match scan.result.try_recv() {
                Ok(results) => {
                    if scan.key == key && !scan.cancel.load(Ordering::Relaxed) {
                        self.tracks.extend(results);
                    }
                    self.scan = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    if scan.key == key && !scan.cancel.load(Ordering::Relaxed) {
                        for state in self.tracks.values_mut() {
                            if matches!(state, Timing::Loading) {
                                *state = Timing::Unavailable("Could not read subtitle timing");
                            }
                        }
                    }
                    self.scan = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        // One worker per app, even during rapid loads/toggles. The old worker
        // wakes the UI after it stops, and its results cannot overwrite a new key.
        if self.scan.is_none()
            && let Some(plan) = self.pending.take()
        {
            let (send, result) = mpsc::channel();
            let cancel = Arc::new(AtomicBool::new(false));
            let cancelled = cancel.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let data = run_plan(plan, &cancelled);
                let _ = send.send(data);
                ctx.request_repaint();
            });
            self.scan = Some(Scan {
                key,
                cancel,
                result,
            });
        }
    }
}

fn local_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    match path.to_str().and_then(|value| url::Url::parse(value).ok()) {
        Some(url) if url.scheme() == "file" => url.to_file_path().ok(),
        Some(_) => None,
        None => Some(path.to_path_buf()),
    }
}

fn plan(media: Option<&Path>, peer: bool, tracks: &[MediaTrackInfo]) -> (Plan, Results) {
    let mut plan = Plan::new();
    let mut results = Results::new();
    let base = media.and_then(local_path);
    for track in tracks.iter().filter(|t| t.kind == MediaTrackType::Subtitle) {
        let unavailable = |reason| Timing::Unavailable(reason);
        if media.is_none() {
            continue;
        }
        if peer {
            results.insert(
                track.id,
                unavailable("Subtitle timing is not available from this playback host"),
            );
            continue;
        }
        let external = track.external == Some(true) || track.external_filename.is_some();
        let path = if external {
            track
                .external_filename
                .as_deref()
                .and_then(|source| local_path(Path::new(source)))
                .and_then(|path| {
                    if path.is_absolute() {
                        Some(path)
                    } else {
                        base.as_ref()?.parent().map(|parent| parent.join(path))
                    }
                })
        } else {
            base.clone()
        };
        let Some(path) = path else {
            results.insert(
                track.id,
                unavailable("Cue timing needs a local copy of this media or subtitle file"),
            );
            continue;
        };
        if !external && track.ffmpeg_index.filter(|index| *index >= 0).is_none() {
            results.insert(track.id, unavailable("No verified subtitle stream index"));
            continue;
        }
        let key = (path, external);
        if plan.len() >= MAX_SOURCES && !plan.contains_key(&key) {
            results.insert(track.id, unavailable("Too many subtitle files to index"));
            continue;
        }
        plan.entry(key).or_default().push(Track {
            id: track.id,
            index: track.ffmpeg_index.filter(|index| *index >= 0),
            codec: track.codec.clone(),
        });
        results.insert(track.id, Timing::Loading);
    }
    (plan, results)
}

#[derive(Default, Deserialize)]
struct Format {
    start_time: Option<String>,
}
#[derive(Deserialize)]
struct Stream {
    index: i64,
    codec_name: Option<String>,
}
#[derive(Deserialize)]
struct Packet {
    stream_index: i64,
    pts_time: Option<String>,
    duration_time: Option<String>,
}
#[derive(Deserialize)]
struct Probe {
    #[serde(default)]
    format: Format,
    #[serde(default)]
    streams: Vec<Stream>,
    #[serde(default)]
    packets: Vec<Packet>,
}

fn time(value: Option<&str>) -> Option<f64> {
    value?.parse::<f64>().ok().filter(|time| time.is_finite())
}

fn spans(probe: &Probe, track: &Track, external: bool) -> Timing {
    let index = track
        .index
        .or_else(|| (external && probe.streams.len() == 1).then(|| probe.streams[0].index));
    let Some(stream) = probe
        .streams
        .iter()
        .find(|stream| Some(stream.index) == index)
    else {
        return Timing::Unavailable("Subtitle stream was not found in the source");
    };
    if let (Some(expected), Some(actual)) = (&track.codec, &stream.codec_name)
        && expected != actual
    {
        return Timing::Unavailable("Subtitle stream identity did not match");
    }
    let origin = if external {
        0.0
    } else {
        time(probe.format.start_time.as_deref()).unwrap_or(0.0)
    };
    let mut ranges = Vec::new();
    for packet in probe
        .packets
        .iter()
        .filter(|packet| packet.stream_index == stream.index)
    {
        let Some(start) = time(packet.pts_time.as_deref()).map(|pts| pts - origin) else {
            return Timing::Unavailable("Subtitle source does not report all cue starts");
        };
        let Some(duration) =
            time(packet.duration_time.as_deref()).filter(|duration| *duration > 0.0)
        else {
            return Timing::Unavailable("Subtitle source does not report all cue ends");
        };
        let end = start + duration;
        if !end.is_finite() {
            return Timing::Unavailable("Subtitle timing is outside the supported range");
        }
        if end > 0.0 {
            ranges.push(Span {
                start: start.max(0.0),
                end,
            });
        }
    }
    Timing::Ready(Spans::new(ranges))
}

fn run_plan(plan: Plan, cancel: &AtomicBool) -> Results {
    let deadline = Instant::now() + SCAN_LIMIT;
    let mut results = Results::new();
    for ((path, external), tracks) in plan {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let report = probe_file(&path, cancel, deadline);
        for track in tracks {
            results.insert(
                track.id,
                match &report {
                    Ok(report) => spans(report, &track, external),
                    Err(reason) => Timing::Unavailable(reason),
                },
            );
        }
    }
    results
}

fn probe_file(path: &Path, cancel: &AtomicBool, deadline: Instant) -> Result<Probe, &'static str> {
    if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
        return Err("Subtitle timing scan stopped");
    }
    let path = path
        .canonicalize()
        .map_err(|_| "Local subtitle source is not available")?;
    if !path.is_file() {
        return Err("Subtitle source is not a regular file");
    }
    let executable = pealayer_downloads::tools::find("ffprobe")
        .ok_or("Install FFprobe to show source subtitle timings")?;
    let mut command = Command::new(executable);
    command.args(["-v","error","-protocol_whitelist","file",
        "-format_whitelist","matroska,webm,mov,mp4,m4a,3gp,3g2,mj2,avi,mpeg,mpegts,ogg,asf,flv,srt,ass,webvtt,microdvd,mov_text,subviewer,subviewer1,sami,jacosub,realtext,ttml,sup",
        "-select_streams","s","-show_packets","-show_streams","-show_format",
        "-show_entries","packet=stream_index,pts_time,duration_time:stream=index,codec_name:format=start_time",
        "-of","json","-i"])
        .arg(path).stdin(Stdio::null()).stderr(Stdio::null()).stdout(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "Install FFprobe to show source subtitle timings"
        } else {
            "Could not start the subtitle timing reader"
        }
    })?;
    let pipe = child.stdout.take().expect("ffprobe stdout is piped");
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe
            .take((MAX_OUTPUT + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    let mut bytes = None;
    let result = loop {
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            break Err("Subtitle timing scan stopped or timed out");
        }
        if let Ok(output) = receive.try_recv() {
            match output {
                Ok(output) if output.len() <= MAX_OUTPUT => bytes = Some(output),
                Ok(_) => break Err("Subtitle timing exceeds the indexing limit"),
                Err(_) => break Err("Could not read subtitle timing"),
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => {
                break Err("This source does not expose readable subtitle timing");
            }
            Ok(Some(_)) if bytes.is_some() => {
                break serde_json::from_slice(bytes.as_deref().unwrap())
                    .map_err(|_| "Invalid subtitle timing data");
            }
            Err(_) => break Err("Subtitle timing reader failed"),
            _ => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_timing_preference_is_an_opt_in_config_field() {
        let config = crate::config::AppConfig::default();
        assert!(!config.timeline_subtitle_segments);
        let enabled = config
            .apply_patch(&serde_json::json!({"timeline_subtitle_segments": true}))
            .unwrap();
        assert!(enabled.timeline_subtitle_segments);
        assert!(
            crate::preferences_contract::preference_controls(&enabled)
                .iter()
                .any(|control| control.key == "timeline_subtitle_segments")
        );
    }

    #[test]
    #[ignore = "requires FFprobe on PATH"]
    fn subtitle_timing_reads_real_sidecar_packets() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subtitle-timing.srt");
        let probe = probe_file(&path, &AtomicBool::new(false), Instant::now() + SCAN_LIMIT)
            .expect("FFprobe should open the fixture");
        let track = Track {
            id: 1,
            index: None,
            codec: Some("subrip".into()),
        };
        let Timing::Ready(cues) = spans(&probe, &track, true) else {
            panic!("Expected the real subtitle cue intervals");
        };
        assert_eq!(
            cues.visible(0.0, 10.0).collect::<Vec<_>>(),
            [
                Span {
                    start: 0.625,
                    end: 1.25
                },
                Span {
                    start: 3.5,
                    end: 4.75
                }
            ]
        );
    }
    #[test]
    fn embedded_streams_rebase_and_external_cues_keep_their_start_offset() {
        let probe:Probe=serde_json::from_str(r#"{"format":{"start_time":"1.000"},"streams":[{"index":2,"codec_name":"subrip"},{"index":5,"codec_name":"ass"}],"packets":[{"stream_index":5,"pts_time":"7","duration_time":"1"},{"stream_index":2,"pts_time":"4.5","duration_time":"1.25"},{"stream_index":2,"pts_time":"1.5","duration_time":"0.5"}]}"#).unwrap();
        let t = Track {
            id: 8,
            index: Some(2),
            codec: Some("subrip".into()),
        };
        let Timing::Ready(embedded) = spans(&probe, &t, false) else {
            panic!()
        };
        assert_eq!(
            embedded.visible(0.0, 9.0).collect::<Vec<_>>(),
            [
                Span {
                    start: 0.5,
                    end: 1.0
                },
                Span {
                    start: 3.5,
                    end: 4.75
                }
            ]
        );
        let Timing::Ready(external) = spans(&probe, &t, true) else {
            panic!()
        };
        assert_eq!(
            external.visible(0.0, 2.0).collect::<Vec<_>>(),
            [Span {
                start: 1.5,
                end: 2.0
            }]
        );
        assert!(embedded.visible(1.0, 3.5).next().is_none());
        assert!(matches!(
            spans(
                &probe,
                &Track {
                    codec: Some("ass".into()),
                    ..t
                },
                false
            ),
            Timing::Unavailable(_)
        ));
    }
    #[test]
    fn missing_end_is_not_fabricated_and_overlapping_spans_survive_viewport_culling() {
        let probe: Probe = serde_json::from_str(
            r#"{"streams":[{"index":0}],"packets":[{"stream_index":0,"pts_time":"2"}]}"#,
        )
        .unwrap();
        assert!(matches!(
            spans(
                &probe,
                &Track {
                    id: 1,
                    index: None,
                    codec: None
                },
                true
            ),
            Timing::Unavailable(_)
        ));
        let spans = Spans::new(vec![
            Span {
                start: 0.0,
                end: 10.0,
            },
            Span {
                start: 1.0,
                end: 2.0,
            },
            Span {
                start: 5.0,
                end: 6.0,
            },
        ]);
        assert_eq!(spans.visible(4.0, 6.0).count(), 2);
    }
    #[test]
    fn remote_urls_are_not_local_probe_sources() {
        assert!(local_path(Path::new("https://files.invalid/video.mkv")).is_none());
        assert!(local_path(Path::new("rtsp://camera.invalid/live")).is_none());
        assert!(local_path(Path::new("movie.mkv")).is_some());
    }
}
