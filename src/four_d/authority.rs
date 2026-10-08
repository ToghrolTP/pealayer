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
}
impl Status {
    pub fn may_publish(&self, id: &str) -> bool {
        !self.owner_id.is_empty() && self.owner_id == id
    }
    pub fn request_pending(&self, id: &str) -> bool {
        self.pending.iter().any(|claim| claim.client_id == id)
    }
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
        };
        assert!(state.may_publish("a"));
        assert!(!state.may_publish("b"));
        assert!(serde_json::from_str::<Status>(r#"{"owner_id":"a"}"#).is_err());
    }
}
