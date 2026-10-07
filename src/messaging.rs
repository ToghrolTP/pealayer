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
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Toast {
    pub id: String,
    pub title: String,
    pub message: String,
    pub severity: Severity,
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
        request.validate()?;
        self.expire(now);
        let id = request
            .id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        self.state.toasts.retain(|toast| toast.id != id);
        if self.state.toasts.len() >= 32 {
            self.state.toasts.remove(0);
        }
        self.state.toasts.push(Toast {
            id: id.clone(),
            title: request.title,
            message: request.message,
            severity: request.severity,
            source: source.into(),
            created_at_ms: now,
            expires_at_ms: (request.timeout_ms != 0)
                .then(|| now.saturating_add(request.timeout_ms)),
        });
        self.state.revision += 1;
        Ok(id)
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
        ToastRequest {
            id: Some(id.into()),
            title: "".into(),
            message: "Ready".into(),
            severity: Severity::Success,
            timeout_ms: 1000,
        }
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
