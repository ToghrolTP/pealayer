//! Local deterministic fixtures; never access the Internet or a production queue.
use pealayer_downloads::{AddRequest, Manager, State};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct Fixture {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new(mode: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/fixture.bin", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let captured = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = thread::spawn(move || {
            let mut workers = Vec::new();
            while !stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let captured = captured.clone();
                        workers.push(thread::spawn(move || respond(stream, mode, captured)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            url,
            requests,
            stop,
            thread: Some(worker),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn payload() -> Vec<u8> {
    (0..256 * 1024).map(|n| (n % 251) as u8).collect()
}
fn respond(mut stream: TcpStream, mode: &str, captured: Arc<Mutex<Vec<String>>>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    while !bytes.ends_with(b"\r\n\r\n") && bytes.len() < 8192 {
        let Ok(n) = stream.read(&mut buf) else {
            return;
        };
        if n == 0 {
            return;
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let request = String::from_utf8(bytes).unwrap();
    let number = {
        let mut requests = captured.lock().unwrap();
        requests.push(request.clone());
        requests.len()
    };
    let range = request.lines().find_map(|line| {
        line.to_ascii_lowercase()
            .strip_prefix("range: bytes=")
            .and_then(|v| v.split('-').next())
            .and_then(|n| n.parse::<usize>().ok())
    });
    let data = payload();
    let start = if mode == "ignore-range" {
        0
    } else {
        range.unwrap_or(0)
    };
    let partial = start > 0;
    let etag = if mode == "changed" && number > 1 {
        "v2"
    } else {
        "v1"
    };
    let headers = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nETag: \"{etag}\"\r\nConnection: close\r\n{}\r\n",
        if partial {
            "206 Partial Content"
        } else {
            "200 OK"
        },
        data.len() - start,
        if partial {
            format!(
                "Content-Range: bytes {start}-{}/{}\r\n",
                data.len() - 1,
                data.len()
            )
        } else {
            String::new()
        }
    );
    if stream.write_all(headers.as_bytes()).is_err() {
        return;
    }
    let end = if mode == "interrupt" && number == 1 {
        32 * 1024
    } else {
        data.len()
    };
    for chunk in data[start..end].chunks(4096) {
        if stream.write_all(chunk).is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(4));
    }
}
struct Storage(PathBuf);
impl Storage {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "pealayer-transfer-fixture-{}",
            uuid::Uuid::new_v4()
        )))
    }
}
impl Drop for Storage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn add(manager: &Manager, url: &str) -> String {
    manager
        .add(AddRequest {
            url: url.into(),
            use_proxy: false,
            proxy_url: None,
            filename: None,
            engine: pealayer_downloads::Engine::Native,
            connections: 1,
        })
        .unwrap()
}
fn wait(
    manager: &Manager,
    id: &str,
    predicate: impl Fn(&pealayer_downloads::PublicJob) -> bool,
) -> pealayer_downloads::PublicJob {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let job = manager
            .snapshot()
            .jobs
            .into_iter()
            .find(|j| j.id == id)
            .unwrap();
        if predicate(&job) {
            return job;
        }
        assert!(
            Instant::now() < deadline,
            "fixture timed out: {:?} {:?}",
            job.state,
            job.error
        );
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn pause_resume_validates_range_and_commits_exact_data() {
    let fixture = Fixture::new("normal");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    wait(&manager, &id, |j| j.downloaded >= 16 * 1024);
    manager.action(&id, "pause").unwrap();
    wait(&manager, &id, |j| j.actions.contains(&"remove"));
    let before = fs::metadata(storage.0.join(&id).join("payload.part"))
        .unwrap()
        .len();
    assert!(before > 0 && before < payload().len() as u64);
    manager.action(&id, "resume").unwrap();
    let job = wait(&manager, &id, |j| {
        matches!(j.state, State::Complete | State::Failed)
    });
    assert_eq!(job.state, State::Complete, "{:?}", job.error);
    assert_eq!(fs::read(job.output.unwrap()).unwrap(), payload());
    let requests = fixture.requests.lock().unwrap();
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains(&format!("range: bytes={before}-"))
    );
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("if-range: \"v1\"")
    );
}
#[test]
fn ignored_range_restarts_instead_of_appending() {
    let fixture = Fixture::new("ignore-range");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    wait(&manager, &id, |j| j.downloaded >= 16 * 1024);
    manager.action(&id, "pause").unwrap();
    wait(&manager, &id, |j| j.actions.contains(&"remove"));
    manager.action(&id, "resume").unwrap();
    let job = wait(&manager, &id, |j| {
        matches!(j.state, State::Complete | State::Failed)
    });
    assert_eq!(job.state, State::Complete, "{:?}", job.error);
    assert_eq!(fs::read(job.output.unwrap()).unwrap(), payload());
}
#[test]
fn changed_source_refuses_resume_without_touching_partial() {
    let fixture = Fixture::new("changed");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    wait(&manager, &id, |j| j.downloaded >= 16 * 1024);
    manager.action(&id, "pause").unwrap();
    wait(&manager, &id, |j| j.actions.contains(&"remove"));
    let before = fs::read(storage.0.join(&id).join("payload.part")).unwrap();
    manager.action(&id, "resume").unwrap();
    let job = wait(&manager, &id, |j| j.state == State::Failed);
    assert_eq!(job.error.as_deref(), Some("Source changed; resume refused"));
    assert_eq!(
        fs::read(storage.0.join(&id).join("payload.part")).unwrap(),
        before
    );
    assert!(job.output.is_none());
}
#[test]
fn truncated_body_keeps_resumable_data_and_reports_failure() {
    let fixture = Fixture::new("interrupt");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    let job = wait(&manager, &id, |j| j.state == State::Failed);
    assert!(job.output.is_none());
    assert!(job.downloaded > 0);
    manager.action(&id, "resume").unwrap();
    let job = wait(&manager, &id, |j| {
        matches!(j.state, State::Complete | State::Failed)
    });
    assert_eq!(job.state, State::Complete, "{:?}", job.error);
    assert_eq!(fs::read(job.output.unwrap()).unwrap(), payload());
}
#[test]
fn cancel_never_commits_or_removes_saved_partial() {
    let fixture = Fixture::new("normal");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    wait(&manager, &id, |j| j.downloaded >= 16 * 1024);
    manager.action(&id, "cancel").unwrap();
    let job = wait(&manager, &id, |j| j.actions.contains(&"remove"));
    assert_eq!(job.state, State::Cancelled);
    assert!(job.output.is_none());
    assert!(storage.0.join(&id).join("payload.part").exists());
    manager.action(&id, "remove").unwrap();
    assert!(manager.snapshot().jobs.is_empty());
    assert!(storage.0.join(&id).join("payload.part").exists());
}
#[test]
fn a_second_process_cannot_own_the_same_queue() {
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    assert!(Manager::open(storage.0.clone()).is_err());
    drop(manager);
}

