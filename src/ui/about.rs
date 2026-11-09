use crate::app::PealayerApp;
use eframe::egui;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const COMMIT: &str = env!("PEALAYER_GIT_COMMIT");
const TABS: [(&str, &str); 4] = [
    (crate::ui::icons::INFO, "Overview"),
    (crate::ui::icons::CPU, "Build & system"),
    (crate::ui::icons::CIRCUITRY, "Connected board"),
    (crate::ui::icons::LIST_CHECKS, "Libraries & licenses"),
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AboutTab {
    #[default]
    Overview,
    Build,
    Board,
    Libraries,
}

impl AboutTab {
    pub fn index(self) -> usize {
        self as usize
    }
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_about_dialog {
        return;
    }
    ensure_icon(app, ui.ctx());

    let mut open = true;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(680.0), bounds.height().min(560.0));
    let default_size = egui::vec2(max_size.x.min(600.0), max_size.y.min(480.0));
    let min_size = egui::vec2(max_size.x.min(360.0), max_size.y.min(300.0));
    let default_rect = egui::Rect::from_center_size(bounds.center(), default_size);
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }
    let title = format!(
        "{} {} {}",
        crate::ui::icons::INFO,
        app.tr("About"),
        app.app_name
    );
    egui::Window::new(&title)
        // Keep the resize memory independent of localized/branded titles and reset
        // the formerly unbounded persisted geometry once for this corrected layout.
        .id(egui::Id::new("pealayer_about_dialog_bounded_v2"))
        .title_bar(false)
        .frame(crate::ui::dialog::opaque_window_frame(ui))
        .default_rect(default_rect)
        .min_size(min_size)
        .max_size(max_size)
        .constrain_to(bounds)
        .movable(true)
        .resizable(true)
        .collapsible(false)
        .show(ui.ctx(), |ui| {
            // Window sizes include chrome. Widening this child to max_size.x
            // expanded the body beyond the already painted title bar.
            let width = ui.available_width();
            ui.set_width(width);
            if title_row(ui, &title, &app.tr("Close")).clicked() {
                open = false;
            }
            ui.separator();
            ui.add_space(6.0);
            header(app, ui);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            let labels = TABS.map(|(_, label)| app.tr(label));
            let navigation_width = labels
                .iter()
                .map(|label| crate::ui::dialog::navigation_width(ui, label))
                .fold(144.0_f32, f32::max);
            let narrow = ui.available_width() < navigation_width + 340.0;
            if narrow {
                draw_tabs(app, ui, true, navigation_width);
                ui.separator();
                draw_content(app, ui);
            } else {
                let content_height = ui.available_height();
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(navigation_width, content_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(navigation_width);
                            draw_tabs(app, ui, false, navigation_width);
                        },
                    );
                    ui.separator();
                    draw_content(app, ui);
                });
            }
        });
    app.show_about_dialog = open;
}

fn title_row(ui: &mut egui::Ui, title: &str, close_label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = crate::ui::dialog::close_button(ui, close_label);
            let width = ui.available_width().max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, crate::ui::dialog::CLOSE_BUTTON_SIDE),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(title).strong()).truncate())
                        .on_hover_text(title);
                },
            );
            close
        })
        .inner
    })
    .inner
}

fn ensure_icon(app: &mut PealayerApp, ctx: &egui::Context) {
    let stamp = (
        crate::branding::current_state(),
        crate::platform::interop::live_config_revision(),
    );
    let id = egui::Id::new("about-branding-stamp");
    if app.about_icon.is_some()
        && ctx.data(|data| data.get_temp::<(crate::branding::PlaybackIconState, u64)>(id))
            == Some(stamp)
    {
        return;
    }
    let config = crate::platform::interop::get_live_config();
    if let Some(icon) = crate::branding::icon_bytes(&config, stamp.0)
        .and_then(|(_, bytes)| crate::branding::icon_data_from_bytes(&bytes))
    {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        let options =
            egui::TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));
        if let Some(texture) = &mut app.about_icon {
            texture.set(image, options);
        } else {
            app.about_icon = Some(ctx.load_texture("pealayer_about_icon", image, options));
        }
        ctx.data_mut(|data| data.insert_temp(id, stamp));
    }
}

