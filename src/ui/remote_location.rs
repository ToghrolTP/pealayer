//! One shared remote-browser state; no directory parsing or network I/O on the UI thread.
use crate::ui::dropdown::DropdownUiExt;
use super::{dialog, icons};
use crate::{app::PealayerApp, remote_location as remote};
use eframe::egui;

#[derive(Clone, Default)]
struct View {
    target: String,
    request_id: u64,
    filter: String,
    edited_at: Option<std::time::Instant>,
    loading_at: Option<std::time::Instant>,
    proxy_override: Option<bool>,
    last_sent_target: String,
}

pub fn draw(app: &mut PealayerApp, ctx: &egui::Context) {
    let state = remote::snapshot();
    let id = egui::Id::new("remote-folder-view");
    if !state.visible {
        ctx.data_mut(|d| { if let Some(mut view) = d.get_temp::<View>(id) { view.proxy_override = None; view.edited_at = None; d.insert_temp(id, view); } });
        return;
    }
    let mut view = ctx.data_mut(|d| d.get_temp::<View>(id)).unwrap_or_default();
    if view.request_id != state.request_id {
        if !(view.edited_at.is_some() && state.target == view.last_sent_target) {
            view.target = state.target.clone();
            view.edited_at = None;
        }
    }
    view.request_id = state.request_id;
    if state.loading { view.loading_at.get_or_insert_with(std::time::Instant::now); } else { view.loading_at = None; }
    let geometry = dialog::bounded_geometry(
        ctx.content_rect(),
        20.0,
        egui::vec2(860.0, 580.0),
        egui::vec2(400.0, 300.0),
        egui::vec2(1100.0, 850.0),
    );
    let mut open = true;
    let mut close = false;
    let mut navigate = None;
    let mut selected = None;
    let mut proxy = state.use_proxy;
    egui::Window::new(format!("{}  Remote location", icons::FOLDER_OPEN))
        .id(id).open(&mut open).resizable(true)
        .default_rect(geometry.default_rect).min_size(geometry.min_size).max_size(geometry.max_size)
        .constrain_to(geometry.bounds).frame(dialog::opaque_window_frame_from_context(ctx))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(icons::LINK);
                let response = ui.add(crate::ui::dialog::singleline_text_edit(&mut view.target).hint_text("https://host/folder/").desired_width((ui.available_width() - 88.0).max(90.0)));
                if response.changed() { view.edited_at = Some(std::time::Instant::now()); }
                response.context_menu(|ui| { if ui.button(format!("{}  Copy", icons::COPY)).clicked() { ui.ctx().copy_text(view.target.clone()); ui.close(); } if ui.button("Paste").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste); ui.close(); } });
                if ui.add_enabled(!state.loading && remote::normalize(&view.target).is_ok(), egui::Button::new(format!("{} Browse", icons::MAGNIFYING_GLASS))).clicked() || response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { navigate = Some(view.target.clone()); }
            });
            ui.horizontal(|ui| {
                if ui.checkbox(&mut proxy, "Use proxy").on_hover_text("A manual choice overrides automatic route selection for this dialog. Off bypasses system and custom proxies.").changed() { view.proxy_override = Some(proxy); }
                if let Some(listing) = &state.listing {
                    ui.separator(); ui.label(format!("{} {}", icons::GLOBE, listing.host));
                    if let Some(server) = &listing.server { ui.label(egui::RichText::new(server).weak()); }
                    if let Some(parent) = &listing.parent_url { if ui.button(format!("{} Parent folder", icons::ARROW_UP)).clicked() { navigate = Some(parent.clone()); } }
                }
            });
            if proxy != state.use_proxy && !view.target.is_empty() { navigate = Some(view.target.clone()); }
            // Reserve one stable status row, but do not flash a spinner for fast replies.
            let show_loading = view.loading_at.is_some_and(|t| t.elapsed() >= std::time::Duration::from_millis(250));
            ui.allocate_ui(egui::vec2(ui.available_width(), 24.0), |ui| {
                let alpha = ui.ctx().animate_bool(egui::Id::new("remote-loading-fade"), show_loading);
                if alpha > 0.0 { ui.multiply_opacity(alpha); ui.horizontal(|ui| {ui.spinner(); ui.label("Reading remote location…");}); }
            });
            if state.loading { ui.ctx().request_repaint_after(std::time::Duration::from_millis(50)); }
            if let Some(error) = &state.error { ui.colored_label(ui.visuals().error_fg_color, error); }
            draw_route_errors(ui, &state.route_errors);
            if let Some(listing) = &state.listing {
                if let Some(warning) = &listing.warning { ui.label(egui::RichText::new(warning).weak()); }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(icons::MAGNIFYING_GLASS);
                    ui.add(crate::ui::dialog::singleline_text_edit(&mut view.filter).hint_text("Filter files...").desired_width(180.0));
                    for (by, label) in [(remote::SortBy::Name, "Name"), (remote::SortBy::Date, "Date"), (remote::SortBy::Size, "Size")] {
                        if ui.dropdown_choice(state.sort == by, label).clicked() { remote::sort(by, if state.sort == by { !state.descending } else { false }); }
                    }
                    ui.label(egui::RichText::new(if state.descending { "Descending" } else { "Ascending" }).weak());
                });
                let parent = listing.parent_url.as_ref().map(|url| remote::RemoteEntry { name: "Back · Parent folder".into(), url: url.clone(), is_dir: true, playable: false, size_bytes: None, modified: None });
                let entries: Vec<_> = parent.iter().chain(listing.entries.iter().filter(|e| e.name.to_lowercase().contains(&view.filter.to_lowercase()))).collect();
                egui::ScrollArea::vertical().id_salt("remote-files").max_height((ui.available_height()-95.0).clamp(100.0,500.0)).auto_shrink([false,false]).show_rows(ui, 52.0, entries.len(), |ui, range| {
                    for index in range {
                        let entry = entries[index];
                        ui.push_id(&entry.url, |ui| {
                            let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), egui::Sense::click());
                            if state.selected.as_deref() == Some(&entry.url) { ui.painter().rect_filled(rect, 6.0, ui.visuals().selection.bg_fill); }
                            let thumb_rect = egui::Rect::from_min_size(rect.min + egui::vec2(6.0,5.0), egui::vec2(66.0,40.0));
                            let mut shown = false;
                            if state.thumbnails && entry.playable {
                                if let Ok(Some(path)) = remote::thumbnail(&entry.url, ui.ctx()) {
                                    let texture_id = egui::Id::new(("remote-thumb", &entry.url));
                                    let texture = ui.ctx().data_mut(|d| d.get_temp::<egui::TextureHandle>(texture_id)).or_else(|| {
                                        let image = image::open(path).ok()?.thumbnail(132,80).to_rgba8();
                                        let texture = ui.ctx().load_texture(format!("remote:{}", entry.url), egui::ColorImage::from_rgba_unmultiplied([image.width() as usize,image.height() as usize],image.as_raw()), egui::TextureOptions::LINEAR);
                                        ui.ctx().data_mut(|d| d.insert_temp(texture_id,texture.clone())); Some(texture)
                                    });
                                    if let Some(texture) = texture { ui.painter().image(texture.id(),thumb_rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),egui::Color32::WHITE); shown = true; }
                                }
                            }
                            if !shown { ui.painter().text(thumb_rect.center(),egui::Align2::CENTER_CENTER,if entry.is_dir {icons::FOLDER_OPEN} else {icons::FILE_VIDEO},egui::FontId::proportional(24.0),ui.visuals().weak_text_color()); }
                            let name_rect = egui::Rect::from_min_max(rect.min+egui::vec2(82.0,7.0), egui::pos2(rect.right()-80.0,rect.bottom()));
                            ui.scope_builder(egui::UiBuilder::new().max_rect(name_rect), |ui| {
                                ui.add(egui::Label::new(egui::RichText::new(&entry.name).strong()).truncate()).on_hover_text(&entry.name);
                                ui.label(egui::RichText::new(format!("{}    {}",entry.modified.as_deref().unwrap_or("Date unavailable"),format_bytes(entry.size_bytes))).small().weak());
                            });
                            ui.scope_builder(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(rect.right()-70.0,rect.top()+10.0),egui::vec2(65.0,30.0))), |ui| {
                                if ui.add_enabled(!state.loading && (entry.playable || entry.is_dir),egui::Button::new(if entry.is_dir {icons::ARROW_RIGHT} else {icons::PLAY})).on_hover_text(if entry.is_dir {"Open folder"} else {"Play"}).clicked() { if entry.is_dir {navigate=Some(entry.url.clone());} else {selected=Some((entry.url.clone(),true));} }
                            });
                            if !state.loading && response.clicked() { if parent.as_ref().is_some_and(|p| p.url == entry.url) { navigate=Some(entry.url.clone()); } else { selected=Some((entry.url.clone(),false)); } }
                            if !state.loading && response.double_clicked() { if entry.is_dir {navigate=Some(entry.url.clone());} else if entry.playable {selected=Some((entry.url.clone(),true));} }
                            response.context_menu(|ui| {
                                if ui.add_enabled(entry.playable,egui::Button::new(format!("{} Play",icons::PLAY))).clicked(){selected=Some((entry.url.clone(),true));ui.close();}
                                if entry.is_dir && ui.button(format!("{} Open folder",icons::FOLDER_OPEN)).clicked(){navigate=Some(entry.url.clone());ui.close();}
                                if ui.button(format!("{} Copy URL",icons::COPY)).clicked(){ui.ctx().copy_text(entry.url.clone());ui.close();}
                                ui.separator();ui.label(&entry.name);ui.label(format_bytes(entry.size_bytes));if let Some(date)=&entry.modified{ui.label(date);}
                            });
                        });
                    }
                });
                if entries.is_empty() { ui.label(if listing.entries.is_empty() {"This folder is empty."} else {"No matching files."}); }
                ui.label(egui::RichText::new(format!("{} items",listing.entries.len())).small().weak());
            }
            ui.separator();
            let mut auto_next=state.auto_next; let mut thumbnails=state.thumbnails;
            ui.horizontal_wrapped(|ui| {ui.checkbox(&mut auto_next,"Automatically play next file");ui.checkbox(&mut thumbnails,"Thumbnails");});
            if auto_next!=state.auto_next||thumbnails!=state.thumbnails {app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::UpdateConfig {values:serde_json::json!({"remote_folder_auto_next":auto_next,"remote_folder_thumbnails":thumbnails})},"Remote browser");}
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if dialog::action_button(ui,icons::X,"Close").clicked(){close=true;}
                ui.add_enabled_ui(!state.loading && state.selected.as_ref().is_some_and(|url|state.listing.as_ref().is_some_and(|l|l.entries.iter().any(|e|&e.url==url&&e.playable))), |ui| {if dialog::primary_action_button(ui,icons::PLAY,"Play selected").clicked(){selected=state.selected.clone().map(|url|(url,true));}});
                if state.previous_file.is_some() || state.next_file.is_some() {
                    for (enabled, icon, label, command) in [(state.next_file.is_some(), icons::SKIP_FORWARD, "Next file", crate::platform::interop::InteropCommand::Next), (state.previous_file.is_some(), icons::SKIP_BACK, "Previous file", crate::platform::interop::InteropCommand::Previous)] {
                        if ui.add_enabled(enabled,egui::Button::new(icon)).on_hover_text(label).clicked() { app.apply_interop_command(ctx,command,"Remote browser"); }
                    }
                }
            });
        });
    if let Some(edited) = view.edited_at {
        if edited.elapsed() >= std::time::Duration::from_millis(500) {
            if remote::normalize(&view.target).is_ok() && crate::platform::interop::get_live_config().open_url_fetch_remote_info { navigate = Some(view.target.clone()); }
            view.edited_at = None;
        } else { ctx.request_repaint_after(std::time::Duration::from_millis(500) - edited.elapsed()); }
    }
    if let Some(target) = &navigate { view.last_sent_target = target.clone(); }
    let proxy_override = view.proxy_override;
    ctx.data_mut(|d| d.insert_temp(id, view));
    if !open || close || dialog::escape_pressed(ctx) {
        remote::close();
        return;
    }
    if let Some(target) = navigate {
        if let Err(error) = remote::request(&target, proxy_override, false, ctx) {
            app.set_osd(error);
        }
    }
    if let Some((target, play)) = selected {
        if let Err(error) = remote::select(&target, play) {
            app.set_osd(error);
        }
    }
}

pub fn draw_route_errors(ui: &mut egui::Ui, errors: &[remote::RouteFailure]) {
    for error in errors {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new(error.label()).strong().color(ui.visuals().error_fg_color));
            ui.add(egui::Label::new(&error.message).wrap());
        });
    }
}

pub fn format_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "Size unavailable".into();
    };
    let mut n = bytes as f64;
    let mut unit = 0;
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    while n >= 1024.0 && unit < units.len() - 1 {
        n /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{n:.1} {}", units[unit])
    }
}
