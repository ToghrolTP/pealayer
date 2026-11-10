//! One host-owned terminal API. Private screens never enter general status/WS.
//! Native pairing issues a revocable, memory-only bearer grant; browser input
//! cannot mint grants, change policy or choose an arbitrary OS executable.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::{Condvar, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Snapshot,
    Create {
        target: crate::ui::terminal::TerminalTarget,
    },
    CreateProfile {
        profile_id: String,
    },
    Select {
        id: u64,
    },
    Close {
        id: u64,
    },
    Clear {
        id: u64,
    },
    Input {
        id: u64,
        data: String,
    },
    Scroll {
        id: u64,
        lines: i32,
    },
    RefreshProfiles,
}
pub struct Pending {
    pub operation: Operation,
    pub grant: Option<[u8; 32]>,
    pub deadline: Instant,
    pub reply: mpsc::SyncSender<Result<Value, String>>,
}
#[derive(Default)]
struct State {
    grant: Option<([u8; 32], Instant)>,
    pending: std::collections::VecDeque<Pending>,
    snapshot: Value,
    revision: u64,
}
fn shared() -> &'static (Mutex<State>, Condvar) {
    static STATE: OnceLock<(Mutex<State>, Condvar)> = OnceLock::new();
    STATE.get_or_init(|| (Mutex::new(State::default()), Condvar::new()))
}
fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}
pub fn pair() -> Result<Value, String> {
    if !cfg!(feature = "workspace-terminal") {
        return Err("Terminal is not included in this build".into());
    }
    let config = crate::platform::interop::get_live_config();
    if !config.ui.terminal.allow_remote || !config.web_allow_control {
        return Err("Enable paired remote terminal access in native Terminal preferences and allow Web control first".into());
    }
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut state = shared().0.lock().unwrap_or_else(|e| e.into_inner());
    state.grant = Some((
        digest(&token),
        Instant::now() + Duration::from_secs(8 * 60 * 60),
    ));
    Ok(
        json!({"access_key":token,"expires_in_seconds":8*60*60,"scope":"host terminal sessions","owner_id":crate::peer::instance_id()}),
    )
}
pub fn revoke() {
    shared().0.lock().unwrap_or_else(|e| e.into_inner()).grant = None;
    shared().1.notify_all();
}
pub fn authorize(token: &str) -> Result<[u8; 32], String> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Pair with the terminal host first".into());
    }
    let key = digest(token);
    check_grant(&key)?;
    Ok(key)
}
pub fn check_grant(key: &[u8; 32]) -> Result<(), String> {
    let config = crate::platform::interop::get_live_config();
    if !config.ui.terminal.allow_remote || !config.web_allow_control {
        return Err("Remote terminal access is disabled".into());
    }
    let state = shared().0.lock().unwrap_or_else(|e| e.into_inner());
    if state.grant.as_ref().is_some_and(|(known, expiry)| {
        *expiry > Instant::now()
            && known
                .iter()
                .zip(key)
                .fold(0u8, |diff, (a, b)| diff | (a ^ b))
                == 0
    }) {
        Ok(())
    } else {
        Err("Terminal access key expired or was revoked".into())
    }
}
pub fn remote_policy_patch_allowed(values: &Value) -> bool {
    let current = crate::platform::interop::get_live_config();
    current.apply_patch(values).is_ok_and(|updated| {
        let a = &current.ui.terminal;
        let b = &updated.ui.terminal;
        (
            a.allow_system_shell,
            a.allow_remote,
            a.allow_remote_system_shell,
        ) == (
            b.allow_system_shell,
            b.allow_remote,
            b.allow_remote_system_shell,
        )
    })
}
pub fn changes_policy(command: &crate::platform::interop::InteropCommand) -> bool {
    use crate::platform::interop::InteropCommand;
    match command {
        InteropCommand::UpdateConfig { values } => !remote_policy_patch_allowed(values),
        InteropCommand::PreviewConfig { config }
        | InteropCommand::CommitPreviewConfig { config } => {
            !remote_policy_patch_allowed(&serde_json::to_value(config).unwrap_or_default())
        }
        _ => false,
    }
}
pub fn request(
    operation: Operation,
    grant: Option<[u8; 32]>,
    ctx: &eframe::egui::Context,
) -> Result<Value, String> {
    if let Operation::Input { data, .. } = &operation {
        if data.is_empty() || data.len() > 16384 {
            return Err("Terminal input must contain 1–16384 bytes".into());
        }
    }
    if let Some(grant) = grant {
        check_grant(&grant)?;
    }
    let (tx, rx) = mpsc::sync_channel(1);
    {
        let mut state = shared().0.lock().unwrap_or_else(|e| e.into_inner());
        if state.pending.len() >= 16 {
            return Err("Terminal request queue is full".into());
        }
        state.pending.push_back(Pending {
            operation,
            grant,
            deadline: Instant::now() + Duration::from_secs(if grant.is_some() { 3 } else { 1 }),
            reply: tx,
        });
    }
    crate::platform::interop::wake_command_dispatcher(ctx);
    rx.recv_timeout(Duration::from_secs(4))
        .map_err(|_| "Terminal owner did not respond; request expired".to_string())?
}
pub fn pending() -> Vec<Pending> {
    shared()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .pending
        .drain(..)
        .collect()
}
pub fn watched() -> bool {
    shared()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .grant
        .is_some()
}
pub fn publish(value: Value) {
    let mut state = shared().0.lock().unwrap_or_else(|e| e.into_inner());
    if state.snapshot != value {
        state.snapshot = value;
        state.revision = state.revision.wrapping_add(1);
        shared().1.notify_all();
    }
}
pub fn changes(key: [u8; 32], after: u64) -> Result<Value, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static READERS: AtomicUsize = AtomicUsize::new(0);
    if READERS.fetch_add(1, Ordering::AcqRel) >= 8 {
        READERS.fetch_sub(1, Ordering::AcqRel);
        return Err("Too many terminal viewers (maximum eight)".into());
    }
    struct ReaderGuard;
    impl Drop for ReaderGuard {
        fn drop(&mut self) {
            READERS.fetch_sub(1, Ordering::AcqRel);
        }
    }
    let _reader = ReaderGuard;
    // Event-driven long-poll, bounded wait and periodic revocation checks.
    let started = Instant::now();
    loop {
        check_grant(&key)?;
        let state = shared().0.lock().unwrap_or_else(|e| e.into_inner());
        if state.revision != after || started.elapsed() > Duration::from_secs(15) {
            return Ok(json!({"revision":state.revision,"state":state.snapshot}));
        }
        drop(
            shared()
                .1
                .wait_timeout(state, Duration::from_secs(1))
                .unwrap_or_else(|e| e.into_inner()),
        );
    }
}
pub fn native_payload(payload: &str, ctx: &eframe::egui::Context) -> Option<String> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let method = value["method"].as_str()?;
    if !matches!(
        method,
        "pealayer.terminal.pair" | "pealayer.terminal.revoke" | "pealayer.terminal.control"
    ) {
        return None;
    }
    if value["jsonrpc"] != "2.0"
        || (!value["id"].is_null() && !value["id"].is_string() && !value["id"].is_number())
    {
        return Some(crate::platform::interop::json_rpc_error(
            &Value::Null,
            -32600,
            "Invalid terminal JSON-RPC request",
        ));
    }
    let result = match method {
        "pealayer.terminal.pair" => pair(),
        "pealayer.terminal.revoke" => {
            revoke();
            Ok(json!({"revoked":true}))
        }
        "pealayer.terminal.control" => serde_json::from_value::<Operation>(value["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|op| request(op, None, ctx)),
        _ => return None,
    };
    crate::platform::interop::wake_command_dispatcher(ctx);
    Some(match result {
        Ok(result) => {
            let response = crate::platform::interop::json_rpc_result(&value["id"], result);
            if response.len() > 60 * 1024 {
                crate::platform::interop::json_rpc_error(
                    &value["id"],
                    -32003,
                    "Screen exceeds native reply limit; use the paired HTTP terminal API",
                )
            } else {
                response
            }
        }
        Err(error) => crate::platform::interop::json_rpc_error(&value["id"], -32003, &error),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_keys_and_executable_injection_fail_closed() {
        assert!(authorize("").is_err());
        assert!(authorize(&"x".repeat(64)).is_err());
        assert!(
            serde_json::from_value::<Operation>(
                json!({"operation":"create","target":"system_shell","program":"cmd.exe"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<Operation>(json!({"operation":"input","data":"play"}))
                .is_err()
        );
    }
}
