use crate::app::{MediaTrackInfo, MediaTrackKey, MediaTrackType, PealayerApp};
use crate::media_info::{MediaCollectionEntry, MediaFileInfo, MediaPropertyGroup};
use eframe::egui;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorSelection {
    File,
    Track(MediaTrackKey),
}

fn selection_id() -> egui::Id {
    egui::Id::new("media-inspector-selection")
}

fn filter_id() -> egui::Id {
    egui::Id::new("media-inspector-filter")
}

fn track_icon(kind: MediaTrackType) -> &'static str {
    match kind {
        MediaTrackType::Video => crate::ui::icons::FILE_VIDEO,
        MediaTrackType::Audio => crate::ui::icons::SPEAKER_HIGH,
        MediaTrackType::Subtitle => crate::ui::icons::SUBTITLES,
    }
}

fn track_title(track: &MediaTrackInfo) -> String {
    track
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} {}", track.kind.label(), track.id))
}

fn matches(filter: &str, values: &[&str]) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(&filter))
}

fn property_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.label(egui::RichText::new(key).monospace().weak())
        .on_hover_text(format!("libmpv property: {key}"));
    let response = ui.add(egui::Label::new(value).wrap());
    response.context_menu(|ui| {
        if ui
            .button(format!("{}  Copy value", crate::ui::icons::COPY))
            .clicked()
        {
            ui.ctx().copy_text(value.to_owned());
            ui.close();
        }
        if ui
            .button(format!("{}  Copy property", crate::ui::icons::CLIPBOARD))
            .clicked()
        {
            ui.ctx().copy_text(format!("{key}: {value}"));
            ui.close();
        }
    });
    ui.end_row();
}

fn property_group(ui: &mut egui::Ui, group: &MediaPropertyGroup, filter: &str) -> bool {
    let rows = group
        .properties
        .iter()
        .filter(|(key, value)| matches(filter, &[&group.name, key, value]))
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return false;
    }
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if crate::ui::icons::disclosure_header(
                ui,
                ("media-inspector-group", group.name.as_str()),
                &group.name,
                group.name == "File and source",
            ) {
                ui.add_space(4.0);
                egui::Grid::new(("media-inspector-grid", group.name.as_str()))
                    .num_columns(2)
                    .min_col_width(145.0)
                    .spacing([18.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (key, value) in rows {
                            property_row(ui, key, value);
                        }
                    });
            }
        });
    true
}

fn metadata_group(
    ui: &mut egui::Ui,
    title: &str,
    id: &'static str,
    values: &BTreeMap<String, String>,
    filter: &str,
) -> bool {
    let rows = values
        .iter()
        .filter(|(key, value)| matches(filter, &[title, key, value]))
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return false;
    }
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if crate::ui::icons::disclosure_header(ui, id, title, true) {
                ui.add_space(4.0);
                egui::Grid::new((id, "grid"))
                    .num_columns(2)
                    .min_col_width(145.0)
                    .spacing([18.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (key, value) in rows {
                            property_row(ui, key, value);
                        }
                    });
            }
        });
    true
}

fn collection_group(
    ui: &mut egui::Ui,
    title: &str,
    id: &'static str,
    entries: &[MediaCollectionEntry],
    filter: &str,
) -> bool {
    let entries = entries
        .iter()
        .filter(|entry| {
            matches(filter, &[title, &entry.index.to_string()])
                || entry
                    .properties
                    .iter()
                    .any(|(key, value)| matches(filter, &[key, value]))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return false;
    }
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if crate::ui::icons::disclosure_header(
                ui,
                id,
                &format!("{title} ({})", entries.len()),
                false,
            ) {
                for entry in entries {
                    ui.strong(format!("{title} {}", entry.index + 1));
                    egui::Grid::new((id, entry.index))
                        .num_columns(2)
                        .min_col_width(145.0)
                        .spacing([18.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (key, value) in &entry.properties {
                                property_row(ui, key, value);
                            }
                        });
                    ui.add_space(5.0);
                }
            }
        });
    true
}

