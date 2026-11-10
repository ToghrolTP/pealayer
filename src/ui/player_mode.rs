//! Shared native mode menu: both workspace adapters use the public command.
use crate::{app::PealayerApp, mpv::external::Mode, platform::interop::InteropCommand};
use eframe::egui;

fn request(ctx: &egui::Context, mode: Mode) {
    // Apply next turn, after the workspace adapter returns its borrowed layout.
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("requested-player-mode"), mode));
    ctx.request_repaint();
}
pub fn take_request(ctx: &egui::Context) -> Option<InteropCommand> {
    let mode =
        ctx.data_mut(|data| data.remove_temp::<Mode>(egui::Id::new("requested-player-mode")));
    mode.map(|mode| InteropCommand::SetPlayerMode { mode })
}

pub fn rich_choice(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    title: &str,
    description: &str,
    width: f32,
) -> egui::Response {
    let width = width.min(ui.available_width()).max(1.0);
    let padding = ui.spacing().button_padding;
    let mut job = egui::text::LayoutJob::default();
    let font = egui::TextStyle::Body.resolve(ui.style());
    job.append(
        &format!("{icon}  {title}\n"),
        0.0,
        egui::TextFormat {
            font_id: font.clone(),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    job.append(
        description,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new((font.size - 1.0).max(10.0), font.family),
            color: ui.visuals().weak_text_color(),
            ..Default::default()
        },
    );
    job.wrap.max_width = (width - padding.x * 2.0).max(1.0);
    let galley = ui.painter().layout_job(job);
    let height = (galley.size().y + padding.y * 2.0).max(ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let x = if ui.layout().prefer_right_to_left() {
        rect.right() - padding.x - galley.size().x
    } else {
        rect.left() + padding.x
    };
    ui.painter().galley(
        egui::pos2(x, rect.center().y - galley.size().y * 0.5),
        galley,
        visuals.text_color(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::RadioButton, ui.is_enabled(), selected, title)
    });
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let response = crate::ui::dropdown::register_choice(ui, response, selected);
    if response.clicked() {
        ui.close();
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mode_changes_wait_until_workspace_paint_has_returned() {
        let ctx = egui::Context::default();
        request(&ctx, Mode::Dual);
        request(&ctx, Mode::External);
        assert!(matches!(
            take_request(&ctx),
            Some(InteropCommand::SetPlayerMode {
                mode: Mode::External
            })
        ));
        assert!(
            take_request(&ctx).is_none(),
            "one coalesced command, not a per-frame restart"
        );
    }
    #[test]
    fn multiline_choices_fit_phone_tablet_desktop_and_accept_real_clicks() {
        for width in [240.0, 600.0, 1200.0] {
            for rtl in [false, true] {
                for height in [18.0, 36.0] {
                    let ctx = egui::Context::default();
                    let mut clicked = false;
                    let mut render = |events| {
                        let mut target = egui::Rect::NOTHING;
                        ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 300.0))), events, ..Default::default() }, |ui| {
                            ui.spacing_mut().interact_size.y = height;
                            ui.with_layout(if rtl { egui::Layout::right_to_left(egui::Align::Min) } else { egui::Layout::left_to_right(egui::Align::Min) }, |ui| {
                                let response = rich_choice(ui, true, super::super::icons::MONITOR_PLAY, "Both players",
                                    "External playback with a muted internal preview. Falls back to internal when mpv closes.", (width - 24.0).min(328.0));
                                assert!(response.rect.width() <= width);
                                assert!(response.rect.height() > height, "description needs its own line, not clipping");
                                target = response.rect; clicked |= response.clicked();
                            });
                        }).drop_without_applying_deltas();
                        target
                    };
                    let position = render(Vec::new()).center();
                    render(vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]);
                    render(vec![egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    }]);
                    assert!(clicked);
                }
            }
        }
    }
}

pub fn menu(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let status = crate::mpv::external::status();
    let configured = crate::platform::interop::external_mpv_settings().mode;
    let effective = if configured == Mode::Dual && status.closed {
        Mode::Internal
    } else {
        configured
    };
    if !status.connected && (configured == Mode::Internal || status.closed) {
        if ui
            .button(format!(
                "{}  {}",
                super::icons::ARROW_SQUARE_OUT,
                app.tr("Launch external player")
            ))
            .clicked()
        {
            request(
                ui.ctx(),
                if configured == Mode::External {
                    Mode::External
                } else {
                    Mode::Dual
                },
            );
            ui.close();
        }
    }
    ui.menu_button(
        format!("{}  {}", super::icons::MONITOR_PLAY, app.tr("Player mode")),
        |ui| {
            let controls = crate::preferences_contract::preference_controls(
                &crate::platform::interop::get_live_config(),
            );
            let control = controls
                .iter()
                .find(|control| control.key == "external_mpv.mode")
                .unwrap();
            let width = (ui.ctx().content_rect().width() - 32.0).min(328.0).max(1.0);
            for option in &control.options {
                let mode: Mode = serde_json::from_value(option.value.clone()).unwrap();
                if mode == Mode::Remote {
                    continue;
                } // Existing-player attachment stays in Preferences.
                if rich_choice(
                    ui,
                    mode == effective && (effective == Mode::Internal || !status.closed),
                    super::icons::named_control_icon(option.icon.unwrap_or("monitor-play"))
                        .unwrap_or(super::icons::MONITOR_PLAY),
                    &app.tr(option.label),
                    &crate::ui::i18n::visual_text(
                        app.language,
                        option.description.as_deref().unwrap_or_default(),
                    ),
                    width,
                )
                .clicked()
                {
                    request(ui.ctx(), mode);
                }
            }
        },
    );
}