fn draw_tabs(app: &mut PealayerApp, ui: &mut egui::Ui, compact: bool, navigation_width: f32) {
    let columns = if compact
        && ui.available_width() >= navigation_width * 2.0 + ui.spacing().item_spacing.x
    {
        2
    } else {
        1
    };
    let width = if compact {
        (ui.available_width() - ui.spacing().item_spacing.x * (columns - 1) as f32) / columns as f32
    } else {
        navigation_width
    };
    egui::Grid::new("about_navigation")
        .num_columns(columns)
        .show(ui, |ui| {
            for (index, (icon, label)) in TABS.into_iter().enumerate() {
                if crate::ui::dialog::navigation_button(
                    ui,
                    app.about_tab == index,
                    icon,
                    &app.tr(label),
                    width,
                )
                .clicked()
                {
                    app.about_tab = index;
                }
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
}

fn draw_content(app: &mut PealayerApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt(("about_content", app.about_tab))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // A ScrollArea inherits the surrounding horizontal layout. Without
            // an explicit vertical child, entire sections and even their text
            // rows are placed side-by-side, forcing a huge content width and
            // producing one-character wrapping. Keep a fixed viewport-width
            // column so every tab remains bounded and scrolls only vertically.
            let content_width = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(content_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(content_width);
                    match app.about_tab {
                        0 => overview(app, ui),
                        1 => build_and_system(app, ui),
                        2 => connected_board(app, ui),
                        _ => libraries_and_licenses(app, ui),
                    }
                },
            );
        });
}

fn header(app: &PealayerApp, ui: &mut egui::Ui) {
    ui.horizontal_top(|ui| {
        if let Some(icon) = &app.about_icon {
            ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(64.0, 64.0)));
        } else {
            ui.label(egui::RichText::new(crate::ui::icons::MONITOR_PLAY).size(52.0));
        }
        ui.add_space(8.0);
        let text_width = ui.available_width().max(120.0);
        ui.allocate_ui_with_layout(
            egui::vec2(text_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(&app.app_name).size(25.0).strong()).wrap(),
                );
                let description = crate::platform::interop::live_app_description();
                if !description.is_empty() { ui.label(description); }
                ui.label(
                    egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .size(17.0)
                        .strong(),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Commit").strong());
                    ui.hyperlink_to(short_commit(), commit_url());
                    if env!("PEALAYER_GIT_DIRTY") == "true" {
                        ui.label(
                            egui::RichText::new("locally modified build")
                                .color(ui.visuals().warn_fg_color),
                        );
                    }
                });
                ui.hyperlink_to("GitHub repository", REPOSITORY);
            },
        );
    });
}

fn overview(app: &PealayerApp, ui: &mut egui::Ui) {
    section(ui, "Project", |ui| {
        ui.add(egui::Label::new(env!("CARGO_PKG_DESCRIPTION")).wrap());
        ui.add_space(5.0);
        ui.add(egui::Label::new(format!("{} combines media playback, a non-linear physical-effect timeline, live hardware monitoring, and PCController integration for immersive cinema production and playback.", app.app_name)).wrap());
    });
    section(ui, "Release identity", |ui| {
        egui::Grid::new("about_release_identity")
            .num_columns(2)
            .max_col_width((ui.available_width() * 0.62).max(160.0))
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                strong_row(ui, "Version", env!("CARGO_PKG_VERSION"));
                ui.label(egui::RichText::new("Commit").strong());
                ui.hyperlink_to(short_commit(), commit_url());
                ui.end_row();
                strong_row(ui, "Branch / ref", env!("PEALAYER_GIT_BRANCH"));
                ui.label(egui::RichText::new("Repository").strong());
                ui.hyperlink_to("Open repository", REPOSITORY);
                ui.end_row();
                strong_row(ui, "License", env!("CARGO_PKG_LICENSE"));
            });
    });
    if let Some(publisher) = &app.app_publisher {
        ui.label(egui::RichText::new(publisher).strong());
    }
    if let Some(copyright) = &app.app_copyright {
        ui.label(egui::RichText::new(copyright).small().weak());
    }
}

fn build_and_system(app: &PealayerApp, ui: &mut egui::Ui) {
    section(ui, "Build", |ui| {
        egui::Grid::new("about_build")
            .num_columns(2)
            .max_col_width((ui.available_width() * 0.68).max(170.0))
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                row(ui, "Profile", env!("PEALAYER_BUILD_PROFILE"));
                row(ui, "Target", env!("PEALAYER_BUILD_TARGET"));
                row(ui, "Compiler", env!("PEALAYER_RUSTC_VERSION"));
                row(ui, "Source date epoch", env!("PEALAYER_SOURCE_DATE_EPOCH"));
                row(ui, "Working tree modified", env!("PEALAYER_GIT_DIRTY"));
            });
    });
    section(ui, "This system", |ui| {
        let host = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "Not reported".to_string());
        egui::Grid::new("about_system")
            .num_columns(2)
            .max_col_width((ui.available_width() * 0.68).max(170.0))
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                row(ui, "Host", &host);
                row(ui, "Operating system", std::env::consts::OS);
                row(ui, "Architecture", std::env::consts::ARCH);
                row(ui, "Hardware endpoint", &app.serial_port);
                let language = match app.language {
                    crate::config::AppLanguage::System => "System default",
                    crate::config::AppLanguage::English => "English",
                    crate::config::AppLanguage::Persian => "Persian / Farsi",
                };
                row(ui, "Application language", language);
            });
    });
}

