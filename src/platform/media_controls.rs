//! One native media-session path: Windows SMTC, macOS Now Playing, Linux/BSD MPRIS.
//! Never register a second set of global media-key shortcuts.
use crate::platform::interop::InteropCommand;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
use std::time::Duration;

pub fn map_media_control_event(
    event: MediaControlEvent,
    seek_seconds: f64,
) -> Option<InteropCommand> {
    let seek_seconds = if seek_seconds.is_finite() {
        seek_seconds.max(0.0)
    } else {
        10.0
    };
    Some(match event {
        MediaControlEvent::Play => InteropCommand::Play,
        MediaControlEvent::Pause => InteropCommand::Pause,
        MediaControlEvent::Toggle => InteropCommand::TogglePause,
        MediaControlEvent::Next => InteropCommand::Next,
        MediaControlEvent::Previous => InteropCommand::Previous,
        MediaControlEvent::Stop => InteropCommand::Stop,
        MediaControlEvent::Seek(direction) => InteropCommand::Seek {
            seconds: directed_seconds(direction, seek_seconds),
        },
        MediaControlEvent::SeekBy(direction, duration) => InteropCommand::Seek {
            seconds: directed_seconds(direction, duration.as_secs_f64()),
        },
        MediaControlEvent::SetPosition(position) => InteropCommand::SeekTo {
            seconds: position.0.as_secs_f64(),
        },
        MediaControlEvent::SetVolume(volume) if volume.is_finite() => InteropCommand::SetVolume {
            value: volume.clamp(0.0, 1.3) * 100.0,
        },
        MediaControlEvent::SetVolume(_) => return None,
        MediaControlEvent::OpenUri(target) => InteropCommand::Open { target },
        MediaControlEvent::Raise => InteropCommand::Activate,
        MediaControlEvent::Quit => InteropCommand::Quit,
    })
}

fn directed_seconds(direction: SeekDirection, seconds: f64) -> f64 {
    match direction {
        SeekDirection::Forward => seconds,
        SeekDirection::Backward => -seconds,
    }
}

fn playback_state(has_media: bool, is_paused: bool, time: f64) -> MediaPlayback {
    let progress = if time.is_finite() && time >= 0.0 {
        Duration::try_from_secs_f64(time).ok().map(MediaPosition)
    } else {
        None
    };
    if !has_media {
        MediaPlayback::Stopped
    } else if is_paused {
        MediaPlayback::Paused { progress }
    } else {
        MediaPlayback::Playing { progress }
    }
}

fn forward_event(
    active: &AtomicBool,
    tx: &Sender<MediaControlEvent>,
    ctx: &eframe::egui::Context,
    event: MediaControlEvent,
) {
    if active.load(Ordering::Acquire) && tx.send(event).is_ok() {
        ctx.request_repaint();
    }
}

pub struct MediaControlsManager {
    controls: Option<MediaControls>,
    active: Arc<AtomicBool>,
    title: Option<String>,
    duration: Option<Duration>,
    last_playback: Option<MediaPlayback>,
    app_name: String,
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    last_volume: Option<f64>,
}

impl MediaControlsManager {
    pub fn new(
        hwnd_raw: isize,
        event_tx: Sender<MediaControlEvent>,
        egui_ctx: eframe::egui::Context,
        app_name: &str,
    ) -> Self {
        let mut manager = Self {
            controls: None,
            active: Arc::new(AtomicBool::new(true)),
            title: None,
            duration: None,
            last_playback: None,
            app_name: app_name.to_owned(),
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            last_volume: None,
        };
        #[cfg(target_os = "windows")]
        let hwnd = {
            if hwnd_raw == 0 {
                log::warn!("System media controls require a registered window");
                return manager;
            }
            Some(hwnd_raw as *mut std::ffi::c_void)
        };
        #[cfg(not(target_os = "windows"))]
        let hwnd = {
            let _ = hwnd_raw;
            None
        };
        let config = PlatformConfig {
            dbus_name: "pealayer",
            display_name: app_name,
            hwnd,
        };
        match MediaControls::new(config) {
            Ok(mut controls) => {
                let active = manager.active.clone();
                match controls
                    .attach(move |event| forward_event(&active, &event_tx, &egui_ctx, event))
                {
                    Ok(()) => manager.controls = Some(controls),
                    Err(error) => log::warn!("Could not attach system media controls: {error}"),
                }
            }
            Err(error) => log::warn!("Could not initialize system media controls: {error}"),
        }
        manager
    }

    fn publish_metadata(&mut self) {
        if let Some(controls) = &mut self.controls {
            let _ = controls.set_metadata(MediaMetadata {
                title: Some(self.title.as_deref().unwrap_or(&self.app_name)),
                album: Some(&self.app_name),
                duration: self.duration,
                ..Default::default()
            });
        }
    }

    pub fn update_metadata(&mut self, title: Option<&str>) {
        self.title = title.map(str::to_owned);
        self.publish_metadata();
    }

    pub fn has_branding(&self, app_name: &str) -> bool {
        self.app_name == app_name
    }

