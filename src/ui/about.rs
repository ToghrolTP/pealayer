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

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_about_dialog {
        return;
    }
    ensure_icon(app, ui.ctx());

    let mut open = true;
    let bounds = ui.ctx().content_rect().shrink(18.0);
    let default_size = egui::vec2(bounds.width().min(640.0), bounds.height().min(500.0));
    let min_size = egui::vec2(bounds.width().min(400.0), bounds.height().min(330.0));
    egui::Window::new(format!(
        "{} {} {}",
        crate::ui::icons::INFO,
        app.tr("About"),
        app.app_name
    ))
    .open(&mut open)
    .default_size(default_size)
    .min_size(min_size)
    .max_size(bounds.size())
    .constrain_to(bounds)
    .resizable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        header(app, ui);
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        let narrow = ui.available_width() < 560.0;
        if narrow {
            ui.horizontal_wrapped(|ui| draw_tabs(app, ui, true));
            ui.separator();
            draw_content(app, ui);
        } else {
            let content_height = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(150.0, content_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| draw_tabs(app, ui, false),
                );
                ui.separator();
                draw_content(app, ui);
            });
        }
    });
    app.show_about_dialog = open;
}

fn ensure_icon(app: &mut PealayerApp, ctx: &egui::Context) {
    if app.about_icon.is_some() {
        return;
    }
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/pealayer-icon.png"))
    {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        app.about_icon = Some(ctx.load_texture(
            "pealayer_about_icon",
            image,
            egui::TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)),
        ));
    }
}

fn draw_tabs(app: &mut PealayerApp, ui: &mut egui::Ui, compact: bool) {
    for (index, (icon, label)) in TABS.into_iter().enumerate() {
        let text = if compact {
            format!("{icon} {}", app.tr(label))
        } else {
            format!("{icon}  {}", app.tr(label))
        };
        let width = if compact {
            (ui.available_width() / 2.0 - 4.0).max(118.0)
        } else {
            144.0
        };
        if ui
            .add_sized(
                [width, 34.0],
                egui::Button::new(text).selected(app.about_tab == index),
            )
            .clicked()
        {
            app.about_tab = index;
        }
    }
}

fn draw_content(app: &mut PealayerApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("about_content")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            match app.about_tab {
                0 => overview(app, ui),
                1 => build_and_system(app, ui),
                2 => connected_board(app, ui),
                _ => libraries_and_licenses(app, ui),
            }
        });
}

fn header(app: &PealayerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if let Some(icon) = &app.about_icon {
            ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(64.0, 64.0)));
        } else {
            ui.label(egui::RichText::new(crate::ui::icons::MONITOR_PLAY).size(52.0));
        }
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(&app.app_name).size(25.0).strong());
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
        });
    });
}

fn overview(app: &PealayerApp, ui: &mut egui::Ui) {
    section(ui, "Project", |ui| {
        ui.add(egui::Label::new(env!("CARGO_PKG_DESCRIPTION")).wrap());
        ui.add_space(5.0);
        ui.add(egui::Label::new("Pealayer combines media playback, a non-linear physical-effect timeline, live hardware monitoring, and PCController integration for immersive cinema production and playback.").wrap());
    });
    section(ui, "Release identity", |ui| {
        egui::Grid::new("about_release_identity")
            .num_columns(2)
            .max_col_width((ui.available_width() * 0.62).max(160.0))
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                strong_row(ui, "Version", env!("CARGO_PKG_VERSION"));
                ui.label(egui::RichText::new("Commit").strong());
                ui.hyperlink_to(COMMIT, commit_url());
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
                row(ui, "Macros", &board.macros.len().to_string());
                row(
                    ui,
                    "Strip renderers",
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
        egui::Grid::new("about_libraries")
            .num_columns(3)
            .max_col_width((ui.available_width() * 0.46).max(120.0))
            .spacing([18.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                for (name, version, license, purpose) in [
                    (
                        "egui / eframe",
                        "0.34.3",
                        "MIT OR Apache-2.0",
                        "Native interface and rendering",
                    ),
                    ("egui_dock", "0.19.1", "MIT", "Dockable workspace"),
                    ("egui-phosphor", "0.12.0", "MIT", "Vector icon vocabulary"),
                    (
                        "libmpv2 / mpv",
                        "6.0.0",
                        "MIT / upstream mpv terms",
                        "Media playback and video rendering",
                    ),
                    (
                        "serde / serde_json",
                        "1.0",
                        "MIT OR Apache-2.0",
                        "Configuration and protocol data",
                    ),
                    (
                        "tungstenite",
                        "0.30.0",
                        "MIT OR Apache-2.0",
                        "WebSocket transport",
                    ),
                    (
                        "serialport",
                        "4.10.1",
                        "MPL-2.0",
                        "Serial device discovery and diagnostics",
                    ),
                    ("souvlaki", "0.8.3", "MIT", "Desktop media controls"),
                    ("rfd", "0.17.2", "MIT", "Native file dialogs"),
                    (
                        "uuid",
                        "1.26.1",
                        "MIT OR Apache-2.0",
                        "Stable project object identifiers",
                    ),
                ] {
                    ui.label(egui::RichText::new(name).strong());
                    ui.label(format!("{version} · {license}"));
                    ui.label(purpose);
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
        ui.add(egui::Label::new(egui::RichText::new("This is a concise runtime inventory. Packaged notices and each dependency's source license remain authoritative.").small().weak()).wrap());
        ui.hyperlink_to(
            "Inspect the complete dependency manifest",
            format!("{REPOSITORY}/blob/main/Cargo.toml"),
        );
    });
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).heading().strong());
            ui.add_space(6.0);
            body(ui);
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
