//! Process-local lifecycle control. Session commands still belong to the peer.
//! Reuse the updater's graceful-exit waiter; never replace binaries or force-kill.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectRequest {
    pub operation_id: String,
    pub endpoint: String,
    pub client_port: u16,
}
impl ConnectRequest {
    pub fn validate(&self) -> Result<(), String> {
        uuid::Uuid::parse_str(&self.operation_id).map_err(|_| "operation_id must be a UUID")?;
        if self.endpoint.len() > 2048 || self.client_port == 0 {
            return Err("Invalid remote endpoint or local port".into());
        }
        crate::peer::endpoint(&self.endpoint)?;
        Ok(())
    }
}

#[derive(Clone, Serialize)]
struct Operation {
    #[serde(flatten)]
    request: ConnectRequest,
    phase: &'static str,
    error: Option<String>,
}
static OPERATION: Mutex<Option<Operation>> = Mutex::new(None);

pub fn status() -> Value {
    json!({"process_id":std::process::id(), "git_commit":env!("PEALAYER_GIT_COMMIT"),
        "git_dirty":env!("PEALAYER_GIT_DIRTY")=="true", "runtime":crate::platform::interop::runtime_identity(),
        "local_control_port":crate::config::control_port(),
        "peer":crate::peer::diagnostics(), "playback_clock":crate::peer::playback_clock_diagnostics(),
        "connect":OPERATION.lock().ok().and_then(|value|value.clone()),
        "effective_unattended_handoff":crate::platform::interop::allow_unattended_hardware_takeover()})
}

fn begin(slot: &mut Option<Operation>, request: &ConnectRequest) -> Result<bool, String> {
    if let Some(previous) = slot {
        if previous.request.operation_id == request.operation_id {
            return if previous.request == *request {
                Ok(false)
            } else {
                Err("operation_id was already used for a different connection".into())
            };
        }
        if matches!(previous.phase, "checking" | "restarting") {
            return Err("A process connection change is already in progress".into());
        }
    }
    *slot = Some(Operation {
        request: request.clone(),
        phase: "checking",
        error: None,
    });
    Ok(true)
}

fn finish(phase: &'static str, error: Option<String>) {
    if let Ok(mut slot) = OPERATION.lock() {
        if let Some(operation) = slot.as_mut() {
            operation.phase = phase;
            operation.error = error;
        }
    }
}

fn release_required(
    authority: &crate::four_d::authority::Status,
    actor: &str,
    paused: bool,
) -> Result<bool, String> {
    if !authority.may_publish(actor) {
        return Ok(false);
    }
    if authority.exclusive {
        return Err("Unlock production before changing this publisher to a remote consumer".into());
    }
    if !paused {
        return Err("Pause playback before changing this publisher to a remote consumer".into());
    }
    Ok(true)
}

/// Called by the unified command dispatcher, not a separate GUI implementation.
pub fn connect(
    request: ConnectRequest,
    context: eframe::egui::Context,
    is_connected: std::sync::Arc<std::sync::atomic::AtomicBool>,
    serial_port: std::sync::Arc<Mutex<String>>,
    mpv: &'static libmpv2::Mpv,
) -> Result<(), String> {
    request.validate()?;
    {
        let mut slot = OPERATION
            .lock()
            .map_err(|_| "Process operation state unavailable")?;
        if !begin(&mut slot, &request)? {
            return Ok(());
        }
    }
    std::thread::Builder::new()
        .name("pealayer-peer-transition".into())
        .spawn(move || {
            let result = (|| -> Result<(), String> {
                let (origin, _, _, _) = crate::peer::probe_session(&request.endpoint)?;
                if crate::peer::client().is_some_and(|client| {
                    client.origin == origin && client.local_port == request.client_port
                }) && crate::peer::diagnostics()["connected"] == true
                {
                    finish("connected", None);
                    return Ok(());
                }
                let endpoint = serial_port
                    .lock()
                    .map_err(|_| "Controller endpoint state unavailable")?
                    .clone();
                if !crate::peer::active() && is_connected.load(std::sync::atomic::Ordering::Acquire)
                {
                    if !crate::four_d::controller::is_controller_endpoint(&endpoint) {
                        return Err(
                            "Disconnect direct hardware before changing the process role".into(),
                        );
                    }
                    let mut controller =
                        crate::four_d::controller::ControllerClient::connect(&endpoint)?;
                    let authority: crate::four_d::authority::Status = serde_json::from_value(
                        controller.call("controller.media.authority.get", json!({}))?,
                    )
                    .map_err(|error| error.to_string())?;
                    let actor = crate::platform::interop::controller_instance_id();
                    let paused = mpv
                        .get_property::<bool>("pause")
                        .map_err(|error| error.to_string())?;
                    if release_required(&authority, &actor, paused)? {
                        let released: crate::four_d::authority::Status =
                            serde_json::from_value(controller.call(
                                "controller.media.authority.change",
                                json!({"client_id":actor,"operation":"release"}),
                            )?)
                            .map_err(|error| format!("Invalid release acknowledgement: {error}"))?;
                        if released.may_publish(&actor) {
                            return Err("Controller did not acknowledge publication release".into());
                        }
                    }
                }
                let plan = ReconnectPlan {
                    parent_pid: std::process::id(),
                    request,
                    fallback_arguments: std::env::args().skip(1).collect(),
                    web_only: std::env::args()
                        .any(|arg| matches!(arg.as_str(), "--web-only" | "--headless")),
                };
                let mut helper =
                    Command::new(std::env::current_exe().map_err(|error| error.to_string())?);
                helper
                    .arg("--process-connect-helper")
                    .arg(serde_json::to_string(&plan).map_err(|error| error.to_string())?);
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    helper.creation_flags(0x08000000);
                }
                helper
                    .spawn()
                    .map_err(|error| format!("Start process connection helper: {error}"))?;
                finish("restarting", None);
                // Closing this local viewport does not forward Quit/Pause to the server.
                context.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
                crate::platform::interop::wake_command_dispatcher(&context);
                Ok(())
            })();
            if let Err(error) = result {
                finish("failed", Some(error.clone()));
                let _ = crate::messaging::publish(
                    crate::messaging::ToastRequest {
                        id: Some("peer.connect".into()),
                        title: "Remote connection change failed".into(),
                        message: error,
                        severity: crate::messaging::Severity::Error,
                        timeout_ms: 10000,
                    },
                    "process",
                );
                crate::platform::interop::wake_command_dispatcher(&context);
            }
        })
        .map_err(|error| {
            finish("failed", Some(error.to_string()));
            error.to_string()
        })?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconnectPlan {
    parent_pid: u32,
    request: ConnectRequest,
    fallback_arguments: Vec<String>,
    web_only: bool,
}

