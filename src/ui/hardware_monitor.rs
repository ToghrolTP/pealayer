//! Hardware Monitor presentation follows live transport AND board state. A
//! retained catalog is useful metadata, not evidence of a live connection.
use crate::four_d::{controller::HardwareCapabilities, programming::HardwareAvailability};
use eframe::egui;

pub(crate) fn live_board(connected: bool, caps: Option<&HardwareCapabilities>) -> bool {
    connected && caps.is_some_and(|caps| caps.board_connected)
}
pub(crate) fn show_rf(connected: bool, caps: Option<&HardwareCapabilities>) -> bool {
    live_board(connected, caps)
        && caps.is_some_and(|caps| caps.supports_rf_transmit && caps.outputs_available())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Connect,
    Refresh,
    Preferences,
}

/// Pure, palette-adapted empty-state view. Opening it never starts a connection,
/// refreshes the board, changes authority or sends an RF/output command.
pub(crate) fn empty_card(
    ui: &mut egui::Ui,
    title: &str,
    detail: &str,
    progress: bool,
    primary: Option<(&str, Action)>,
    preferences: &str,
) -> egui::InnerResponse<Option<Action>> {
    let width = ui.available_width().max(1.0);
    let tokens = crate::ui::craft_theme::current(ui.ctx());
    egui::Frame::new()
        .fill(tokens.panel_bg)
        .stroke(egui::Stroke::new(1.0, tokens.border_subtle))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width((width - 34.0).max(1.0));
            let mut action = None;
            let color = ui.visuals().weak_text_color();
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 8.0, ui.visuals().faint_bg_color);
                let glyph = if progress {
                    crate::ui::icons::HOURGLASS_MEDIUM
                } else {
                    crate::ui::icons::PLUG
                };
                let galley = ui.painter().layout_no_wrap(
                    glyph.into(),
                    egui::FontId::proportional(21.0),
                    color,
                );
                let ink = galley.mesh_bounds;
                ui.painter()
                    .galley(rect.center() - ink.center().to_vec2(), galley, color);
                ui.add(egui::Label::new(egui::RichText::new(title).size(17.0).strong()).wrap());
            });
            ui.add_space(8.0);
            ui.add(egui::Label::new(egui::RichText::new(detail).weak()).wrap());
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                if let Some((label, value)) = primary {
                    if ui
                        .button(format!(
                            "{} {label}",
                            if value == Action::Connect {
                                crate::ui::icons::PLUG
                            } else {
                                crate::ui::icons::ARROW_CLOCKWISE
                            }
                        ))
                        .clicked()
                    {
                        action = Some(value);
                    }
                }
                if ui
                    .button(format!("{} {preferences}", crate::ui::icons::GEAR))
                    .clicked()
                {
                    action = Some(Action::Preferences);
                }
            });
            action
        })
}

pub(crate) fn draw_unavailable(
    app: &mut crate::app::PealayerApp,
    ui: &mut egui::Ui,
    caps: Option<&HardwareCapabilities>,
) {
    let requested = app
        .engine_handle
        .connection_requested
        .load(std::sync::atomic::Ordering::Relaxed);
    let availability =
        crate::four_d::programming::hardware_availability(app.is_connected, requested, caps);
    let (title, detail, progress, primary) = if !app.is_connected {
        (
            app.tr(if requested {
                "Connecting to PCController…"
            } else {
                "Hardware Disconnected"
            }),
            app.connection_notice
                .clone()
                .filter(|notice| !notice.trim().is_empty())
                .unwrap_or_else(|| {
                    app.tr(if requested {
                        "Waiting for the configured PCController connection."
                    } else {
                        "Connect to PCController to discover live board controls."
                    })
                }),
            requested,
            (!requested && !app.serial_port.trim().is_empty())
                .then(|| (app.tr("Connect"), Action::Connect)),
        )
    } else if caps.is_none() {
        (
            app.tr(if app.connection_notice.is_some() {
                "Hardware catalog unavailable"
            } else {
                "Loading hardware…"
            }),
            app.connection_notice.clone().unwrap_or_else(|| {
                app.tr("Waiting for PCController to advertise the board capabilities.")
            }),
            app.connection_notice.is_none(),
            Some((app.tr("Refresh"), Action::Refresh)),
        )
    } else {
        let programming = matches!(
            availability,
            HardwareAvailability::Programming
                | HardwareAvailability::ProgrammingLatch
                | HardwareAvailability::ProgrammingRecovery
        );
        (
            app.tr(if programming {
                availability.label()
            } else {
                "No board connected"
            }),
            if programming {
                app.tr(
                    "Board controls are unavailable while programming or recovery is in progress.",
                )
            } else {
                app.hardware_unavailable_detail(caps)
            },
            programming,
            Some((app.tr("Refresh"), Action::Refresh)),
        )
    };
    let preferences = app.tr("Hardware preferences");
    let response = empty_card(
        ui,
        &title,
        &detail,
        progress,
        primary
            .as_ref()
            .map(|(text, action)| (text.as_str(), *action)),
        &preferences,
    );
    match response.inner {
        Some(Action::Preferences) => {
            app.preferences_tab = 2;
            crate::ui::preferences::open(app, ui.ctx());
        }
        Some(Action::Connect) => app.apply_interop_command(
            ui.ctx(),
            crate::platform::interop::InteropCommand::SetHardwareConnection { connected: true },
            "ui",
        ),
        Some(Action::Refresh) => app.apply_interop_command(
            ui.ctx(),
            crate::platform::interop::InteropCommand::RefreshHardwareCatalog,
            "ui",
        ),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disconnected_cached_board_cannot_restore_live_cards_or_rf() {
        let mut caps = HardwareCapabilities {
            board_connected: true,
            supports_rf_transmit: true,
            ..Default::default()
        };
        assert!(live_board(true, Some(&caps)));
        assert!(show_rf(true, Some(&caps)));
        assert!(!live_board(false, Some(&caps)));
        assert!(!show_rf(false, Some(&caps)));
        caps.board_connected = false;
        assert!(!show_rf(true, Some(&caps)));
        assert!(!live_board(true, Some(&caps)));
        caps.board_connected = true;
        caps.supports_rf_transmit = false;
        assert!(live_board(true, Some(&caps)));
        assert!(!show_rf(true, Some(&caps)));
        assert!(!live_board(true, None));
        assert!(!show_rf(true, None));
    }
    #[test]
    fn empty_card_wraps_within_phone_panel_and_never_dispatches() {
        for light in [false, true] {
            for width in [220.0, 320.0, 640.0] {
                let ctx = egui::Context::default();
                ctx.set_theme(if light {
                    egui::Theme::Light
                } else {
                    egui::Theme::Dark
                });
                ctx.set_visuals(if light {
                    egui::Visuals::light()
                } else {
                    egui::Visuals::dark()
                });
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 700.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let clip = ui.clip_rect();
                        let response = empty_card(
                            ui,
                            "Hardware Disconnected",
                            "Connect to PCController to discover live board controls.",
                            false,
                            Some(("Connect", Action::Connect)),
                            "Hardware preferences",
                        );
                        assert_eq!(response.inner, None);
                        assert!(
                            response.response.rect.right() <= clip.right() + 0.5,
                            "card exceeds {width}"
                        );
                    },
                );
                assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::epaint::Shape::Text(text) if text.galley.job.text.contains("Hardware Disconnected"))));
                output.textures_delta.clear();
            }
        }
    }
}
