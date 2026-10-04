use crate::app::{MediaTrackInfo, MediaTrackKey, MediaTrackType, PealayerApp};
use eframe::egui;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PropertySection {
    title: &'static str,
    icon: &'static str,
    rows: Vec<(&'static str, String)>,
}

fn yes_no(value: bool) -> String {
    if value { "Yes" } else { "No" }.to_string()
}

fn byte_rate(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return value.to_string();
    }
    if value >= 1_000_000.0 {
        format!("{:.2} Mb/s", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1} kb/s", value / 1_000.0)
    } else {
        format!("{value:.0} b/s")
    }
}

fn sample_rate(value: i64) -> String {
    if value >= 1_000 {
        format!("{:.1} kHz", value as f64 / 1_000.0)
    } else {
        format!("{value} Hz")
    }
}

fn add<T: ToString>(rows: &mut Vec<(&'static str, String)>, label: &'static str, value: Option<T>) {
    if let Some(value) = value {
        let value = value.to_string();
        if !value.trim().is_empty() {
            rows.push((label, value));
        }
    }
}

fn add_bool(rows: &mut Vec<(&'static str, String)>, label: &'static str, value: Option<bool>) {
    add(rows, label, value.map(yes_no));
}

fn property_sections(track: &MediaTrackInfo) -> Vec<PropertySection> {
    let mut identity = Vec::new();
    add(&mut identity, "Type", Some(track.kind.label()));
    add(&mut identity, "Track ID", Some(track.id));
    add(&mut identity, "Track-list index", Some(track.list_index));
    add(&mut identity, "Source ID", track.source_id);
    add(&mut identity, "FFmpeg stream index", track.ffmpeg_index);
    add(&mut identity, "Title", track.title.clone());
    add(&mut identity, "Language", track.language.clone());
    add_bool(&mut identity, "Selected", track.selected);
    add(&mut identity, "Selection slot", track.main_selection);

    let mut codec = Vec::new();
    add(&mut codec, "Codec", track.codec.clone());
    add(
        &mut codec,
        "Codec description",
        track.codec_description.clone(),
    );
    add(&mut codec, "Codec profile", track.codec_profile.clone());
    add(&mut codec, "Decoder", track.decoder.clone());
    add(
        &mut codec,
        "Decoder description",
        track.decoder_description.clone(),
    );
    add(&mut codec, "Demuxer format", track.format_name.clone());

    let mut stream = Vec::new();
    if let (Some(width), Some(height)) = (track.demux_width, track.demux_height) {
        add(
            &mut stream,
            "Coded dimensions",
            Some(format!("{width} × {height}")),
        );
    } else {
        add(&mut stream, "Coded width", track.demux_width);
        add(&mut stream, "Coded height", track.demux_height);
    }
    if track.crop_x.is_some()
        || track.crop_y.is_some()
        || track.crop_width.is_some()
        || track.crop_height.is_some()
    {
        add(
            &mut stream,
            "Demux crop",
            Some(format!(
                "{} × {} at {}, {}",
                track.crop_width.map_or("?".to_string(), |v| v.to_string()),
                track.crop_height.map_or("?".to_string(), |v| v.to_string()),
                track.crop_x.map_or("?".to_string(), |v| v.to_string()),
                track.crop_y.map_or("?".to_string(), |v| v.to_string()),
            )),
        );
    }
    add(
        &mut stream,
        "Frame rate",
        track.fps.map(|value| format!("{value:.3} fps")),
    );
    add(&mut stream, "Bit rate", track.bitrate.map(byte_rate));
    add(
        &mut stream,
        "Rotation",
        track.rotation.map(|value| format!("{value}°")),
    );
    add(
        &mut stream,
        "Pixel aspect ratio",
        track.pixel_aspect_ratio.map(|value| format!("{value:.6}")),
    );
    add(&mut stream, "Audio channels", track.channel_count);
    add(&mut stream, "Channel layout", track.channel_layout.clone());
    add(
        &mut stream,
        "Sample rate",
        track.sample_rate.map(sample_rate),
    );
    add(
        &mut stream,
        "Dolby Vision profile",
        track.dolby_vision_profile,
    );
    add(&mut stream, "Dolby Vision level", track.dolby_vision_level);

    let mut flags = Vec::new();
    add_bool(&mut flags, "Default track", track.is_default);
    add_bool(&mut flags, "Forced track", track.forced);
    add_bool(&mut flags, "Dependent track", track.dependent);
    add_bool(&mut flags, "Visual impairment", track.visual_impaired);
    add_bool(&mut flags, "Hearing impairment", track.hearing_impaired);
    add_bool(&mut flags, "Image stream", track.image);
    add_bool(&mut flags, "Album artwork", track.album_art);
    add_bool(&mut flags, "External track", track.external);
    add(&mut flags, "External file", track.external_filename.clone());
    add(
        &mut flags,
        "HLS advertised bitrate",
        track.hls_bitrate.map(|value| byte_rate(value as f64)),
    );
    add(&mut flags, "Program ID", track.program_id);

    let mut replay_gain = Vec::new();
    add(
        &mut replay_gain,
        "Track peak",
        track
            .replaygain_track_peak
            .map(|value| format!("{value:.6}")),
    );
    add(
        &mut replay_gain,
        "Track gain",
        track
            .replaygain_track_gain
            .map(|value| format!("{value:.2} dB")),
    );
    add(
        &mut replay_gain,
        "Album peak",
        track
            .replaygain_album_peak
            .map(|value| format!("{value:.6}")),
    );
    add(
        &mut replay_gain,
        "Album gain",
        track
            .replaygain_album_gain
            .map(|value| format!("{value:.2} dB")),
    );

    let mut metadata = track
        .metadata
        .iter()
        .map(|(key, value)| {
            // Metadata keys come from the file and cannot be represented by a
            // static label lifetime. They are rendered in their own table.
            (key.clone(), value.clone())
        })
        .collect::<Vec<_>>();
    metadata.sort_by(|left, right| left.0.to_lowercase().cmp(&right.0.to_lowercase()));

    let mut sections = vec![
        PropertySection {
            title: "Identity",
            icon: crate::ui::icons::INFO,
            rows: identity,
        },
        PropertySection {
            title: "Codec and decoding",
            icon: crate::ui::icons::CIRCUITRY,
            rows: codec,
        },
        PropertySection {
            title: "Stream format",
            icon: crate::ui::icons::WAVEFORM,
            rows: stream,
        },
        PropertySection {
            title: "Flags and source",
            icon: crate::ui::icons::LIST_CHECKS,
            rows: flags,
        },
        PropertySection {
            title: "ReplayGain",
            icon: crate::ui::icons::SPEAKER_HIGH,
            rows: replay_gain,
        },
    ];
    sections.retain(|section| !section.rows.is_empty());

    if !metadata.is_empty() {
        // The dynamic metadata table is rendered separately; this marker keeps
        // the section ordering and its icon in one presentation model.
        sections.push(PropertySection {
            title: "Track metadata",
            icon: crate::ui::icons::CLIPBOARD,
            rows: Vec::new(),
        });
    }
    sections
}

fn property_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).weak());
    let response = ui.add(egui::Label::new(value).wrap());
    response.context_menu(|ui| {
        if ui
            .button(format!("{}  Copy", crate::ui::icons::COPY))
            .clicked()
        {
            ui.ctx().copy_text(value.to_owned());
            ui.close();
        }
    });
    ui.end_row();
}

