use eframe::egui;

/// Shared geometry for every single-line navigation row and footer action.
///
/// Keep these values here instead of allowing individual dialogs to invent
/// their own button metrics. This is deliberately public within the crate so
/// regression tests can enforce a single dialog design system.
pub const NAVIGATION_HEIGHT: f32 = 32.0;
pub const ACTION_HEIGHT: f32 = 28.0;
pub const NAVIGATION_DETAIL_HEIGHT: f32 = 44.0;
const ACTION_BUTTON_WIDTH: f32 = 104.0;
const CONTROL_CORNER_RADIUS: f32 = 7.0;
const CONTROL_TEXT_SIZE: f32 = 13.0;

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

/// Finite, internally consistent geometry for a resizable in-app dialog.
///
/// `egui::Window` persists its last requested size. During an interactive
/// resize it also runs a sizing pass, so feeding it an inverted constraint or
/// a minimum larger than its current viewport can poison the remembered
/// rectangle and terminate the render loop. Derive every constraint from the
/// same clamped bounds instead of recomputing unrelated min/max values in each
/// dialog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogGeometry {
    pub bounds: egui::Rect,
    pub default_rect: egui::Rect,
    pub min_size: egui::Vec2,
    pub max_size: egui::Vec2,
}

pub fn bounded_geometry(
    content_rect: egui::Rect,
    margin: f32,
    desired_size: egui::Vec2,
    requested_min: egui::Vec2,
    requested_max: egui::Vec2,
) -> DialogGeometry {
    let content_rect =
        if content_rect.is_finite() && content_rect.width() > 0.0 && content_rect.height() > 0.0 {
            content_rect
        } else {
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(1.0))
        };
    let maximum_inset = ((content_rect.width().min(content_rect.height()) - 1.0) * 0.5).max(0.0);
    let safe_margin = margin.max(0.0).min(maximum_inset);
    let bounds = content_rect.shrink(safe_margin);
    let available = egui::vec2(bounds.width().max(1.0), bounds.height().max(1.0));
    let max_size = egui::vec2(
        requested_max.x.max(1.0).min(available.x),
        requested_max.y.max(1.0).min(available.y),
    );
    let min_size = egui::vec2(
        requested_min.x.max(1.0).min(max_size.x),
        requested_min.y.max(1.0).min(max_size.y),
    );
    let default_size = egui::vec2(
        desired_size.x.clamp(min_size.x, max_size.x),
        desired_size.y.clamp(min_size.y, max_size.y),
    );
    DialogGeometry {
        bounds,
        default_rect: centered_default_rect(bounds, default_size),
        min_size,
        max_size,
    }
}

/// Build a window frame whose body is always fully opaque.
///
/// The application's Mica/DWM theme may deliberately give `window_fill` an
/// alpha channel. That looks appropriate for the main viewport, but applying
/// it to a floating editor lets controls from the workspace show through the
/// dialog and makes both surfaces unreadable. Dialogs use the same theme RGB
/// while explicitly discarding that alpha component.
pub fn opaque_window_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::window(ui.style()).fill(opaque_color(ui.visuals().window_fill()))
}

pub fn opaque_window_frame_from_context(ctx: &egui::Context) -> egui::Frame {
    let style = ctx.global_style();
    egui::Frame::window(&style).fill(opaque_color(style.visuals.window_fill()))
}

fn opaque_color(fill: egui::Color32) -> egui::Color32 {
    let [red, green, blue, _alpha] = fill.to_srgba_unmultiplied();
    egui::Color32::from_rgb(red, green, blue)
}

/// A consistent, keyboard-focusable dialog action with a Phosphor icon.
pub fn action_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    action_button_with_kind(ui, icon, label, false)
}

/// The visually emphasized variant of [`action_button`]. Native and embedded
/// dialog hosts use this same control, so dialog contents do not fork merely
/// to obtain platform-appropriate action hierarchy.
pub fn primary_action_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    action_button_with_kind(ui, icon, label, true)
}

