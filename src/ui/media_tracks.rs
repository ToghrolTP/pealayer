use crate::app::{MediaTrackInfo, MediaTrackKey, MediaTrackType, PealayerApp};
use eframe::egui;

pub(crate) fn icon(kind: MediaTrackType) -> &'static str {
    match kind {
        MediaTrackType::Video => crate::ui::icons::FILE_VIDEO,
        MediaTrackType::Audio => crate::ui::icons::SPEAKER_HIGH,
        MediaTrackType::Subtitle => crate::ui::icons::SUBTITLES,
    }
}

pub(crate) fn track_label(track: &MediaTrackInfo) -> String {
    crate::ui::menu::format_track_label(track.id, track.language.as_deref(), track.title.as_deref())
}

pub(crate) fn current_track_label(app: &PealayerApp, kind: MediaTrackType) -> String {
    let current = app.current_media_track_id(kind);
    if current == "no" {
        return disabled_label(app, kind);
    }
    app.media_tracks
        .iter()
        .find(|track| track.kind == kind && track.id.to_string() == current)
        .map(track_label)
        .unwrap_or_else(|| format!("{} {current}", app.tr("Track")))
}

fn disabled_label(app: &PealayerApp, kind: MediaTrackType) -> String {
    app.tr(match kind {
        MediaTrackType::Video => "Video disabled",
        MediaTrackType::Audio => "No audio track",
        MediaTrackType::Subtitle => "Subtitles hidden",
    })
}

pub(crate) fn draw_track_menu(app: &mut PealayerApp, ui: &mut egui::Ui, kind: MediaTrackType) {
    ui.set_min_width(230.0);
    ui.horizontal(|ui| {
        ui.label(icon(kind));
        ui.strong(app.tr(kind.label()));
    });
    ui.label(
        egui::RichText::new(current_track_label(app, kind))
            .small()
            .weak(),
    );
    ui.separator();

    if kind == MediaTrackType::Audio {
        let mut muted = app.is_muted;
        if ui
            .checkbox(&mut muted, app.tr("Mute audio"))
            .on_hover_text(app.tr("Mute playback without changing the selected audio track."))
            .changed()
        {
            app.set_audio_muted(muted);
        }
        ui.separator();
    }

    let tracks = app
        .media_tracks
        .iter()
        .filter(|track| track.kind == kind)
        .cloned()
        .collect::<Vec<_>>();
    let current = app.current_media_track_id(kind).to_string();
    let disabled = current == "no";
    if ui
        .selectable_label(disabled, disabled_label(app, kind))
        .clicked()
    {
        app.disable_media_track(kind);
        ui.close();
    }
    ui.separator();

    if tracks.is_empty() {
        ui.add_enabled(false, egui::Label::new(app.tr("No tracks available")));
        return;
    }
    for track in tracks {
        let selected = current == track.id.to_string();
        if ui.selectable_label(selected, track_label(&track)).clicked() {
            app.select_media_track(MediaTrackKey { kind, id: track.id });
            ui.close();
        }
    }
}

pub(crate) fn menu_button(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    kind: MediaTrackType,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let tooltip = format!(
        "{}: {}",
        app.tr(kind.label()),
        current_track_label(app, kind)
    );
    ui.push_id(id_salt, |ui| {
        let response = egui::containers::menu::MenuButton::from_button(egui::Button::new(format!(
            "{} {}",
            icon(kind),
            crate::ui::icons::CARET_DOWN
        )))
        .ui(ui, |ui| draw_track_menu(app, ui, kind))
        .0;
        response.on_hover_text(tooltip)
    })
    .inner
}
