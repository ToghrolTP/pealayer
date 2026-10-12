//! One native media-session path: Windows SMTC, macOS Now Playing, Linux/BSD MPRIS.
//! Never register a second set of global media-key shortcuts.
use crate::platform::interop::InteropCommand;
use souvlaki::{MediaControlEvent, MediaControls, MediaPlayback, MediaPosition, SeekDirection};
#[cfg(not(target_os = "windows"))]
use souvlaki::{MediaMetadata, PlatformConfig};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum NativeMediaEvent {
    Control(MediaControlEvent),
    SetRate(f64),
}
impl From<MediaControlEvent> for NativeMediaEvent {
    fn from(event: MediaControlEvent) -> Self {
        Self::Control(event)
    }
}
pub fn map_native_media_event(
    event: NativeMediaEvent,
    seek_seconds: f64,
) -> Option<InteropCommand> {
    match event {
        NativeMediaEvent::Control(event) => map_media_control_event(event, seek_seconds),
        NativeMediaEvent::SetRate(rate) if rate.is_finite() && (0.05..=16.0).contains(&rate) => {
            Some(InteropCommand::SetRate { rate })
        }
        NativeMediaEvent::SetRate(_) => None,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub video: bool,
}

/// Resolve factual tags from the existing cached file inspection, never probe
/// libmpv or remote artwork while painting. Omit unknown artist/album values.
pub fn session_metadata(
    target: Option<&str>,
    info: &crate::media_info::MediaFileInfo,
    video: bool,
) -> SessionMetadata {
    let Some(target) = target else {
        return SessionMetadata::default();
    };
    let tag = |names: &[&str]| {
        names.iter().find_map(|name| {
            info.filtered_metadata
                .iter()
                .chain(info.metadata.iter())
                .find(|(key, value)| key.eq_ignore_ascii_case(name) && !value.trim().is_empty())
                .map(|(_, value)| crate::media::redact_media_target(value.trim()))
        })
    };
    SessionMetadata {
        title: tag(&["title"]).or_else(|| {
            let label = if crate::media::is_remote_media_target(target) {
                url::Url::parse(target).ok().map(|url| {
                    url.path_segments()
                        .and_then(|mut parts| parts.rfind(|part| !part.is_empty()))
                        .map(str::to_owned)
                        .unwrap_or_else(|| url.host_str().unwrap_or("Media").to_owned())
                })
            } else {
                Some(crate::media::media_target_label(target))
            };
            label
        }),
        artist: tag(&["artist", "album_artist", "albumartist"]),
        album: tag(&["album"]),
        video,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionCapabilities {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_stop: bool,
    pub can_seek: bool,
    pub can_next: bool,
    pub can_previous: bool,
}
impl SessionCapabilities {
    pub fn for_player(
        loaded: bool,
        paused: bool,
        eof: bool,
        seekable: bool,
        next: bool,
        previous: bool,
    ) -> Self {
        Self {
            can_play: loaded && (paused || eof),
            can_pause: loaded && !paused && !eof,
            can_stop: loaded,
            can_seek: loaded && seekable,
            can_next: loaded && next,
            can_previous: loaded && previous,
        }
    }
}

pub fn playlist_neighbors(info: &crate::media_info::MediaFileInfo) -> (bool, bool) {
    let current = info.playlist.iter().position(|entry| {
        ["current", "playing"].iter().any(|key| {
            entry
                .properties
                .get(*key)
                .is_some_and(|value| matches!(value.as_str(), "yes" | "true" | "1"))
        })
    });
    current
        .map(|index| (index + 1 < info.playlist.len(), index > 0))
        .unwrap_or((false, false))
}

fn should_publish_playback(previous: Option<&MediaPlayback>, next: &MediaPlayback) -> bool {
    match (previous, next) {
        (
            Some(MediaPlayback::Playing {
                progress: Some(before),
            }),
            MediaPlayback::Playing {
                progress: Some(after),
            },
        ) => after.0 < before.0 || after.0.saturating_sub(before.0) >= Duration::from_millis(250),
        _ => previous != Some(next),
    }
}

/// Scope OS transport independently of local/hardware shortcuts.
/// Focus changes never create additional native registrations.
pub fn media_controls_allowed(enabled: bool, background: bool, foreground: bool) -> bool {
    enabled && (background || foreground)
}

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

pub(super) fn forward_event(
    active: &AtomicBool,
    tx: &Sender<NativeMediaEvent>,
    ctx: &eframe::egui::Context,
    event: NativeMediaEvent,
) {
    if active.load(Ordering::Acquire) && tx.send(event).is_ok() {
        ctx.request_repaint();
    }
}

pub struct MediaControlsManager {
    controls: Option<MediaControls>,
    active: Arc<AtomicBool>,
    metadata: SessionMetadata,
    last_metadata: Option<(SessionMetadata, Option<Duration>, u64)>,
    duration: Option<Duration>,
    last_playback: Option<MediaPlayback>,
    app_name: String,
    last_capabilities: Option<SessionCapabilities>,
    last_rate: Option<f64>,
    artwork_stamp: Option<(crate::branding::PlaybackIconState, u64)>,
    artwork_signature: Option<u64>,
    artwork_revision: u64,
    artwork: Option<Vec<u8>>,
    cover_url: Option<String>,
    retry_at: Option<Instant>,
    context: Option<eframe::egui::Context>,
    #[cfg(target_os = "windows")]
    windows: Option<super::windows_media_session::WindowsMediaSession>,
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    last_volume: Option<f64>,
}

impl MediaControlsManager {
    #[cfg(test)]
    pub(crate) fn memory_only_test_manager(app_name: &str) -> Self {
        Self {
            controls: None,
            active: Arc::new(AtomicBool::new(true)),
            metadata: SessionMetadata::default(),
            last_metadata: None,
            duration: None,
            last_playback: None,
            app_name: app_name.into(),
            last_capabilities: None,
            last_rate: None,
            artwork_stamp: None,
            artwork_signature: None,
            artwork_revision: 0,
            artwork: None,
            cover_url: None,
            retry_at: None,
            context: None,
            #[cfg(target_os = "windows")]
            windows: None,
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            last_volume: None,
        }
    }
    pub fn new(
        hwnd_raw: isize,
        event_tx: Sender<NativeMediaEvent>,
        egui_ctx: eframe::egui::Context,
        app_name: &str,
    ) -> Self {
        let mut manager = Self {
            controls: None,
            active: Arc::new(AtomicBool::new(true)),
            metadata: SessionMetadata::default(),
            last_metadata: None,
            duration: None,
            last_playback: None,
            app_name: app_name.to_owned(),
            last_capabilities: None,
            last_rate: None,
            artwork_stamp: None,
            artwork_signature: None,
            artwork_revision: 0,
            artwork: None,
            cover_url: None,
            retry_at: None,
            context: Some(egui_ctx.clone()),
            #[cfg(target_os = "windows")]
            windows: None,
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            last_volume: None,
        };
        #[cfg(target_os = "windows")]
        {
            if hwnd_raw == 0 {
                log::warn!("System media controls require a registered window");
                return manager;
            }
            match super::windows_media_session::WindowsMediaSession::new(
                hwnd_raw,
                manager.active.clone(),
                event_tx,
                egui_ctx,
            ) {
                Ok(session) => manager.windows = Some(session),
                Err(error) => {
                    log::warn!("Could not initialize system media controls: {error}");
                    manager.failed();
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = hwnd_raw;
            let config = PlatformConfig {
                dbus_name: "pealayer",
                display_name: app_name,
                hwnd: None,
            };
            match MediaControls::new(config) {
                Ok(mut controls) => {
                    let active = manager.active.clone();
                    match controls.attach(move |event| {
                        forward_event(&active, &event_tx, &egui_ctx, event.into())
                    }) {
                        Ok(()) => manager.controls = Some(controls),
                        Err(error) => {
                            manager.active.store(false, Ordering::Release);
                            log::warn!("Could not attach system media controls: {error}");
                            manager.failed();
                        }
                    }
                }
                Err(error) => {
                    log::warn!("Could not initialize system media controls: {error}");
                    manager.failed();
                }
            }
        }
        manager
    }

    fn failed(&mut self) {
        self.retry_at = Some(Instant::now() + Duration::from_secs(5));
        if let Some(ctx) = &self.context {
            ctx.request_repaint_after(Duration::from_secs(5));
        }
    }
    fn can_publish(&self) -> bool {
        self.retry_at.is_none_or(|when| Instant::now() >= when)
    }
    pub fn needs_registration_retry(&self) -> bool {
        // Inert test managers never initialize a real session.
        self.context.is_some()
            && self.controls.is_none()
            && {
                #[cfg(target_os = "windows")]
                {
                    self.windows.is_none()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    true
                }
            }
            && self.can_publish()
    }

    fn publish_metadata(&mut self) {
        let desired = (self.metadata.clone(), self.duration, self.artwork_revision);
        if self.last_metadata.as_ref() == Some(&desired) || !self.can_publish() {
            return;
        }
        #[cfg(target_os = "windows")]
        if let Some(session) = &mut self.windows {
            let artwork = self.metadata.title.as_ref().and(self.artwork.as_deref());
            match session.metadata(
                &self.metadata,
                self.duration,
                artwork,
                self.artwork_revision,
            ) {
                Ok(()) => {
                    self.last_metadata = Some(desired);
                    self.last_playback = None;
                    self.retry_at = None;
                }
                Err(error) => {
                    log::warn!("Publish system media metadata: {error}");
                    self.failed();
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        if let Some(controls) = &mut self.controls {
            let result = controls.set_metadata(MediaMetadata {
                title: self.metadata.title.as_deref(),
                artist: self.metadata.artist.as_deref(),
                album: self.metadata.album.as_deref(),
                cover_url: self.metadata.title.as_ref().and(self.cover_url.as_deref()),
                duration: self.duration,
            });
            match result {
                Ok(()) => {
                    self.last_metadata = Some(desired);
                    self.retry_at = None;
                }
                Err(error) => {
                    log::warn!("Publish system media metadata: {error}");
                    self.failed();
                }
            }
        }
    }

    pub fn update_metadata(&mut self, metadata: SessionMetadata) {
        self.metadata = metadata;
    }

    pub fn needs_artwork_update(
        &self,
        state: crate::branding::PlaybackIconState,
        revision: u64,
    ) -> bool {
        self.artwork_stamp != Some((state, revision))
    }

    pub fn update_artwork(
        &mut self,
        config: &crate::config::AppConfig,
        state: crate::branding::PlaybackIconState,
        revision: u64,
    ) {
        if self.artwork_stamp == Some((state, revision)) {
            return;
        }
        self.artwork_stamp = Some((state, revision));
        let signature = crate::branding::icon_source_signature(config, state);
        if self.artwork_signature == Some(signature) {
            return;
        }
        self.artwork_signature = Some(signature);
        self.artwork =
            crate::branding::icon_bytes(config, state).and_then(|(_, bytes)| artwork_png(&bytes));
        self.cover_url = crate::branding::resolved_icon_path(config, state)
            .and_then(|path| url::Url::from_file_path(path).ok())
            .map(|url| url.to_string());
        self.artwork_revision = self.artwork_revision.wrapping_add(1);
    }

    pub fn update_capabilities(&mut self, capabilities: SessionCapabilities, rate: f64) {
        if !self.can_publish() {
            return;
        }
        #[cfg(target_os = "windows")]
        if let Some(session) = &self.windows {
            if self.last_capabilities != Some(capabilities) {
                match session.capabilities(capabilities) {
                    Ok(()) => self.last_capabilities = Some(capabilities),
                    Err(error) => {
                        log::warn!("Publish system media capabilities: {error}");
                        self.failed();
                        return;
                    }
                }
            }
            if rate.is_finite() && rate > 0.0 && self.last_rate != Some(rate) {
                match session.playback_rate(rate) {
                    Ok(()) => self.last_rate = Some(rate),
                    Err(error) => {
                        log::warn!("Publish system media rate: {error}");
                        self.failed();
                    }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (capabilities, rate);
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
        }
        self.publish_metadata();
        let time = if time.is_finite() {
            self.duration
                .map(|duration| time.min(duration.as_secs_f64()))
                .unwrap_or(time)
        } else {
            time
        };
        let playback = playback_state(has_media, paused, time);
        if !self.can_publish() || !should_publish_playback(self.last_playback.as_ref(), &playback) {
            return;
        }
        #[cfg(target_os = "windows")]
        if let Some(session) = &self.windows {
            match session.playback(&playback, self.duration) {
                Ok(()) => self.last_playback = Some(playback),
                Err(error) => {
                    log::warn!("Publish system media playback: {error}");
                    self.failed();
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        if let Some(controls) = &mut self.controls {
            match controls.set_playback(playback.clone()) {
                Ok(()) => self.last_playback = Some(playback),
                Err(error) => {
                    log::warn!("Publish system media playback: {error}");
                    self.failed();
                }
            }
        }
    }

    pub fn update_volume(&mut self, volume: f64) {
        if !volume.is_finite() || !self.can_publish() {
            return;
        }
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
        #[cfg(target_os = "windows")]
        self.windows.take();
        self.controls.take(); // Souvlaki detaches on drop.
    }
}

fn artwork_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(bytes).ok()?.thumbnail(512, 512);
    let mut output = std::io::Cursor::new(Vec::new());
    image.write_to(&mut output, image::ImageFormat::Png).ok()?;
    Some(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn master_and_background_permission_cover_every_focus_state() {
        for enabled in [false, true] {
            for background in [false, true] {
                for foreground in [false, true] {
                    assert_eq!(
                        media_controls_allowed(enabled, background, foreground),
                        enabled && (background || foreground)
                    );
                }
            }
        }
        assert!(
            media_controls_allowed(true, true, false),
            "minimized/unfocused must remain usable"
        );
        assert!(
            !media_controls_allowed(true, false, false),
            "foreground-only must deny background"
        );
        assert!(
            media_controls_allowed(true, false, true),
            "foreground-only must accept foreground"
        );
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn missing_windows_window_does_not_panic_or_register() {
        let (tx, rx) = std::sync::mpsc::channel();
        let manager =
            MediaControlsManager::new(0, tx, eframe::egui::Context::default(), "Custom player");
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
        forward_event(&active, &tx, &ctx, MediaControlEvent::Toggle.into());
        assert_eq!(
            rx.try_recv().unwrap(),
            NativeMediaEvent::Control(MediaControlEvent::Toggle)
        );
        let mut manager = MediaControlsManager::memory_only_test_manager("Test player");
        manager.active = active.clone();
        drop(manager);
        forward_event(&active, &tx, &ctx, MediaControlEvent::Toggle.into());
        forward_event(&active, &tx, &ctx, NativeMediaEvent::SetRate(1.5));
        forward_event(
            &AtomicBool::new(true),
            &tx,
            &ctx,
            MediaControlEvent::Play.into(),
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            NativeMediaEvent::Control(MediaControlEvent::Play)
        );
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

    #[test]
    fn metadata_uses_real_tags_redacts_credentials_and_clears_on_stop() {
        let mut info = crate::media_info::MediaFileInfo::default();
        info.metadata
            .insert("TITLE".into(), "Embedded title".into());
        info.metadata.insert("Artist".into(), "Artist".into());
        info.metadata.insert("ALBUM".into(), "Album".into());
        info.filtered_metadata
            .insert("title".into(), "Preferred title".into());
        let metadata = session_metadata(Some("movie.mkv"), &info, true);
        assert_eq!(metadata.title.as_deref(), Some("Preferred title"));
        assert_eq!(metadata.artist.as_deref(), Some("Artist"));
        assert_eq!(metadata.album.as_deref(), Some("Album"));
        assert!(metadata.video);
        assert_eq!(
            session_metadata(None, &info, true),
            SessionMetadata::default()
        );
        let metadata = session_metadata(
            Some("https://user:secret@example.org/movie?token=private"),
            &crate::media_info::MediaFileInfo::default(),
            false,
        );
        assert_eq!(metadata.title.as_deref(), Some("movie"));
        assert!(metadata.artist.is_none() && metadata.album.is_none());
    }

    #[test]
    fn contextual_capabilities_and_playlist_boundaries_are_not_invented() {
        assert_eq!(
            SessionCapabilities::for_player(false, false, false, true, true, true),
            SessionCapabilities::default()
        );
        let playing = SessionCapabilities::for_player(true, false, false, true, true, false);
        assert!(
            !playing.can_play
                && playing.can_pause
                && playing.can_seek
                && playing.can_next
                && !playing.can_previous
        );
        let eof = SessionCapabilities::for_player(true, false, true, false, false, true);
        assert!(eof.can_play && !eof.can_pause && !eof.can_seek && eof.can_previous);
        let mut info = crate::media_info::MediaFileInfo::default();
        assert_eq!(playlist_neighbors(&info), (false, false));
        for index in 0..3 {
            info.playlist.push(crate::media_info::MediaCollectionEntry {
                index,
                properties: Default::default(),
            });
        }
        for (index, expected) in [(0, (true, false)), (1, (true, true)), (2, (false, true))] {
            for entry in &mut info.playlist {
                entry.properties.clear();
            }
            info.playlist[index]
                .properties
                .insert("current".into(), "yes".into());
            assert_eq!(playlist_neighbors(&info), expected);
        }
    }

    #[test]
    fn position_publication_is_bounded_but_transitions_and_backward_seeks_are_immediate() {
        let playing = |millis| MediaPlayback::Playing {
            progress: Some(MediaPosition(Duration::from_millis(millis))),
        };
        let last = playing(1000);
        for millis in [1000, 1016, 1100, 1249] {
            assert!(!should_publish_playback(Some(&last), &playing(millis)));
        }
        assert!(should_publish_playback(Some(&last), &playing(1250)));
        assert!(should_publish_playback(Some(&last), &playing(999)));
        assert!(should_publish_playback(
            Some(&last),
            &MediaPlayback::Stopped
        ));
        assert!(should_publish_playback(
            Some(&last),
            &MediaPlayback::Paused {
                progress: Some(MediaPosition(Duration::from_millis(1016)))
            }
        ));
    }

    #[test]
    fn native_rate_requests_use_shared_validation_without_volume_key_capture() {
        assert_eq!(
            map_native_media_event(NativeMediaEvent::SetRate(1.25), 10.0),
            Some(InteropCommand::SetRate { rate: 1.25 })
        );
        for rate in [f64::NAN, f64::INFINITY, -1.0, 0.0, 17.0] {
            assert_eq!(
                map_native_media_event(NativeMediaEvent::SetRate(rate), 10.0),
                None
            );
        }
    }

    #[test]
    fn session_artwork_preserves_alpha_and_is_bounded_without_disk_output() {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1024,
            768,
            image::Rgba([10, 130, 220, 128]),
        ));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let png = artwork_png(bytes.get_ref()).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().into_rgba8();
        assert!(decoded.width() <= 512 && decoded.height() <= 512);
        assert_eq!(decoded.get_pixel(0, 0).0, [10, 130, 220, 128]);
        assert!(artwork_png(b"invalid").is_none());
    }
}
