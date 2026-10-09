//! Opt-in acceptance against installed engines and synthetic local media only.
use pealayer_downloads::{AddRequest, Engine, EngineSettings, Manager, State, tools};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct Work {
    root: PathBuf,
    server: Option<thread::JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    url: String,
}
impl Work {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("pealayer-engine-fixture-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let ffmpeg = tools::find("ffmpeg").expect("install FFmpeg for engine acceptance");
        let input = root.join("fixture.mp4");
        let mut command = Command::new(ffmpeg);
        hidden(&mut command);
        command
            .args([
                "-nostdin",
                "-n",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=10",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=44100",
                "-t",
                "0.6",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&input);
        assert!(command.status().unwrap().success());
        let data = Arc::new(fs::read(input).unwrap());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/fixture.mp4", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let server = thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut chunk = [0; 1024];
                        while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                            let Ok(n) = stream.read(&mut chunk) else {
                                break;
                            };
                            if n == 0 {
                                break;
                            }
                            request.extend_from_slice(&chunk[..n]);
                        }
                        let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
                        let range = request
                            .lines()
                            .find_map(|line| line.strip_prefix("range: bytes="));
                        let start = range
                            .and_then(|v| v.split('-').next()?.parse::<usize>().ok())
                            .unwrap_or(0)
                            .min(data.len() - 1);
                        let end = range
                            .and_then(|v| v.split('-').nth(1)?.parse::<usize>().ok())
                            .unwrap_or(data.len() - 1)
                            .min(data.len() - 1);
                        let headers = format!(
                            "HTTP/1.1 {}\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"fixture\"\r\nConnection: close\r\n{}\r\n",
                            if range.is_some() {
                                "206 Partial Content"
                            } else {
                                "200 OK"
                            },
                            end - start + 1,
                            if range.is_some() {
                                format!("Content-Range: bytes {start}-{end}/{}\r\n", data.len())
                            } else {
                                String::new()
                            }
                        );
                        let _ = stream.write_all(headers.as_bytes());
                        if !request.starts_with("head ") {
                            let _ = stream.write_all(&data[start..=end]);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("fixture server: {error}"),
                }
            }
        });
        Self {
            root,
            server: Some(server),
            stop,
            url,
        }
    }
    fn request(&self, engine: Engine) -> AddRequest {
        AddRequest {
            url: self.url.clone(),
            use_proxy: false,
            proxy_url: None,
            filename: None,
            engine,
            connections: if engine == Engine::Aria2 { 4 } else { 1 },
        }
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.server.take().unwrap().join().unwrap();
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn hidden(command: &mut Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}
fn completed(manager: &Manager, id: &str) -> PathBuf {
    let deadline = Instant::now() + Duration::from_secs(35);
    loop {
        let snapshot = manager.snapshot();
        let job = snapshot.jobs.iter().find(|j| j.id == id).unwrap();
        match job.state {
            State::Complete => return PathBuf::from(job.output.as_ref().unwrap()),
            State::Failed => panic!("engine failed: {:?}", job.error),
            _ => {
                assert!(Instant::now() < deadline, "engine timed out");
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires installed aria2 and FFmpeg; run engine acceptance explicitly"]
fn external_aria2_downloads_via_the_shared_queue() {
    let work = Work::new();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut command =
        Command::new(tools::find("aria2c").expect("install aria2 for engine acceptance"));
    hidden(&mut command);
    command
        .args([
            "--enable-rpc",
            "--rpc-listen-all=false",
            "--rpc-secret=local-test-token",
            "--disable-ipv6=true",
            "--console-log-level=error",
        ])
        .arg(format!("--rpc-listen-port={port}"));
    let _process = Process(command.spawn().unwrap());
    let endpoint = format!("http://127.0.0.1:{port}/jsonrpc");
    let client =
        pealayer_downloads::aria2::Client::new(&endpoint, Some("local-test-token".into())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while client.version().is_err() {
        assert!(Instant::now() < deadline, "aria2 did not start");
        thread::sleep(Duration::from_millis(50));
    }
    let manager = Manager::open(work.root.join("queue")).unwrap();
    manager
        .configure_engines(EngineSettings {
            aria2_endpoint: Some(endpoint),
            aria2_secret: Some("local-test-token".into()),
        })
        .unwrap();
    let id = manager.add(work.request(Engine::Aria2)).unwrap();
    let output = completed(&manager, &id);
    assert_eq!(
        fs::read(output).unwrap(),
        fs::read(work.root.join("fixture.mp4")).unwrap()
    );
}

#[test]
#[ignore = "requires installed yt-dlp and FFmpeg; run engine acceptance explicitly"]
fn yt_dlp_extracts_local_media_and_reports_final_output() {
    let work = Work::new();
    assert!(tools::find("yt-dlp").is_some());
    let manager = Manager::open(work.root.join("queue")).unwrap();
    let id = manager.add(work.request(Engine::YtDlp)).unwrap();
    let output = completed(&manager, &id);
    assert_eq!(
        fs::read(output).unwrap(),
        fs::read(work.root.join("fixture.mp4")).unwrap()
    );
}

#[test]
#[ignore = "requires installed FFmpeg; run engine acceptance explicitly"]
fn ffmpeg_remux_and_audio_extraction_are_owned_queued_operations() {
    let work = Work::new();
    let manager = Manager::open(work.root.join("queue")).unwrap();
    let source = manager.add(work.request(Engine::Native)).unwrap();
    completed(&manager, &source);
    let remux = manager
        .process(&source, pealayer_downloads::MediaOperation::RemuxMp4)
        .unwrap();
    let remux = completed(&manager, &remux);
    assert!(fs::metadata(remux).unwrap().len() > 0);
    let audio = manager
        .process(&source, pealayer_downloads::MediaOperation::ExtractAudio)
        .unwrap();
    let audio = completed(&manager, &audio);
    assert_eq!(audio.extension().unwrap(), "m4a");
    assert!(fs::metadata(audio).unwrap().len() > 0);
}
