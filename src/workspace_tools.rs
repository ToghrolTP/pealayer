//! Process-local workspace controls; shell input requires native local IPC.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    Panel {
        panel: crate::ui::layout::PealayerTab,
        #[serde(flatten)]
        operation: PanelOperation,
    },
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
    Editor {
        open: bool,
    },
    Terminal {
        #[serde(flatten)]
        operation: TerminalOperation,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum PanelOperation {
    Toggle,
    Sidebar,
    Dock { #[serde(default)] anchor: Option<crate::ui::layout::PealayerTab>, #[serde(default)] zone: Option<craft_ui::docking::Zone> },
    SidebarWidth { width: f32 },
    SidebarMenu,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum TerminalOperation {
    RefreshProfiles,
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
                    | TerminalOperation::RefreshProfiles
                    | TerminalOperation::Write { .. }
                    | TerminalOperation::Resize { .. }
            }
        )
    }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Panel { operation: PanelOperation::SidebarWidth { width }, .. }
                if !width.is_finite() || !(240.0..=600.0).contains(width) => Err("Sidebar width must be between 240 and 600 points".into()),
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
            "palette_open":app.toolbar_state.palette_open,
            "editor_open":app.toolbar_state.editor_open},
        "terminal":app.terminal_state.status(),
        "panels": crate::ui::layout::PealayerTab::ALL.iter().map(|panel| json!({"panel":panel,
            "in_dock":app.is_tab_open_in_dock(*panel),"in_sidebar":app.is_tab_open_in_sidebar(*panel),
            "location":app.dock_state.layout.location(panel).ok().or_else(||app.dock_state.hidden.iter().find(|(p,_)|p==panel).map(|(_,l)|l.clone()))})).collect::<Vec<_>>(),
        "sidebar": app.sidebar_rail_state});
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
        Request::Panel { panel, operation } => {
            if !panel.available() { return Err("Panel is unavailable in this build".into()); }
            if app.dock_state.locked && !matches!(operation, PanelOperation::SidebarMenu) { return Err("Unlock the workspace before changing a panel".into()); }
            match operation {
                PanelOperation::Toggle => app.toggle_rail_panel(ctx, panel)?,
                PanelOperation::Sidebar => {
                    let pane = crate::ui::sidebar_rail::SidebarPane::try_from(panel).map_err(|_|"This panel does not support the sidebar")?;
                    app.open_sidebar_pane(pane)?;
                }
                PanelOperation::Dock { anchor, zone } => {
                    crate::ui::sidebar_shortcuts::dock_panel(app, panel, anchor,
                        craft_ui::docking::Placement::Split(zone.unwrap_or(craft_ui::docking::Zone::Center)))
                        .map_err(|error|format!("Cannot dock panel: {error:?}"))?;
                    app.pending_tab_reveals.push(panel);
                    app.sync_terminal_visibility_from_workspace();
                }
                PanelOperation::SidebarWidth { width } => {
                    if !app.is_tab_open_in_sidebar(panel) { return Err("Open this panel in the sidebar before resizing it".into()); }
                    app.sidebar_rail_state.apply_pane_request(craft_ui::panes::PaneRequest::Resize(panel.try_into().unwrap(), width));
                }
                PanelOperation::SidebarMenu => {
                    if !app.is_tab_open_in_sidebar(panel) { return Err("Open this panel in the sidebar before opening its header menu".into()); }
                    ctx.data_mut(|data| data.insert_temp(eframe::egui::Id::unique("sidebar-menu-request"), panel));
                }
            }
            app.deferred_config_save = true;
        }
        Request::SetVisible {
            toolbar,
            terminal,
            persist,
        } => {
            if terminal.is_some_and(|wanted| {
                wanted
                    != app
                        .dock_state
                        .layout
                        .contains(&crate::ui::layout::PealayerTab::Terminal)
            }) && app.dock_state.locked
            {
                return Err(
                    "Unlock the workspace before showing or hiding its terminal pane".into(),
                );
            }
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
        Request::Editor { open } => {
            if !cfg!(feature = "workspace-toolbar") {
                return Err("Toolbar editor is not included in this build".into());
            }
            app.toolbar_state.editor_open = open;
            if open {
                app.toolbar_state.palette_open = false;
            }
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
    fn sidebar_panel_api_round_trips_toggles_and_rejects_invalid_resize_without_changes() {
        use crate::ui::layout::PealayerTab;
        let _guard=crate::app::tests::lock_app_tests();
        let ctx=eframe::egui::Context::default(); let mut app=crate::app::PealayerApp::default();
        let request=Request::Panel {panel:PealayerTab::EffectsLibrary,operation:PanelOperation::Toggle};
        assert_eq!(serde_json::from_value::<Request>(serde_json::to_value(&request).unwrap()).unwrap(),request);
        let layout=app.dock_state.layout.clone();
        for _ in 0..2 {apply(&mut app,&ctx,request.clone(),"HTTP").unwrap();}
        assert_eq!(app.dock_state.layout,layout);
        apply(&mut app,&ctx,Request::Panel {panel:PealayerTab::EffectsLibrary,operation:PanelOperation::Sidebar},"HTTP").unwrap();
        apply(&mut app,&ctx,Request::Panel {panel:PealayerTab::EffectsLibrary,operation:PanelOperation::SidebarWidth {width:420.0}},"HTTP").unwrap();
        assert_eq!(snapshot()["sidebar"]["width"],420.0);
        let before=app.sidebar_rail_state;
        for width in [f32::NAN,10.0,700.0] {
            assert!(apply(&mut app,&ctx,Request::Panel {panel:PealayerTab::EffectsLibrary,operation:PanelOperation::SidebarWidth {width}},"HTTP").is_err());
            assert_eq!(app.sidebar_rail_state,before);
        }
        app.dock_state.locked=true;
        assert!(apply(&mut app,&ctx,request,"HTTP").is_err());
        assert_eq!(app.sidebar_rail_state,before);
        apply(&mut app,&ctx,Request::Panel {panel:PealayerTab::EffectsLibrary,operation:PanelOperation::SidebarMenu},"HTTP").unwrap();
    }
    #[cfg(feature = "workspace-terminal")]
    #[test]
    fn locked_terminal_visibility_api_fails_without_partial_toolbar_mutation() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = crate::app::PealayerApp::default();
        app.dock_state.locked = true;
        let request = Request::SetVisible {
            toolbar: Some(true),
            terminal: Some(true),
            persist: false,
        };
        assert!(
            apply(&mut app, &eframe::egui::Context::default(), request, "IPC")
                .unwrap_err()
                .contains("Unlock")
        );
        assert!(!app.toolbar_state.preferences.enabled);
        assert!(!app.terminal_state.preferences.enabled);
    }
    #[test]
    fn terminal_execution_is_native_only_and_bounded() {
        for operation in [
            TerminalOperation::RefreshProfiles,
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
        let editor = Request::Editor { open: true };
        assert!(!editor.requires_native_sender());
        assert_eq!(
            serde_json::to_value(&editor).unwrap(),
            json!({"action":"editor","open":true})
        );
        assert_eq!(
            serde_json::from_value::<Request>(json!({"action":"editor","open":true})).unwrap(),
            editor
        );
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

    #[cfg(feature = "workspace-toolbar")]
    #[test]
    fn local_editor_api_only_changes_editor_visibility_and_publishes_it() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = crate::app::PealayerApp::default();
        let ctx = eframe::egui::Context::default();
        let workspace = app.dock_state.clone();
        let preferences = app.toolbar_state.preferences.clone();
        apply(&mut app, &ctx, Request::Editor { open: true }, "HTTP").unwrap();
        assert!(app.toolbar_state.editor_open);
        assert!(!app.toolbar_state.palette_open);
        assert_eq!(snapshot()["toolbar"]["editor_open"], true);
        assert_eq!(app.dock_state, workspace);
        assert_eq!(app.toolbar_state.preferences, preferences);
        assert!(!app.terminal_state.busy());
        apply(&mut app, &ctx, Request::Editor { open: false }, "HTTP").unwrap();
        assert!(!app.toolbar_state.editor_open);
        assert_eq!(snapshot()["toolbar"]["editor_open"], false);
    }
}
