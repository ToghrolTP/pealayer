//! The peer's filesystem, never the consumer's native file picker.
use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum Purpose {
    Media,
    Audio,
    Subtitle,
    TimelineOpen,
    TimelineSave,
    ConfigImport,
    ConfigExport(Box<crate::config::AppConfig>),
    PreferenceFile { key: String, extensions: Vec<String> },
}

fn preference_file_id(key: &str) -> egui::Id { egui::Id::new(("preference-file-selection", key)) }

pub fn take_preference_file(ctx: &egui::Context, key: &str) -> Option<String> {
    ctx.data_mut(|data| data.remove_temp::<String>(preference_file_id(key)))
}

fn selectable_file(purpose: &Purpose, path: &str) -> bool {
    match purpose {
        Purpose::PreferenceFile { extensions, .. } => path.rsplit('.').next()
            .is_some_and(|extension| extensions.iter().any(|allowed| allowed.eq_ignore_ascii_case(extension))),
        _ => true,
    }
}

fn valid_selection(state: &State) -> bool {
    if state.busy || state.selected.is_empty() || !selectable_file(&state.purpose, &state.selected) {
        return false;
    }
    !matches!(state.purpose, Purpose::PreferenceFile { .. }) || state.listing.as_ref()
        .is_some_and(|listing| listing.entries.iter().any(|entry| entry.path == state.selected && !entry.is_dir))
}
struct State {
    open: bool,
    path: String,
    selected: String,
    filter: String,
    busy: bool,
    error: Option<String>,
    listing: Option<crate::server::fs_api::DirectoryBrowseResponse>,
    purpose: Purpose,
}
#[derive(Clone)]
struct Browser(Arc<Mutex<State>>);
fn id() -> egui::Id {
    egui::Id::new("remote_server_file_browser")
}
#[derive(Clone)]
struct Connection {
    open: bool,
    endpoint: String,
    port: u16,
    error: Option<String>,
}
pub fn connection_dialog(ctx: &egui::Context) {
    connection_dialog_to(ctx, None);
}

pub fn connection_dialog_to(ctx: &egui::Context, endpoint: Option<&str>) {
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new("peer_connection_dialog"),
            Connection {
                open: true,
                endpoint: endpoint.unwrap_or("pealayer://").into(),
                port: crate::config::control_port(),
                error: None,
            },
        )
    });
}
pub fn draw_connection(ui: &mut egui::Ui) -> Option<crate::process_control::ConnectRequest> {
    let id = egui::Id::new("peer_connection_dialog");
    let Some(mut state) = ui.ctx().data_mut(|data| data.get_temp::<Connection>(id)) else {
        return None;
    };
    if !state.open {
        return None;
    }
    let mut open = state.open;
    let mut request = None;
    egui::Window::new(format!("{} Connect to Pealayer", crate::ui::icons::GLOBE))
        .id(id)
        .open(&mut open)
        .frame(crate::ui::dialog::opaque_window_frame(ui))
        .default_width(380.0)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label("Server");
            ui.add(
                crate::ui::dialog::singleline_text_edit(&mut state.endpoint)
                    .hint_text("pealayer://host:8080")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                ui.label("Client Web/API port");
                ui.add(egui::DragValue::new(&mut state.port).range(1..=65535));
            });
            if let Some(error) = &state.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            if crate::ui::dialog::primary_action_button(ui, crate::ui::icons::PLUG, "Connect")
                .clicked()
            {
                let value = crate::process_control::ConnectRequest {
                    operation_id: uuid::Uuid::new_v4().to_string(), endpoint:state.endpoint.clone(), client_port:state.port,
                };
                match value.validate() {
                    Ok(()) => { state.open = false; request = Some(value); }
                    Err(error) => state.error = Some(error),
                }
            }
        });
    state.open = state.open && open;
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    request
}

