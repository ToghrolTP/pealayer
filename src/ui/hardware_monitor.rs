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

#[derive(Clone, Copy)]
pub(crate) struct ConnectionHint<'a> {
    pub title: &'a str,
    pub copy: &'a str,
    pub detail: &'a str,
}

fn connection_hint_contents(ui: &mut egui::Ui, hint: ConnectionHint<'_>) {
    ui.set_max_width((ui.ctx().viewport_rect().width() - 40.0).clamp(120.0, 440.0));
    ui.strong(hint.title);
    ui.separator();
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            ui.add(egui::Label::new(hint.detail).wrap().selectable(true));
        });
    if ui
        .button(format!("{} {}", crate::ui::icons::COPY, hint.copy))
        .clicked()
    {
        ui.ctx().copy_text(hint.detail.to_owned());
    }
}

/// Pure, palette-adapted empty-state view. Opening it never starts a connection,
/// refreshes the board, changes authority or sends an RF/output command.
pub(crate) fn empty_card(
    ui: &mut egui::Ui,
    rtl: bool,
    title: &str,
    detail: &str,
    diagnostics: Option<ConnectionHint<'_>>,
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
            ui.horizontal_top(|ui| {
                // Allocate trailing actions first, but do not let their reverse
                // layout right-anchor a shrink-to-fit English message block.
                ui.with_layout(crate::ui::i18n::layout(!rtl, egui::Align::Min), |ui| {
                    if let Some(hint) = diagnostics {
                        let response = ui
                            .menu_button(crate::ui::icons::INFO, |ui| {
                                connection_hint_contents(ui, hint)
                            })
                            .response;
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                response.enabled(),
                                hint.title,
                            )
                        });
                        response.on_hover_ui(|ui| connection_hint_contents(ui, hint));
                    }
                    let text_width = ui.available_width().max(1.0);
                    ui.allocate_ui_with_layout(egui::vec2(text_width, 0.0),
                        crate::ui::i18n::vertical_layout(rtl), |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(detail).weak())
                                .halign(if rtl {egui::Align::Max} else {egui::Align::Min}).wrap());
                        });
                });
            });
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
            if app
                .connection_notice
                .as_ref()
                .is_some_and(|notice| !notice.trim().is_empty())
            {
                app.tr(
                    "Couldn't connect to PCController.\nCheck that it is running, then try again.",
                )
            } else {
                app.tr(if requested {
                    "Waiting for the configured PCController connection."
                } else {
                    "Connect to PCController to discover live board controls."
                })
            },
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
            if app
                .connection_notice
                .as_ref()
                .is_some_and(|notice| !notice.trim().is_empty())
            {
                app.tr("Couldn't load the hardware controls. Try refreshing the connection.")
            } else {
                app.tr("Waiting for PCController to advertise the board capabilities.")
            },
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
                app.tr("PCController is reachable, but no board is connected. Connect the board and check its USB cable, power, and port.")
            },
            programming,
            Some((app.tr("Refresh"), Action::Refresh)),
        )
    };
    // Keep original diagnostics intact, but never use them as default card copy.
    // State, not message-string matching, determines the plain-language summary.
    let diagnostic = app
        .connection_notice
        .clone()
        .filter(|notice| !notice.trim().is_empty())
        .or_else(|| {
            (app.is_connected && caps.is_some()).then(|| app.hardware_unavailable_detail(caps))
        })
        .filter(|notice| notice != &detail);
    let hint_title = app.tr("Connection details");
    let copy = app.tr("Copy");
    let preferences = app.tr("Hardware preferences");
    let response = empty_card(
        ui,
        app.rtl,
        &title,
        &detail,
        diagnostic.as_deref().map(|detail| ConnectionHint {
            title: &hint_title,
            copy: &copy,
            detail,
        }),
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

    const DIAGNOSTIC: &str = "embedded PCController unavailable (no packaged PCController library found; set PEALAYER_PCCONTROLLER_LIBRARY); external coordinator unavailable (connection timed out)";
    fn has_text(output: &egui::FullOutput, value: &str) -> bool {
        output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text)
            if text.galley.text().contains(value))
        })
    }
    fn render(
        ctx: &egui::Context,
        app: &mut crate::app::PealayerApp,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| draw_unavailable(app, ui, None),
        );
        output.textures_delta.clear();
        output
    }
    #[test]
    fn connection_copy_uses_the_leading_edge_and_an_explicit_instruction_line() {
        let english="Couldn't connect to PCController.\nCheck that it is running, then try again.";
        for rtl in [false,true] {for width in [220.0,390.0,640.0] {for scale in [1.0,1.5,2.0] {
        for dark in [false,true] {for diagnostic in [false,true] {
            let ctx=egui::Context::default();ctx.set_pixels_per_point(scale);
            ctx.set_theme(if dark {egui::Theme::Dark} else {egui::Theme::Light});
            crate::ui::i18n::configure_ui_fonts(&ctx,rtl);
            let screen=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,500.0));
            // Resolve pending DPI/viewport sizing before measuring the card.
            for _ in 0..2 {
                let mut warm=ctx.run_ui(egui::RawInput {screen_rect:Some(screen),..Default::default()},|_|{});
                warm.textures_delta.clear();
            }
            let detail=crate::ui::i18n::tr(if rtl {crate::config::AppLanguage::Persian} else {crate::config::AppLanguage::English},english);
            let mut bounds=egui::Rect::NOTHING;
            let mut output=ctx.run_ui(egui::RawInput {
                screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,500.0))),..Default::default()
            },|ui| {
                let response=empty_card(ui,rtl,"Connecting to PCController…",&detail,
                    diagnostic.then_some(ConnectionHint {title:"Connection details",copy:"Copy",detail:DIAGNOSTIC}),
                    true,None,"Hardware preferences");
                assert_eq!(response.inner,None);bounds=response.response.rect;
            });
            output.textures_delta.clear();
            let text=output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text()==detail=>Some(text),_=>None
            }).expect("Connection summary");
            assert!(text.galley.text().contains('\n'));
            let expected=if rtl {bounds.right()-17.0} else {bounds.left()+17.0};
            for row in &text.galley.rows {
                let edge=if rtl {text.pos.x+row.rect().right()} else {text.pos.x+row.rect().left()};
                assert!((edge-expected).abs()<1.1,"Wrong message edge: rtl={rtl}, width={width}, scale={scale}, diagnostic={diagnostic}, edge={edge}, expected={expected}");
            }
            if width==640.0 && !rtl {
                assert_eq!(text.galley.rows.len(),2);
                let instruction:String=text.galley.rows[1].glyphs.iter().map(|glyph|glyph.chr).collect();
                assert!(instruction.starts_with("Check that"),"Instruction must begin on its own line");
            }
            assert!(bounds.right()<=width+0.1,"Card overflow: rtl={rtl}, width={width}, scale={scale}, diagnostic={diagnostic}, bounds={bounds:?}");
            assert!(!has_text(&output,"PEALAYER_PCCONTROLLER_LIBRARY"));
        } } } } }
    }
    #[test]
    fn connection_errors_are_plain_by_default_and_the_info_control_reveals_full_details() {
        for width in [220.0, 320.0, 640.0] {
            for light in [false, true] {
                let ctx = egui::Context::default();
                ctx.set_visuals(if light {
                    egui::Visuals::light()
                } else {
                    egui::Visuals::dark()
                });
                crate::ui::i18n::configure_ui_fonts(&ctx, false);
                let mut app = crate::app::PealayerApp::default();
                app.engine_handle = crate::four_d::engine::memory_only_test_engine();
                app.is_connected = false;
                app.connection_notice = Some(DIAGNOSTIC.into());
                let _ = render(&ctx, &mut app, width, vec![]);
                let output = render(&ctx, &mut app, width, vec![]);
                assert!(has_text(&output, "Couldn't connect to PCController."));
                assert!(!has_text(&output, "PEALAYER_PCCONTROLLER_LIBRARY"));
                let icon = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == crate::ui::icons::INFO => {
                            Some(text.pos + text.galley.mesh_bounds.center().to_vec2())
                        }
                        _ => None,
                    })
                    .expect("circled information control must be visible");
                for pressed in [true, false] {
                    let _ = render(
                        &ctx,
                        &mut app,
                        width,
                        vec![
                            egui::Event::PointerMoved(icon),
                            egui::Event::PointerButton {
                                pos: icon,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::default(),
                            },
                        ],
                    );
                }
                let output = render(&ctx, &mut app, width, vec![egui::Event::PointerMoved(icon)]);
                assert!(
                    has_text(&output, "Connection details"),
                    "details menu missing at {width}"
                );
                assert!(
                    has_text(&output, "PEALAYER_PCCONTROLLER_LIBRARY"),
                    "diagnostic lost at {width}"
                );
                assert!(has_text(&output, "Copy"));
                assert_eq!(app.connection_notice.as_deref(), Some(DIAGNOSTIC));
                assert!(
                    !app.engine_handle
                        .connection_requested
                        .load(std::sync::atomic::Ordering::Relaxed)
                );
            }
        }
    }
    #[test]
    fn catalog_errors_do_not_claim_a_lost_controller_connection() {
        let ctx = egui::Context::default();
        let mut app = crate::app::PealayerApp::default();
        app.engine_handle = crate::four_d::engine::memory_only_test_engine();
        app.is_connected = true;
        app.connection_notice = Some(DIAGNOSTIC.into());
        let _ = render(&ctx, &mut app, 400.0, vec![]);
        let output = render(&ctx, &mut app, 400.0, vec![]);
        assert!(has_text(&output, "Couldn't load the hardware controls."));
        assert!(!has_text(&output, "Couldn't connect to PCController."));
        assert!(!has_text(&output, "PEALAYER_PCCONTROLLER_LIBRARY"));
    }
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
                            false,
                            "Hardware Disconnected",
                            "Connect to PCController to discover live board controls.",
                            None,
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
