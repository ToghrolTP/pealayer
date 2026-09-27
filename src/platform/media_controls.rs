use crate::platform::interop::InteropCommand;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub fn map_media_control_event(event: MediaControlEvent) -> Option<InteropCommand> {
    match event {
        MediaControlEvent::Play => Some(InteropCommand::Play),
        MediaControlEvent::Pause => Some(InteropCommand::Pause),
        MediaControlEvent::Toggle => Some(InteropCommand::TogglePause),
        MediaControlEvent::Next => Some(InteropCommand::Seek { seconds: 10.0 }),
        MediaControlEvent::Previous => Some(InteropCommand::Seek { seconds: -10.0 }),
        MediaControlEvent::Stop => Some(InteropCommand::Pause),
        _ => None,
    }
}

pub struct MediaControlsManager {
    controls: Option<MediaControls>,
}

impl MediaControlsManager {
    pub fn new(
        hwnd_raw: isize,
        cmd_tx: Sender<InteropCommand>,
        egui_ctx: eframe::egui::Context,
    ) -> Self {
        #[cfg(target_os = "windows")]
        let hwnd = if hwnd_raw != 0 {
            Some(hwnd_raw as *mut std::ffi::c_void)
        } else {
            None
        };

        #[cfg(not(target_os = "windows"))]
        let hwnd = {
            let _ = hwnd_raw;
            None
        };

        let config = PlatformConfig {
            dbus_name: "pealayer",
            display_name: "Pealayer",
            hwnd,
        };

        let mut controls = match MediaControls::new(config) {
            Ok(c) => Some(c),
            Err(e) => {
                log::warn!("Could not initialize system media controls: {:?}", e);
                None
            }
        };

        if let Some(ref mut c) = controls {
            let tx = cmd_tx.clone();
            let ctx = egui_ctx.clone();
            if let Err(e) = c.attach(move |event: MediaControlEvent| {
                if let Some(cmd) = map_media_control_event(event) {
                    let _ = tx.send(cmd);
                    ctx.request_repaint();
                }
            }) {
                log::warn!("Failed to attach system media controls listener: {:?}", e);
            }
        }

        Self { controls }
    }

    pub fn update_metadata(&mut self, title: Option<&str>) {
        if let Some(ref mut controls) = self.controls {
            let _ = controls.set_metadata(MediaMetadata {
                title,
                album: Some("Pealayer"),
                ..Default::default()
            });
        }
    }

    pub fn update_playback(&mut self, is_paused: bool, playback_time: f64, duration: f64) {
        if let Some(ref mut controls) = self.controls {
            let pos = if playback_time >= 0.0 {
                Some(MediaPosition(Duration::from_secs_f64(playback_time)))
            } else {
                None
            };

            let playback = if duration <= 0.0 {
                MediaPlayback::Stopped
            } else if is_paused {
                MediaPlayback::Paused { progress: pos }
            } else {
                MediaPlayback::Playing { progress: pos }
            };

            let _ = controls.set_playback(playback);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_media_control_event() {
        assert!(matches!(
            map_media_control_event(MediaControlEvent::Play),
            Some(InteropCommand::Play)
        ));
        assert!(matches!(
            map_media_control_event(MediaControlEvent::Pause),
            Some(InteropCommand::Pause)
        ));
        assert!(matches!(
            map_media_control_event(MediaControlEvent::Toggle),
            Some(InteropCommand::TogglePause)
        ));
        assert!(matches!(
            map_media_control_event(MediaControlEvent::Stop),
            Some(InteropCommand::Pause)
        ));
        if let Some(InteropCommand::Seek { seconds }) =
            map_media_control_event(MediaControlEvent::Next)
        {
            assert_eq!(seconds, 10.0);
        } else {
            panic!("Expected Seek for Next");
        }
        if let Some(InteropCommand::Seek { seconds }) =
            map_media_control_event(MediaControlEvent::Previous)
        {
            assert_eq!(seconds, -10.0);
        } else {
            panic!("Expected Seek for Previous");
        }
    }
}