    pub fn update_playback(&mut self, has_media: bool, paused: bool, time: f64, duration: f64) {
        // Unknown-duration and live media must not be advertised as stopped.
        let duration = if has_media && duration.is_finite() && duration > 0.0 {
            Duration::try_from_secs_f64(duration).ok()
        } else {
            None
        };
        if self.duration != duration {
            self.duration = duration;
            self.publish_metadata();
        }
        let playback = playback_state(has_media, paused, time);
        if self.last_playback.as_ref() != Some(&playback)
            && let Some(controls) = &mut self.controls
            && controls.set_playback(playback.clone()).is_ok()
        {
            self.last_playback = Some(playback);
        }
    }

    pub fn update_volume(&mut self, volume: f64) {
        // MPRIS requires acknowledging volume changes. Windows/macOS system
        // volume keys remain OS-owned, not globally captured by Pealayer.
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        ))]
        if self.last_volume != Some(volume)
            && let Some(controls) = &mut self.controls
            && controls.set_volume(volume).is_ok()
        {
            self.last_volume = Some(volume);
        }
        #[cfg(not(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        )))]
        let _ = volume;
    }
}

impl Drop for MediaControlsManager {
    fn drop(&mut self) {
        // Give every native registration a separate token: callbacks retained
        // by an OS backend cannot revive after disabling and re-enabling.
        self.active.store(false, Ordering::Release);
        self.controls.take(); // Souvlaki detaches on drop.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    #[test]
    fn missing_windows_window_does_not_panic_or_register() {
        let (tx, rx) = std::sync::mpsc::channel();
        let manager = MediaControlsManager::new(0, tx, eframe::egui::Context::default(), "Custom player");
        assert!(manager.controls.is_none());
        assert!(rx.try_recv().is_err());
    }
    #[test]
    fn transport_media_keys_use_shared_commands() {
        for (event, command) in [
            (MediaControlEvent::Play, InteropCommand::Play),
            (MediaControlEvent::Pause, InteropCommand::Pause),
            (MediaControlEvent::Toggle, InteropCommand::TogglePause),
            (MediaControlEvent::Stop, InteropCommand::Stop),
            (MediaControlEvent::Next, InteropCommand::Next),
            (MediaControlEvent::Previous, InteropCommand::Previous),
            (MediaControlEvent::Raise, InteropCommand::Activate),
            (MediaControlEvent::Quit, InteropCommand::Quit),
        ] {
            assert_eq!(map_media_control_event(event, 7.5), Some(command));
        }
    }
    #[test]
    fn seek_and_volume_events_preserve_native_values() {
        assert_eq!(
            map_media_control_event(MediaControlEvent::Seek(SeekDirection::Backward), 7.5),
            Some(InteropCommand::Seek { seconds: -7.5 })
        );
        assert_eq!(
            map_media_control_event(
                MediaControlEvent::SeekBy(SeekDirection::Forward, Duration::from_millis(1250)),
                10.0
            ),
            Some(InteropCommand::Seek { seconds: 1.25 })
        );
        assert_eq!(
            map_media_control_event(
                MediaControlEvent::SetPosition(MediaPosition(Duration::from_millis(3429))),
                10.0
            ),
            Some(InteropCommand::SeekTo { seconds: 3.429 })
        );
        assert_eq!(
            map_media_control_event(MediaControlEvent::SetVolume(0.42), 10.0),
            Some(InteropCommand::SetVolume { value: 42.0 })
        );
        assert_eq!(
            map_media_control_event(MediaControlEvent::SetVolume(f64::NAN), 10.0),
            None
        );
        assert_eq!(
            map_media_control_event(
                MediaControlEvent::OpenUri("https://example.org/video.mkv".into()),
                10.0
            ),
            Some(InteropCommand::Open {
                target: "https://example.org/video.mkv".into()
            })
        );
    }
    #[test]
    fn disabled_or_stale_callbacks_cannot_enqueue_actions() {
        let (tx, rx) = std::sync::mpsc::channel();
        let active = Arc::new(AtomicBool::new(true));
        let ctx = eframe::egui::Context::default();
        forward_event(&active, &tx, &ctx, MediaControlEvent::Toggle);
        assert_eq!(rx.try_recv().unwrap(), MediaControlEvent::Toggle);
        let manager = MediaControlsManager {
            controls: None,
            active: active.clone(),
            title: None,
            duration: None,
            last_playback: None,
            app_name: String::new(),
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            last_volume: None,
        };
        drop(manager);
        forward_event(&active, &tx, &ctx, MediaControlEvent::Toggle);
        forward_event(&AtomicBool::new(true), &tx, &ctx, MediaControlEvent::Play);
        assert_eq!(rx.try_recv().unwrap(), MediaControlEvent::Play);
        assert!(rx.try_recv().is_err());
    }
    #[test]
    fn live_media_and_paused_media_have_truthful_os_state() {
        assert_eq!(playback_state(false, false, 0.0), MediaPlayback::Stopped);
        assert!(matches!(
            playback_state(true, false, 0.0),
            MediaPlayback::Playing { .. }
        ));
        assert!(matches!(
            playback_state(true, true, 3.0),
            MediaPlayback::Paused { .. }
        ));
        assert_eq!(
            playback_state(true, true, f64::INFINITY),
            MediaPlayback::Paused { progress: None }
        );
    }
}
