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
    let close_label = app.tr("Close");
    let dismiss_label = app.tr("Dismiss");
    let dismiss_help = app.tr("Close this prompt without changing publishing authority or pending requests.");
    let heading = app.tr("Publishing authority");
    let response = egui::Modal::new(popup_id).show(ctx,|ui| {
        ui.set_width((ctx.content_rect().width() - 64.0).clamp(180.0, 460.0));
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(egui::RichText::new(format!("{} {heading}",
                crate::ui::icons::WARNING)).heading()).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                dismiss |= ui.add_sized([32.0, 32.0], egui::Button::new(
                    egui::RichText::new("×").size(22.0)).frame(false))
                    .on_hover_text(&close_label).clicked();
            });
        });
        if owner {
            let claim=pending.as_ref().unwrap();
            ui.label(format!("{} requests control of the media clock and hardware timeline.",claim.label));
            if !app.is_paused {ui.label("Pause playback before accepting the handoff.");}
            if crate::platform::interop::allow_unattended_hardware_takeover() && !state.exclusive {
                ui.label("Unattended handoff is enabled. Playback pauses before control is transferred.");
            }
            ui.horizontal_wrapped(|ui| {
                if crate::ui::dialog::action_button(ui,crate::ui::icons::X,"Decline").clicked(){request(app,"reject",&claim.client_id);dismiss=true;}
                ui.add_enabled_ui(app.is_paused && !state.exclusive,|ui|{
                    if crate::ui::dialog::primary_action_button(ui,crate::ui::icons::CHECK,"Accept handoff").clicked(){request(app,"accept",&claim.client_id);dismiss=true;}
                });
            });
        } else {
            ui.label(format!("{} owns hardware playback. This instance is not publishing a competing timeline.",state.owner_label));
            if state.exclusive {ui.label("Production is locked. Hardware controls are read-only; E-STOP remains available.");}
            ui.horizontal_wrapped(|ui| {
                if connect_button(ui, &state) { dismiss = true; }
                ui.add_enabled_ui(!state.exclusive && !state.request_pending(&id),|ui|{
                    if crate::ui::dialog::primary_action_button(ui,crate::ui::icons::BROADCAST,"Request handoff").clicked(){request(app,"request","");dismiss=true;}
                });
            });
        }
        ui.separator();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            dismiss |= crate::ui::dialog::action_button(ui, crate::ui::icons::X, &dismiss_label)
                .on_hover_text(&dismiss_help)
                .clicked();
        });
    });
    dismiss |= response.should_close();
    if dismiss {
        ctx.data_mut(|data| data.insert_temp(popup_id, state.revision));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_authority_prompt_dismisses_without_sending_a_handoff_command() {
        use eframe::egui;
        let _guard = crate::app::tests::lock_app_tests();
        for owner in [false, true] {
            for exclusive in [false, true] {
                for action in ["Dismiss", "×", "Escape", "backdrop"] {
                    let mut app = crate::app::PealayerApp::default();
                    app.is_paused = false; // Dismiss must work even when Accept is disabled.
                    let (sender, commands) = std::sync::mpsc::channel();
                    app.engine_handle.sender = sender;
                    let state = Status {
                        owner_id: if owner { "this" } else { "other" }.into(),
                        owner_label: "Production".into(), exclusive, revision: 9,
                        pending: vec![Claim { client_id: if owner { "other" } else { "this" }.into(),
                            label: "Observer".into(), requested_at: "now".into() }],
                        owner_endpoint: None,
                    };
                    {
                        let mut plan = app.engine_handle.prepared_timeline.lock().unwrap();
                        plan.authority_client_id = "this".into();
                        plan.authority = Some(state.clone());
                    }
                    let context = egui::Context::default();
                    crate::ui::i18n::configure_ui_fonts(&context, false);
                    let render = |app: &mut crate::app::PealayerApp, events, time| {
                        context.run_ui(egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
                            events, time: Some(time), ..Default::default()
                        }, |_| draw_warning(app, &context))
                    };
                    for time in [0.0, 0.05, 0.1] {
                        let mut output = render(&mut app, vec![], time);
                        output.textures_delta.clear();
                    }
                    let mut output = render(&mut app, vec![], 0.15);
                    let text_rect = |label: &str| output.shapes.iter().find_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text) if text.galley.job.text.ends_with(label) =>
                            Some(egui::Rect::from_min_size(text.pos, text.galley.size())),
                        _ => None,
                    }).unwrap_or_else(|| panic!("Missing {label} in owner={owner}, locked={exclusive}"));
                    let close = text_rect("×");
                    assert!(close.left() > text_rect("Publishing authority").right());
                    let dismiss = text_rect("Dismiss");
                    let point = match action { "Dismiss" => dismiss.center(), "×" => close.center(),
                        _ => egui::pos2(4.0, 4.0) };
                    output.textures_delta.clear();
                    if action == "Escape" {
                        let mut output = render(&mut app, vec![egui::Event::Key {
                            key: egui::Key::Escape, physical_key: None, pressed: true,
                            repeat: false, modifiers: egui::Modifiers::NONE,
                        }], 0.2);
                        output.textures_delta.clear();
                    } else {
                        for (time, pressed) in [(0.2, true), (0.25, false)] {
                            let mut output = render(&mut app, vec![egui::Event::PointerMoved(point),
                                egui::Event::PointerButton { pos: point, button: egui::PointerButton::Primary,
                                    pressed, modifiers: egui::Modifiers::NONE }], time);
                            output.textures_delta.clear();
                        }
                    }
                    assert_eq!(context.data(|data| data.get_temp::<u64>(egui::Id::new("media_authority_conflict"))),
                        Some(9), "{action}, owner={owner}, locked={exclusive}");
                    assert_eq!(app.engine_handle.prepared_timeline.lock().unwrap().authority.as_ref(), Some(&state));
                    assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)),
                        "Dismissal must not request, reject, accept or release authority");
                    let mut output = render(&mut app, vec![], 0.3);
                    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::epaint::Shape::Text(text) if text.galley.job.text.contains("Publishing authority"))));
                    output.textures_delta.clear();
                    app.engine_handle.prepared_timeline.lock().unwrap().authority.as_mut().unwrap().revision = 10;
                    for time in [0.35, 0.4, 0.45] {
                        let mut output = render(&mut app, vec![], time);
                        if time == 0.45 {
                            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                                egui::epaint::Shape::Text(text) if text.galley.job.text.contains("Publishing authority"))));
                        }
                        output.textures_delta.clear();
                    }
                }
            }
        }
    }
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
