#![cfg_attr(windows, windows_subsystem = "windows")]
mod icon;
struct Downloader {
    manager: Result<pealayer_downloads::Manager, String>,
    view: pealayer_downloads::ui::View,
    error: Option<String>,
}
impl eframe::App for Downloader {
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        if let Ok(manager) = &self.manager {
            let _ = manager.shutdown(std::time::Duration::from_secs(7));
        }
    }
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(eframe::egui::Color32::LIGHT_RED, error);
            }
            match &self.manager {
                Err(error) => {
                    ui.label(error);
                }
                Ok(manager) => {
                    if let Some(path) = self.view.draw(ui, manager) {
                        #[cfg(windows)]
                        let player = std::env::var_os("LOCALAPPDATA").map(|p| {
                            std::path::PathBuf::from(p).join("Programs/Pealayer/bin/pealayer.exe")
                        });
                        #[cfg(not(windows))]
                        let player = Some(std::path::PathBuf::from("pealayer"));
                        match player {
                            Some(player) => {
                                let mut command = std::process::Command::new(player);
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    command.creation_flags(0x0800_0000);
                                }
                                if command.arg(path).spawn().is_err() {
                                    self.error = Some(
                                        "Could not launch Pealayer; verify its installation".into(),
                                    );
                                }
                            }
                            None => {
                                self.error = Some("Pealayer installation is unavailable".into())
                            }
                        }
                    }
                }
            }
        });
    }
}
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([840.0, 620.0])
            .with_min_inner_size([440.0, 360.0])
            .with_app_id("Pealayer.Downloader")
            .with_icon(eframe::egui::IconData {
                rgba: icon::rgba(64),
                width: 64,
                height: 64,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Pealayer Downloader",
        options,
        Box::new(|creation| {
            let mut fonts = eframe::egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            creation.egui_ctx.set_fonts(fonts);
            creation
                .egui_ctx
                .set_theme(eframe::egui::ThemePreference::System);
            Ok(Box::new(Downloader {
                manager: pealayer_downloads::default_root()
                    .and_then(pealayer_downloads::Manager::open),
                view: Default::default(),
                error: None,
            }))
        }),
    )
}
