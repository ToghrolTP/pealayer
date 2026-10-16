use std::path::PathBuf;
use serde_json::json;
use crate::four_d::controller::ControllerClient;
use crate::four_d::embedded_host::{EmbeddedHost, EmbeddedHostOptions, library_candidates};
use crate::four_d::virtual_board::VirtualBoardSession;

pub struct DemoCoordinatorSession {
    pub virtual_board: VirtualBoardSession,
    pub host: EmbeddedHost,
    pub client: ControllerClient,
    pub temp_root: PathBuf,
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

        let options = EmbeddedHostOptions {
            data_root: temp_root.clone(),
            app_id: "pealayer.demo".into(),
            app_name: "Pealayer Demo".into(),
            disable_auto_connect: true,
            disable_native: true,
            enable_integrations: false,
            http_address: Some("127.0.0.1:0".into()),
        };

        let candidates = library_candidates();
        if candidates.is_empty() {
            return Err("No packaged PCController library found. Place pccontroller.so / pccontroller.dll beside Pealayer or set PEALAYER_PCCONTROLLER_LIBRARY.".to_string());
        }

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

        let mut client = ControllerClient::connect(&address)
            .map_err(|e| format!("connect to demo coordinator at {address}: {e}"))?;

        let connected = client.call("controller.connect", json!({"port": format!("tcp://127.0.0.1:{}", vb.port)}))
            .map_err(|e| format!("attach demo coordinator to virtual board: {e}"))?;
        if connected.get("connected").and_then(|v| v.as_bool()) != Some(true) {
            return Err("demo coordinator reported failed virtual board attachment".to_string());
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
            host,
            client,
            temp_root,
        })
    }
}

impl Drop for DemoCoordinatorSession {
    fn drop(&mut self) {
        let _ = self.host.shutdown();
        let _ = std::fs::remove_dir_all(&self.temp_root);
    }
}
