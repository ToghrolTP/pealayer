//! Transient, host-owned messages shared by native, web and terminal clients.
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ToastAction {
    pub id: String,
    pub label: String,
    /// Existing Pealayer text command, dispatched only after an explicit click.
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ToastRequest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub actions: Vec<ToastAction>,
    #[serde(default)]
    pub default_action: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
}
fn default_timeout() -> u64 {
    5000
}
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}
impl ToastRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.message.trim().is_empty()
            || self.message.chars().count() > 2048
            || self.title.chars().count() > 120
            || self.timeout_ms > 300_000
            || (self.timeout_ms != 0 && self.timeout_ms < 500)
            || self.id.as_deref().is_some_and(|id| !valid_id(id))
        {
            return Err("toast requires text (1..2048 characters), title <=120, valid ID, and timeout 0 or 500..300000 ms".into());
        }
        if self.actions.len() > 4 || self.details.as_ref().is_some_and(|v| v.chars().count() > 8192) {
            return Err("notification supports up to four actions and 8192 diagnostic characters".into());
        }
        let mut ids = std::collections::HashSet::new();
        for action in &self.actions {
            if !valid_id(&action.id) || !ids.insert(&action.id) || action.label.trim().is_empty()
                || action.label.chars().count() > 80 || action.command.len() > 2048 {
                return Err("invalid notification action".into());
            }
            action_command(action)?;
        }
        if self.default_action.as_ref().is_some_and(|id| !ids.contains(id)) {
            return Err("default action must identify a bound notification action".into());
        }
        Ok(())
    }
}
fn action_command(action: &ToastAction) -> Result<crate::platform::interop::InteropCommand, String> {
    use crate::platform::interop::InteropCommand;
    let command = crate::platform::interop::parse_text_command(&action.command)?;
    if matches!(command, InteropCommand::PublishToast { .. } | InteropCommand::InvokeToastAction { .. }) {
        return Err("notification actions cannot publish or invoke another notification".into());
    }
    command.validate()?;
    Ok(command)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Toast {
    pub id: String,
    pub title: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub severity: Severity,
    pub actions: Vec<ToastAction>,
    pub default_action: Option<String>,
    pub source: String,
    pub created_at_ms: u64,
    pub expires_at_ms: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MessageSnapshot {
    pub instance_id: String,
    pub revision: u64,
    pub toasts: Vec<Toast>,
}
pub struct MessageHub {
    state: MessageSnapshot,
}
impl Default for MessageHub {
    fn default() -> Self {
        Self {
            state: MessageSnapshot {
                instance_id: uuid::Uuid::new_v4().to_string(),
                ..Default::default()
            },
        }
    }
}
impl MessageHub {
    pub fn publish(
        &mut self,
        request: ToastRequest,
        source: &str,
        now: u64,
    ) -> Result<String, String> {
        self.publish_with_details(request, source, now, None)
    }
    fn publish_with_details(&mut self, request: ToastRequest, source: &str, now: u64,
        details: Option<String>) -> Result<String, String> {
        if details.as_ref().is_some_and(|text| text.chars().count() > 8192) {
            return Err("notification details exceed 8192 characters".into());
        }
        request.validate()?;
        self.expire(now);
        let id = request
            .id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let created_at = self.state.toasts.iter().find(|toast| toast.id == id)
            .map_or(now, |toast| now.max(toast.created_at_ms.saturating_add(1)));
        self.state.toasts.retain(|toast| toast.id != id);
        if self.state.toasts.len() >= 32 {
            self.state.toasts.remove(0);
        }
        self.state.toasts.push(Toast {
            id: id.clone(),
            title: request.title,
            message: request.message,
            details: details.or(request.details),
            actions: request.actions,
            default_action: request.default_action,
            severity: request.severity,
            source: source.into(),
            created_at_ms: created_at,
            expires_at_ms: (request.timeout_ms != 0)
                .then(|| now.saturating_add(request.timeout_ms)),
        });
        self.state.revision += 1;
        Ok(id)
    }
    pub fn take_action(&mut self, id: &str, created_at_ms: u64, action_id: &str, now: u64)
        -> Result<crate::platform::interop::InteropCommand, String> {
        self.expire(now);
        let toast = self.state.toasts.iter().find(|t| t.id == id && t.created_at_ms == created_at_ms)
            .ok_or("notification expired or was replaced")?;
        let action = toast.actions.iter().find(|a| a.id == action_id).ok_or("unknown notification action")?;
        let command = action_command(action)?;
        self.dismiss(id); // One activation; duplicate clicks cannot replay a hardware command.
        Ok(command)
    }
    pub fn dismiss(&mut self, id: &str) {
        let before = self.state.toasts.len();
        self.state.toasts.retain(|toast| toast.id != id);
        if before != self.state.toasts.len() {
            self.state.revision += 1;
        }
    }
    fn expire(&mut self, now: u64) {
        let before = self.state.toasts.len();
        self.state
            .toasts
            .retain(|toast| toast.expires_at_ms.is_none_or(|until| until > now));
        if before != self.state.toasts.len() {
            self.state.revision += 1;
        }
    }
    pub fn snapshot(&mut self, now: u64) -> MessageSnapshot {
        self.expire(now);
        self.state.clone()
    }
}
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn hub() -> &'static Mutex<MessageHub> {
    static HUB: OnceLock<Mutex<MessageHub>> = OnceLock::new();
    HUB.get_or_init(Default::default)
}
pub fn publish(request: ToastRequest, source: &str) -> Result<String, String> {
    hub()
        .lock()
        .map_err(|_| "message service unavailable".to_string())?
        .publish(request, source, now_ms())
}
pub fn publish_detailed(request: ToastRequest, source: &str, details: String) -> Result<String, String> {
    hub().lock().map_err(|_| "message service unavailable".to_string())?
        .publish_with_details(request, source, now_ms(), Some(details))
}
pub fn take_action(id: &str, created_at_ms: u64, action_id: &str)
    -> Result<crate::platform::interop::InteropCommand, String> {
    hub().lock().map_err(|_| "message service unavailable".to_string())?
        .take_action(id, created_at_ms, action_id, now_ms())
}
pub fn dismiss(id: &str) {
    if let Ok(mut hub) = hub().lock() {
        hub.dismiss(id);
    }
}
pub fn snapshot() -> MessageSnapshot {
    hub()
        .lock()
        .map(|mut hub| hub.snapshot(now_ms()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(id: &str) -> ToastRequest {
        ToastRequest { actions: Vec::new(), default_action: None, details: None,
            id: Some(id.into()),
            title: "".into(),
            message: "Ready".into(),
            severity: Severity::Success,
            timeout_ms: 1000,
        }
    }
    #[test]
    fn bound_actions_are_validated_fenced_and_consumed_once() {
        let mut hub = MessageHub::default();
        let mut r = request("test");
        r.actions.push(ToastAction { id: "inspect".into(), label: "Inspect".into(), command: "preferences".into() });
        r.default_action = Some("missing".into());
        assert!(r.validate().is_err());
        r.default_action = Some("inspect".into());
        hub.publish(r.clone(), "test", 10).unwrap();
        hub.publish(r.clone(), "test", 10).unwrap();
        assert!(hub.take_action("test", 10, "inspect", 20).is_err());
        assert!(hub.take_action("test", 11, "inspect", 20).is_ok());
        assert!(hub.take_action("test", 11, "inspect", 20).is_err());
        hub.publish(r.clone(), "test", 30).unwrap();
        assert!(hub.take_action("test", 30, "inspect", 1030).is_err());
        r.actions.push(r.actions[0].clone());
        assert!(r.validate().is_err());
    }
    #[test]
    fn concise_notification_retains_bounded_original_diagnostics() {
        let mut hub = MessageHub::default();
        hub.publish_with_details(request("connection"), "hardware", 10, Some("original transport diagnostic".into())).unwrap();
        let state = hub.snapshot(20);
        assert_eq!(state.toasts[0].message, "Ready");
        assert_eq!(state.toasts[0].details.as_deref(), Some("original transport diagnostic"));
        assert!(hub.publish_with_details(request("connection"), "hardware", 30, Some("x".repeat(8193))).is_err());
        assert_eq!(hub.snapshot(40).toasts[0].details, state.toasts[0].details, "rejected details cannot erase last-good toast");
    }
    #[test]
    fn upsert_dismiss_and_expire_are_shared() {
        let mut hub = MessageHub::default();
        hub.publish(request("job"), "IPC", 10).unwrap();
        hub.publish(request("job"), "Web", 20).unwrap();
        let state = hub.snapshot(30);
        assert_eq!(state.toasts.len(), 1);
        assert_eq!(state.revision, 2);
        assert_eq!(state.toasts[0].source, "Web");
        hub.dismiss("job");
        assert!(hub.snapshot(40).toasts.is_empty());
        hub.publish(request("expires"), "CLI", 50).unwrap();
        assert!(hub.snapshot(1050).toasts.is_empty());
    }
    #[test]
    fn bounds_and_persistent_messages() {
        let mut hub = MessageHub::default();
        for i in 0..40 {
            let mut r = request(&format!("m{i}"));
            r.timeout_ms = 0;
            hub.publish(r, "TUI", 0).unwrap();
        }
        assert_eq!(hub.snapshot(u64::MAX).toasts.len(), 32);
        let mut invalid = request("bad id");
        assert!(invalid.validate().is_err());
        invalid.id = None;
        invalid.message.clear();
        assert!(invalid.validate().is_err());
        invalid.message = "ok".into();
        invalid.timeout_ms = 1;
        assert!(invalid.validate().is_err());
    }
}
