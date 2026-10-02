pub mod about;
pub mod audio;
pub mod board_info;
pub mod controls;
pub mod dialog;
pub mod effects_library;
pub mod error;
pub mod four_d;
pub mod hardware_control;
pub mod i18n;
pub mod icons;
pub mod layout;
pub mod menu;
pub mod open_url;
pub mod preferences;
pub mod status_bar;
pub mod subtitles;
pub mod video;

/// Configure desktop-style interaction defaults. Static captions are not
/// documents, so they should not expose a text-selection cursor or highlight
/// when the user is trying to click, drag, or operate the surrounding control.
/// Editable widgets (and any labels explicitly marked selectable) retain their
/// normal text-editing behavior.
pub fn configure_interaction_style(style: &mut eframe::egui::Style) {
    style.interaction.selectable_labels = false;
    style.interaction.multi_widget_text_select = false;
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_style_disables_accidental_caption_selection() {
        let mut style = eframe::egui::Style::default();
        style.interaction.selectable_labels = true;
        style.interaction.multi_widget_text_select = true;

        super::configure_interaction_style(&mut style);

        assert!(!style.interaction.selectable_labels);
        assert!(!style.interaction.multi_widget_text_select);
    }
}
