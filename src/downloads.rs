//! One download service shared by native windows and all existing RPC transports.
use pealayer_downloads::Manager;
use std::sync::{Mutex, OnceLock};

static MANAGER: OnceLock<Result<Manager, String>> = OnceLock::new();
pub fn manager() -> Result<&'static Manager, String> {
    MANAGER
        .get_or_init(|| pealayer_downloads::default_root().and_then(Manager::open))
        .as_ref()
        .map_err(Clone::clone)
}

pub fn shutdown() {
    if let Some(Ok(manager)) = MANAGER.get() {
        if let Err(error) = manager.shutdown(std::time::Duration::from_secs(7)) {
            log::warn!("Download shutdown: {error}");
        }
    }
}

pub fn rpc_payload(payload: &str, web: bool) -> Option<String> {
    let request: crate::platform::interop::JsonRpcRequest = serde_json::from_str(payload).ok()?;
    if !request.method.starts_with("pealayer.downloads.") {
        return None;
    }
    if web {
        let config = crate::platform::interop::get_live_config();
        if !config.web_allow_file_access
            || (request.method != "pealayer.downloads.list" && !config.web_allow_control)
        {
            return Some(crate::platform::interop::json_rpc_error(
                &request.id,
                -32003,
                "Download file/control permission is disabled",
            ));
        }
    }
    let result = if let Some(client) = crate::peer::client() {
        client.post("/api/rpc", &serde_json::json!({"jsonrpc":"2.0", "id":request.id, "method":request.method, "params":request.params}))
            .and_then(|value| value.get("result").cloned().ok_or_else(|| "Authority rejected download operation".into()))
    } else {
        dispatch(&request.method, request.params)
    };
    Some(match result {
        Ok(value) => crate::platform::interop::json_rpc_result(&request.id, value),
        Err(error) => crate::platform::interop::json_rpc_error(&request.id, -32000, &error),
    })
}

fn dispatch(method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    if !matches!(
        method,
        "pealayer.downloads.list"
            | "pealayer.downloads.add"
            | "pealayer.downloads.action"
            | "pealayer.downloads.configure"
            | "pealayer.downloads.engines.configure"
    ) {
        return Err("Unknown download method".into());
    }
    let manager = manager()?;
    match method {
        "pealayer.downloads.engines.configure" => {
            let settings = serde_json::from_value(params).map_err(|_| "Invalid engine settings")?;
            manager.configure_engines(settings)?;
            Ok(serde_json::json!({"accepted":true}))
        }
        "pealayer.downloads.list" => serde_json::to_value(manager.snapshot())
            .map_err(|_| "Cannot serialize download state".into()),
        "pealayer.downloads.add" => {
            let request = serde_json::from_value(params).map_err(|_| "Invalid download request")?;
            Ok(serde_json::json!({"id":manager.add(request)?}))
        }
        "pealayer.downloads.action" => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Action {
                id: String,
                action: String,
            }
            let action: Action =
                serde_json::from_value(params).map_err(|_| "Invalid download action")?;
            manager.action(&action.id, &action.action)?;
            Ok(serde_json::json!({"accepted":true}))
        }
        "pealayer.downloads.configure" => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Settings {
                max_concurrent: usize,
                bytes_per_second: u64,
            }
            let settings: Settings =
                serde_json::from_value(params).map_err(|_| "Invalid download settings")?;
            manager.configure(settings.max_concurrent, settings.bytes_per_second)?;
            Ok(serde_json::json!({"accepted":true}))
        }
        _ => Err("Unknown download method".into()),
    }
}

pub fn open(ctx: &eframe::egui::Context) {
    ctx.data_mut(|d| d.insert_temp(eframe::egui::Id::new("download-center-open"), true));
}

pub fn draw(app: &mut crate::app::PealayerApp, ui: &mut eframe::egui::Ui) {
    static VIEW: OnceLock<Mutex<pealayer_downloads::ui::View>> = OnceLock::new();
    let id = eframe::egui::Id::new("download-center-open");
    let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    if !open {
        return;
    }
    let mut play = None;
    eframe::egui::Window::new(format!(
        "{} Downloads",
        egui_phosphor::regular::DOWNLOAD_SIMPLE
    ))
    .id(eframe::egui::Id::new("download-center"))
    .open(&mut open)
    .default_size([720.0, 540.0])
    .show(ui.ctx(), |ui| {
        if crate::peer::active() {
            ui.label("Manage the authority’s downloads from its Web Downloads panel.");
        } else {
            match manager() {
                Ok(manager) => {
                    if let Ok(mut view) = VIEW.get_or_init(|| Mutex::new(Default::default())).lock()
                    {
                        play = view.draw(ui, manager);
                    }
                }
                Err(error) => {
                    ui.colored_label(eframe::egui::Color32::LIGHT_RED, error);
                }
            }
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, open));
    if let Some(path) = play {
        app.load_media_target(&path);
    }
}
