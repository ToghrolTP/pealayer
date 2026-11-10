//! One shared remote-browser state; no directory parsing or network I/O on the UI thread.
use crate::ui::palette::native::controls::CheckboxUiExt;
use crate::ui::dropdown::DropdownUiExt;
use super::{dialog, icons};
use crate::{app::PealayerApp, remote_location as remote};
use eframe::egui;
#[path = "remote_location_widgets.rs"]
mod widgets;

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
    let mut inspected = None;
    let mut proxy = state.use_proxy;
    egui::Window::new(format!("{}  Remote location", icons::FOLDER_OPEN))
        .id(id).open(&mut open).resizable(true)
        .default_rect(geometry.default_rect).min_size(geometry.min_size).max_size(geometry.max_size)
        .constrain_to(geometry.bounds).frame(dialog::opaque_window_frame_from_context(ctx))
        .show(ctx, |ui| {
            widgets::apply_style(ui);
            if let Some(action)=super::mouse_buttons::take_action(ui,ui.max_rect(),app.mouse_bindings.remote_browser) {
                use crate::pointer_bindings::Action as M;
                let command=match action {
                    M::HistoryBack=>Some(crate::platform::interop::InteropCommand::RemoteHistory{direction:-1,parent:false}),
                    M::HistoryForward=>Some(crate::platform::interop::InteropCommand::RemoteHistory{direction:1,parent:false}),
                    M::ParentFolder=>Some(crate::platform::interop::InteropCommand::RemoteHistory{direction:-1,parent:true}),_=>None,
                };
                if let Some(command)=command {app.apply_interop_command(ctx,command,"Remote browser mouse");}
            }
            ui.horizontal(|ui| {
                for (available,icon,caption,direction) in [
                    (state.history_back_url.is_some()||state.listing.as_ref().is_some_and(|l|l.parent_url.is_some()),icons::ARROW_LEFT,"Back",-1),
                    (state.history_forward_url.is_some(),icons::ARROW_RIGHT,"Forward",1)] {
                    if ui.add_enabled(!state.loading&&available,egui::Button::new(icon)).on_hover_text(caption).clicked() {
                        app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::RemoteHistory{direction,parent:false},"Remote browser");
                    }
                }
                ui.label(icons::LINK);
                let browse_caption = format!("{} Browse",icons::MAGNIFYING_GLASS);
                let browse_width = ui.painter().layout_no_wrap(browse_caption.clone(),egui::TextStyle::Button.resolve(ui.style()),ui.visuals().text_color()).size().x
                    + ui.spacing().button_padding.x*2.0 + ui.spacing().item_spacing.x;
                let response = ui.add(crate::ui::dialog::singleline_text_edit(ui, &mut view.target).hint_text("https://host/folder/").desired_width((ui.available_width() - browse_width).max(60.0)));
                if response.changed() { view.edited_at = Some(std::time::Instant::now()); }
                response.context_menu(|ui| { if ui.button(format!("{}  Copy", icons::COPY)).clicked() { ui.ctx().copy_text(view.target.clone()); ui.close(); } if ui.button("Paste").clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste); ui.close(); } });
                if ui.add_enabled(!state.loading && remote::normalize(&view.target).is_ok(), egui::Button::new(browse_caption)).clicked() || response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { navigate = Some(view.target.clone()); }
            });
            ui.horizontal(|ui| {
                if ui.themed_checkbox(&mut proxy, "Use proxy").on_hover_text("A manual choice overrides automatic route selection for this dialog. Off bypasses system and custom proxies.").changed() { view.proxy_override = Some(proxy); }
                if let Some(listing) = &state.listing {
                    ui.separator(); ui.label(format!("{} {}", icons::GLOBE, listing.host));
                    if let Some(server) = &listing.server { ui.label(egui::RichText::new(server).weak()); }
                    if let Some(parent) = &listing.parent_url { if ui.add_enabled(!state.loading,egui::Button::new(format!("{} Parent folder", icons::ARROW_UP))).clicked() { navigate = Some(parent.clone()); } }
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
                    let mut grid = state.view_mode == remote::ViewMode::Grid;
                    icons::search_view_header(ui, &mut view.filter, ui.id().with("remote-search"), &mut grid, "Filter files...", app.language);
                    if grid != (state.view_mode == remote::ViewMode::Grid) {
                        app.apply_interop_command(ctx, crate::platform::interop::InteropCommand::UpdateConfig {values:serde_json::json!({"remote_folder_view":if grid {"grid"} else {"list"}})}, "Remote browser layout");
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Sort by").small().weak());
                    for (by, label) in [(remote::SortBy::Name, "Name"), (remote::SortBy::Date, "Date"), (remote::SortBy::Size, "Size")] {
                        if ui.dropdown_choice(state.sort == by, label).clicked() { app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::SortRemote {by,descending:if state.sort == by { !state.descending } else { false }},"Remote browser sort"); }
                    }
                    ui.label(egui::RichText::new(if state.descending { "Descending" } else { "Ascending" }).weak());
                });
                let parent = listing.parent_url.as_ref().map(|url| remote::RemoteEntry { name: "Parent folder".into(), url: url.clone(), is_dir: true, playable: false, size_bytes: None, modified: None });
                let filter = view.filter.to_lowercase();
                let entries: Vec<_> = parent.iter().chain(listing.entries.iter()
                    .chain(listing.file.iter().filter(|file|!listing.entries.iter().any(|entry|entry.url==file.url)))
                    .filter(|e| e.name.to_lowercase().contains(&filter))).collect();
                let grid = state.view_mode == remote::ViewMode::Grid;
                let (columns, width, height) = widgets::item_geometry(ui, grid);
                egui::ScrollArea::vertical().id_salt(("remote-files",grid))
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .max_height((ui.available_height()-100.0).max(80.0)).auto_shrink([false,false])
                    .show_rows(ui, height+8.0, entries.len().div_ceil(columns), |ui, range| {
                    for row in range {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for entry in entries.iter().skip(row*columns).take(columns) {
                                let is_parent=parent.as_ref().is_some_and(|parent|parent.url==entry.url);
                                ui.push_id(&entry.url, |ui| {
                                    match widgets::entry(ui,entry,is_parent,&state,grid,egui::vec2(width,height)) {
                                        Some(widgets::Action::Open(target)) => navigate=Some(target),
                                        Some(widgets::Action::Select(target,play)) => selected=Some((target,play)),
                                        Some(widgets::Action::Inspect(target)) => inspected=Some(target),
                                        Some(widgets::Action::Retry(target)) => app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::RetryRemoteThumbnail {target},"Remote thumbnail retry"),
                                        None => {}
                                    }
                                });
                            }
                        });
                    }
                });
                if entries.is_empty() { ui.label(if listing.entries.is_empty() {"This folder is empty."} else {"No matching files."}); }
                ui.label(egui::RichText::new(format!("{} items",listing.entries.len())).small().weak());
            }
            ui.separator();
            let mut auto_next=state.auto_next; let mut thumbnails=state.thumbnails;
            ui.horizontal_wrapped(|ui| {ui.themed_checkbox(&mut auto_next,"Automatically play next file");ui.themed_checkbox(&mut thumbnails,"Thumbnails");});
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
    if !open || close || state.inspected.is_none() && dialog::escape_pressed(ctx) {
        app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::CloseRemoteBrowser,"Remote browser");
        return;
    }
    if let Some(target) = navigate {
        app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::BrowseRemote {target,use_proxy:proxy_override,clipboard:false},"Remote browser");
        ctx.request_repaint();
    }
    if let Some((target, play)) = selected {
        app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::SelectRemote {target,play},"Remote browser");
    }
    if let Some(target) = inspected {
        app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::InspectRemote {target:Some(target)},"Remote browser properties");
    }
    if let Some(entry) = &state.inspected {
        if widgets::properties(ctx,entry) {app.apply_interop_command(ctx,crate::platform::interop::InteropCommand::InspectRemote {target:None},"Remote browser properties");}
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

fn entry_details(entry:&remote::RemoteEntry,is_parent:bool)->Option<String>{
    if is_parent{return None;}
    let mut parts=Vec::new();
    if let Some(date)=entry.modified.as_deref().filter(|date|!date.trim().is_empty()){parts.push(date.to_string());}
    if let Some(bytes)=entry.size_bytes{parts.push(format_bytes(Some(bytes)));}
    (!parts.is_empty()).then(||parts.join("    "))
}

pub fn format_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "—".into();
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

#[cfg(test)] mod tests {
    #[test] fn navigation_rows_have_no_fabricated_file_metadata(){
        let mut entry=crate::remote_location::RemoteEntry{name:"Parent folder".into(),url:"https://index.invalid/".into(),is_dir:true,playable:false,size_bytes:None,modified:None};
        assert!(super::entry_details(&entry,true).is_none());assert!(super::entry_details(&entry,false).is_none());
        entry.modified=Some("2026-01-02".into());entry.size_bytes=Some(2048);
        assert!(super::entry_details(&entry,true).is_none());assert_eq!(super::entry_details(&entry,false).unwrap(),"2026-01-02    2.0 KiB");
        assert_eq!(super::format_bytes(None),"—");
    }
}