fn connected_board(app: &PealayerApp, ui: &mut egui::Ui) {
    let Some(board) = app
        .advertised_hardware()
        .filter(|board| board.board_connected)
    else {
        section(ui, "No live board", |ui| {
            ui.label("Connect a board to inspect its authoritative identity, firmware, profile, port, and advertised capabilities here.");
        });
        return;
    };
    ui.heading(app.display_text(&board.board_name));
    ui.label(egui::RichText::new("Live information advertised by the connected board").weak());
    ui.add_space(8.0);
    section(ui, "Identity and firmware", |ui| {
        let build_hash = board
            .board_identity
            .build_hash
            .map(|value| format!("{value:08X}"))
            .unwrap_or_else(|| "Not advertised".to_string());
        let build_timestamp = board
            .board_identity
            .build_timestamp
            .as_deref()
            .unwrap_or("Not advertised");
        egui::Grid::new("about_board_identity")
            .num_columns(2)
            .max_col_width((ui.available_width() * 0.68).max(170.0))
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                strong_row(ui, "Board name", &board.board_name);
                row(
                    ui,
                    "Board kind",
                    &board.board_identity.board_kind.to_string(),
                );
                row(ui, "Firmware build hash", &build_hash);
                row(ui, "Firmware build time", build_timestamp);
                if let Some(profile) = &board.board_profile {
                    row(ui, "Profile", &profile.key);
                    row(ui, "Mode", &profile.mode);
                    row(ui, "Profile revision", &profile.revision);
                }
            });
    });
    section(ui, "Connection and capabilities", |ui| {
        egui::Grid::new("about_board_connection")
            .num_columns(2)
            .max_col_width((ui.available_width() * 0.68).max(170.0))
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                row(ui, "Port", &board.port.name);
                row(ui, "USB product", &board.port.product);
                row(
                    ui,
                    "VID / PID",
                    &format!("{} / {}", board.port.vid, board.port.pid),
                );
                row(ui, "Host instance", &board.host_instance_id);
                row(
                    ui,
                    "Capability bits",
                    &format!("0x{:08X}", board.capability_bits),
                );
                row(ui, "Controls", &board.controls.len().to_string());
                row(ui, "Peripherals", &board.peripherals.len().to_string());
                row(ui, "Timed effects", &board.macros.len().to_string());
                row(
                    ui,
                    "Lighting effects",
                    &board.strip_effects.len().to_string(),
                );
            });
    });
}