fn file_as_text(info: &MediaFileInfo) -> String {
    let mut output = String::new();
    for group in &info.groups {
        output.push_str(&format!("[{}]\n", group.name));
        for (key, value) in &group.properties {
            output.push_str(&format!("{key}: {value}\n"));
        }
        output.push('\n');
    }
    for (title, values) in [
        ("Metadata", &info.metadata),
        ("Filtered metadata", &info.filtered_metadata),
    ] {
        if !values.is_empty() {
            output.push_str(&format!("[{title}]\n"));
            for (key, value) in values {
                output.push_str(&format!("{key}: {value}\n"));
            }
            output.push('\n');
        }
    }
    for (title, entries) in [
        ("Chapters", &info.chapters),
        ("Editions", &info.editions),
        ("Playlist", &info.playlist),
    ] {
        if !entries.is_empty() {
            output.push_str(&format!("[{title}]\n"));
            for entry in entries {
                output.push_str(&format!("#{}\n", entry.index + 1));
                for (key, value) in &entry.properties {
                    output.push_str(&format!("{key}: {value}\n"));
                }
            }
            output.push('\n');
        }
    }
    output
}

fn draw_file_details(
    ui: &mut egui::Ui,
    info: &MediaFileInfo,
    filter: &str,
    playback_time: f64,
    duration: f64,
    paused: bool,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(crate::ui::icons::PLAY).strong());
                ui.strong(if paused { "Paused" } else { "Playing" });
                ui.separator();
                ui.label(format!(
                    "{} / {}",
                    crate::ui::controls::format_player_time(
                        playback_time,
                        duration >= 3600.0,
                        true
                    ),
                    crate::ui::controls::format_player_time(duration, duration >= 3600.0, true),
                ));
            });
        });
    ui.add_space(8.0);

    let mut rendered = false;
    for group in &info.groups {
        rendered |= property_group(ui, group, filter);
        if rendered {
            ui.add_space(6.0);
        }
    }
    rendered |= metadata_group(
        ui,
        "Metadata",
        "media-inspector-metadata",
        &info.metadata,
        filter,
    );
    rendered |= metadata_group(
        ui,
        "Filtered metadata",
        "media-inspector-filtered-metadata",
        &info.filtered_metadata,
        filter,
    );
    rendered |= collection_group(
        ui,
        "Chapters",
        "media-inspector-chapters",
        &info.chapters,
        filter,
    );
    rendered |= collection_group(
        ui,
        "Editions",
        "media-inspector-editions",
        &info.editions,
        filter,
    );
    rendered |= collection_group(
        ui,
        "Playlist",
        "media-inspector-playlist",
        &info.playlist,
        filter,
    );
    if !rendered {
        ui.label(egui::RichText::new("No matching libmpv properties.").weak());
    }
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let has_media = app.current_video_path.is_some();
    let refresh_label = app.tr("Refresh from libmpv");
    let copy_label = app.tr("Copy all");
    let search_hint = app.tr("Search properties");

    ui.horizontal(|ui| {
        ui.heading(app.tr("Media Inspector"));
        ui.add_space(4.0);
        if ui
            .add_enabled(
                has_media,
                egui::Button::new(format!(
                    "{}  {refresh_label}",
                    crate::ui::icons::ARROW_CLOCKWISE
                )),
            )
            .clicked()
        {
            app.refresh_media_tracks();
        }
    });
    if !has_media {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(crate::ui::icons::MAGNIFYING_GLASS)
                    .size(32.0)
                    .weak(),
            );
            ui.heading(app.tr("No media loaded"));
            ui.label(
                egui::RichText::new(app.tr("Open media to inspect its libmpv properties.")).weak(),
            );
        });
        return;
    }

    let mut filter = ui
        .ctx()
        .data(|data| data.get_temp::<String>(filter_id()).unwrap_or_default());
    ui.horizontal(|ui| {
        ui.label(crate::ui::icons::MAGNIFYING_GLASS);
        let response = ui.add_sized(
            [ui.available_width().min(360.0), 24.0],
            crate::ui::dialog::singleline_text_edit(&mut filter).hint_text(search_hint),
        );
        if response.changed() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(filter_id(), filter.clone()));
        }
    });
    ui.separator();

    let info = app.media_file_info.clone();
    let tracks = app.media_tracks.clone();
    let mut selection = ui.ctx().data(|data| {
        data.get_temp::<InspectorSelection>(selection_id())
            .unwrap_or(InspectorSelection::File)
    });
    if let InspectorSelection::Track(key) = selection
        && !tracks
            .iter()
            .any(|track| track.kind == key.kind && track.id == key.id)
    {
        selection = InspectorSelection::File;
    }

    let available = ui.available_size();
    let nav_width = (available.x * 0.28).clamp(170.0, 250.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(nav_width, available.y),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("media-inspector-navigation")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if ui
                            .selectable_label(
                                selection == InspectorSelection::File,
                                format!("{}  File and metadata", crate::ui::icons::INFO),
                            )
                            .clicked()
                        {
                            selection = InspectorSelection::File;
                        }
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("Tracks ({})", tracks.len())).strong(),
                        );
                        for track in &tracks {
                            let key = MediaTrackKey {
                                kind: track.kind,
                                id: track.id,
                            };
                            let mut label = track_title(track);
                            if let Some(language) = track.language.as_deref() {
                                label.push_str(&format!(" · {language}"));
                            }
                            if ui
                                .selectable_label(
                                    selection == InspectorSelection::Track(key),
                                    format!("{}  {label}", track_icon(track.kind)),
                                )
                                .on_hover_text(format!(
                                    "{} · libmpv track-list/{}",
                                    track.kind.label(),
                                    track.list_index
                                ))
                                .clicked()
                            {
                                selection = InspectorSelection::Track(key);
                            }
                        }
                    });
            },
        );
        ui.separator();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), available.y),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.horizontal(|ui| {
                    match selection {
                        InspectorSelection::File => ui.heading("File and metadata"),
                        InspectorSelection::Track(key) => {
                            let title = tracks
                                .iter()
                                .find(|track| track.kind == key.kind && track.id == key.id)
                                .map(track_title)
                                .unwrap_or_else(|| format!("{} {}", key.kind.label(), key.id));
                            ui.heading(title)
                        }
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(format!("{}  {copy_label}", crate::ui::icons::COPY))
                            .clicked()
                        {
                            let text = match selection {
                                InspectorSelection::File => file_as_text(&info),
                                InspectorSelection::Track(key) => tracks
                                    .iter()
                                    .find(|track| track.kind == key.kind && track.id == key.id)
                                    .map(crate::ui::media_track_properties::as_text)
                                    .unwrap_or_default(),
                            };
                            ui.ctx().copy_text(text);
                        }
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("media-inspector-details")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        match selection {
                            InspectorSelection::File => draw_file_details(
                                ui,
                                &info,
                                &filter,
                                app.playback_time,
                                app.duration,
                                app.is_paused,
                            ),
                            InspectorSelection::Track(key) => {
                                if let Some(track) = tracks
                                    .iter()
                                    .find(|track| track.kind == key.kind && track.id == key.id)
                                    && !crate::ui::media_track_properties::draw_embedded(
                                        ui, track, &filter,
                                    )
                                {
                                    ui.label(
                                        egui::RichText::new("No matching track properties.").weak(),
                                    );
                                }
                            }
                        }
                    });
            },
        );
    });

    ui.ctx()
        .data_mut(|data| data.insert_temp(selection_id(), selection));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_filter_matches_exact_property_names_and_values() {
        assert!(matches("pixel", &["video-params/pixelformat", "yuv420p"]));
        assert!(matches("YUV", &["video-params/pixelformat", "yuv420p"]));
        assert!(!matches("audio", &["video-params/pixelformat", "yuv420p"]));
    }

    #[test]
    fn file_export_preserves_raw_libmpv_keys_and_metadata() {
        let info = MediaFileInfo {
            loaded: true,
            groups: vec![MediaPropertyGroup {
                name: "File and source".to_string(),
                properties: vec![("file-format".to_string(), "matroska".to_string())],
            }],
            metadata: BTreeMap::from([("TITLE".to_string(), "Example".to_string())]),
            ..Default::default()
        };
        let text = file_as_text(&info);
        assert!(text.contains("file-format: matroska"));
        assert!(text.contains("TITLE: Example"));
    }
}