fn action_button_with_kind(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    primary: bool,
) -> egui::Response {
    let selection = ui.visuals().selection;
    // Primary actions are distinguished by color and stroke, not by changing
    // their geometry. Equal widths keep paired actions such as Open / Cancel
    // visually balanced in every dialog host.
    let minimum_width = ACTION_BUTTON_WIDTH;
    let mut text = egui::RichText::new(format!("{icon}  {label}")).size(CONTROL_TEXT_SIZE);
    let mut button = egui::Button::new(text.clone())
        .min_size(egui::vec2(minimum_width, ACTION_HEIGHT))
        .corner_radius(CONTROL_CORNER_RADIUS);
    if primary {
        text = text.color(selection.stroke.color);
        button = egui::Button::new(text)
            .min_size(egui::vec2(minimum_width, ACTION_HEIGHT))
            .corner_radius(CONTROL_CORNER_RADIUS)
            .fill(selection.bg_fill)
            .stroke(egui::Stroke::new(
                1.0_f32,
                selection.bg_fill.gamma_multiply(1.35),
            ));
    }
    ui.add(button)
}

/// A shared, left-aligned navigation row for modal/dialog side rails.
pub fn navigation_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    label: &str,
    width: f32,
) -> egui::Response {
    navigation_control(ui, selected, icon, label, None, width, NAVIGATION_HEIGHT)
}

/// A two-line navigation row for modal/dialog lists that need compact metadata.
pub fn navigation_detail_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    label: &str,
    metadata: &str,
    width: f32,
) -> egui::Response {
    navigation_control(
        ui,
        selected,
        icon,
        label,
        Some(metadata),
        width,
        NAVIGATION_DETAIL_HEIGHT,
    )
}

fn navigation_control(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    label: &str,
    metadata: Option<&str>,
    width: f32,
    height: f32,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width.max(80.0), height), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        let selection = ui.visuals().selection;
        let fill = if selected {
            selection.bg_fill.gamma_multiply(0.22)
        } else {
            visuals.weak_bg_fill
        };
        let stroke = if response.has_focus() {
            egui::Stroke::new(1.5_f32, selection.bg_fill.gamma_multiply(1.25))
        } else if selected {
            egui::Stroke::new(1.0_f32, selection.bg_fill.gamma_multiply(1.18))
        } else {
            visuals.bg_stroke
        };
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect(
            rect,
            CONTROL_CORNER_RADIUS,
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );

        let icon_x = rect.left() + 17.0;
        painter.text(
            egui::pos2(icon_x, rect.center().y),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(15.0),
            if selected {
                selection.bg_fill
            } else {
                visuals.fg_stroke.color
            },
        );
        let text_x = rect.left() + 33.0;
        let title_y = if metadata.is_some() {
            rect.center().y - 7.0
        } else {
            rect.center().y
        };
        painter.text(
            egui::pos2(text_x, title_y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(CONTROL_TEXT_SIZE),
            if selected {
                ui.visuals().widgets.hovered.fg_stroke.color
            } else {
                ui.visuals().text_color()
            },
        );
        if let Some(metadata) = metadata {
            painter.text(
                egui::pos2(text_x, rect.center().y + 9.0),
                egui::Align2::LEFT_CENTER,
                metadata,
                egui::FontId::proportional(10.5),
                ui.visuals().weak_text_color(),
            );
        }
    }
    response
}

