use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Claim {
    pub client_id: String,
    pub label: String,
    pub requested_at: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Status {
    pub owner_id: String,
    pub owner_label: String,
    pub exclusive: bool,
    pub revision: u64,
    pub pending: Vec<Claim>,
    /// Optional, validated origin advertised by the registered owner instance.
    #[serde(default)]
    pub owner_endpoint: Option<String>,
}
impl Status {
    pub fn may_publish(&self, id: &str) -> bool {
        !self.owner_id.is_empty() && self.owner_id == id
    }
    pub fn request_pending(&self, id: &str) -> bool {
        self.pending.iter().any(|claim| claim.client_id == id)
    }
    pub fn unattended_claim(&self, id: &str, enabled: bool) -> Option<&Claim> {
        (enabled && !self.exclusive && !id.is_empty() && self.owner_id == id)
            .then(|| self.pending.iter().find(|claim| claim.client_id != id))
            .flatten()
    }
}

pub fn owner_endpoint(owner_id: &str, instance: &serde_json::Value) -> Option<String> {
    if owner_id.is_empty() || instance["id"] != owner_id || instance["surface"] != "pealayer" {
        return None;
    }
    let origin = instance["values"]["peer_origin"].as_str()?;
    crate::peer::endpoint(origin).ok().map(|url| url.to_string())
}

fn connect_button(ui: &mut eframe::egui::Ui, state: &Status) -> bool {
    if crate::ui::dialog::action_button(ui, crate::ui::icons::PLUG, "Connect to authority")
        .on_hover_text("Open a synchronized remote client instead of requesting hardware publishing. The server address can be edited.")
        .clicked()
    {
        crate::ui::peer_browser::connection_dialog_to(ui.ctx(), state.owner_endpoint.as_deref());
        return true;
    }
    false
}

pub fn request(app: &mut crate::app::PealayerApp, operation: &str, requester_id: &str) {
    let actor = app
        .engine_handle
        .prepared_timeline
        .lock()
        .map(|plan| plan.authority_client_id.clone())
        .unwrap_or_else(|_| crate::platform::interop::controller_instance_id());
    let params =
        serde_json::json!({"operation":operation,"client_id":actor,"requester_id":requester_id});
    if let Err(error) = app.engine_handle.request_controller_call(
        "hardware-authority",
        "controller.media.authority.change",
        params,
    ) {
        let _ = crate::messaging::publish(
            crate::messaging::ToastRequest {
                id: Some("hardware.authority".into()),
                title: "Publishing authority".into(),
                message: error,
                severity: crate::messaging::Severity::Error,
                timeout_ms: 8000,
            },
            "hardware",
        );
    }
}

pub fn draw_controls(app: &mut crate::app::PealayerApp, ui: &mut eframe::egui::Ui) {
    let state = app
        .engine_handle
        .prepared_timeline
        .lock()
        .ok()
        .and_then(|plan| plan.authority.clone());
    let Some(state) = state else { return };
    let id = app
        .engine_handle
        .prepared_timeline
        .lock()
        .map(|plan| plan.authority_client_id.clone())
        .unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "{} {}",
            crate::ui::icons::BROADCAST,
            if state.owner_id.is_empty() {
                "No publisher"
            } else {
                &state.owner_label
            }
        ));
        if state.owner_id == id {
            if ui
                .button(format!(
                    "{} {}",
                    crate::ui::icons::LOCK,
                    if state.exclusive {
                        "Unlock production"
                    } else {
                        "Lock production"
                    }
                ))
                .clicked()
            {
                request(app, if state.exclusive { "unlock" } else { "lock" }, "");
            }
            if ui
                .add_enabled(
                    app.is_paused,
                    eframe::egui::Button::new("Release authority"),
                )
                .on_hover_text("Pause before handing over publishing")
                .clicked()
            {
                request(app, "release", "");
            }
        } else if !state.exclusive {
            if ui
                .add_enabled(
                    !state.request_pending(&id),
                    eframe::egui::Button::new(if state.request_pending(&id) {
                        "Handoff requested"
                    } else {
                        "Request authority"
                    }),
                )
                .clicked()
            {
                request(app, "request", "");
            }
        } else {
            ui.label(format!(
                "{} Production locked · monitoring only",
                crate::ui::icons::LOCK
            ));
        }
        if !state.owner_id.is_empty() && state.owner_id != id {
            connect_button(ui, &state);
        }
    });
}