fn libraries_and_licenses(_app: &PealayerApp, ui: &mut egui::Ui) {
    section(ui, "Pealayer license", |ui| {
        ui.label("Pealayer is open-source software licensed under the MIT License.");
        ui.hyperlink_to(
            "Read the project license",
            format!("{REPOSITORY}/blob/main/LICENSE"),
        );
    });
    section(ui, "Core libraries", |ui| {
        for (index, (name, version, license, purpose)) in [
            (
                "egui / eframe",
                env!("PEALAYER_DEP_EFRAME"),
                "MIT OR Apache-2.0",
                "Native interface and rendering",
            ),
            (
                "craft-ui",
                env!("PEALAYER_DEP_CRAFT_UI"),
                "MIT OR Apache-2.0",
                "Dockable workspace and UI components",
            ),
            (
                "egui-phosphor",
                env!("PEALAYER_DEP_EGUI_PHOSPHOR"),
                "MIT",
                "Vector icon vocabulary",
            ),
            (
                "libmpv2 (Rust binding)",
                env!("PEALAYER_DEP_LIBMPV2"),
                "MIT / upstream mpv terms",
                "Media playback and video rendering",
            ),
            (
                "serde / serde_json",
                concat!(
                    env!("PEALAYER_DEP_SERDE"),
                    " / ",
                    env!("PEALAYER_DEP_SERDE_JSON")
                ),
                "MIT OR Apache-2.0",
                "Configuration and protocol data",
            ),
            (
                "tungstenite",
                env!("PEALAYER_DEP_TUNGSTENITE"),
                "MIT OR Apache-2.0",
                "WebSocket transport",
            ),
            (
                "serialport",
                env!("PEALAYER_DEP_SERIALPORT"),
                "MPL-2.0",
                "Serial device discovery and diagnostics",
            ),
            (
                "souvlaki",
                env!("PEALAYER_DEP_SOUVLAKI"),
                "MIT",
                "Desktop media controls",
            ),
            (
                "rfd",
                env!("PEALAYER_DEP_RFD"),
                "MIT",
                "Native file dialogs",
            ),
            (
                "uuid",
                env!("PEALAYER_DEP_UUID"),
                "MIT OR Apache-2.0",
                "Stable project object identifiers",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if index > 0 {
                ui.separator();
            }
            library_row(ui, name, &format!("{version} · {license}"), purpose);
        }
        #[cfg(feature = "workspace-terminal")]
        {
            ui.separator();
            library_row(ui, "PortableShell", "512c1c6 · MIT", "Embedded Pealayer shell and terminal profile import");
            ui.collapsing("PortableShell license", |ui| {
                ui.add(egui::Label::new(include_str!("../../third_party/portable-shell/LICENSE")).wrap());
            });
        }
        ui.add_space(6.0);
        ui.add(egui::Label::new(egui::RichText::new("This is a concise runtime inventory. Packaged notices and each dependency's source license remain authoritative.").small().weak()).wrap());
        ui.hyperlink_to(
            "Inspect the complete dependency manifest",
            format!("{REPOSITORY}/blob/main/Cargo.toml"),
        );
    });
}

fn library_row(ui: &mut egui::Ui, name: &str, detail: &str, purpose: &str) {
    let width = ui.available_width();
    let heading_width = (width * 0.40).clamp(128.0, 180.0);
    let heading = |ui: &mut egui::Ui| {
        ui.add(egui::Label::new(egui::RichText::new(name).strong()).wrap());
        ui.add(egui::Label::new(egui::RichText::new(detail).small().weak()).wrap());
    };
    if width < 340.0 {
        heading(ui);
        ui.add(egui::Label::new(purpose).wrap());
    } else {
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(heading_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    // allocate_ui's desired width is only a hint: reserve it
                    // explicitly or each description follows its name's width.
                    ui.set_width(heading_width);
                    heading(ui);
                },
            );
            ui.add(egui::Label::new(purpose).wrap());
        });
    }
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).heading().strong());
                ui.add_space(6.0);
                body(ui);
            });
        });
    ui.add_space(10.0);
}

fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).strong());
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
}

fn strong_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).strong());
    ui.add(egui::Label::new(egui::RichText::new(value).strong()).wrap());
    ui.end_row();
}

fn short_commit() -> &'static str {
    COMMIT.get(..12).unwrap_or(COMMIT)
}