/// Align dialog actions to the conventional trailing edge while respecting
/// the reading direction. Add the primary action first, then secondary ones.
pub fn action_row(ui: &mut egui::Ui, rtl: bool, body: impl FnOnce(&mut egui::Ui)) {
    let layout = if rtl {
        egui::Layout::left_to_right(egui::Align::Center)
    } else {
        egui::Layout::right_to_left(egui::Align::Center)
    };
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ACTION_HEIGHT),
        layout,
        body,
    );
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
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ACTION_HEIGHT),
        outer,
        |ui| {
            leading(ui);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ACTION_HEIGHT),
                inner,
                trailing,
            );
        },
    );
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
    fn resizable_dialog_geometry_stays_finite_and_ordered_at_every_viewport_size() {
        for size in [
            egui::vec2(0.0, 0.0),
            egui::vec2(1.0, 1.0),
            egui::vec2(32.0, 24.0),
            egui::vec2(420.0, 280.0),
            egui::vec2(1_920.0, 1_080.0),
        ] {
            let geometry = bounded_geometry(
                egui::Rect::from_min_size(egui::Pos2::ZERO, size),
                24.0,
                egui::vec2(860.0, 720.0),
                egui::vec2(600.0, 420.0),
                egui::vec2(1_100.0, 820.0),
            );
            assert!(geometry.bounds.is_finite());
            assert!(geometry.default_rect.is_finite());
            assert!(geometry.min_size.x > 0.0 && geometry.min_size.y > 0.0);
            assert!(geometry.min_size.x <= geometry.default_rect.width());
            assert!(geometry.min_size.y <= geometry.default_rect.height());
            assert!(geometry.default_rect.width() <= geometry.max_size.x);
            assert!(geometry.default_rect.height() <= geometry.max_size.y);
            assert!(geometry.max_size.x <= geometry.bounds.width().max(1.0));
            assert!(geometry.max_size.y <= geometry.bounds.height().max(1.0));
        }

        let geometry = bounded_geometry(
            egui::Rect::from_min_max(egui::pos2(f32::NAN, 0.0), egui::pos2(10.0, f32::NAN)),
            24.0,
            egui::vec2(860.0, 720.0),
            egui::vec2(600.0, 420.0),
            egui::vec2(1_100.0, 820.0),
        );
        assert!(geometry.bounds.is_finite());
        assert!(geometry.default_rect.is_finite());
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
    fn primary_actions_are_emphasized_without_changing_geometry() {
        assert_eq!(ACTION_BUTTON_WIDTH, 104.0);
        assert!((27.0..=29.0).contains(&ACTION_HEIGHT));
        assert!(NAVIGATION_HEIGHT > ACTION_HEIGHT);
        assert_eq!(NAVIGATION_DETAIL_HEIGHT, 44.0);
    }

    #[test]
    fn navigation_and_footer_actions_share_one_control_height() {
        assert_eq!(NAVIGATION_HEIGHT, 32.0);
        assert_eq!(ACTION_HEIGHT, 28.0);
        for (name, source) in [
            ("preferences", include_str!("preferences.rs")),
            ("about", include_str!("about.rs")),
            ("board information", include_str!("board_info.rs")),
        ] {
            assert!(
                source.contains("dialog::navigation_button"),
                "{name} must use the shared dialog navigation control"
            );
        }
        let preferences = include_str!("preferences.rs");
        assert!(!preferences.contains(concat!("const PREFERENCES_", "TAB_HEIGHT")));
        assert!(!preferences.contains(concat!("const PREFERENCES_", "ACTION_HEIGHT")));
        assert!(!preferences.contains(concat!("fn preferences_", "action_button(")));
        assert!(include_str!("effects_library.rs").contains("dialog::navigation_detail_button"));
        let shared_dialog = include_str!("dialog.rs");
        assert!(shared_dialog.contains("ui.visuals().text_color()"));
        assert!(!shared_dialog.contains(concat!("strong_", "text_color()")));
    }

    #[test]
    fn unsupported_native_dialogs_fall_back_to_the_embedded_host() {
        assert_eq!(preferred_host(true, true), DialogHost::Native);
        assert_eq!(preferred_host(true, false), DialogHost::Embedded);
        assert_eq!(preferred_host(false, true), DialogHost::Embedded);
    }

    #[test]
    fn floating_dialog_frame_discards_theme_transparency() {
        let translucent = egui::Color32::from_rgba_unmultiplied(17, 34, 51, 40);
        let [red, green, blue, _alpha] = translucent.to_srgba_unmultiplied();
        let actual = opaque_color(translucent);
        assert_eq!(actual, egui::Color32::from_rgb(red, green, blue));
        assert_eq!(actual.a(), 255);
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
                ui.label(
                    egui::RichText::new(format!("{icon}  {title}"))
                        .strong()
                        .color(ui.visuals().widgets.hovered.fg_stroke.color),
                );
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

/// A responsive settings row shared by compact media dialogs. Labels retain
/// their natural reading order, supporting copy sits directly below them, and
/// the interactive control remains aligned at the trailing edge. At narrow
/// widths the control wraps beneath the label instead of clipping either side.
pub fn setting_row(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    description: Option<&str>,
    body: impl FnOnce(&mut egui::Ui),
) {
    let available = ui.available_width();
    if available < 330.0 {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).color(ui.visuals().selection.bg_fill));
                ui.label(egui::RichText::new(label).strong());
            });
            if let Some(description) = description {
                ui.label(egui::RichText::new(description).small().weak());
            }
            ui.add_space(3.0);
            body(ui);
        });
    } else {
        let label_width = (available * 0.44).clamp(145.0, 210.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(label_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).color(ui.visuals().selection.bg_fill));
                        ui.label(egui::RichText::new(label).strong());
                    });
                    if let Some(description) = description {
                        ui.label(egui::RichText::new(description).small().weak());
                    }
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 0.0),
                egui::Layout::right_to_left(egui::Align::Center),
                body,
            );
        });
    }
}

