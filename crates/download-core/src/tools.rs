//! Constrained media-tool command construction; no shell or caller-provided switches.
use std::{
    path::Path,
    process::{Command, Stdio},
};

fn command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// yt-dlp owns extraction and invokes FFmpeg for its format merge when available.
/// The caller owns process lifecycle and drains both streams; never use a shell.
pub fn media_download(
    executable: &Path,
    request: &crate::AddRequest,
    directory: &Path,
    speed_limit: u64,
) -> Result<Command, String> {
    crate::validate_request(request)?;
    let mut cmd = command(
        executable
            .to_str()
            .ok_or("Invalid yt-dlp executable path")?,
    );
    cmd.args([
        "--ignore-config",
        "--no-playlist",
        "--continue",
        "--no-overwrites",
        "--newline",
        "--no-color",
        "--progress",
        "--no-cache-dir",
        "--progress-template",
        "download:%(progress)j",
        "--print",
        "after_move:done:%(filepath)s",
    ]);
    cmd.arg("--paths")
        .arg(directory)
        .arg("--output")
        .arg("%(title).120B [%(id)s].%(ext)s");
    if !request.use_proxy {
        cmd.args(["--proxy", ""]);
    } else if let Some(proxy) = &request.proxy_url {
        cmd.arg("--proxy").arg(proxy);
    }
    if let Some(ffmpeg) = find("ffmpeg") {
        cmd.arg("--ffmpeg-location").arg(ffmpeg);
    }
    if speed_limit > 0 {
        cmd.arg("--limit-rate").arg(speed_limit.to_string());
    }
    cmd.arg("--").arg(&request.url);
    Ok(cmd)
}

/// Remux to a separate output without replacing source data or overwriting output.
pub fn remux(executable: &Path, input: &Path, output: &Path) -> Result<Command, String> {
    if !input.is_file() || input == output || output.exists() {
        return Err("Remux requires an existing input and a new separate output".into());
    }
    let mut cmd = command(
        executable
            .to_str()
            .ok_or("Invalid FFmpeg executable path")?,
    );
    cmd.args([
        "-nostdin",
        "-n",
        "-hide_banner",
        "-loglevel",
        "error",
        "-progress",
        "pipe:1",
    ]);
    cmd.arg("-i")
        .arg(input)
        .args(["-map", "0", "-c", "copy"])
        .arg(output);
    Ok(cmd)
}

pub fn yt_dlp_progress(line: &str) -> Option<serde_json::Value> {
    line.strip_prefix("download:")
        .and_then(|v| serde_json::from_str(v).ok())
}

pub fn ffmpeg_progress(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    matches!(key, "out_time_us" | "total_size" | "speed" | "progress").then_some((key, value))
}

pub fn extract_audio(executable: &Path, input: &Path, output: &Path) -> Result<Command, String> {
    if !input.is_file() || input == output || output.exists() {
        return Err("Audio extraction requires a separate new output".into());
    }
    let mut cmd = command(
        executable
            .to_str()
            .ok_or("Invalid FFmpeg executable path")?,
    );
    cmd.args([
        "-nostdin",
        "-n",
        "-hide_banner",
        "-loglevel",
        "error",
        "-progress",
        "pipe:1",
        "-i",
    ])
    .arg(input)
    .args(["-map", "0:a:0", "-vn", "-c:a", "aac", "-b:a", "192k"])
    .arg(output);
    Ok(cmd)
}

/// Bundled tools are separate executable assets; PATH is an explicit external-tool fallback.
pub fn find(name: &str) -> Option<std::path::PathBuf> {
    if !matches!(name, "aria2c" | "yt-dlp" | "ffmpeg" | "ffprobe") {
        return None;
    }
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    };
    let executable = std::env::current_exe().ok()?;
    let parent = executable.parent()?;
    for directory in [
        parent.join("tools"),
        parent.join("../tools"),
        parent.to_path_buf(),
    ] {
        let path = directory.join(&filename);
        if path.is_file() {
            return Some(path);
        }
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|directory| directory.join(&filename))
        .find(|path| path.is_file())
}

