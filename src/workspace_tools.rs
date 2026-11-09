//! Process-local workspace controls; shell input requires native local IPC.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    SetVisible {
        #[serde(default)]
        toolbar: Option<bool>,
        #[serde(default)]
        terminal: Option<bool>,
        #[serde(default)]
        persist: bool,
    },
    Palette {
        open: bool,
    },
    Terminal {
        #[serde(flatten)]
        operation: TerminalOperation,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum TerminalOperation {
    Connect,
    Disconnect,
    Clear,
    Write { data: String },
    Resize { rows: u16, cols: u16 },
}

impl Request {
    pub fn requires_native_sender(&self) -> bool {
        matches!(
            self,
            Self::Terminal {
                operation: TerminalOperation::Connect
                    | TerminalOperation::Write { .. }
                    | TerminalOperation::Resize { .. }
            }
        )
    }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Terminal {
                operation: TerminalOperation::Write { data },
            } if data.is_empty() || data.len() > 16 * 1024 || data.contains('\0') => {
                Err("Terminal input must contain 1 to 16384 bytes without NUL".into())
            }
            Self::Terminal {
                operation: TerminalOperation::Resize { rows, cols },
            } if !(1..=256).contains(rows) || !(1..=512).contains(cols) => {
                Err("Terminal size must be 1–256 rows and 1–512 columns".into())
            }
            _ => Ok(()),
        }
    }
}

fn shared() -> &'static Mutex<Value> {
    static STATE: OnceLock<Mutex<Value>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(json!({"initialized":false})))
}
pub fn snapshot() -> Value {
    shared().lock().unwrap_or_else(|e| e.into_inner()).clone()
}
pub fn publish(app: &crate::app::PealayerApp) {
    let value = json!({"initialized":true,
        "toolbar":{"available":cfg!(feature="workspace-toolbar"),
            "visible":app.toolbar_state.preferences.enabled,
            "palette_open":app.toolbar_state.palette_open},
        "terminal":app.terminal_state.status()});
    let mut state = shared().lock().unwrap_or_else(|e| e.into_inner());
    if *state != value {
        *state = value;
    }
}
pub fn apply(
    app: &mut crate::app::PealayerApp,
    ctx: &eframe::egui::Context,
    request: Request,
    source: &str,
) -> Result<(), String> {
    request.validate()?;
    if request.requires_native_sender() && source != "IPC" {
        return Err(
            "Terminal execution and input require native local IPC or the terminal UI".into(),
        );
    }
    match request {
        Request::SetVisible {
            toolbar,
            terminal,
            persist,
        } => {
            if toolbar.is_some() && !cfg!(feature = "workspace-toolbar") {
                return Err("Command toolbar is not included in this build".into());
            }
            if terminal.is_some() && !cfg!(feature = "workspace-terminal") {
                return Err("Terminal is not included in this build".into());
            }
            if let Some(value) = toolbar {
                app.toolbar_state.preferences.enabled = value;
            }
            if let Some(value) = terminal {
                app.terminal_state.preferences.enabled = value;
            }
            if persist {
                app.deferred_config_save = true;
            }
        }
        Request::Palette { open } => {
            if !cfg!(feature = "workspace-toolbar") {
                return Err("Command palette is not included in this build".into());
            }
            app.toolbar_state.palette_open = open;
        }
        Request::Terminal { operation } => crate::ui::terminal::control(app, ctx, operation)?,
    }
    publish(app);
    ctx.request_repaint();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_execution_is_native_only_and_bounded() {
        for operation in [
            TerminalOperation::Connect,
            TerminalOperation::Write {
                data: "help".into(),
            },
            TerminalOperation::Resize { rows: 24, cols: 80 },
        ] {
            let request = Request::Terminal { operation };
            assert!(request.requires_native_sender());
            assert!(request.validate().is_ok());
            assert_eq!(
                serde_json::from_value::<Request>(serde_json::to_value(&request).unwrap()).unwrap(),
                request
            );
        }
        assert!(!Request::Palette { open: true }.requires_native_sender());
        assert!(
            Request::Terminal {
                operation: TerminalOperation::Write {
                    data: "x".repeat(16385)
                }
            }
            .validate()
            .is_err()
        );
        assert!(
            Request::Terminal {
                operation: TerminalOperation::Resize { rows: 0, cols: 80 }
            }
            .validate()
            .is_err()
        );
    }
}