/// A compact numeric input flanked by explicit decrement/increment actions.
/// This is intentionally one shared control so Subtitle and Audio settings do
/// not drift in sizing, keyboard input, bounds, or icon treatment.
pub fn numeric_stepper(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
    suffix: &str,
) -> bool {
    let min = *range.start();
    let max = *range.end();
    let mut changed = false;
    ui.horizontal(|ui| {
        let decrement = ui
            .add_sized(
                [28.0, 26.0],
                egui::Button::new(crate::ui::icons::MINUS).corner_radius(6.0),
            )
            .on_hover_text(format!("Decrease by {step}"));
        if decrement.clicked() {
            *value = (*value - step).clamp(min, max);
            changed = true;
        }

        let response = ui.add_sized(
            [86.0, 26.0],
            egui::DragValue::new(value)
                .speed(step)
                .range(range)
                .fixed_decimals(decimals)
                .suffix(suffix),
        );
        changed |= response.changed();

        let increment = ui
            .add_sized(
                [28.0, 26.0],
                egui::Button::new(crate::ui::icons::PLUS).corner_radius(6.0),
            )
            .on_hover_text(format!("Increase by {step}"));
        if increment.clicked() {
            *value = (*value + step).clamp(min, max);
            changed = true;
        }
    });
    changed
}

/// Force scroll content into one viewport-width vertical column. Scroll areas
/// otherwise inherit a surrounding horizontal layout and can grow sideways.
pub fn scroll_column(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    max_height: Option<f32>,
    body: impl FnOnce(&mut egui::Ui),
) {
    scroll_column_impl(ui, id, max_height, false, body);
}

/// A vertically content-sized scroll column for compact dialogs. It behaves
/// like [`scroll_column`], but does not stretch a short body to fill the whole
/// window. A height limit still lets the body scroll when the user makes the
/// dialog smaller than its contents.
pub fn fit_scroll(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    max_height: Option<f32>,
    body: impl FnOnce(&mut egui::Ui),
) {
    scroll_column_impl(ui, id, max_height, true, body);
}

fn scroll_column_impl(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    max_height: Option<f32>,
    shrink_height_to_content: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, shrink_height_to_content]);
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
