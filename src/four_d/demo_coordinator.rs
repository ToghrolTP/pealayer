use std::net::TcpListener;
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
    _temp_root: DemoDataRoot,
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

struct DemoDataRoot(PathBuf);

impl Drop for DemoDataRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl DemoCoordinatorSession {
    pub fn start() -> Result<Self, String> {
        crate::four_d::demo_cleanup::cleanup_stale_demo_resources();

        let vb = VirtualBoardSession::spawn()?;
        vb.send_command("door closed")
            .map_err(|error| format!("initialize virtual enclosure: {error}"))?;

        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("pealayer-demo-coordinator-{}-{run_id}", std::process::id()));
        std::fs::create_dir(&temp_root).map_err(|e| format!("create demo coordinator temp root: {e}"))?;
        let temp_root = DemoDataRoot(temp_root);

        let config_path = temp_root.0.join("config.json");
        // The private coordinator uses the normal host contract without
        // broadcasting or opening local physical-device integrations.
        let travel_ms = crate::platform::interop::get_live_config().virtual_seat_travel_ms.clamp(100, 60_000);
        let config = json!({
            "connection": { "virtual_seat_travel_ms": travel_ms },
            "ui": { "peripheral_names": { "seat.a": "Left side", "seat.b": "Right side" } },
            "integrations": {
                "discovery": {
                    "mndp_enabled": false,
                    "mdns_enabled": false,
                    "dns_sd_enabled": false,
                    "ssdp_enabled": false,
                    "upnp_enabled": false,
                    "ws_discovery_enabled": false,
                    "broadcast_enabled": false,
                    "netbios_enabled": false
                },
                "local_device": { "enabled": false }
            }
        });
        std::fs::write(&config_path, config.to_string())
            .map_err(|error| format!("write isolated demo controller configuration: {error}"))?;
        let (supervisor, client) = if let Some(binary) = resolve_controller_binary() {
            let mut last_err = String::new();
            let mut started = None;

            for attempt in 1..=3 {
                let listener = match TcpListener::bind(("127.0.0.1", 0)) {
                    Ok(l) => l,
                    Err(e) => {
                        last_err = format!("reserve ephemeral loopback port for coordinator: {e}");
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                };
                let coordinator_port = match listener.local_addr() {
                    Ok(addr) => addr.port(),
                    Err(e) => {
                        last_err = format!("resolve reserved coordinator port: {e}");
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                };
                drop(listener);

                let mut cmd = Command::new(&binary);
                cmd.args([
                    "ipc",
                    "serve",
                    "--listen",
                    &format!("127.0.0.1:{coordinator_port}"),
                    "--no-connect",
                ])
                .env("PCCONTROLLER_DATA_DIR", temp_root.0.join("data"))
                .env("PCCONTROLLER_CONFIG", &config_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null());

                let mut child = match ManagedChild::spawn(cmd) {
                    Ok(c) => c,
                    Err(e) => {
                        last_err = format!("spawn PCController coordinator process ({}): {e}", binary.display());
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                };

                let address = format!("127.0.0.1:{coordinator_port}");
                let expected_host = format!("ipc:{}", child.id());
                let deadline = Instant::now() + Duration::from_secs(8);
                while Instant::now() < deadline {
                    if let Ok(Some(status)) = child.try_wait() {
                        last_err = format!("PCController coordinator exited prematurely with status: {status}");
                        break;
                    }
                    match ControllerClient::connect(&address) {
                        Ok(mut client) => match client.call("controller.snapshot", json!({})) {
                            Ok(snapshot) if snapshot["host_instance_id"].as_str() == Some(expected_host.as_str()) => {
                                started = Some(client);
                                break;
                            }
                            Ok(_) => {
                                last_err = format!("another coordinator owns {address}");
                                break;
                            }
                            Err(error) => last_err = error,
                        },
                        Err(error) => last_err = error,
                    }
                    thread::sleep(Duration::from_millis(50));
                }

                if let Some(client) = started.take() {
                    return Self::attach(vb, CoordinatorSupervisor::Process(child), client, temp_root);
                }

                drop(child);
                thread::sleep(Duration::from_millis(50 * attempt as u64));
            }

            return Err(format!("PCController coordinator did not start after 3 attempts: {last_err}"));
        } else {
            let candidates = library_candidates();
            if candidates.is_empty() {
                return Err("No packaged PCController coordinator binary or library found. Place controller / controller.exe beside Pealayer or set PEALAYER_PCCONTROLLER_BINARY.".to_string());
            }

            let options = EmbeddedHostOptions {
                data_root: temp_root.0.clone(),
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

            let client = ControllerClient::connect(&address)
                .map_err(|error| format!("connect to demo coordinator at {address}: {error}"))?;
            (CoordinatorSupervisor::Host(host), client)
        };
        Self::attach(vb, supervisor, client, temp_root)
    }

    fn attach(
        vb: VirtualBoardSession,
        supervisor: CoordinatorSupervisor,
        mut client: ControllerClient,
        temp_root: DemoDataRoot,
    ) -> Result<Self, String> {
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
            .map_err(|error| format!("read demo board profile: {error}"))?;
        let revision = profile.get("revision")
            .ok_or("demo board profile did not advertise a revision")?;
        client.call("controller.board_profile.update", json!({
            "key": "demo",
            "mode": "cinema-seat-motion",
            "expected_revision": revision,
        })).map_err(|error| format!("configure demo board profile: {error}"))?;

        Ok(Self {
            virtual_board: vb,
            supervisor,
            client,
            _temp_root: temp_root,
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
    }
}
