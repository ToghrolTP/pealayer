//! Shared Simple/NLE action layout. Sliders are not drag handles; customizing
//! never invokes transport, and painting only queues a narrow preference edit.
use super::dropdown::DropdownUiExt;
use crate::app::{MediaTrackType, PealayerApp};
use crate::config::{ControlBarAction as Action, ControlBarSurface as Surface};
use eframe::egui::{self, Id};

#[derive(Clone, Debug)]
struct Drag {
    action: Action,
    owner: Id,
    grab_offset: egui::Vec2,
}
pub(crate) fn label(action: Action) -> &'static str {
    match action {
        Action::PlayPause => "Play / Pause",
        Action::Stop => "Stop",
        Action::Back => "Back",
        Action::Forward => "Forward",
        Action::PreviousChapter => "Previous chapter",
        Action::NextChapter => "Next chapter",
        Action::VideoTrack => "Video Track",
        Action::AudioTrack => "Audio Track",
        Action::SubtitleTrack => "Subtitle Track",
        Action::Fullscreen => "Fullscreen",
        Action::Pin => "Pin controls",
        Action::AudioSettings => "Audio Settings...",
        Action::SubtitleSettings => "Subtitle Settings...",
        Action::Mute => "Mute",
        Action::Preferences => "Preferences...",
    }
}
fn kind(action: Action) -> Option<MediaTrackType> {
    match action {
        Action::VideoTrack => Some(MediaTrackType::Video),
        Action::AudioTrack => Some(MediaTrackType::Audio),
        Action::SubtitleTrack => Some(MediaTrackType::Subtitle),
        _ => None,
    }
}
fn supported(action: Action, surface: Surface) -> bool {
    action != Action::Pin || surface == Surface::Simple
}
fn enabled(app: &PealayerApp, action: Action) -> bool {
    match action {
        Action::Pin
        | Action::Preferences
        | Action::Fullscreen
        | Action::AudioSettings
        | Action::SubtitleSettings => true,
        Action::PreviousChapter | Action::NextChapter => {
            app.current_video_path.is_some() && !app.media_chapters().is_empty() && app.is_seekable
        }
        Action::Back | Action::Forward => {
            app.current_video_path.is_some() && (app.is_paused || app.is_seekable)
        }
        _ => app.current_video_path.is_some(),
    }
}
fn tooltip(app: &PealayerApp, action: Action) -> String {
    match action {
        Action::PlayPause => app.tr(if app.is_playback_finished() {
            "Replay"
        } else if app.is_paused {
            "Play"
        } else {
            "Pause"
        }),
        Action::Mute => format!(
            "{} (M)",
            app.tr(if app.is_muted { "Unmute" } else { "Mute" })
        ),
        Action::Pin => app.tr(if app.pin_controls {
            "Unpin Controls"
        } else {
            "Pin Controls"
        }),
        Action::Fullscreen => format!("{} (F)", app.tr("Fullscreen")),
        Action::Back | Action::Forward => format!(
            "{} {} {}",
            app.tr(label(action)),
            if app.is_paused {
                app.frame_step_count.to_string()
            } else {
                super::controls::compact_number(app.quick_seek_seconds)
            },
            app.tr(if app.is_paused { "frames" } else { "seconds" })
        ),
        _ => app.tr(label(action)),
    }
}
fn caption(app: &PealayerApp, item: Action) -> String {
    use super::icons::*;
    match item {
        Action::VideoTrack | Action::AudioTrack | Action::SubtitleTrack => {
            return super::media_tracks::menu_button_label(kind(item).unwrap());
        }
        Action::PlayPause => {
            if app.is_playback_finished() {
                ARROW_COUNTER_CLOCKWISE
            } else if app.is_paused {
                PLAY
            } else {
                PAUSE
            }
        }
        Action::Stop => STOP,
        Action::Back => {
            if app.is_paused {
                SKIP_BACK
            } else {
                REWIND
            }
        }
        Action::Forward => {
            if app.is_paused {
                SKIP_FORWARD
            } else {
                FAST_FORWARD
            }
        }
        Action::PreviousChapter => SKIP_BACK,
        Action::NextChapter => SKIP_FORWARD,
        Action::Fullscreen => ARROWS_OUT,
        Action::Pin => {
            if app.pin_controls {
                PUSH_PIN_SLASH
            } else {
                PUSH_PIN
            }
        }
        Action::AudioSettings => MUSIC_NOTE,
        Action::SubtitleSettings => SUBTITLES,
        Action::Mute => {
            if app.is_muted {
                SPEAKER_SLASH
            } else {
                SPEAKER_HIGH
            }
        }
        Action::Preferences => GEAR,
    }
    .into()
}
fn invoke(app: &mut PealayerApp, ctx: &egui::Context, action: Action) {
    use crate::platform::interop::InteropCommand;
    match action {
        Action::PlayPause => app.toggle_playback(),
        Action::Stop => app.apply_interop_command(ctx, InteropCommand::Stop, "Control bar"),
        Action::Back | Action::Forward => app.apply_interop_command(
            ctx,
            InteropCommand::TransportNudge {
                direction: if action == Action::Back { -1 } else { 1 },
            },
            "Control bar",
        ),
        Action::PreviousChapter => {
            app.apply_interop_command(ctx, InteropCommand::PreviousChapter, "Control bar")
        }
        Action::NextChapter => {
            app.apply_interop_command(ctx, InteropCommand::NextChapter, "Control bar")
        }
        Action::Fullscreen => app.toggle_fullscreen(ctx),
        Action::Pin => {
            app.pin_controls = !app.pin_controls;
            app.save_config();
        }
        Action::AudioSettings => app.show_audio_settings = !app.show_audio_settings,
        Action::SubtitleSettings => app.show_sub_settings = !app.show_sub_settings,
        Action::Mute => app.apply_interop_command(ctx, InteropCommand::ToggleMute, "Control bar"),
        Action::Preferences => super::preferences::open(app, ctx),
        _ => {}
    }
}
pub(crate) fn width(ui: &egui::Ui, action: Action) -> f32 {
    let height = super::control_density::height(ui);
    if matches!(action, Action::Back | Action::Forward) {
        height + 20.0
    } else if kind(action).is_some() {
        height + 14.0
    } else {
        height
    }
}
pub(crate) fn group_width(app: &PealayerApp, ui: &egui::Ui, surface: Surface, end: bool) -> f32 {
    let layout = app.control_bars.layout(surface);
    let actions: Vec<_> = layout
        .ordered()
        .into_iter()
        .filter(|a| {
            supported(*a, surface) && !layout.hidden[*a as usize] && layout.end[*a as usize] == end
        })
        .collect();
    (actions.iter().map(|a| width(ui, *a)).sum::<f32>()
        + actions.len().saturating_sub(1) as f32 * ui.spacing().item_spacing.x
        + if end {
            super::control_density::height(ui)
                + if actions.is_empty() {
                    0.0
                } else {
                    ui.spacing().item_spacing.x
                }
        } else {
            0.0
        })
    .max(super::control_density::height(ui))
}
/// Computes visible action groups versus overflowing actions for customizable control bars.
///
/// Uses `craft_ui::layout::fit_toolbar` to calculate visible vs overflowing actions.
#[allow(dead_code)]
pub(crate) fn compute_toolbar_fit(
    natural_widths: &[f32],
    available: f32,
    gap: f32,
    overflow_width: f32,
) -> craft_ui::layout::ToolbarFit {
    craft_ui::layout::fit_toolbar(natural_widths, available, gap, overflow_width)
        .unwrap_or_else(|| craft_ui::layout::ToolbarFit {
            shown: Vec::new(),
            overflow: (0..natural_widths.len()).collect(),
            overflow_width,
        })
}
fn menu(app: &mut PealayerApp, ui: &mut egui::Ui, surface: Surface, action: Option<Action>) {
    let mut layout = app.control_bars.layout(surface);
    let mut changed = false;
    if let Some(action) = action {
        ui.strong(format!(
            "{} {}",
            caption(app, action),
            app.tr(label(action))
        ));
        if matches!(
            action,
            Action::Back | Action::Forward | Action::PlayPause | Action::Stop
        ) {
            super::controls::transport_context_menu(app, ui);
            ui.separator();
        }
        for (end, label) in [(false, "Place at start"), (true, "Place at end")] {
            if ui
                .dropdown_choice(layout.end[action as usize] == end, app.tr(label))
                .clicked()
            {
                layout.end[action as usize] = end;
                changed = true;
                ui.close();
            }
        }
        let order: Vec<_> = layout
            .ordered()
            .into_iter()
            .filter(|a| {
                supported(*a, surface)
                    && !layout.hidden[*a as usize]
                    && layout.end[*a as usize] == layout.end[action as usize]
            })
            .collect();
        let index = order.iter().position(|a| *a == action).unwrap();
        for (earlier, title) in [(true, "Move earlier"), (false, "Move later")] {
            let target = if earlier {
                index.checked_sub(1)
            } else {
                (index + 1 < order.len()).then_some(index + 1)
            };
            if ui
                .add_enabled_ui(target.is_some(), |ui| {
                    ui.dropdown_choice(false, app.tr(title))
                })
                .inner
                .clicked()
                && let Some(target) = target
            {
                changed |= layout.move_to(action, order[target], !earlier);
                ui.close();
            }
        }
        ui.separator();
        if ui
            .dropdown_choice(
                false,
                format!(
                    "{} {} {} {}",
                    super::icons::EYE_SLASH,
                    app.tr("Hide"),
                    caption(app, action),
                    app.tr(label(action))
                ),
            )
            .clicked()
        {
            layout.hidden[action as usize] = true;
            changed = true;
            ui.close();
        }
        ui.separator();
    }
    ui.menu_button(app.tr("Show / hide actions"), |ui| {
        for action in layout
            .ordered()
            .into_iter()
            .filter(|a| supported(*a, surface))
        {
            let visible = !layout.hidden[action as usize];
            if ui
                .dropdown_choice(
                    visible,
                    format!("{} {}", caption(app, action), app.tr(label(action))),
                )
                .clicked()
            {
                layout.hidden[action as usize] = visible;
                changed = true;
            }
        }
    });
    if ui.dropdown_choice(false, app.tr("Reset toolbar")).clicked() {
        layout = crate::config::ControlBarLayout::defaults(surface);
        changed = true;
        ui.close();
    }
    if changed {
        *app.control_bars.layout_mut(surface) = layout;
        app.request_control_bar_save(ui.ctx());
    }
}
pub(crate) fn draw_group(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    surface: Surface,
    end: bool,
    owner: Id,
) {
    let height = super::control_density::height(ui);
    let gap = ui.spacing().item_spacing.x;
    let desired = group_width(app, ui, surface, end).max(height);
    let available = if surface == Surface::Simple {
        ui.available_width()
            .min(ui.ctx().content_rect().width() * 0.42)
    } else {
        ui.available_width()
    };
    let total = desired.min(available.max(height));
    let direction = if app.rtl {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    let area = ui.allocate_ui_with_layout(egui::vec2(total, height), direction, |ui| {
        let action_width = (total - if end { height + gap } else { 0.0 }).max(1.0);
        if group_width(app, ui, surface, end) > if end { height } else { 0.0 } {
            ui.allocate_ui_with_layout(egui::vec2(action_width, height), direction, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt(owner.with(("scroll", end)))
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .min_scrolled_width(0.0)
                    .auto_shrink([false, true])
                    .max_width(action_width)
                    .show(ui, |ui| {
                        ui.with_layout(direction, |ui| draw_actions(app, ui, surface, end, owner));
                    });
            });
        }
        if end {
            draw_overflow(app, ui, surface, owner);
        }
    });
    // Empty-side and inter-button space are useful drop targets too. A button
    // drop already consumed its typed payload, so it cannot be applied twice.
    if area
        .response
        .dnd_hover_payload::<Drag>()
        .is_some_and(|drag| drag.owner == owner)
        && let Some(drag) = super::action_customization::drop_payload::<Drag>(&area.response)
    {
        app.control_bars.layout_mut(surface).end[drag.action as usize] = end;
        app.request_control_bar_save(ui.ctx());
    }
}
fn draw_actions(app: &mut PealayerApp, ui: &mut egui::Ui, surface: Surface, end: bool, owner: Id) {
    let layout = app.control_bars.layout(surface);
    let actions = layout.ordered().into_iter().filter(|a| {
        supported(*a, surface) && !layout.hidden[*a as usize] && layout.end[*a as usize] == end
    });
    let mut requested = None;
    for action in actions {
        let id = owner.with(action);
        let w = width(ui, action);
        let h = super::control_density::height(ui);
        let active_drag = egui::DragAndDrop::payload::<Drag>(ui.ctx())
            .filter(|d| d.owner == owner && d.action == action);
        let dragging = active_drag.is_some() && ui.input(|i| i.pointer.primary_down());
        let layer = if dragging {
            egui::LayerId::new(egui::Order::Tooltip, id)
        } else {
            ui.layer_id()
        };
        let is_enabled = enabled(app, action);
        let response = ui
            .scope_builder(egui::UiBuilder::new().layer_id(layer), |ui| {
                if dragging {
                    ui.set_clip_rect(ui.ctx().content_rect());
                }
                let trash = ui
                    .ctx()
                    .data(|d| d.get_temp::<egui::Rect>(owner.with("trash-rect")));
                let removing = dragging
                    && trash
                        .zip(ui.ctx().pointer_interact_pos())
                        .is_some_and(|(rect, pos)| rect.contains(pos));
                let alpha = super::motion::hover_alpha(ui, id.with("deleted-preview"), removing);
                if alpha > 0.0 {
                    ui.set_opacity(1.0 - alpha * 0.65);
                }
                let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                let role = match action {
                    Action::PlayPause => Some(super::controls::playback_button_role(app)),
                    Action::Stop => Some("red"),
                    Action::Mute => Some("amber"),
                    _ => None,
                };
                if let Some(role) = role {
                    super::controls::contextual_transport_style(
                        ui.style_mut(),
                        app.color_palette,
                        role,
                        action == Action::PlayPause && app.is_active_playback()
                            || action == Action::Mute && app.is_muted,
                        is_enabled,
                    );
                }
                let text = if matches!(action, Action::Back | Action::Forward) {
                    String::new()
                } else {
                    caption(app, action)
                };
                let response = super::action_customization::control(
                    ui,
                    rect,
                    id,
                    &text,
                    &app.tr(label(action)),
                    is_enabled,
                    action == Action::Pin && app.pin_controls,
                    true,
                );
                if matches!(action, Action::Back | Action::Forward) {
                    super::controls::paint_transport_nudge(
                        app,
                        ui,
                        rect,
                        id,
                        if action == Action::Back { -1 } else { 1 },
                        is_enabled,
                    );
                }
                if alpha > 0.0 {
                    ui.painter().line_segment(
                        [rect.left_bottom(), rect.right_top()],
                        egui::Stroke::new(1.5, ui.visuals().error_fg_color.gamma_multiply(alpha)),
                    );
                }
                response
            })
            .inner;
        super::action_customization::register_drag(
            &response,
            super::action_customization::moved_for_drag(ui.ctx()),
            |grab_offset| Drag {
                action,
                owner,
                grab_offset,
            },
        );
        if let Some(drag) = active_drag
            && dragging
            && let Some(pointer) = ui.ctx().pointer_interact_pos()
        {
            ui.ctx().transform_layer_shapes(
                layer,
                egui::emath::TSTransform::from_translation(
                    pointer - drag.grab_offset - response.rect.min,
                ),
            );
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if response
            .dnd_hover_payload::<Drag>()
            .is_some_and(|d| d.owner == owner && d.action != action)
        {
            let after = ui
                .ctx()
                .pointer_interact_pos()
                .is_some_and(|p| (p.x > response.rect.center().x) != app.rtl);
            let x = if after != app.rtl {
                response.rect.right()
            } else {
                response.rect.left()
            };
            ui.painter().vline(
                x,
                response.rect.y_range(),
                egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
            );
            if let Some(drag) = super::action_customization::drop_payload::<Drag>(&response) {
                app.control_bars
                    .layout_mut(surface)
                    .move_to(drag.action, action, after);
                app.request_control_bar_save(ui.ctx());
            }
        }
        let has_drag = egui::DragAndDrop::has_payload_of_type::<Drag>(ui.ctx());
        if let Some(kind) = kind(action)
            && is_enabled
            && !has_drag
        {
            egui::Popup::menu(&response)
                .show(|ui| super::media_tracks::draw_track_menu(app, ui, kind));
        } else if response.clicked() && is_enabled && !has_drag {
            requested = Some(action);
        }
        response.context_menu(|ui| menu(app, ui, surface, Some(action)));
        response.on_hover_text(tooltip(app, action));
    }
    if let Some(action) = requested {
        invoke(app, ui.ctx(), action);
    }
}
fn draw_overflow(app: &mut PealayerApp, ui: &mut egui::Ui, surface: Surface, owner: Id) {
    let h = super::control_density::height(ui);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(h, h), egui::Sense::hover());
    ui.ctx()
        .data_mut(|d| d.insert_temp(owner.with("trash-rect"), rect));
    let drag = egui::DragAndDrop::payload::<Drag>(ui.ctx()).filter(|d| d.owner == owner);
    let response = super::action_customization::control(
        ui,
        rect,
        owner.with("overflow"),
        if drag.is_some() {
            super::icons::TRASH
        } else {
            super::icons::DOTS_THREE
        },
        &app.tr("Control bar"),
        true,
        false,
        false,
    );
    let remove = response
        .dnd_hover_payload::<Drag>()
        .is_some_and(|d| d.owner == owner);
    let alpha = super::motion::hover_alpha(ui, owner.with("removal-preview"), remove);
    if alpha > 0.0 {
        ui.painter().rect_filled(
            rect.shrink(1.0),
            4.0,
            ui.visuals().error_fg_color.gamma_multiply(alpha * 0.15),
        );
    }
    if remove && let Some(drag) = super::action_customization::drop_payload::<Drag>(&response) {
        app.control_bars.layout_mut(surface).hidden[drag.action as usize] = true;
        app.request_control_bar_save(ui.ctx());
    }
    if drag.is_none() {
        egui::Popup::menu(&response).show(|ui| menu(app, ui, surface, None));
    }
    response.context_menu(|ui| menu(app, ui, surface, None));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pointer(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn render(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        surface: Surface,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        draw_group(
                            app,
                            ui,
                            surface,
                            false,
                            super::super::action_customization::owner(surface),
                        )
                    });
                    ui.add_space(60.0);
                    ui.horizontal(|ui| {
                        draw_group(
                            app,
                            ui,
                            surface,
                            true,
                            super::super::action_customization::owner(surface),
                        )
                    });
                });
            },
        );
        output.textures_delta.clear();
        output
    }
    fn rect(ctx: &egui::Context, surface: Surface, action: Action) -> egui::Rect {
        ctx.read_response(super::super::action_customization::owner(surface).with(action))
            .unwrap()
            .rect
    }
    #[test]
    fn real_bars_support_disabled_drag_cross_side_reorder_and_trash_without_invocation() {
        let _lock = crate::app::tests::lock_app_tests();
        for surface in [Surface::Simple, Surface::Monitor] {
            let ctx = egui::Context::default();
            let mut app = PealayerApp::default();
            app.control_bars.layout_mut(surface).hidden[Action::Stop as usize] = false;
            render(&ctx, &mut app, surface, vec![]);
            render(&ctx, &mut app, surface, vec![]);
            let source = rect(&ctx, surface, Action::Stop);
            let press = source.min + egui::vec2(5.0, 12.0);
            assert_eq!(source.width(), source.height());
            render(
                &ctx,
                &mut app,
                surface,
                vec![
                    egui::Event::PointerMoved(press),
                    pointer(press, egui::PointerButton::Primary, true),
                ],
            );
            render(
                &ctx,
                &mut app,
                surface,
                vec![egui::Event::PointerMoved(press + egui::vec2(35.0, 25.0))],
            );
            let payload =
                egui::DragAndDrop::payload::<Drag>(&ctx).expect("disabled action is customizable");
            assert_eq!(payload.grab_offset, press - source.min);
            let target = rect(&ctx, surface, Action::Mute).center();
            render(
                &ctx,
                &mut app,
                surface,
                vec![egui::Event::PointerMoved(target)],
            );
            render(
                &ctx,
                &mut app,
                surface,
                vec![pointer(target, egui::PointerButton::Primary, false)],
            );
            assert!(
                app.control_bars.layout(surface).end[Action::Stop as usize],
                "cross-side drop"
            );
            assert!(!app.control_bars.layout(surface).hidden[Action::Stop as usize]);
            assert!(app.pending_control_bar_save.is_some());
            assert!(app.current_video_path.is_none());
            render(&ctx, &mut app, surface, vec![]);
            render(&ctx, &mut app, surface, vec![]);
            let source = rect(&ctx, surface, Action::Stop).center();
            render(
                &ctx,
                &mut app,
                surface,
                vec![
                    egui::Event::PointerMoved(source),
                    pointer(source, egui::PointerButton::Primary, true),
                ],
            );
            render(
                &ctx,
                &mut app,
                surface,
                vec![egui::Event::PointerMoved(source + egui::vec2(35.0, 20.0))],
            );
            assert!(
                egui::DragAndDrop::payload::<Drag>(&ctx).is_some(),
                "second drag source {source:?}"
            );
            let trash = ctx
                .read_response(super::super::action_customization::owner(surface).with("overflow"))
                .unwrap()
                .rect
                .center();
            render(
                &ctx,
                &mut app,
                surface,
                vec![egui::Event::PointerMoved(trash)],
            );
            assert!(
                ctx.read_response(
                    super::super::action_customization::owner(surface).with("overflow")
                )
                .unwrap()
                .dnd_hover_payload::<Drag>()
                .is_some(),
                "trash hover {trash:?}"
            );
            render(
                &ctx,
                &mut app,
                surface,
                vec![pointer(trash, egui::PointerButton::Primary, false)],
            );
            assert!(app.control_bars.layout(surface).hidden[Action::Stop as usize]);
        }
    }
    #[test]
    fn right_click_and_escape_never_create_or_restart_a_drag_or_hide() {
        let _lock = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut app = PealayerApp::default();
        let surface = Surface::Monitor;
        render(&ctx, &mut app, surface, vec![]);
        render(&ctx, &mut app, surface, vec![]);
        let source = rect(&ctx, surface, Action::Stop).center();
        render(
            &ctx,
            &mut app,
            surface,
            vec![
                egui::Event::PointerMoved(source),
                pointer(source, egui::PointerButton::Secondary, true),
            ],
        );
        render(
            &ctx,
            &mut app,
            surface,
            vec![pointer(source, egui::PointerButton::Secondary, false)],
        );
        let menu = render(&ctx, &mut app, surface, vec![]);
        assert!(egui::DragAndDrop::payload::<Drag>(&ctx).is_none());
        assert!(menu.shapes.iter().any(|shape|matches!(&shape.shape,egui::epaint::Shape::Text(text) if text.galley.job.text.contains("Show / hide actions"))));
        egui::Popup::close_all(&ctx);
        render(
            &ctx,
            &mut app,
            surface,
            vec![
                egui::Event::PointerMoved(source),
                pointer(source, egui::PointerButton::Primary, true),
            ],
        );
        render(
            &ctx,
            &mut app,
            surface,
            vec![egui::Event::PointerMoved(source + egui::vec2(40.0, 20.0))],
        );
        assert!(egui::DragAndDrop::payload::<Drag>(&ctx).is_some());
        render(
            &ctx,
            &mut app,
            surface,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        render(
            &ctx,
            &mut app,
            surface,
            vec![egui::Event::PointerMoved(source + egui::vec2(60.0, 20.0))],
        );
        assert!(egui::DragAndDrop::payload::<Drag>(&ctx).is_none());
        render(
            &ctx,
            &mut app,
            surface,
            vec![pointer(source, egui::PointerButton::Primary, false)],
        );
        assert_eq!(
            app.control_bars,
            crate::config::ControlBarsConfig::default()
        );
        assert!(app.pending_control_bar_save.is_none());
    }
    #[test]
    fn overflow_stays_square_and_reachable_when_every_action_is_hidden_or_too_wide() {
        let _lock = crate::app::tests::lock_app_tests();
        for hidden in [true, false] {
            for rtl in [false, true] {
                for density in crate::config::UiControlDensity::ALL {
                    let ctx = egui::Context::default();
                    ctx.all_styles_mut(|style| {
                        super::super::control_density::configure(style, density)
                    });
                    let mut app = PealayerApp::default();
                    app.rtl = rtl;
                    app.control_bars.monitor.hidden = [hidden; Action::COUNT];
                    app.control_bars.monitor.end = [true; Action::COUNT];
                    let mut slot = egui::Rect::NOTHING;
                    for _ in 0..2 {
                        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(100.0, super::super::control_density::height(ui)),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    slot = ui.max_rect();
                                    draw_group(
                                        &mut app,
                                        ui,
                                        Surface::Monitor,
                                        true,
                                        super::super::action_customization::owner(Surface::Monitor),
                                    );
                                    assert!(
                                        ui.min_rect().width() <= 100.5,
                                        "action strip overflowed: {:?}",
                                        ui.min_rect()
                                    );
                                },
                            );
                        });
                        output.textures_delta.clear();
                    }
                    let overflow = ctx
                        .read_response(
                            super::super::action_customization::owner(Surface::Monitor)
                                .with("overflow"),
                        )
                        .unwrap()
                        .rect;
                    assert_eq!(overflow.width(), overflow.height());
                    assert!(
                        slot.contains_rect(overflow),
                        "overflow must remain reachable {overflow:?} in {slot:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn control_bar_computes_fit_with_craft_toolbar_fit() {
        let natural = [32.0, 32.0, 32.0];
        let fit = craft_ui::layout::fit_toolbar(&natural, 80.0, 4.0, 32.0);
        assert!(fit.is_some());
        let fit = fit.unwrap();
        assert_eq!(fit.shown.len(), 1);
        assert_eq!(fit.overflow.len(), 2);

        let computed = compute_toolbar_fit(&natural, 80.0, 4.0, 32.0);
        assert_eq!(computed.shown.len(), 1);
        assert_eq!(computed.overflow.len(), 2);
        assert_eq!(computed.overflow_width, 32.0);

        let tiny = compute_toolbar_fit(&natural, 20.0, 4.0, 32.0);
        assert!(tiny.shown.is_empty());
        assert_eq!(tiny.overflow.len(), 3);
        assert_eq!(tiny.overflow_width, 32.0);

        let exact = compute_toolbar_fit(&[20.0, 20.0], 50.0, 4.0, 20.0);
        assert_eq!(exact.shown.len(), 2);
        assert!(exact.overflow.is_empty());
    }
}