fn commit_url() -> String {
    format!("{REPOSITORY}/commit/{COMMIT}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<&egui::epaint::TextShape> {
        fn visit<'a>(shape: &'a egui::Shape, result: &mut Vec<&'a egui::epaint::TextShape>) {
            match shape {
                egui::Shape::Text(text) => result.push(text),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        visit(shape, result);
                    }
                }
                _ => {}
            }
        }
        let mut result = Vec::new();
        for shape in shapes {
            visit(&shape.shape, &mut result);
        }
        result
    }

    #[test]
    fn about_library_descriptions_share_column_and_stack_at_narrow_widths() {
        for dark in [false, true] {
            for scale in [1.0, 1.25, 2.0] {
                for size in [14.0, 24.0] {
                    for width in [240.0, 360.0, 600.0] {
                        let ctx = egui::Context::default();
                        ctx.set_pixels_per_point(scale);
                        ctx.set_visuals(if dark {
                            egui::Visuals::dark()
                        } else {
                            egui::Visuals::light()
                        });
                        ctx.all_styles_mut(|style| {
                            style
                                .text_styles
                                .insert(egui::TextStyle::Body, egui::FontId::proportional(size));
                        });
                        let mut left = 0.0;
                        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                            ui.set_width(width);
                            left = ui.cursor().left();
                            for (name, detail, purpose) in [
                                (
                                    "egui / eframe",
                                    "0.37 · MIT OR Apache-2.0",
                                    "Native rendering",
                                ),
                                ("rfd", "0.17 · MIT", "File dialogs"),
                                (
                                    "serde / serde_json",
                                    "1.0 · MIT OR Apache-2.0",
                                    "Protocol data",
                                ),
                            ] {
                                library_row(ui, name, detail, purpose);
                                ui.separator();
                            }
                            assert!(ui.min_rect().width() <= width + 1.0);
                        });
                        let painted = texts(&output.shapes);
                        let mut previous_x: Option<f32> = None;
                        for (name, purpose) in [
                            ("egui / eframe", "Native rendering"),
                            ("rfd", "File dialogs"),
                            ("serde / serde_json", "Protocol data"),
                        ] {
                            let text = painted
                                .iter()
                                .find(|text| text.galley.text() == purpose)
                                .expect("purpose painted");
                            let heading = painted
                                .iter()
                                .find(|text| text.galley.text() == name)
                                .expect("heading painted");
                            if let Some(x) = previous_x {
                                assert!((text.pos.x - x).abs() < 0.01);
                            }
                            previous_x = Some(text.pos.x);
                            assert!(text.pos.x + text.galley.size().x <= left + width + 1.0);
                            if width >= 340.0 {
                                assert!((text.pos.y - heading.pos.y).abs() < 1.0);
                            } else {
                                assert!(text.pos.y > heading.pos.y);
                                assert!((text.pos.x - left).abs() < 1.0);
                            }
                        }
                        output.textures_delta.clear();
                    }
                }
            }
        }
    }

    #[test]
    fn about_title_and_navigation_keep_trailing_space_across_states() {
        for scale in [1.0, 1.25, 2.0] {
            for width in [240.0, 600.0] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                let label = "Libraries & licenses";
                let mut baseline = None;
                let mut close_rect = egui::Rect::NOTHING;
                for selected in [false, true, false] {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            events: if close_rect.is_positive() {
                                vec![egui::Event::PointerMoved(close_rect.center())]
                            } else {
                                vec![]
                            },
                            ..Default::default()
                        },
                        |ui| {
                            ui.set_width(width);
                            let right = ui.max_rect().right();
                            close_rect = title_row(
                                ui,
                                "About a very long application branding title",
                                "Close",
                            )
                            .rect;
                            assert!((close_rect.right() - right).abs() < 1.0);
                            let required = crate::ui::dialog::navigation_width(ui, label);
                            let response = crate::ui::dialog::navigation_button(
                                ui,
                                selected,
                                crate::ui::icons::LIST_CHECKS,
                                label,
                                required,
                            );
                            assert!(ui.min_rect().width() <= width + 1.0);
                            if let Some(previous) = baseline {
                                assert_eq!(response.rect, previous);
                            }
                            baseline = Some(response.rect);
                        },
                    );
                    let painted = texts(&output.shapes);
                    let text = painted
                        .iter()
                        .find(|text| text.galley.text() == label)
                        .expect("navigation label");
                    assert!(!text.galley.elided);
                    assert!(text.pos.x + text.galley.size().x <= baseline.unwrap().right() - 11.0);
                    output.textures_delta.clear();
                }
            }
        }
    }

    #[test]
    fn about_window_stays_bounded_for_every_tab() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.show_about_dialog = true;
        app.app_name = "Pealayer UI review".into();
        for width in [360.0, 420.0, 1000.0] {
            for tab in 0..4 {
                app.about_tab = tab;
                let ctx = egui::Context::default();
                for _ in 0..4 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, 900.0),
                            )),
                            ..Default::default()
                        },
                        |ui| draw(&mut app, ui),
                    );
                    let rect = ctx
                        .memory(|memory| {
                            memory.area_rect(egui::Id::new("pealayer_about_dialog_bounded_v2"))
                        })
                        .expect("About window");
                    assert!(
                        rect.width() <= (width - 40.0).min(680.0) + 1.0,
                        "tab={tab} width={width} rect={rect:?}"
                    );
                    assert!(
                        rect.height() <= 561.0,
                        "tab={tab} width={width} rect={rect:?}"
                    );
                    output.textures_delta.clear();
                }
            }
        }
    }

    #[test]
    fn about_escape_closes_without_editing_application_state() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.show_about_dialog = true;
        let ctx = egui::Context::default();
        for pressed in [false, true] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 800.0),
                    )),
                    events: if pressed {
                        vec![egui::Event::Key {
                            key: egui::Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
                |ui| draw(&mut app, ui),
            );
            assert_eq!(app.show_about_dialog, !pressed);
            output.textures_delta.clear();
        }
    }
}