#[test]
fn progressive_reads_combine_cached_prefix_with_validated_origin_bytes() {
    let fixture = Fixture::new("normal");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    wait(&manager, &id, |job| job.downloaded >= 16 * 1024);
    manager.action(&id, "pause").unwrap();
    wait(&manager, &id, |job| job.actions.contains(&"remove"));
    let info = manager.cache_info(&id).unwrap();
    assert!(info.buffered < info.total);
    let mut reader = manager.read_range(&id, 0, info.total - 1).unwrap();
    let mut result = Vec::new();
    reader.read_to_end(&mut result).unwrap();
    assert_eq!(result, payload());
    let count = fixture.requests.lock().unwrap().len();
    let mut reader = manager.read_range(&id, 0, 99).unwrap();
    let mut result = Vec::new();
    reader.read_to_end(&mut result).unwrap();
    assert_eq!(result, payload()[..100]);
    assert_eq!(
        fixture.requests.lock().unwrap().len(),
        count,
        "cached bytes must not perform another network request"
    );
}

#[test]
fn progressive_reads_refuse_a_different_source_identity() {
    let fixture = Fixture::new("changed");
    let storage = Storage::new();
    let manager = Manager::open(storage.0.clone()).unwrap();
    let id = add(&manager, &fixture.url);
    wait(&manager, &id, |job| job.downloaded >= 16 * 1024);
    manager.action(&id, "pause").unwrap();
    wait(&manager, &id, |job| job.actions.contains(&"remove"));
    assert!(
        manager
            .read_range(&id, 0, payload().len() as u64 - 1)
            .is_err()
    );
}