/// Both pipes are drained with bounded memory. Cancellation terminates the owned process tree.
pub fn run(
    mut command: Command,
    mut stopped: impl FnMut() -> bool,
    mut progress: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    use std::{io::Read, sync::mpsc, thread, time::Duration};
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|_| "Cannot start media tool; check the installed executable")?;
    #[cfg(windows)]
    let _job = process_tree::Job::attach(&child).map_err(|error| {
        let _ = child.kill();
        let _ = child.wait();
        error
    })?;
    let (tx, rx) = mpsc::sync_channel::<String>(128);
    let mut readers = Vec::new();
    let streams: Vec<Box<dyn Read + Send>> = vec![
        Box::new(child.stdout.take().ok_or("Missing tool stdout")?),
        Box::new(child.stderr.take().ok_or("Missing tool stderr")?),
    ];
    for mut stream in streams {
        let tx = tx.clone();
        readers.push(thread::spawn(move || {
            let mut chunk = [0; 4096];
            let mut line = Vec::new();
            let mut too_long = false;
            while let Ok(count) = stream.read(&mut chunk) {
                if count == 0 {
                    break;
                }
                for byte in &chunk[..count] {
                    if *byte == b'\n' || *byte == b'\r' {
                        if !too_long && !line.is_empty() {
                            let _ = tx.try_send(String::from_utf8_lossy(&line).into_owned());
                        }
                        line.clear();
                        too_long = false;
                    } else if !too_long {
                        if line.len() == 65536 {
                            line.clear();
                            too_long = true;
                        } else {
                            line.push(*byte);
                        }
                    }
                }
            }
            if !too_long && !line.is_empty() {
                let _ = tx.try_send(String::from_utf8_lossy(&line).into_owned());
            }
        }));
    }
    drop(tx);
    let mut outcome = Ok(());
    let mut exited = false;
    loop {
        if stopped() {
            break;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if let Err(error) = progress(&line) {
                    outcome = Err(error);
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                exited = true;
                for line in rx.try_iter() {
                    if let Err(error) = progress(&line) {
                        outcome = Err(error);
                    }
                }
                if !status.success() && outcome.is_ok() {
                    outcome = Err(format!(
                        "Media tool exited unsuccessfully (code {:?})",
                        status.code()
                    ));
                }
                break;
            }
            Ok(None) => {}
            Err(_) => {
                outcome = Err("Cannot inspect media tool process".into());
                break;
            }
        }
    }
    #[cfg(windows)]
    _job.terminate();
    #[cfg(unix)]
    if !exited {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    if !exited {
        let _ = child.kill();
    }
    let _ = child.wait();
    for reader in readers {
        let _ = reader.join();
    }
    // Flush final buffered progress only on successful completion, not after cancellation.
    if outcome.is_ok() && !stopped() {
        for line in rx.try_iter() {
            progress(&line)?;
        }
    }
    outcome
}

#[cfg(windows)]
mod process_tree {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
    };
    pub struct Job(HANDLE);
    impl Job {
        pub fn attach(child: &std::process::Child) -> Result<Self, String> {
            use std::os::windows::io::AsRawHandle;
            unsafe {
                let handle = CreateJobObjectW(None, None)
                    .map_err(|_| "Cannot create media-tool lifecycle guard")?;
                let job = Self(handle);
                let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    std::mem::size_of_val(&limits) as u32,
                )
                .map_err(|_| "Cannot configure media-tool lifecycle guard")?;
                AssignProcessToJobObject(handle, HANDLE(child.as_raw_handle()))
                    .map_err(|_| "Cannot own media-tool process tree")?;
                Ok(job)
            }
        }
        pub fn terminate(&self) {
            unsafe {
                let _ = TerminateJobObject(self.0, 1);
            }
        }
    }
    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}
