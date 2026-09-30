use eframe::egui;

/// A visually consistent, width-bounded section used inside modal dialogs.
pub fn section(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(9.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(format!("{icon}  {title}")).strong());
                ui.add_space(6.0);
                body(ui);
            });
        });
}

/// A compact horizontal row that cannot consume the remaining dialog height.
pub fn compact_row(
    ui: &mut egui::Ui,
    rtl: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let height = ui.spacing().interact_size.y;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        crate::ui::i18n::layout(rtl, egui::Align::Center),
        body,
    );
}

/// Force scroll content into one viewport-width vertical column. Scroll areas
/// otherwise inherit a surrounding horizontal layout and can grow sideways.
pub fn scroll_column(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    max_height: Option<f32>,
    body: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false]);
    if let Some(max_height) = max_height {
        scroll = scroll.max_height(max_height);
    }
    scroll.show(ui, |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(width);
                body(ui);
            },
        );
    });
}