/// One root modal per meaningful authority revision, not one per panel/repaint.
pub fn draw_warning(app: &mut crate::app::PealayerApp, ctx: &eframe::egui::Context) {
    use eframe::egui;
    let state = app
        .engine_handle
        .prepared_timeline
        .lock()
        .ok()
        .and_then(|plan| plan.authority.clone());
    let Some(state) = state else { return };
    let id = app
        .engine_handle
        .prepared_timeline
        .lock()
        .map(|plan| plan.authority_client_id.clone())
        .unwrap_or_default();
    let owner = state.owner_id == id;
    let conflict = !state.owner_id.is_empty() && !owner;
    let pending = state.pending.first().cloned();
    if !conflict && !(owner && pending.is_some()) {
        return;
    }
    let popup_id = egui::Id::new("media_authority_conflict");
    let acknowledged = ctx.data(|data| data.get_temp::<u64>(popup_id));
    if acknowledged == Some(state.revision) {
        return;
    }
    let mut dismiss = false;
    egui::Modal::new(popup_id).show(ctx,|ui| {
        ui.set_max_width(460.0);
        ui.heading(format!("{} Publishing authority",crate::ui::icons::WARNING));
        if owner {
            let claim=pending.as_ref().unwrap();
            ui.label(format!("{} requests control of the media clock and hardware timeline.",claim.label));
            if !app.is_paused {ui.label("Pause playback before accepting the handoff.");}
            if crate::platform::interop::allow_unattended_hardware_takeover() && !state.exclusive {
                ui.label("Unattended handoff is enabled. Playback pauses before control is transferred.");
            }
            ui.horizontal(|ui| {
                if crate::ui::dialog::action_button(ui,crate::ui::icons::X,"Decline").clicked(){request(app,"reject",&claim.client_id);dismiss=true;}
                ui.add_enabled_ui(app.is_paused && !state.exclusive,|ui|{
                    if crate::ui::dialog::primary_action_button(ui,crate::ui::icons::CHECK,"Accept handoff").clicked(){request(app,"accept",&claim.client_id);dismiss=true;}
                });
            });
        } else {
            ui.label(format!("{} owns hardware playback. This instance is not publishing a competing timeline.",state.owner_label));
            if state.exclusive {ui.label("Production is locked. Hardware controls are read-only; E-STOP remains available.");}
            ui.horizontal(|ui| {
                if crate::ui::dialog::action_button(ui,crate::ui::icons::EYE,"Monitor").clicked(){dismiss=true;}
                if connect_button(ui, &state) { dismiss = true; }
                ui.add_enabled_ui(!state.exclusive && !state.request_pending(&id),|ui|{
                    if crate::ui::dialog::primary_action_button(ui,crate::ui::icons::BROADCAST,"Request handoff").clicked(){request(app,"request","");dismiss=true;}
                });
            });
        }
    });
    if dismiss {
        ctx.data_mut(|data| data.insert_temp(popup_id, state.revision));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ownership_is_not_connection_state() {
        let state = Status {
            owner_id: "a".into(),
            owner_label: "Production".into(),
            exclusive: false,
            revision: 1,
            pending: vec![],
            owner_endpoint: None,
        };
        assert!(state.may_publish("a"));
        assert!(!state.may_publish("b"));
        assert!(serde_json::from_str::<Status>(r#"{"owner_id":"a"}"#).is_err());
    }

    #[test]
    fn unattended_consent_is_owner_only_opt_in_and_never_overrides_production() {
        let mut state = Status { owner_id: "a".into(), owner_label: "Owner".into(),
            exclusive: false, revision: 1, pending: vec![Claim { client_id: "b".into(),
                label: "Observer".into(), requested_at: "now".into() }], owner_endpoint: None };
        assert!(state.unattended_claim("a", true).is_some());
        assert!(state.unattended_claim("a", false).is_none());
        assert!(state.unattended_claim("b", true).is_none());
        state.exclusive = true;
        assert!(state.unattended_claim("a", true).is_none());
        assert!(!crate::config::AppConfig::default().allow_unattended_hardware_takeover);
    }

    #[test]
    fn remote_alternative_uses_only_matching_registered_safe_origin() {
        let mut instance = serde_json::json!({"id":"owner", "surface":"pealayer",
            "values":{"peer_origin":"http://publisher.example:8080/"}});
        assert_eq!(owner_endpoint("owner", &instance).as_deref(), Some("http://publisher.example:8080/"));
        assert!(owner_endpoint("different", &instance).is_none());
        for value in ["javascript:alert(1)", "http://user:secret@example/", "http://example/api/rpc", "http://example/?token=secret", ""] {
            instance["values"]["peer_origin"] = value.into();
            assert!(owner_endpoint("owner", &instance).is_none());
        }
    }
}