pub fn open(ctx: &egui::Context, purpose: Purpose, path: Option<String>) {
    let browser = Browser(Arc::new(Mutex::new(State {
        open: true,
        path: path.unwrap_or_default(),
        selected: String::new(),
        filter: String::new(),
        busy: false,
        error: None,
        listing: None,
        purpose,
    })));
    ctx.data_mut(|data| data.insert_temp(id(), browser.clone()));
    refresh(&browser, ctx);
}
fn refresh(browser: &Browser, ctx: &egui::Context) {
    let Some(client) = crate::peer::client().cloned() else {
        return;
    };
    let path = if let Ok(mut state) = browser.0.lock() {
        if state.busy {
            return;
        }
        state.busy = true;
        state.error = None;
        state.listing = None;
        state.selected.clear();
        state.path.clone()
    } else {
        return;
    };
    let browser = browser.clone();
    let context = ctx.clone();
    std::thread::spawn(move || {
        let result = client.url("/api/fs/browse").and_then(|mut url| {
            url.query_pairs_mut().append_pair("path", &path);
            client.get_json::<crate::server::fs_api::DirectoryBrowseResponse>(&format!(
                "{}?{}",
                url.path(),
                url.query().unwrap_or("")
            ))
        });
        if let Ok(mut state) = browser.0.lock() {
            state.busy = false;
            match result {
                Ok(listing) => {
                    state.path = listing.current_path.clone();
                    state.listing = Some(listing)
                }
                Err(error) => state.error = Some(error),
            }
        }
        context.request_repaint();
    });
}
pub fn draw(app: &mut crate::app::PealayerApp, ui: &mut egui::Ui) {
    let Some(browser) = ui.ctx().data_mut(|data| data.get_temp::<Browser>(id())) else {
        return;
    };
    let Ok(mut state) = browser.0.lock() else {
        return;
    };
    if !state.open {
        return;
    }
    let ctx = ui.ctx().clone();
    let mut open = state.open;
    let mut cancel = false;
    let mut reload = false;
    let mut commit = false;
    let bounds = crate::ui::dialog::bounded_geometry(
        ctx.content_rect(),
        20.0,
        egui::vec2(680.0, 480.0),
        egui::vec2(360.0, 260.0),
        egui::vec2(1000.0, 700.0),
    );
    egui::Window::new(format!("{} Server files", crate::ui::icons::FOLDER_OPEN))
        .id(id())
        .open(&mut open)
        .frame(crate::ui::dialog::opaque_window_frame(ui))
        .default_rect(bounds.default_rect)
        .min_size(bounds.min_size)
        .max_size(bounds.max_size)
        .constrain_to(bounds.bounds)
        .show(&ctx, |ui| {
            ui.label(egui::RichText::new(crate::peer::client().unwrap().origin.as_str()).weak());
            ui.horizontal(|ui| {
                let parent = state
                    .listing
                    .as_ref()
                    .and_then(|listing| listing.parent_path.clone());
                if ui
                    .add_enabled(
                        parent.is_some() && !state.busy,
                        egui::Button::new(crate::ui::icons::ARROW_UP),
                    )
                    .on_hover_text("Parent folder")
                    .clicked()
                {
                    state.path = parent.unwrap();
                    reload = true;
                }
                let path = ui.add(
                    crate::ui::dialog::singleline_text_edit(&mut state.path)
                        .desired_width(ui.available_width() - 44.0),
                );
                if path.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    reload = true
                }
                if ui
                    .button(crate::ui::icons::ARROW_CLOCKWISE)
                    .on_hover_text("Load folder")
                    .clicked()
                {
                    reload = true;
                }
            });
            ui.add(
                crate::ui::dialog::singleline_text_edit(&mut state.filter)
                    .hint_text("Filter files")
                    .desired_width(f32::INFINITY),
            );
            ui.separator();
            if state.busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading server folder…");
                });
            }
            if let Some(error) = &state.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            let filter = state.filter.to_lowercase();
            let rows = state
                .listing
                .as_ref()
                .map(|listing| {
                    listing
                        .entries
                        .iter()
                        .filter(|entry| entry.name.to_lowercase().contains(&filter))
                        .map(|entry| {
                            (
                                entry.path.clone(),
                                entry.name.clone(),
                                entry.is_dir,
                                entry.is_media,
                                entry.size_bytes,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            egui::ScrollArea::vertical()
                .id_salt("peer_files")
                .max_height((ui.available_height() - 78.0).max(80.0))
                .show(ui, |ui| {
                    for (path, name, directory, media, size) in rows {
                        if !directory && !selectable_file(&state.purpose, &path) { continue; }
                        let glyph = if directory {
                            crate::ui::icons::FOLDER_OPEN
                        } else if media {
                            crate::ui::icons::FILE_VIDEO
                        } else {
                            crate::ui::icons::LIST
                        };
                        let suffix = if directory {
                            String::new()
                        } else {
                            format!("   {}", format_size(size))
                        };
                        let response = ui.add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new(format!("{glyph}  {name}{suffix}"))
                                .selected(state.selected == path)
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        );
                        if response.clicked() && (!directory || !matches!(state.purpose, Purpose::PreferenceFile { .. })) {
                            state.selected = path.clone();
                        }
                        if response.double_clicked() {
                            if directory {
                                state.path = path.clone();
                                reload = true;
                            } else {
                                state.selected = path.clone();
                                commit = true;
                            }
                        }
                        response.context_menu(|ui| {
                            if ui
                                .button(format!("{} Copy path", crate::ui::icons::COPY))
                                .clicked()
                            {
                                ui.ctx().copy_text(path.clone());
                                ui.close();
                            }
                        });
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("File");
                ui.add(
                    crate::ui::dialog::singleline_text_edit(&mut state.selected)
                        .desired_width((ui.available_width() - 220.0).max(80.0)),
                );
                if crate::ui::dialog::action_button(ui, crate::ui::icons::X, "Cancel").clicked() {
                    cancel = true;
                }
                let label = match state.purpose {
                    Purpose::Media | Purpose::Audio | Purpose::Subtitle | Purpose::TimelineOpen => {
                        "Open"
                    }
                    Purpose::TimelineSave | Purpose::ConfigExport(_) => "Save",
                    Purpose::ConfigImport => "Import",
                    Purpose::PreferenceFile { .. } => "Select",
                };
                if ui
                    .add_enabled(
                        valid_selection(&state),
                        egui::Button::new(format!("{} {label}", crate::ui::icons::CHECK)),
                    )
                    .clicked()
                {
                    commit = true;
                }
            });
        });
    state.open = open && !cancel;
    if commit && valid_selection(&state) {
        let selected = state.selected.clone();
        let purpose = state.purpose.clone();
        state.busy = true;
        if matches!(purpose, Purpose::Media) {
            app.load_media_target(&selected);
            state.open = false;
            state.busy = false;
        } else if let Purpose::PreferenceFile { key, .. } = purpose {
            ctx.data_mut(|data| data.insert_temp(preference_file_id(&key), selected));
            state.open = false;
            state.busy = false;
            ctx.request_repaint();
        } else {
            let browser = browser.clone();
            let context = ctx.clone();
            std::thread::spawn(move || {
                let client = crate::peer::client().unwrap();
                let result=match purpose {
                    Purpose::ConfigImport=>crate::config::AppConfig::load_from_path(std::path::Path::new(&selected)).map(|config|{context.data_mut(|data|data.insert_temp(egui::Id::new("peer_imported_config"),config));}),
                    Purpose::ConfigExport(config)=>client.post("/api/peer/files",&serde_json::json!({"operation":"export_config","path":selected,"config":config})).map(|_|()),
                    Purpose::TimelineOpen=>client.post("/api/peer/files",&serde_json::json!({"operation":"open_timeline","path":selected})).map(|_|()),
                    Purpose::TimelineSave=>client.post("/api/peer/files",&serde_json::json!({"operation":"save_timeline","path":selected})).map(|_|()),
                    Purpose::Audio=>client.post("/api/peer/media",&serde_json::json!({"operation":"command","name":"audio-add","args":[selected]})).map(|_|()),
                    Purpose::Subtitle=>client.post("/api/peer/media",&serde_json::json!({"operation":"command","name":"sub-add","args":[selected]})).map(|_|()),
                    Purpose::Media=>Ok(()),
                    Purpose::PreferenceFile { .. }=>unreachable!("preference selection is returned to the draft, not executed remotely"),
                };
                if let Ok(mut state) = browser.0.lock() {
                    state.busy = false;
                    match result {
                        Ok(()) => state.open = false,
                        Err(error) => state.error = Some(error),
                    }
                }
                context.request_repaint();
            });
        }
    }
    drop(state);
    if reload {
        refresh(&browser, &ctx);
    }
}
fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1048576 {
        format!("{:.1} KiB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", size as f64 / 1048576.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_file_selection_requires_a_supported_non_directory_from_current_listing() {
        let mut state = State {
            open: true, path: String::new(), selected: "icon.PNG".into(), filter: String::new(),
            busy: false, error: None, listing: None,
            purpose: Purpose::PreferenceFile { key: "app_icon".into(), extensions: vec!["png".into()] },
        };
        assert!(!valid_selection(&state));
        state.listing = Some(crate::server::fs_api::DirectoryBrowseResponse {
            current_path: String::new(), parent_path: None,
            entries: vec![crate::server::fs_api::FileEntryInfo {
                path: "icon.PNG".into(), name: "icon.PNG".into(), is_dir: false, is_media: false,
                size_bytes: 1, has_thumbnail: false,
            }],
        });
        assert!(valid_selection(&state));
        state.listing.as_mut().unwrap().entries[0].is_dir = true;
        assert!(!valid_selection(&state));
        state.listing.as_mut().unwrap().entries[0].is_dir = false;
        state.busy = true;
        assert!(!valid_selection(&state));
        assert!(!selectable_file(&state.purpose, "icon.png.exe"));
        assert!(selectable_file(&Purpose::TimelineSave, "untitled.json"));
    }
}
