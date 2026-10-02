use eframe::egui;

const ACTION_BUTTON_SIZE: egui::Vec2 = egui::vec2(96.0, 32.0);
const PRIMARY_ACTION_BUTTON_SIZE: egui::Vec2 = egui::vec2(108.0, 32.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogHost {
    Embedded,
    Native,
}

/// Resolve the presentation host independently from a dialog's contents.
/// Dialog implementations can therefore keep one body and add a native host
/// without cloning their controls or behavior.
pub const fn preferred_host(native_requested: bool, native_supported: bool) -> DialogHost {
    if native_requested && native_supported {
        DialogHost::Native
    } else {
        DialogHost::Embedded
    }
}

/// Center a dialog the first time it opens without pinning it there. Unlike
/// `Window::anchor`, this leaves the remembered position free to follow title-
/// bar drags on subsequent frames.
pub fn centered_default_rect(bounds: egui::Rect, desired_size: egui::Vec2) -> egui::Rect {
    egui::Rect::from_center_size(bounds.center(), desired_size.min(bounds.size()))
}

/// A consistent, keyboard-focusable dialog action with a Phosphor icon.
pub fn action_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    ui.add_sized(
        ACTION_BUTTON_SIZE,
        egui::Button::new(egui::RichText::new(format!("{icon}  {label}")).size(15.0)),
    )
}

/// The visually emphasized variant of [`action_button`]. Native and embedded
/// dialog hosts use this same control, so dialog contents do not fork merely
/// to obtain platform-appropriate action hierarchy.
pub fn primary_action_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    let selection = ui.visuals().selection;
    ui.add_sized(
        PRIMARY_ACTION_BUTTON_SIZE,
        egui::Button::new(
            egui::RichText::new(format!("{icon}  {label}"))
                .size(15.0)
                .color(selection.stroke.color),
        )
        .fill(selection.bg_fill)
        .stroke(selection.stroke),
    )
}

/// Align dialog actions to the conventional trailing edge while respecting
/// the reading direction. Add the primary action first, then secondary ones.
pub fn action_row(ui: &mut egui::Ui, rtl: bool, body: impl FnOnce(&mut egui::Ui)) {
    let layout = if rtl {
        egui::Layout::left_to_right(egui::Align::Center)
    } else {
        egui::Layout::right_to_left(egui::Align::Center)
    };
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 32.0), layout, body);
}

/// A full-width footer with a utility action at the leading edge and primary /
/// cancel actions at the trailing edge.
pub fn action_bar(
    ui: &mut egui::Ui,
    rtl: bool,
    leading: impl FnOnce(&mut egui::Ui),
    trailing: impl FnOnce(&mut egui::Ui),
) {
    let outer = if rtl {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    let inner = if rtl {
        egui::Layout::left_to_right(egui::Align::Center)
    } else {
        egui::Layout::right_to_left(egui::Align::Center)
    };
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 32.0), outer, |ui| {
        leading(ui);
        ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 32.0), inner, trailing);
    });
}

pub fn escape_pressed(ctx: &egui::Context) -> bool {
    ctx.input(|input| input.key_pressed(egui::Key::Escape))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_default_rect_is_bounded_without_pinning() {
        let bounds = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(400.0, 300.0));
        let rect = centered_default_rect(bounds, egui::vec2(600.0, 200.0));
        assert_eq!(rect.center(), bounds.center());
        assert_eq!(rect.size(), egui::vec2(400.0, 200.0));
    }

    #[test]
    fn modal_dialogs_are_not_frame_anchored() {
        for (name, source) in [
            ("application dialogs", include_str!("../app.rs")),
            ("subtitle settings", include_str!("subtitles.rs")),
            ("audio settings", include_str!("audio.rs")),
            ("error dialog", include_str!("error.rs")),
        ] {
            assert!(
                !source.contains(".anchor("),
                "{name} must use default_rect so title-bar dragging persists"
            );
        }
    }

    #[test]
    fn primary_actions_are_more_prominent_than_secondary_actions() {
        assert!(PRIMARY_ACTION_BUTTON_SIZE.x > ACTION_BUTTON_SIZE.x);
        assert_eq!(PRIMARY_ACTION_BUTTON_SIZE.y, ACTION_BUTTON_SIZE.y);
        assert!((31.0..=33.0).contains(&ACTION_BUTTON_SIZE.y));
    }

    #[test]
    fn unsupported_native_dialogs_fall_back_to_the_embedded_host() {
        assert_eq!(preferred_host(true, true), DialogHost::Native);
        assert_eq!(preferred_host(true, false), DialogHost::Embedded);
        assert_eq!(preferred_host(false, true), DialogHost::Embedded);
    }
}

/// A visually consistent, width-bounded section used inside modal dialogs.
pub fn section(ui: &mut egui::Ui, icon: &str, title: &str, body: impl FnOnce(&mut egui::Ui)) {
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
pub fn compact_row(ui: &mut egui::Ui, rtl: bool, body: impl FnOnce(&mut egui::Ui)) {
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
