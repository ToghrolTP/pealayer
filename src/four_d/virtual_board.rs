use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::four_d::process_supervisor::ManagedChild;

pub struct VirtualBoardSession {
    child: ManagedChild,
    pub port: u16,
    temp_eeprom: Option<PathBuf>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl VirtualBoardSession {
    pub fn resolve_binary_path() -> Option<PathBuf> {
        if let Some(path) = std::env::var_os("PEALAYER_VIRTUAL_BOARD_BINARY") {
            let pb = PathBuf::from(path);
            if pb.is_file() {
                return Some(pb);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidates = [
                    dir.join("virtual_board"),
                    dir.join("virtual_board.exe"),
                    dir.join("bin").join("virtual_board"),
                    dir.join("bin").join("virtual_board.exe"),
                ];
                for candidate in candidates {
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            let candidates = [
                cwd.join("bin").join("virtual_board"),
                cwd.join("bin").join("virtual_board.exe"),
                cwd.join("virtual_board"),
                cwd.join("virtual_board.exe"),
            ];
            for candidate in candidates {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        #[cfg(target_os = "windows")]
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let candidate = PathBuf::from(local_app_data)
                .join("Programs")
                .join("Pealayer")
                .join("bin")
                .join("virtual_board.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        let dev_candidates = [
            PathBuf::from("scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board"),
            PathBuf::from("scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board.exe"),
            PathBuf::from("scratch/PCController/Tools/VirtualBoard/.build/bin/virtual_board"),
        ];
        for candidate in dev_candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn spawn() -> Result<Self, String> {
        Self::spawn_with_retries(3)
    }

    pub fn spawn_with_retries(max_retries: usize) -> Result<Self, String> {
        crate::four_d::demo_cleanup::cleanup_stale_demo_resources();

        let binary = Self::resolve_binary_path().ok_or_else(|| {
            "virtual_board binary could not be found. Check bin/virtual_board or set PEALAYER_VIRTUAL_BOARD_BINARY.".to_string()
        })?;

        let mut last_error = String::new();
        let retries = max_retries.max(1);

        for attempt in 1..=retries {
            let listener = match TcpListener::bind(("127.0.0.1", 0)) {
                Ok(l) => l,
                Err(e) => {
                    last_error = format!("reserve ephemeral loopback port for virtual board: {e}");
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };
            let port = match listener.local_addr() {
                Ok(addr) => addr.port(),
                Err(e) => {
                    last_error = format!("resolve reserved loopback port: {e}");
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };
            drop(listener);

            let run_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let temp_eeprom = std::env::temp_dir().join(format!(
                "virtual-mcu-eeprom-{}-{}-{run_id}.bin",
                std::process::id(),
                attempt
            ));

            let mut cmd = Command::new(&binary);
            cmd.args([
                "--bind",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--eeprom",
                &temp_eeprom.to_string_lossy(),
                "--quiet",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

            let mut child = match ManagedChild::spawn(cmd) {
                Ok(c) => c,
                Err(e) => {
                    last_error = format!("spawn virtual_board child process ({}): {e}", binary.display());
                    let _ = std::fs::remove_file(&temp_eeprom);
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };

            let stdin = Arc::new(Mutex::new(child.take_stdin()));

            let mut session = Self {
                child,
                port,
                temp_eeprom: Some(temp_eeprom),
                stdin,
            };
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut ready = false;
            while Instant::now() < deadline {
                if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                    ready = true;
                    break;
                }
                if let Ok(Some(status)) = session.child.try_wait() {
                    last_error = format!("virtual_board exited prematurely with status: {status}");
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }

            if ready {
                return Ok(session);
            }

            // Cleanup attempt before retrying
            drop(session);
            thread::sleep(Duration::from_millis(50 * attempt as u64));
        }

        Err(format!("virtual_board failed to start after {retries} attempts: {last_error}"))
    }

    /// Sends a line-oriented simulation command to the virtual board over stdin.
    pub fn send_command(&self, command: &str) -> Result<(), String> {
        let mut guard = self.stdin.lock().map_err(|e| e.to_string())?;
        if let Some(stdin) = guard.as_mut() {
            use std::io::Write;
            writeln!(stdin, "{command}").map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("virtual board stdin is unavailable".to_string())
        }
    }
}

impl Drop for VirtualBoardSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(ref path) = self.temp_eeprom {
            let _ = std::fs::remove_file(path);
        }
    }
}