fn section(ui: &mut egui::Ui, section: &PropertySection, track: &MediaTrackInfo) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(section.icon).size(16.0));
                ui.strong(section.title);
            });
            ui.add_space(6.0);
            if section.title == "Track metadata" {
                egui::Grid::new("media_track_metadata_grid")
                    .num_columns(2)
                    .min_col_width(150.0)
                    .spacing([22.0, 7.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (key, value) in &track.metadata {
                            property_row(ui, key, value);
                        }
                    });
            } else {
                egui::Grid::new(("media_track_property_grid", section.title))
                    .num_columns(2)
                    .min_col_width(150.0)
                    .spacing([22.0, 7.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (label, value) in &section.rows {
                            property_row(ui, label, value);
                        }
                    });
            }
        });
}

fn matches_filter(filter: &str, values: &[&str]) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(&filter))
}

/// Render the shared, factual libmpv track-property surface inside either the
/// movable Properties dialog or the optional Media Inspector workspace panel.
/// Keeping one renderer prevents the two inspection entry points from drifting.
pub(crate) fn draw_embedded(ui: &mut egui::Ui, track: &MediaTrackInfo, filter: &str) -> bool {
    let mut rendered = false;
    for mut section_info in property_sections(track) {
        if section_info.title == "Track metadata" {
            if !track
                .metadata
                .iter()
                .any(|(key, value)| matches_filter(filter, &[section_info.title, key, value]))
            {
                continue;
            }
        } else if !matches_filter(filter, &[section_info.title]) {
            section_info
                .rows
                .retain(|(label, value)| matches_filter(filter, &[label, value]));
            if section_info.rows.is_empty() {
                continue;
            }
        }
        section(ui, &section_info, track);
        ui.add_space(8.0);
        rendered = true;
    }
    rendered
}

pub(crate) fn as_text(track: &MediaTrackInfo) -> String {
    let mut output = format!("{} track {}\n", track.kind.label(), track.id);
    for section_info in property_sections(track) {
        output.push_str(&format!("\n[{}]\n", section_info.title));
        if section_info.title == "Track metadata" {
            for (key, value) in &track.metadata {
                output.push_str(&format!("{key}: {value}\n"));
            }
        } else {
            for (label, value) in section_info.rows {
                output.push_str(&format!("{label}: {value}\n"));
            }
        }
    }
    output
}

fn track_icon(kind: MediaTrackType) -> &'static str {
    match kind {
        MediaTrackType::Video => crate::ui::icons::FILE_VIDEO,
        MediaTrackType::Audio => crate::ui::icons::SPEAKER_HIGH,
        MediaTrackType::Subtitle => crate::ui::icons::SUBTITLES,
    }
}

