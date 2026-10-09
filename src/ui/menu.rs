use crate::app::PealayerApp;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrackMenuState {
    submenu_enabled: bool,
    none_enabled: bool,
}

fn track_menu_state(media_loaded: bool, discovered_track_count: usize) -> TrackMenuState {
    TrackMenuState {
        submenu_enabled: media_loaded,
        none_enabled: media_loaded && discovered_track_count > 0,
    }
}

fn estop_button(
    ui: &mut egui::Ui,
    active: bool,
    language: crate::config::AppLanguage,
) -> egui::Response {
    let label = crate::ui::i18n::tr(language, "E-STOP");
    let (fill, stroke, help) = if active {
        (
            egui::Color32::from_rgb(127, 29, 29),
            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(69, 10, 10)),
            "E-STOP is active; click to release",
        )
    } else {
        (
            egui::Color32::from_rgb(192, 57, 43),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(110, 20, 20)),
            "Emergency stop: pause playback and stop hardware output",
        )
    };

    // The stop mark is painted as a vector octagon instead of relying on an
    // emoji glyph, whose appearance and availability vary by platform/font.
    let response = ui.add_sized(
        egui::vec2(104.0, 26.0),
        egui::Button::new(
            egui::RichText::new(format!("      {label}"))
                .color(egui::Color32::WHITE)
                .strong()
                .size(11.0),
        )
        .fill(fill)
        .stroke(stroke)
        .selected(active),
    );
    let center = egui::pos2(response.rect.left() + 15.0, response.rect.center().y);
    let radius = 8.0;
    let points = (0..8)
        .map(|index| {
            let angle = std::f32::consts::FRAC_PI_8 + index as f32 * std::f32::consts::FRAC_PI_4;
            center + egui::vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        egui::Color32::WHITE,
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(110, 20, 20)),
    ));
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "!",
        egui::FontId::proportional(10.0),
        fill,
    );
    response.on_hover_text(help)
}

pub(crate) fn draw_estop_release_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_estop_release_dialog {
        return;
    }

    // If another interface already released the shared PCController latch,
    // the local confirmation is no longer actionable.
    if !app.estop_active {
        app.show_estop_release_dialog = false;
        return;
    }

    let title = app.tr("Release E-STOP?");
    let message = app.tr("Hardware outputs and effects will be allowed again.");
    let skip_label = app.tr("Do not ask again");
    let cancel_label = app.tr("Cancel");
    let release_label = app.tr("Release E-STOP");
    let rtl = app.rtl;

    let modal = egui::Modal::new(egui::Id::new("estop_release_confirmation_v1"))
        .frame(
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(18))
                .corner_radius(10),
        )
        .show(ui.ctx(), |ui| {
            ui.set_min_width(360.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(crate::ui::icons::WARNING)
                        .size(24.0)
                        .color(egui::Color32::from_rgb(220, 74, 62)),
                );
                ui.heading(title);
            });
            ui.add_space(6.0);
            ui.label(message);
            ui.add_space(10.0);
            ui.checkbox(&mut app.skip_estop_release_confirmation_draft, skip_label);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            let mut release = false;
            let mut cancel = false;
            crate::ui::dialog::action_row(ui, rtl, |ui| {
                if crate::ui::dialog::action_button(ui, crate::ui::icons::X, &cancel_label)
                    .clicked()
                {
                    cancel = true;
                }
                if crate::ui::dialog::primary_action_button(
                    ui,
                    crate::ui::icons::POWER,
                    &release_label,
                )
                .clicked()
                {
                    release = true;
                }
            });
            (release, cancel)
        });

    let (release, cancel) = modal.inner;
    if release {
        app.show_estop_release_dialog = false;
        if app.skip_estop_release_confirmation_draft {
            app.confirm_estop_release = false;
            app.save_config();
        }
        app.set_emergency_stop(false);
    } else if cancel || modal.should_close() {
        app.show_estop_release_dialog = false;
        app.skip_estop_release_confirmation_draft = false;
    }
}

