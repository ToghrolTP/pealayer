use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::four_d::process_supervisor::ManagedChild;

pub struct VirtualBoardSession {
    child: ManagedChild,
    pub port: u16,
    temp_eeprom: Option<PathBuf>,
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
        crate::four_d::demo_cleanup::cleanup_stale_demo_resources();

        let binary = Self::resolve_binary_path().ok_or_else(|| {
            "virtual_board binary could not be found. Check bin/virtual_board or set PEALAYER_VIRTUAL_BOARD_BINARY.".to_string()
        })?;

        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| format!("reserve ephemeral loopback port for virtual board: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("resolve reserved loopback port: {e}"))?
            .port();
        drop(listener);

        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_eeprom = std::env::temp_dir().join(format!("virtual-mcu-eeprom-{}-{run_id}.bin", std::process::id()));

        let mut cmd = Command::new(&binary);
        cmd.args([
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--eeprom",
            &temp_eeprom.to_string_lossy(),
            "--no-stdin",
            "--quiet",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());

        let child = ManagedChild::spawn(cmd)
            .map_err(|e| format!("spawn virtual_board child process ({}): {e}", binary.display()))?;

        let mut session = Self { child, port, temp_eeprom: Some(temp_eeprom) };
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut ready = false;
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                ready = true;
                break;
            }
            if let Ok(Some(status)) = session.child.try_wait() {
                return Err(format!("virtual_board exited prematurely with status: {status}"));
            }
            thread::sleep(Duration::from_millis(20));
        }

        if !ready {
            let _ = session.child.kill();
            let _ = session.child.wait();
            return Err("virtual_board did not start listening within 3 seconds".to_string());
        }

        Ok(session)
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
