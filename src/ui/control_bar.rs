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
fn layout_label(surface: Surface, action: Action) -> &'static str {
    if surface == Surface::Monitor && action == Action::Mute { "Volume" } else { label(action) }
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
pub(crate) fn width(ui: &egui::Ui, action: Action, surface: Surface) -> f32 {
    let height = super::control_density::height(ui);
    if surface == Surface::Monitor && action == Action::Mute {
        height + ui.spacing().item_spacing.x + 160.0
    } else if matches!(action, Action::Back | Action::Forward) {
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
    (actions.iter().map(|a| width(ui, *a, surface)).sum::<f32>()
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
fn menu(app: &mut PealayerApp, ui: &mut egui::Ui, surface: Surface, action: Option<Action>, owner: Id) {
    let mut layout = app.control_bars.layout(surface);
    let mut changed = false;
    if action.is_none() {
        let mut overflow = Vec::new();
        for end in [false, true] {
            for item in ui.data(|data| data.get_temp::<Vec<Action>>(owner.with(("overflow-actions",end)))).unwrap_or_default() {
                if !overflow.contains(&item) { overflow.push(item); }
            }
        }
        for item in &overflow {
            let label = format!("{}  {}", caption(app,*item), tooltip(app,*item));
            ui.add_enabled_ui(enabled(app,*item), |ui| {
                if *item == Action::Mute && surface == Surface::Monitor {
                    ui.allocate_ui_with_layout(egui::vec2(200.0, super::control_density::height(ui)),
                        egui::Layout::left_to_right(egui::Align::Center), |ui| super::controls::draw_volume_strip(app, ui));
                } else if let Some(kind) = kind(*item) {
                    ui.menu_button(label, |ui| super::media_tracks::draw_track_menu(app,ui,kind));
                } else if ui.button(label).clicked() { invoke(app,ui.ctx(),*item); ui.close(); }
            });
        }
        if !overflow.is_empty() { ui.separator(); }
    }
    if let Some(action) = action {
        ui.strong(format!(
            "{} {}",
            caption(app, action),
            app.tr(layout_label(surface, action))
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
                    app.tr(layout_label(surface, action))
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
                    format!("{} {}", caption(app, action), app.tr(layout_label(surface, action))),
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
        ui.set_min_size(egui::vec2(total,height));
        let action_width = (total - if end { height + gap } else { 0.0 }).max(1.0);
        let actions: Vec<_> = app.control_bars.layout(surface).ordered().into_iter().filter(|action| {
            let layout=app.control_bars.layout(surface);
            supported(*action,surface) && !layout.hidden[*action as usize] && layout.end[*action as usize]==end
        }).collect();
        // The shared trailing overflow button is already reserved above. Fit
        // the inline prefix without reserving a second, invisible menu slot.
        let terminal = surface == Surface::Monitor && end && actions.last() == Some(&Action::Fullscreen)
            && width(ui, Action::Fullscreen, surface) <= action_width;
        let prefix = &actions[..actions.len() - usize::from(terminal)];
        let prefix_width = action_width - if terminal {height + gap} else {0.0};
        let mut used=0.0;
        let count=prefix.iter().take_while(|action| {
            let next=width(ui,**action,surface)+if used>0.0 {gap} else {0.0};
            if used+next<=prefix_width+0.01 {used+=next;true} else {false}
        }).count();
        let mut shown: Vec<_> = prefix[..count].to_vec();
        if terminal {shown.push(Action::Fullscreen);}
        let overflow: Vec<_> = if surface==Surface::Monitor {prefix[count..].to_vec()} else {Vec::new()};
        ui.data_mut(|data|data.insert_temp(owner.with(("overflow-actions",end)),overflow));
        // Program Monitor's terminal action stays at the outer edge; the
        // customization/overflow control precedes the trailing action group.
        if end && surface == Surface::Monitor { draw_overflow(app, ui, surface, owner); }
        if group_width(app, ui, surface, end) > if end { height } else { 0.0 } {
            ui.allocate_ui_with_layout(egui::vec2(action_width, height), direction, |ui| {
                ui.set_min_size(egui::vec2(action_width,height));
                if surface == Surface::Monitor {
                    draw_actions(app,ui,surface,end,owner,Some(&shown));
                    return;
                }
                egui::ScrollArea::horizontal()
                    .id_salt(owner.with(("scroll", end)))
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .min_scrolled_width(0.0)
                    .auto_shrink([false, true])
                    .max_width(action_width)
                    .show(ui, |ui| {
                        ui.with_layout(direction, |ui| draw_actions(app, ui, surface, end, owner,None));
                    });
            });
        }
        if end && surface != Surface::Monitor {
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
fn idle_backgrounds_id() -> Id {
    Id::unique("transport-idle-backgrounds")
}

pub(super) fn configure_idle_backgrounds(ctx: &egui::Context, show: bool) {
    ctx.data_mut(|data| data.insert_temp(idle_backgrounds_id(), show));
}

fn draw_actions(app: &mut PealayerApp, ui: &mut egui::Ui, surface: Surface, end: bool, owner: Id, shown: Option<&[Action]>) {
    let layout = app.control_bars.layout(surface);
    let actions: Vec<_> = layout.ordered().into_iter().filter(|a| {
        supported(*a, surface) && !layout.hidden[*a as usize] && layout.end[*a as usize] == end
            && shown.is_none_or(|shown| shown.contains(a))
    }).collect();
    let idle_backgrounds = ui.data(|data| data.get_temp(idle_backgrounds_id())).unwrap_or(true);
    let terminal = surface == Surface::Monitor && end && actions.last() == Some(&Action::Fullscreen);
    let mut requested = None;
    for action in actions {
        let id = owner.with(action);
        let w = width(ui, action, surface);
        let h = super::control_density::height(ui);
        if terminal && action == Action::Fullscreen && ui.available_width() > w {
            ui.add_space(ui.available_width() - w);
        }
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
                let text = if matches!(action, Action::Back | Action::Forward) {
                    String::new()
                } else {
                    caption(app, action)
                };
                let volume = surface == Surface::Monitor && action == Action::Mute;
                let button_rect = if volume {
                    egui::Rect::from_min_size(egui::pos2(if app.rtl {rect.right() - h} else {rect.left()}, rect.top()), egui::vec2(h,h))
                } else { rect };
                let label = app.tr(label(action));
                let response = if role.is_some() || matches!(action, Action::Back | Action::Forward
                    | Action::PreviousChapter | Action::NextChapter) {
                    let intent = role.map(|role| super::palette::color(app.color_palette, ui.visuals().dark_mode, role));
                    let active = action == Action::PlayPause && app.is_active_playback()
                        || action == Action::Mute && app.is_muted;
                    super::action_customization::transport_control(ui, button_rect, id, &text, &label,
                        is_enabled, super::action_customization::TransportPaint { intent, active, idle_backgrounds })
                } else {
                    super::action_customization::control(ui, button_rect, id, &text, &label,
                        is_enabled, action == Action::Pin && app.pin_controls, true)
                };
                if volume {
                    let slider_rect = egui::Rect::from_min_max(
                        egui::pos2(if app.rtl {rect.left()} else {button_rect.right() + ui.spacing().item_spacing.x},rect.top()),
                        egui::pos2(if app.rtl {button_rect.left() - ui.spacing().item_spacing.x} else {rect.right()},rect.bottom()));
                    let mut slider = ui.new_child(egui::UiBuilder::new().id_salt(id.with("volume-slider"))
                        .max_rect(slider_rect).layout(if app.rtl {egui::Layout::right_to_left(egui::Align::Center)} else {egui::Layout::left_to_right(egui::Align::Center)}));
                    slider.set_clip_rect(slider_rect.intersect(ui.clip_rect()));
                    #[cfg(test)]
                    ui.data_mut(|data| data.insert_temp(id.with("volume-slider-bounds"), slider_rect));
                    slider.add_enabled_ui(is_enabled, |ui| super::controls::draw_volume_slider(app, ui));
                }
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
                // The secondary-click customization menu has its own lifetime.
                // Sharing the default ID makes it close this menu on left click.
                .id(response.id.with("media-tracks"))
                .show(|ui| super::media_tracks::draw_track_menu(app, ui, kind));
        } else if response.clicked() && is_enabled && !has_drag {
            requested = Some(action);
        }
        response.context_menu(|ui| menu(app, ui, surface, Some(action),owner));
        response.on_hover_text(if surface == Surface::Monitor && action == Action::Mute {
            format!("{}\n{}", tooltip(app,action), app.tr("Drag to move volume"))
        } else { tooltip(app, action) });
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
        egui::Popup::menu(&response).id(response.id.with("control-bar-menu"))
            .show(|ui| menu(app, ui, surface, None,owner));
    }
    response.context_menu(|ui| menu(app, ui, surface, None,owner));
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
    fn transport_paint(
        ctx: &egui::Context,
        output: &egui::FullOutput,
        surface: Surface,
        action: Action,
        glyph: &str,
    ) -> (egui::Rect, egui::Color32, egui::Color32, egui::Color32) {
        let id = super::super::action_customization::owner(surface).with(action);
        let response = ctx.read_response(id).expect("transport action is registered");
        assert_eq!(response.rect, response.interact_rect, "paint uses the full allocated hit target");
        assert!(response.sense.senses_click() && response.sense.senses_drag());
        let frame = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(frame) if frame.rect == response.rect.shrink(0.5) => {
                assert!(shape.clip_rect.contains_rect(frame.rect));
                assert_eq!(frame.stroke.width, 1.0);
                assert_eq!(frame.stroke_kind, egui::StrokeKind::Inside);
                Some(frame)
            }
            _ => None,
        }).expect("transport frame is painted");
        let ink = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == glyph
                && response.rect.contains(text.pos + text.galley.rect.center().to_vec2()) => {
                let ink = text.galley.mesh_bounds.translate(text.pos.to_vec2());
                assert!(response.rect.expand(0.5).contains_rect(ink), "glyph exceeds hit target: {ink:?}");
                Some(text.override_text_color.unwrap_or(text.fallback_color))
            }
            _ => None,
        }).expect("transport glyph is painted");
        (response.rect, ink, frame.stroke.color, frame.fill)
    }

    #[test]
    fn native_transport_paints_semantic_frames_without_changing_density_hit_targets() {
        use crate::config::{AppConfig, ColorPalette, UiAnimationMode, UiControlDensity};
        let _lock = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.color_palette = ColorPalette::Studio;
        app.duration = 12.0;
        app.playback_time = 3.125;
        app.is_seekable = true;
        for surface in [Surface::Monitor, Surface::Simple] {
            // Stop is an opt-in action in Simple.
            app.control_bars.layout_mut(surface).hidden[Action::Stop as usize] = false;
            for density in [UiControlDensity::ExtraCompact, UiControlDensity::Compact, UiControlDensity::Regular] {
                for button_text in [None, Some(26.0)] {
                    for rtl in [false, true] {
                        app.rtl = rtl;
                        app.current_video_path = Some("transport-fixture.mkv".into());
                        app.is_paused = true;
                        app.is_eof = false;
                        app.is_muted = false;
                        let ctx = egui::Context::default();
                        ctx.set_theme(egui::Theme::Dark);
                        crate::ui::configure_native_visuals(&ctx, &AppConfig {
                            color_palette: ColorPalette::Studio,
                            ui_control_density: density,
                            ui_animation_mode: UiAnimationMode::Disabled,
                            ..AppConfig::default()
                        });
                        if let Some(size) = button_text {
                            ctx.all_styles_mut(|style| style.text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = size);
                        }
                        render(&ctx, &mut app, surface, vec![]);
                        let idle = render(&ctx, &mut app, surface, vec![]);
                        let (bounds, idle_ink, idle_stroke, idle_fill) =
                            transport_paint(&ctx, &idle, surface, Action::PlayPause, &caption(&app, Action::PlayPause));
                        let tokens = crate::ui::craft_theme::current(&ctx);
                        let green = crate::ui::palette::color(app.color_palette, true, "green");
                        let amber = crate::ui::palette::color(app.color_palette, true, "amber");
                        let red = crate::ui::palette::color(app.color_palette, true, "red");
                        assert_ne!(idle_ink, green);
                        assert_eq!(idle_stroke, tokens.field_border);
                        assert_eq!(idle_fill, tokens.panel_bg.lerp_to_gamma(tokens.hover, 0.28));
                        assert!((bounds.width() - bounds.height()).abs() <= 0.01);
                        assert!(bounds.height() >= density.base_height());
                        if button_text.is_none() {
                            eprintln!("{surface:?} {density:?} rtl={rtl}: {}", bounds.height());
                        } else {
                            assert!(bounds.height() >= 26.0);
                            eprintln!("{surface:?} {density:?} rtl={rtl} large text: {}", bounds.height());
                        }
                        let assert_intent = |output: &egui::FullOutput, app: &PealayerApp,
                            action, expected_bounds, color, tint| {
                            let actual = transport_paint(&ctx, output, surface, action, &caption(app, action));
                            assert_eq!(actual, (expected_bounds, color,
                                tokens.panel_bg.lerp_to_gamma(color, 0.68), tokens.panel_bg.lerp_to_gamma(color, tint)));
                        };

                        let play = bounds.center();
                        let hover = render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(play)]);
                        assert_intent(&hover, &app, Action::PlayPause, bounds, green, 0.17);
                        let press = render(&ctx, &mut app, surface, vec![pointer(play, egui::PointerButton::Primary, true)]);
                        assert_intent(&press, &app, Action::PlayPause, bounds, green, 0.24);
                        // Cancel the activation without dispatching playback.
                        let outside = egui::pos2(799.0, 599.0);
                        render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(outside),
                            pointer(outside, egui::PointerButton::Primary, false)]);
                        let id = super::super::action_customization::owner(surface).with(Action::PlayPause);
                        ctx.memory_mut(|mem| mem.request_focus(id));
                        let focus = render(&ctx, &mut app, surface, vec![]);
                        assert_intent(&focus, &app, Action::PlayPause, bounds, green, 0.12);
                        assert!(focus.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(r)
                            if r.rect == bounds.shrink(2.5) && r.stroke.color == tokens.focus)));
                        ctx.memory_mut(|mem| mem.surrender_focus(id));

                        app.is_paused = false;
                        app.is_muted = true;
                        let active = render(&ctx, &mut app, surface, vec![]);
                        assert_intent(&active, &app, Action::PlayPause, bounds, amber, 0.12);
                        let mute = rect(&ctx, surface, Action::Mute);
                        assert_intent(&active, &app, Action::Mute, mute, amber, 0.12);
                        let stop = rect(&ctx, surface, Action::Stop);
                        let hover_stop = render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(stop.center())]);
                        assert_intent(&hover_stop, &app, Action::Stop, stop, red, 0.17);

                        app.is_paused = true;
                        app.is_eof = true;
                        let replay = render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(play)]);
                        assert_intent(&replay, &app, Action::PlayPause, bounds, green, 0.17);

                        app.current_video_path = None;
                        let disabled = render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(stop.center())]);
                        let paint = transport_paint(&ctx, &disabled, surface, Action::Stop, &caption(&app, Action::Stop));
                        assert_eq!(paint.0, stop);
                        assert_eq!(paint.2, tokens.field_border.gamma_multiply(0.60));
                        assert_eq!(paint.3, idle_fill.gamma_multiply(0.55));
                        assert_ne!(paint.1, red);

                        app.current_video_path = Some("transport-fixture.mkv".into());
                        app.is_eof = false;
                        configure_idle_backgrounds(&ctx, false);
                        let transparent = render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(outside)]);
                        let paint = transport_paint(&ctx, &transparent, surface, Action::PlayPause, &caption(&app, Action::PlayPause));
                        assert_eq!(paint, (bounds, idle_ink, idle_stroke, egui::Color32::TRANSPARENT));
                    }
                }
            }
        }
    }

    #[test]
    fn monitor_volume_moves_with_speaker_but_slider_gestures_do_not_reorder() {
        let _lock = crate::app::tests::lock_app_tests();
        for presentation in [crate::config::NativeMonitorStyle::Compact,crate::config::NativeMonitorStyle::FilmCraft] {
        for rtl in [false, true] {
            let ctx = egui::Context::default();
            let mut app = PealayerApp::default();
            app.rtl = rtl;
            app.native_monitor_style=presentation;
            let simple = app.control_bars.simple;
            let surface = Surface::Monitor;
            let owner = super::super::action_customization::owner(surface);
            render(&ctx,&mut app,surface,vec![]);
            render(&ctx,&mut app,surface,vec![]);
            let slider_id = owner.with(Action::Mute).with("volume-slider-bounds");
            let before = ctx.data(|data|data.get_temp::<egui::Rect>(slider_id)).unwrap();
            let source = rect(&ctx,surface,Action::Mute).center();
            render(&ctx,&mut app,surface,vec![egui::Event::PointerMoved(source),pointer(source,egui::PointerButton::Primary,true)]);
            render(&ctx,&mut app,surface,vec![egui::Event::PointerMoved(source+egui::vec2(35.0,20.0))]);
            assert_eq!(egui::DragAndDrop::payload::<Drag>(&ctx).unwrap().action,Action::Mute);
            let target = rect(&ctx,surface,Action::Stop).center();
            render(&ctx,&mut app,surface,vec![egui::Event::PointerMoved(target)]);
            render(&ctx,&mut app,surface,vec![pointer(target,egui::PointerButton::Primary,false)]);
            assert!(!app.control_bars.monitor.end[Action::Mute as usize]);
            assert_eq!(app.control_bars.simple,simple);
            assert!(!app.is_muted,"dragging the speaker must not mute");
            render(&ctx,&mut app,surface,vec![]);
            let after = ctx.data(|data|data.get_temp::<egui::Rect>(slider_id)).unwrap();
            assert_ne!(before.top(),after.top(),"slider did not follow the moved speaker");
            assert_eq!(after.top(),rect(&ctx,surface,Action::Mute).top());
            app.current_video_path = Some("volume-pointer-fixture.mkv".into());
            app.volume = 50.0;
            app.mpv.0.set_property("volume",50.0).unwrap();
            render(&ctx,&mut app,surface,vec![]);
            let slider = ctx.data(|data|data.get_temp::<egui::Rect>(slider_id)).unwrap();
            let press = egui::pos2(if rtl {slider.right()-20.0} else {slider.left()+20.0},slider.center().y);
            let layout = app.control_bars;
            render(&ctx,&mut app,surface,vec![egui::Event::PointerMoved(press),pointer(press,egui::PointerButton::Primary,true)]);
            render(&ctx,&mut app,surface,vec![egui::Event::PointerMoved(press+egui::vec2(if rtl {-30.0} else {30.0},0.0))]);
            assert!(egui::DragAndDrop::payload::<Drag>(&ctx).is_none());
            render(&ctx,&mut app,surface,vec![pointer(press,egui::PointerButton::Primary,false)]);
            assert_eq!(app.control_bars,layout);
            assert!((app.mpv.0.get_property::<f64>("volume").unwrap()-50.0).abs()>0.1,"volume slider did not receive the gesture");
        }
        }
    }

    #[test]
    fn monitor_fullscreen_trails_volume_and_menu() {
        let _lock = crate::app::tests::lock_app_tests();
        for rtl in [false,true] {
            let ctx=egui::Context::default(); let mut app=PealayerApp::default(); app.rtl=rtl;
            render(&ctx,&mut app,Surface::Monitor,vec![]);
            let speaker=rect(&ctx,Surface::Monitor,Action::Mute);
            let fullscreen=rect(&ctx,Surface::Monitor,Action::Fullscreen);
            let owner=super::super::action_customization::owner(Surface::Monitor);
            let slider=ctx.data(|data|data.get_temp::<egui::Rect>(owner.with(Action::Mute).with("volume-slider-bounds"))).unwrap();
            let menu=ctx.read_response(owner.with("overflow")).unwrap().rect;
            if rtl {assert!(fullscreen.right()<=slider.left() && slider.right()<=speaker.left() && speaker.right()<=menu.left());}
            else {assert!(menu.right()<=speaker.left() && speaker.right()<=slider.left() && slider.right()<=fullscreen.left());}
        }
    }
    #[test]
    fn overflow_primary_click_opens_a_durable_menu_independent_of_context_menu() {
        let _lock = crate::app::tests::lock_app_tests();
        for surface in [Surface::Monitor, Surface::Simple] {
            let ctx=egui::Context::default(); let mut app=PealayerApp::default();
            let original=app.control_bars;
            for _ in 0..2 { render(&ctx,&mut app,surface,vec![]); }
            let id=super::super::action_customization::owner(surface).with("overflow");
            let pos=ctx.read_response(id).unwrap().rect.center();
            render(&ctx,&mut app,surface,vec![egui::Event::PointerMoved(pos),pointer(pos,egui::PointerButton::Primary,true)]);
            render(&ctx,&mut app,surface,vec![pointer(pos,egui::PointerButton::Primary,false)]);
            for _ in 0..2 {
                let output=render(&ctx,&mut app,surface,vec![]);
                assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Show / hide actions"))), "left-click overflow menu disappeared on {surface:?}");
            }
            assert_eq!(app.control_bars,original);
            egui::Popup::close_all(&ctx);
            render(&ctx,&mut app,surface,vec![pointer(pos,egui::PointerButton::Secondary,true)]);
            render(&ctx,&mut app,surface,vec![pointer(pos,egui::PointerButton::Secondary,false)]);
            assert!(egui::Popup::is_id_open(&ctx,id.with("popup")));
        }
    }
    #[test]
    fn compact_monitor_overflow_keeps_omitted_commands_usable() {
        let _lock=crate::app::tests::lock_app_tests();
        let ctx=egui::Context::default(); let mut app=PealayerApp::default();
        let surface=Surface::Monitor; let owner=super::super::action_customization::owner(surface);
        app.control_bars.monitor.hidden[Action::Preferences as usize]=false;
        let render=|app:&mut PealayerApp,events| {
            let mut output=ctx.run_ui(egui::RawInput {
                screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(700.0,600.0))),events,..Default::default()
            },|ui| {
                ui.allocate_ui_with_layout(egui::vec2(72.0,32.0),egui::Layout::left_to_right(egui::Align::Center),|ui| {
                    draw_group(app,ui,surface,true,owner);
                });
            }); output.textures_delta.clear(); output
        };
        render(&mut app,vec![]); render(&mut app,vec![]);
        let overflow=ctx.data(|data|data.get_temp::<Vec<Action>>(owner.with(("overflow-actions",true)))).unwrap();
        assert!(overflow.contains(&Action::Preferences));
        let pos=ctx.read_response(owner.with("overflow")).unwrap().rect.center();
        render(&mut app,vec![egui::Event::PointerMoved(pos),pointer(pos,egui::PointerButton::Primary,true)]);
        render(&mut app,vec![pointer(pos,egui::PointerButton::Primary,false)]);
        let output=render(&mut app,vec![]);
        let target=output.shapes.iter().find_map(|shape|match &shape.shape {
            egui::Shape::Text(text) if text.galley.text().contains("Preferences...") => Some(text.pos+text.galley.size()*0.5), _=>None,
        }).expect("omitted command must be rendered in overflow");
        render(&mut app,vec![egui::Event::PointerMoved(target),pointer(target,egui::PointerButton::Primary,true)]);
        render(&mut app,vec![pointer(target,egui::PointerButton::Primary,false)]);
        assert!(app.show_preferences_dialog,"overflow command must dispatch the existing Preferences action");
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
    fn track_selectors_keep_primary_popups_open_separate_from_customization() {
        let _lock = crate::app::tests::lock_app_tests();
        for surface in [Surface::Simple, Surface::Monitor] {
            for action in [Action::VideoTrack, Action::AudioTrack, Action::SubtitleTrack] {
                let ctx = egui::Context::default();
                let mut app = PealayerApp::default();
                app.engine_handle = crate::four_d::engine::memory_only_test_engine();
                app.current_video_path = Some("fixture.mp4".into());
                app.control_bars.layout_mut(surface).hidden.fill(true);
                app.control_bars.layout_mut(surface).hidden[action as usize] = false;
                render(&ctx, &mut app, surface, vec![]);
                render(&ctx, &mut app, surface, vec![]);
                let point = rect(&ctx, surface, action).center();
                render(&ctx, &mut app, surface, vec![egui::Event::PointerMoved(point), pointer(point, egui::PointerButton::Primary, true)]);
                render(&ctx, &mut app, surface, vec![pointer(point, egui::PointerButton::Primary, false)]);
                let output = render(&ctx, &mut app, surface, vec![]);
                let id = super::super::action_customization::owner(surface).with(action);
                assert!(egui::Popup::is_id_open(&ctx, id.with("media-tracks")), "primary popup closed for {action:?}");
                assert!(output.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("No tracks available"))));
                assert!(egui::DragAndDrop::payload::<Drag>(&ctx).is_none());
                egui::Popup::close_all(&ctx);
                render(&ctx, &mut app, surface, vec![]);
                render(&ctx, &mut app, surface, vec![pointer(point, egui::PointerButton::Secondary, true)]);
                render(&ctx, &mut app, surface, vec![pointer(point, egui::PointerButton::Secondary, false)]);
                assert!(egui::Popup::is_id_open(&ctx, id.with("popup")), "customization must remain available");
                assert!(!egui::Popup::is_id_open(&ctx, id.with("media-tracks")));
            }
        }
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
                    if !hidden {
                        let edge=if rtl {overflow.right()-slot.right()} else {overflow.left()-slot.left()};
                        assert!(edge.abs()<0.5,"monitor menu must precede its trailing actions: {overflow:?} in {slot:?}, rtl={rtl}");
                    }
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
