//! Coordinator-owned power policy; this editor never keeps per-client recall.
use crate::app::PealayerApp;
use crate::four_d::controller::{HardwareCapabilities, PwmOnBehavior};
use crate::ui::dropdown::DropdownUiExt;
use eframe::egui;

pub(crate) fn configure(app: &PealayerApp, capabilities: &HardwareCapabilities,
    channel: u8, on_behavior: PwmOnBehavior) -> Result<(), String> {
    if !capabilities.board_connected || capabilities.pwm_channel_config.get(&channel)
        .and_then(|config| config.on_behavior).is_none() {
        return Err(app.tr("Shared On policy is not advertised for this channel"));
    }
    if app.hardware_programming_blocked() {
        return Err(app.tr("Programming latch active"));
    }
    app.engine_handle.queue_controller_intent(format!("pwm.{channel}:policy"),
        "controller.pwm.channel.configure",
        serde_json::json!({"channel": channel, "on_behavior": on_behavior}), true)
}

fn label(policy: PwmOnBehavior) -> &'static str {
    match policy { PwmOnBehavior::Full => "Full brightness (100%)",
        PwmOnBehavior::Restore => "Restore last brightness" }
}

pub(crate) fn draw(app: &mut PealayerApp, ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities, channel: u8) {
    let Some(config) = capabilities.pwm_channel_config.get(&channel) else { return; };
    let Some(current) = config.on_behavior else {
        ui.label(egui::RichText::new(app.tr("Shared On policy is not advertised for this channel")).weak());
        return;
    };
    let id = ui.make_persistent_id(("pwm-on-policy", &capabilities.host_instance_id,
        capabilities.connection_generation, channel));
    let mut draft = ui.data_mut(|data| data.get_temp::<(PwmOnBehavior, PwmOnBehavior)>(id))
        .filter(|draft| draft.0 == current).map(|draft| draft.1).unwrap_or(current);
    egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
        ui.strong(format!("{} {}", crate::ui::icons::SLIDERS_HORIZONTAL, app.tr("Shared On behavior")));
        ui.label(egui::RichText::new(app.tr("Applies to all connected PCController clients")).small().weak());
        crate::ui::dropdown::ComboBox::from_id_salt(id)
            .selected_text(app.tr(label(draft))).show_ui(ui, |ui| {
                for policy in [PwmOnBehavior::Full, PwmOnBehavior::Restore] {
                    ui.dropdown_value(&mut draft, policy, app.tr(label(policy)));
                }
            });
        if ui.add_enabled(draft != current && !app.hardware_programming_blocked(),
            egui::Button::new(app.tr("Apply policy"))).clicked() {
            if let Err(error) = configure(app, capabilities, channel, draft) { app.set_osd(error); }
            else { app.set_osd(app.tr("Saving shared output policy…")); }
        }
        if let Some(value) = config.remembered_percent_milli {
            ui.label(format!("{}: {:.1}%", app.tr("Remembered brightness"), f64::from(value) / 1000.0));
        } else {
            ui.label(egui::RichText::new(app.tr("Unknown brightness falls back to 100%")).weak());
        }
        ui.label(egui::RichText::new(app.tr("Brightness memory resets when PCController restarts or the board reconnects")).small().weak());
    });
    ui.data_mut(|data| data.insert_temp(id, (current, draft)));
}
