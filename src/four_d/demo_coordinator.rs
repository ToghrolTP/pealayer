use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::four_d::controller::ControllerClient;
use crate::four_d::embedded_host::{EmbeddedHost, EmbeddedHostOptions, library_candidates};
use crate::four_d::process_supervisor::ManagedChild;
use crate::four_d::virtual_board::VirtualBoardSession;

pub enum CoordinatorSupervisor {
    Process(ManagedChild),
    Host(EmbeddedHost),
}

pub struct DemoCoordinatorSession {
    pub virtual_board: VirtualBoardSession,
    pub supervisor: CoordinatorSupervisor,
    pub client: ControllerClient,
    pub temp_root: PathBuf,
}

pub fn resolve_controller_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PEALAYER_PCCONTROLLER_BINARY") {
        let pb = PathBuf::from(path);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidates = [
                dir.join("controller"),
                dir.join("controller.exe"),
                dir.join("bin").join("controller"),
                dir.join("bin").join("controller.exe"),
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
            cwd.join("bin").join("controller"),
            cwd.join("bin").join("controller.exe"),
            cwd.join("controller"),
            cwd.join("controller.exe"),
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
            .join("controller.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let dev_candidates = [
        PathBuf::from("scratch/PCController/Tools/Controller/bin/controller"),
        PathBuf::from("scratch/PCController/Tools/Controller/bin/controller.exe"),
        PathBuf::from("scratch/PCController/Tools/Controller/.build/bin/controller"),
        PathBuf::from("scratch/PCController/Tools/Controller/.build/bin/controller.exe"),
    ];
    for candidate in dev_candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

impl DemoCoordinatorSession {
    pub fn start() -> Result<Self, String> {
        let vb = VirtualBoardSession::spawn()?;

        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("pealayer-demo-coordinator-{}-{run_id}", std::process::id()));
        std::fs::create_dir_all(&temp_root).map_err(|e| format!("create demo coordinator temp root: {e}"))?;

        let (supervisor, address) = if let Some(binary) = resolve_controller_binary() {
            let listener = TcpListener::bind(("127.0.0.1", 0))
                .map_err(|e| format!("reserve ephemeral loopback port for coordinator: {e}"))?;
            let coordinator_port = listener
                .local_addr()
                .map_err(|e| format!("resolve reserved coordinator port: {e}"))?
                .port();
            drop(listener);

            let mut cmd = Command::new(&binary);
            cmd.args([
                "ipc",
                "serve",
                "--listen",
                &format!("127.0.0.1:{coordinator_port}"),
                "--no-connect",
            ])
            .env("PCCONTROLLER_DATA_DIR", &temp_root)
            .env("PCCONTROLLER_CONFIG_DIR", &temp_root)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

            let mut child = ManagedChild::spawn(cmd)
                .map_err(|e| format!("spawn PCController coordinator process ({}): {e}", binary.display()))?;

            let deadline = Instant::now() + Duration::from_secs(4);
            let mut ready = false;
            while Instant::now() < deadline {
                if TcpStream::connect(("127.0.0.1", coordinator_port)).is_ok() {
                    ready = true;
                    break;
                }
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(format!("PCController coordinator exited prematurely with status: {status}"));
                }
                thread::sleep(Duration::from_millis(20));
            }

            if !ready {
                let _ = child.kill();
                let _ = child.wait();
                return Err("PCController coordinator did not start listening within 4 seconds".to_string());
            }

            (CoordinatorSupervisor::Process(child), format!("127.0.0.1:{coordinator_port}"))
        } else {
            let candidates = library_candidates();
            if candidates.is_empty() {
                return Err("No packaged PCController coordinator binary or library found. Place controller / controller.exe beside Pealayer or set PEALAYER_PCCONTROLLER_BINARY.".to_string());
            }

            let options = EmbeddedHostOptions {
                data_root: temp_root.clone(),
                app_id: "pealayer.demo".into(),
                app_name: "Pealayer Demo".into(),
                disable_auto_connect: true,
                disable_native: true,
                enable_integrations: false,
                http_address: Some("127.0.0.1:0".into()),
            };

            let mut loaded_host = None;
            let mut errors = Vec::new();
            for path in candidates {
                match EmbeddedHost::load_and_start(&path, &options) {
                    Ok(host) => {
                        loaded_host = Some(host);
                        break;
                    }
                    Err(err) => {
                        errors.push(format!("{}: {err}", path.display()));
                    }
                }
            }

            let host = loaded_host.ok_or_else(|| {
                format!("could not start packaged PCController host for demo mode: {}", errors.join("; "))
            })?;

            let address = host
                .endpoints()
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|ep| ep.get("address"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "demo coordinator exposed no valid listening address".to_string())?
                .to_owned();

            (CoordinatorSupervisor::Host(host), address)
        };

        let mut client = ControllerClient::connect(&address)
            .map_err(|e| format!("connect to demo coordinator at {address}: {e}"))?;

        let connected = client.call("controller.connect", json!({"port": format!("tcp://127.0.0.1:{}", vb.port)}))
            .map_err(|e| format!("attach demo coordinator to virtual board: {e}"))?;
        if connected.get("connected").and_then(|v| v.as_bool()) != Some(true) {
            return Err("demo coordinator reported failed virtual board attachment".to_string());
        }

        let status_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < status_deadline {
            if let Ok(status) = client.call("controller.status", json!({})) {
                if status.get("ina219_available").is_some() || status.get("flags").is_some() {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(30));
        }

        let profile = client.call("controller.board_profile.get", json!({}))
            .unwrap_or_else(|_| json!({"revision": 0}));
        let revision = profile.get("revision").cloned().unwrap_or(json!(0));
        let _ = client.call("controller.board_profile.update", json!({
            "key": "demo",
            "mode": "cinema-seat-motion",
            "expected_revision": revision,
        }));

        Ok(Self {
            virtual_board: vb,
            supervisor,
            client,
            temp_root,
        })
    }
}

impl Drop for DemoCoordinatorSession {
    fn drop(&mut self) {
        match &mut self.supervisor {
            CoordinatorSupervisor::Process(child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            CoordinatorSupervisor::Host(host) => {
                let _ = host.shutdown();
            }
        }
        let _ = std::fs::remove_dir_all(&self.temp_root);
    }
}