pub fn open(app: &mut PealayerApp, kind: MediaTrackType, id: i64) {
    app.media_track_properties = Some(MediaTrackKey { kind, id });
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let Some(selection) = app.media_track_properties else {
        return;
    };
    let Some(track) = app.media_track(selection).cloned() else {
        app.media_track_properties = None;
        return;
    };

    let bounds = ui.ctx().content_rect();
    let geometry = crate::ui::dialog::bounded_geometry(
        bounds,
        18.0,
        egui::vec2(720.0, 650.0),
        egui::vec2(430.0, 340.0),
        egui::vec2(900.0, 820.0),
    );
    let title = track
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} {}", track.kind.label(), track.id));
    let mut open = true;
    let mut select_track = false;
    egui::Window::new(format!("{}  Track properties", track_icon(track.kind)))
        .id(egui::Id::new("media_track_properties_dialog"))
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .default_rect(geometry.default_rect)
        .min_size(geometry.min_size)
        .max_size(geometry.max_size)
        .constrain_to(geometry.bounds)
        .frame(crate::ui::dialog::opaque_window_frame(ui))
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(track_icon(track.kind)).size(30.0));
                ui.vertical(|ui| {
                    ui.heading(&title);
                    let mut detail = format!("{} · Track ID {}", track.kind.label(), track.id);
                    if let Some(language) = track.language.as_deref() {
                        detail.push_str(" · ");
                        detail.push_str(language);
                    }
                    ui.label(egui::RichText::new(detail).weak());
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("media_track_properties_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    draw_embedded(ui, &track, "");
                    if track.metadata.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No per-track metadata was advertised by the demuxer.",
                            )
                            .weak(),
                        );
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                let selected = track.selected.unwrap_or(false);
                if ui
                    .add_enabled(
                        !selected,
                        egui::Button::new(format!("{}  Use this track", crate::ui::icons::CHECK)),
                    )
                    .clicked()
                {
                    select_track = true;
                }
                if selected {
                    ui.label(
                        egui::RichText::new(format!(
                            "{}  Active track",
                            crate::ui::icons::CHECK_SQUARE
                        ))
                        .strong(),
                    );
                }
            });
        });

    if select_track {
        app.select_media_track(selection);
        app.refresh_media_tracks();
    }
    if !open || crate::ui::dialog::escape_pressed(ui.ctx()) {
        app.media_track_properties = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> MediaTrackInfo {
        MediaTrackInfo {
            list_index: 2,
            kind: MediaTrackType::Video,
            id: 7,
            source_id: Some(12),
            title: Some("Camera A".to_string()),
            language: Some("eng".to_string()),
            image: Some(false),
            album_art: Some(false),
            is_default: Some(true),
            forced: None,
            dependent: None,
            visual_impaired: None,
            hearing_impaired: None,
            hls_bitrate: None,
            program_id: Some(4),
            codec: Some("h264".to_string()),
            codec_description: Some("H.264 / AVC".to_string()),
            codec_profile: Some("High".to_string()),
            external: Some(false),
            external_filename: None,
            selected: Some(true),
            main_selection: Some(0),
            ffmpeg_index: Some(3),
            decoder: Some("h264".to_string()),
            decoder_description: None,
            demux_width: Some(1920),
            demux_height: Some(1080),
            crop_x: None,
            crop_y: None,
            crop_width: None,
            crop_height: None,
            channel_count: None,
            channel_layout: None,
            sample_rate: None,
            fps: Some(23.976),
            bitrate: Some(4_500_000.0),
            rotation: Some(0),
            pixel_aspect_ratio: Some(1.0),
            format_name: Some("h264".to_string()),
            replaygain_track_peak: None,
            replaygain_track_gain: None,
            replaygain_album_peak: None,
            replaygain_album_gain: None,
            dolby_vision_profile: None,
            dolby_vision_level: None,
            metadata: std::collections::BTreeMap::from([(
                "ENCODER".to_string(),
                "Lavc".to_string(),
            )]),
        }
    }

    #[test]
    fn property_model_includes_real_codec_stream_and_metadata_sections() {
        let sections = property_sections(&track());
        assert!(sections.iter().any(|section| section.title == "Identity"));
        assert!(
            sections
                .iter()
                .any(|section| section.title == "Codec and decoding")
        );
        assert!(
            sections
                .iter()
                .any(|section| section.title == "Stream format")
        );
        assert!(
            sections
                .iter()
                .any(|section| section.title == "Track metadata")
        );
    }

    #[test]
    fn absent_replaygain_is_not_fabricated() {
        let sections = property_sections(&track());
        assert!(!sections.iter().any(|section| section.title == "ReplayGain"));
    }

    #[test]
    fn human_readable_rates_are_used() {
        assert_eq!(byte_rate(4_500_000.0), "4.50 Mb/s");
        assert_eq!(sample_rate(48_000), "48.0 kHz");
    }
}