/// Switch sibling menus on hover while the menubar is active, matching the
/// interaction of native Windows menu bars. egui's root menu buttons otherwise
/// only toggle on click.
fn top_menu_button<R>(
    ui: &mut egui::Ui,
    title: String,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let state_id = egui::Id::new("pealayer_top_menu_hover_state");
    let active_popup = ui.ctx().data(|data| data.get_temp::<egui::Id>(state_id));
    let another_heading_is_open =
        active_popup.is_some_and(|popup_id| egui::Popup::is_id_open(ui.ctx(), popup_id));

    let result = ui.menu_button(title, add_contents);
    let popup_id = egui::Popup::default_response_id(&result.response);

    if result.response.hovered() && another_heading_is_open && active_popup != Some(popup_id) {
        // Opening one popup closes the previously open root popup.
        egui::Popup::open_id(ui.ctx(), popup_id);
        ui.ctx()
            .data_mut(|data| data.insert_temp(state_id, popup_id));
        ui.ctx().request_repaint();
    } else if egui::Popup::is_id_open(ui.ctx(), popup_id) {
        ui.ctx()
            .data_mut(|data| data.insert_temp(state_id, popup_id));
    } else if active_popup == Some(popup_id) {
        ui.ctx().data_mut(|data| data.remove::<egui::Id>(state_id));
    }

    result
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let language = app.language;
    let rtl = app.rtl;

    egui::Panel::top("menu_bar").show_inside(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.with_layout(crate::ui::i18n::layout(rtl, egui::Align::Center), |ui| {
                top_menu_button(ui, app.tr("File"), |ui| {
                    if ui.button(app.tr("Open Video File...")).clicked() {
                        ui.close();
                        app.open_media_file_dialog(&ctx);
                    }

                    if ui.button(app.tr("Open Location / URL...")).clicked() {
                        ui.close();
                        app.show_open_url_dialog = true;
                    }
                    if ui.button(format!("{} Connect to Pealayer...",crate::ui::icons::GLOBE)).clicked(){crate::ui::peer_browser::connection_dialog(&ctx);ui.close();}
                    if ui.button(format!("{}  Browse remote folder...", crate::ui::icons::FOLDER_OPEN)).clicked() {
                        let _ = crate::remote_location::request("", None, false, ui.ctx());
                        ui.close();
                    }

                    crate::ui::icons::submenu(ui, app.tr("Open Recent"), |ui| {
                        if app.recent_media.is_empty() {
                            ui.label(app.tr("No recent media"));
                        } else {
                            for path in app.recent_media.clone() {
                                let target = path.to_string_lossy();
                                let label = crate::media::media_target_label(&target);
                                if ui
                                    .button(app.display_text(&label))
                                    .on_hover_text(crate::media::redact_media_target(&target))
                                    .clicked()
                                {
                                    ui.close();
                                    app.load_media_target(&target);
                                }
                            }
                            ui.separator();
                            if ui.button(app.tr("Clear Recent")).clicked() {
                                ui.close();
                                app.clear_recent_media();
                            }
                        }
                    });

                    let has_video = app.current_video_path.is_some();
                    if ui.add_enabled(has_video, egui::Button::new(format!("{} {}", crate::ui::icons::INFO, app.tr("Media information")))
                        .shortcut_text(&app.application_shortcuts.media_information)).clicked() {
                        app.apply_interop_command(&ctx, crate::platform::interop::InteropCommand::OpenMediaInformation, "menu");
                        ui.close();
                    }
                    let has_local_folder = crate::peer::active()&&app.current_video_path.is_some() || app.current_video_path.as_deref()
                        .is_some_and(|path| crate::application_shortcuts::containing_media_folder(path).is_ok());
                    if ui.add_enabled(has_local_folder, egui::Button::new(format!("{} {}", crate::ui::icons::FOLDER_OPEN, app.tr("Open containing folder")))
                        .shortcut_text(&app.application_shortcuts.media_folder)).clicked() {
                        app.apply_interop_command(&ctx, crate::platform::interop::InteropCommand::OpenMediaFolder, "menu");
                        ui.close();
                    }
                    if ui
                        .add_enabled(has_video, egui::Button::new(app.tr("Close Video")))
                        .clicked()
                    {
                        ui.close();
                        app.close_video();
                    }

                    if ui.button(app.tr("Open Timeline Project...")).clicked() {
                        ui.close();
                        if crate::peer::active(){crate::ui::peer_browser::open(&ctx,crate::ui::peer_browser::Purpose::TimelineOpen,None);}
                        else if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Pealayer Timeline", &["json"])
                            .pick_file()
                        {
                            match crate::four_d::models::Timeline::load_from_file(&path) {
                                Ok(timeline) => {
                                    app.timeline = timeline;
                                    let compiled = crate::four_d::engine::compile_timeline(
                                        &app.timeline,
                                        &app.track_muted,
                                        &app.track_soloed,
                                    );
                                    let _ = app.engine_handle.sender.send(
                                        crate::four_d::engine::EngineMessage::UpdateQueue(compiled),
                                    );
                                }
                                Err(e) => {
                                    app.show_error =
                                        Some(format!("Failed to load timeline: {}", e));
                                }
                            }
                        }
                    }

                    ui.separator();

                    let save_enabled = app.current_video_path.as_ref().is_some_and(|path| {
                        !crate::media::is_remote_media_target(&path.to_string_lossy())
                    });
                    let save_btn = egui::Button::new(app.tr("Save Timeline (Sidecar)"));
                    if ui.add_enabled(save_enabled, save_btn).clicked() {
                        ui.close();
                        if crate::peer::active(){crate::ui::peer_browser::open(&ctx,crate::ui::peer_browser::Purpose::TimelineSave,None);}
                        else if let Some(ref video_path) = app.current_video_path {
                            let mut sidecar = video_path.clone();
                            sidecar.set_extension("4d.json");
                            if let Err(e) = app.timeline.save_to_file(&sidecar) {
                                app.show_error = Some(format!("Failed to save timeline: {}", e));
                            }
                        }
                    }

                    if ui.button(app.tr("Save Timeline As...")).clicked() {
                        ui.close();
                        if crate::peer::active(){crate::ui::peer_browser::open(&ctx,crate::ui::peer_browser::Purpose::TimelineSave,None);}
                        else if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Pealayer Timeline", &["json"])
                            .save_file()
                        {
                            if let Err(e) = app.timeline.save_to_file(&path) {
                                app.show_error = Some(format!("Failed to save timeline: {}", e));
                            }
                        }
                    }

                    ui.separator();
                    if ui.button(format!("{} Downloads", egui_phosphor::regular::DOWNLOAD_SIMPLE)).clicked() {
                        crate::downloads::open(&ctx);
                        ui.close();
                    }
                    if ui.button(app.tr("Quit")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                top_menu_button(ui, app.tr("Edit"), |ui| {
                    if ui
                        .add(egui::Button::new(format!("{} {}", crate::ui::icons::GEAR, app.tr("Preferences...")))
                            .shortcut_text(&app.application_shortcuts.preferences))
                        .clicked()
                    {
                        app.apply_interop_command(&ctx, crate::platform::interop::InteropCommand::OpenPreferences, "menu");
                        ui.close();
                    }
                    if ui.add(egui::Button::new(format!("{} {}", crate::ui::icons::PENCIL_SIMPLE, app.tr("Edit configuration file")))
                        .shortcut_text(&app.application_shortcuts.edit_config)).clicked() {
                        app.apply_interop_command(&ctx, crate::platform::interop::InteropCommand::EditConfiguration, "menu");
                        ui.close();
                    }
                    ui.separator();
                    let mut undo_btn = egui::Button::new(app.tr("Undo"));
                    undo_btn = undo_btn.shortcut_text("Ctrl+Z");
                    if ui.add_enabled(false, undo_btn).clicked() {
                        ui.close();
                    }

                    let mut redo_btn = egui::Button::new(app.tr("Redo"));
                    redo_btn = redo_btn.shortcut_text("Ctrl+Y");
                    if ui.add_enabled(false, redo_btn).clicked() {
                        ui.close();
                    }
                });

                top_menu_button(ui, app.tr("Audio"), |ui| {
                    let state = track_menu_state(
                        app.current_video_path.is_some(),
                        app.audio_tracks.len(),
                    );
                    ui.add_enabled_ui(state.submenu_enabled, |ui| {
                        crate::ui::icons::submenu(ui, app.tr("Audio Track"), |ui| {
                            crate::ui::media_tracks::draw_track_menu(
                                app,
                                ui,
                                crate::app::MediaTrackType::Audio,
                            );
                        });
                    });

                    ui.separator();

                    if ui.button(app.tr("Audio Settings...")).clicked() {
                        app.show_audio_settings = true;
                        ui.close();
                    }
                });

                // Subtitles menu
                top_menu_button(ui, app.tr("Subtitles"), |ui| {
                    let state = track_menu_state(
                        app.current_video_path.is_some(),
                        app.sub_tracks.len(),
                    );
                    ui.add_enabled_ui(state.submenu_enabled, |ui| {
                        crate::ui::icons::submenu(ui, app.tr("Subtitle Track"), |ui| {
                            crate::ui::media_tracks::draw_track_menu(
                                app,
                                ui,
                                crate::app::MediaTrackType::Subtitle,
                            );
                        });
                    });

                    ui.separator();

                    let mut vis = app.sub_visibility;
                    if ui.checkbox(&mut vis, app.tr("Enable Subtitles")).changed() {
                        app.set_subtitle_visibility(vis);
                    }

                    ui.separator();

                    if ui.button(app.tr("Subtitle Settings...")).clicked() {
                        app.show_sub_settings = true;
                        ui.close();
                    }
                });

                enum ChapterAction {
                    Previous,
                    Next,
                    Jump(i64),
                }

                top_menu_button(ui, app.tr("Chapters"), |ui| {
                    if app.media_chapters().is_empty() {
                        ui.add_enabled(false, egui::Label::new(app.tr("No chapters")));
                    } else {
                        let mut action = None;
                        ui.horizontal(|ui| {
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::SKIP_BACK,
                                    app.tr("Previous chapter")
                                ))
                                .clicked()
                            {
                                action = Some(ChapterAction::Previous);
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::SKIP_FORWARD,
                                    app.tr("Next chapter")
                                ))
                                .clicked()
                            {
                                action = Some(ChapterAction::Next);
                                ui.close();
                            }
                        });
                        ui.separator();
                        let current = app.active_media_chapter().map(|chapter| chapter.index);
                        for chapter in app.media_chapters() {
                            let label = format!(
                                "{}  {}",
                                crate::duration::format_time_value_ms(
                                    (chapter.time_seconds * 1_000.0).round() as u64
                                ),
                                chapter.title
                            );
                            if ui
                                .selectable_label(current == Some(chapter.index), label)
                                .clicked()
                            {
                                action = Some(ChapterAction::Jump(chapter.index));
                                ui.close();
                            }
                        }
                        match action {
                            Some(ChapterAction::Previous) => app.previous_media_chapter(),
                            Some(ChapterAction::Next) => app.next_media_chapter(),
                            Some(ChapterAction::Jump(index)) => app.jump_to_media_chapter(index),
                            None => {}
                        }
                    }
                });

                // Workspace switcher
                top_menu_button(ui, app.tr("Workspace"), |ui| {
                    if ui.button(format!("{} {}",crate::ui::icons::RADIO,app.tr("RF controls…"))).clicked() {
                        app.rf.open = true;
                        if let Err(error) = app.request_rf("catalog",serde_json::json!({"read_board":true})) { app.rf.error = error; }
                        ui.close();
                    }
                    ui.separator();
                    for (id, profile) in app.ordered_workspace_profiles() {
                        let active = app.active_workspace_profile.as_deref() == Some(id.as_str());
                        let label = format!(
                            "{}  {}",
                            crate::ui::icons::workspace_icon(&profile.icon),
                            profile.name
                        );
                        if ui.selectable_label(active, label).clicked() {
                            app.restore_workspace_profile(ui.ctx(), &id);
                            ui.close();
                        }
                    }
                    ui.separator();
                    crate::ui::icons::submenu(
                        ui,
                        format!("{} {}", crate::ui::icons::TABS, app.tr("Panels")),
                        |ui| crate::ui::layout::draw_workspace_tab_menu(app, ui),
                    );
                    ui.separator();
                    if ui
                        .button(format!(
                            "{}  {}",
                            crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                            app.tr("Reset Workspace to Default")
                        ))
                        .clicked()
                    {
                        app.dock_state = crate::ui::layout::create_initial_layout();
                        app.save_dock_layout();
                        ui.close();
                    }
                    if ui
                        .button(format!(
                            "{}  {}",
                            crate::ui::icons::FLOPPY_DISK,
                            app.tr("Manage workspaces...")
                        ))
                        .clicked()
                    {
                        app.show_workspace_profiles_dialog = true;
                        ui.close();
                    }
                });

                // Add right-aligned E-STOP and Serial controls
                top_menu_button(ui, app.tr("Help"), |ui| {
                    crate::ui::icons::submenu(ui, app.tr("Language"), |ui| {
                        for (preference, label) in [
                            (
                                crate::config::AppLanguage::System,
                                app.tr("System language"),
                            ),
                            (crate::config::AppLanguage::English, app.tr("English")),
                            (crate::config::AppLanguage::Persian, app.tr("Persian")),
                        ] {
                            if ui
                                .selectable_label(app.language_preference == preference, label)
                                .clicked()
                            {
                                    app.set_language(ui.ctx(), preference);
                                ui.close();
                            }
                        }
                    });
                    crate::ui::icons::submenu(ui, app.tr("Direction"), |ui| {
                        for (preference, label) in [
                            (crate::config::AppDirection::Auto, app.tr("Automatic")),
                            (crate::config::AppDirection::Ltr, app.tr("Left to right")),
                            (crate::config::AppDirection::Rtl, app.tr("Right to left")),
                        ] {
                            if ui
                                .selectable_label(app.direction_preference == preference, label)
                                .clicked()
                            {
                                app.set_direction(preference);
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui
                        .button(format!("{} {}", crate::ui::icons::KEYBOARD, app.tr("Keyboard Shortcuts...")))
                        .clicked()
                    {
                        ui.close();
                        app.show_shortcuts_dialog = true;
                    }
                    if ui
                        .button(format!("{} {} {}", crate::ui::icons::INFO, app.tr("About"), app.app_name))
                        .clicked()
                    {
                        ui.close();
                        app.show_about_dialog = true;
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    // 1. E-STOP toggle and its divider are one visibility
                    // cluster so hiding the control leaves no orphaned rule.
                    if app.show_estop_control {
                        let board_ready = app
                            .advertised_hardware()
                            .is_some_and(|capabilities| capabilities.board_connected);
                        let btn = ui
                            .add_enabled_ui(board_ready || app.estop_active, |ui| {
                                estop_button(ui, app.estop_active, language)
                            })
                            .inner
                            .on_disabled_hover_text(
                                app.tr("E-STOP is available when a live board is connected"),
                            );

                        btn.context_menu(|ui| {
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::EYE_SLASH,
                                    app.tr("Hide E-STOP")
                                ))
                                .clicked()
                            {
                                app.show_estop_control = false;
                                app.save_config();
                                ui.close();
                            }
                        });

                        if btn.clicked() {
                            app.request_emergency_stop_change(!app.estop_active);
                        }

                        ui.separator();
                    }

                    // 2. Coordinator/direct-diagnostic connection toggle & dropdown
                    if let Some(client)=crate::peer::client(){
                        let text=client.origin.as_str().replacen("http://","pealayer://",1);
                        ui.label(format!("{} {text}",crate::ui::icons::GLOBE));
                        ui.label(if app.is_connected{"Remote session"}else{"Disconnected"});
                    } else {
                    let connection_requested = app
                        .engine_handle
                        .connection_requested
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let conn_text = if app.is_connected {
                        app.tr("Disconnect")
                    } else if connection_requested {
                        app.tr("Connecting…")
                    } else {
                        app.tr("Connect")
                    };
                    let connection_icon = if app.is_connected {
                        crate::ui::icons::X
                    } else {
                        crate::ui::icons::PLUG
                    };
                    let conn_btn = ui.add(
                        egui::Button::new(format!("{connection_icon} {conn_text}"))
                            .selected(app.is_connected),
                    );
                    if conn_btn.clicked() {
                        let should_connect = !(app.is_connected || connection_requested);
                        if should_connect {
                            let endpoint = app.serial_port.trim().to_owned();
                            if endpoint.is_empty() {
                                app.connection_notice = Some(
                                    app.tr("Enter a hardware endpoint before connecting."),
                                );
                            } else {
                                app.serial_port = endpoint.clone();
                                if let Ok(mut selected) = app.engine_handle.serial_port.lock() {
                                    *selected = endpoint;
                                }
                                app.connection_notice = None;
                                app.engine_handle.connection_requested.store(
                                    true,
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                                app.save_config();
                            }
                        } else {
                            app.engine_handle.connection_requested.store(
                                false,
                                std::sync::atomic::Ordering::Relaxed,
                            );
                        }
                    }

                    // PCController owns the board during normal operation. Keep discovered
                    // devices convenient, but never make discovery the only way to choose a
                    // coordinator or direct diagnostic path: remote hosts and stable OS device
                    // paths are valid even when they are not visible to the local enumerator.
                    let mut endpoint_changed = false;
                    ui.add_enabled_ui(!app.is_connected && !connection_requested, |ui| {
                        ui.allocate_ui(egui::vec2(154.0, 20.0), |ui| {
                            egui::ComboBox::from_id_salt("hardware_endpoint_select")
                                .selected_text(&app.serial_port)
                                .width(154.0)
                                .height(240.0)
                                .show_ui(ui, |ui| {
                                    for endpoint in crate::four_d::controller::available_endpoints()
                                    {
                                        endpoint_changed |= ui
                                            .selectable_value(
                                                &mut app.serial_port,
                                                endpoint.clone(),
                                                endpoint,
                                            )
                                            .changed();
                                    }

                                    ui.separator();
                                    ui.label(app.tr("Custom endpoint or hardware path"));
                                    let endpoint_hint = app.tr(
                                        "pccontroller://host:port, tcp://host:port, or direct:<device>",
                                    );
                                    endpoint_changed |= ui
                                        .add_sized(
                                            [240.0, 22.0],
                                            crate::ui::dialog::singleline_text_edit(&mut app.serial_port)
                                                .hint_text(endpoint_hint),
                                        )
                                        .on_hover_text(app.tr(
                                            "Enter a PCController endpoint or an OS hardware path.",
                                        ))
                                        .changed();
                                });
                        });
                    });
                    if endpoint_changed {
                        app.save_config();
                    }

                    // Connection visual indicator dot
                    let dot_color = if app.is_connected {
                        egui::Color32::from_rgb(46, 204, 113) // Green
                    } else if connection_requested {
                        egui::Color32::from_rgb(241, 196, 15) // Amber
                    } else {
                        egui::Color32::from_rgb(231, 76, 60) // Red
                    };

                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(dot_rect.center(), 4.0, dot_color);

                    let status_lbl = if app.is_connected {
                        if crate::four_d::controller::is_controller_endpoint(&app.serial_port) {
                            app.advertised_hardware()
                                .filter(|capabilities| capabilities.board_connected)
                                .map(|capabilities| app.display_text(&capabilities.board_name))
                                .filter(|name| !name.trim().is_empty())
                                .unwrap_or_else(|| app.tr("PCController"))
                        } else {
                            format!("{} direct diagnostic connection", app.serial_port)
                        }
                    } else if connection_requested {
                        format!("Connecting to {}…", app.serial_port)
                    } else {
                        app.tr("Hardware Disconnected").to_string()
                    };
                    ui.add(
                        egui::Label::new(egui::RichText::new(status_lbl).size(10.0).weak())
                            .truncate(),
                    )
                    .on_hover_text(&app.serial_port);
                    }
                });
            });
        });
    });
}

pub fn format_track_label(id: i64, lang: Option<&str>, title: Option<&str>) -> String {
    let parts: Vec<&str> = vec![lang.unwrap_or(""), title.unwrap_or("")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        format!("Track {}", id)
    } else {
        format!("Track {} ({})", id, parts.join(" - "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_menu_is_contextual_to_media_and_discovered_tracks() {
        assert_eq!(
            track_menu_state(false, 0),
            TrackMenuState {
                submenu_enabled: false,
                none_enabled: false,
            }
        );
        assert_eq!(
            track_menu_state(true, 0),
            TrackMenuState {
                submenu_enabled: true,
                none_enabled: false,
            }
        );
        assert_eq!(
            track_menu_state(true, 1),
            TrackMenuState {
                submenu_enabled: true,
                none_enabled: true,
            }
        );
    }

    #[test]
    fn test_format_track_label_variations() {
        assert_eq!(format_track_label(1, None, None), "Track 1");
        assert_eq!(format_track_label(2, Some("eng"), None), "Track 2 (eng)");
        assert_eq!(
            format_track_label(3, None, Some("Commentary")),
            "Track 3 (Commentary)"
        );
        assert_eq!(
            format_track_label(4, Some("eng"), Some("Director's Cut")),
            "Track 4 (eng - Director's Cut)"
        );
        assert_eq!(format_track_label(5, Some(""), Some("")), "Track 5");
    }
}