pub fn helper_invocation(args: &[String]) -> Option<&str> {
    args.get(1)
        .filter(|arg| arg.as_str() == "--process-connect-helper")
        .and_then(|_| args.get(2).map(String::as_str))
}

pub fn run_helper(payload: &str) -> Result<(), String> {
    if payload.len() > 65536 {
        return Err("Process reconnect payload too large".into());
    }
    let plan: ReconnectPlan = serde_json::from_str(payload).map_err(|error| error.to_string())?;
    plan.request.validate()?;
    if plan.parent_pid == 0 || plan.parent_pid == std::process::id() {
        return Err("Invalid reconnect parent".into());
    }
    crate::update::wait_for_parent_exit(plan.parent_pid, Duration::from_secs(120))?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(&executable);
    command.args([
        "--connect",
        &plan.request.endpoint,
        "--client-port",
        &plan.request.client_port.to_string(),
    ]);
    if plan.web_only {
        command.arg("--web-only");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    // A failed initial handshake exits before opening the consumer. Restore the
    // original mode rather than leaving the user's application closed.
    for _ in 0..300 {
        if let Some(exit) = child.try_wait().map_err(|error| error.to_string())? {
            if exit.success() {
                return Ok(());
            }
            let mut fallback = Command::new(&executable);
            fallback.args(&plan.fallback_arguments);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                fallback.creation_flags(0x08000000);
            }
            fallback
                .spawn()
                .map_err(|error| format!("Restore previous process mode: {error}"))?;
            return Err("Consumer startup failed; previous mode restarted".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> ConnectRequest {
        ConnectRequest {
            operation_id: uuid::Uuid::new_v4().to_string(),
            endpoint: "pealayer://publisher.example:8080".into(),
            client_port: 8081,
        }
    }
    #[test]
    fn connection_requests_are_bounded_and_current_contract_only() {
        let mut value = request();
        assert!(value.validate().is_ok());
        value.client_port = 0;
        assert!(value.validate().is_err());
        value.client_port = 8081;
        value.endpoint = "http://user:secret@example/".into();
        assert!(value.validate().is_err());
        value = request();
        value.operation_id = "invalid".into();
        assert!(value.validate().is_err());
        let mut encoded = serde_json::to_value(request()).unwrap();
        encoded["force"] = json!(true);
        assert!(serde_json::from_value::<ConnectRequest>(encoded).is_err());
        let invalid=json!({"jsonrpc":"2.0","id":1,"method":"pealayer.process.connect","params":{
            "operation_id":"invalid","endpoint":"pealayer://publisher.example:8080","client_port":8081
        }}).to_string();
        assert!(crate::platform::interop::parse_interop_request(&invalid).is_err());
    }
    #[test]
    fn repeated_operations_never_spawn_another_connection_change() {
        let value = request();
        let mut slot = None;
        assert!(begin(&mut slot, &value).unwrap());
        assert!(!begin(&mut slot, &value).unwrap());
        let mut different = value.clone();
        different.client_port += 1;
        assert!(begin(&mut slot, &different).is_err());
        assert!(begin(&mut slot, &request()).is_err());
        slot.as_mut().unwrap().phase = "failed";
        assert!(!begin(&mut slot, &value).unwrap());
        assert!(begin(&mut slot, &request()).unwrap());
    }
    #[test]
    fn process_lifecycle_cannot_hide_inside_a_forwarded_session_launch() {
        let options = crate::cli::CliOptions {
            target: None,
            fullscreen: false,
            volume: None,
            commands: vec![],
            web_only: false,
        };
        let mut launch = crate::cli::launch_request(&options);
        launch.commands = vec![crate::platform::interop::InteropCommand::QuitLocal];
        assert!(launch.validate().is_err());
        launch.commands = vec![crate::platform::interop::InteropCommand::Play];
        assert!(launch.validate().is_ok());
    }
    #[test]
    fn publication_release_never_bypasses_pause_or_production_lock() {
        let mut state = crate::four_d::authority::Status {
            owner_id: "owner".into(),
            owner_label: "Publisher".into(),
            exclusive: false,
            revision: 1,
            pending: vec![],
            owner_endpoint: None,
        };
        assert!(release_required(&state, "observer", false).is_ok_and(|value| !value));
        assert!(release_required(&state, "owner", false).is_err());
        assert!(release_required(&state, "owner", true).unwrap());
        state.exclusive = true;
        assert!(release_required(&state, "owner", true).is_err());
    }
}
