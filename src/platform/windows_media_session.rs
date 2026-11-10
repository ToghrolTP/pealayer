//! One HWND-owned SMTC registration. No competing global media-key listener.
use super::media_controls::{NativeMediaEvent, SessionCapabilities, SessionMetadata};
use souvlaki::{MediaControlEvent, MediaPlayback, MediaPosition, SeekDirection};
use std::{
    sync::{Arc, atomic::AtomicBool, mpsc::Sender},
    time::Duration,
};
use windows::{
    Foundation::{TimeSpan, TypedEventHandler},
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs,
        PlaybackRateChangeRequestedEventArgs, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    },
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference},
    Win32::{Foundation::HWND, System::WinRT::ISystemMediaTransportControlsInterop},
    core::HSTRING,
};

struct Apartment {
    owned: bool,
    thread: std::thread::ThreadId,
}
impl Apartment {
    fn new() -> windows::core::Result<Self> {
        use windows::Win32::{
            Foundation::RPC_E_CHANGED_MODE,
            System::WinRT::{RO_INIT_SINGLETHREADED, RoInitialize},
        };
        let owned = match unsafe { RoInitialize(RO_INIT_SINGLETHREADED) } {
            Ok(()) => true,
            // A pre-existing MTA is valid too; do not uninitialize its owner.
            Err(error) if error.code() == RPC_E_CHANGED_MODE => false,
            Err(error) => return Err(error),
        };
        Ok(Self {
            owned,
            thread: std::thread::current().id(),
        })
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.owned && self.thread == std::thread::current().id() {
            unsafe {
                windows::Win32::System::WinRT::RoUninitialize();
            }
        }
    }
}

pub(super) struct WindowsMediaSession {
    controls: SystemMediaTransportControls,
    active: Arc<AtomicBool>,
    // Remove every event, including position/rate, before disabling the session.
    buttons: Option<i64>,
    position: Option<i64>,
    rate: Option<i64>,
    artwork: Option<(u64, RandomAccessStreamReference)>,
    // Declared last: release every WinRT handle before our apartment reference.
    _apartment: Apartment,
}

impl WindowsMediaSession {
    pub fn new(
        hwnd: isize,
        active: Arc<AtomicBool>,
        tx: Sender<NativeMediaEvent>,
        ctx: eframe::egui::Context,
    ) -> windows::core::Result<Self> {
        let apartment = Apartment::new()?;
        let interop: ISystemMediaTransportControlsInterop = windows::core::factory::<
            SystemMediaTransportControls,
            ISystemMediaTransportControlsInterop,
        >()?;
        let controls = unsafe { interop.GetForWindow(HWND(hwnd as *mut _)) }?;
        let mut session = Self {
            controls,
            active: active.clone(),
            buttons: None,
            position: None,
            rate: None,
            artwork: None,
            _apartment: apartment,
        };
        session.controls.SetIsEnabled(true)?;
        session.buttons = Some(session.controls.ButtonPressed(&TypedEventHandler::new({
            let active = active.clone();
            let tx = tx.clone();
            let ctx = ctx.clone();
            move |_,
                  args: windows::core::Ref<
                '_,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >| {
                let Some(args) = args.as_ref() else {
                    return Ok(());
                };
                let event = match args.Button()? {
                    SystemMediaTransportControlsButton::Play => MediaControlEvent::Play,
                    SystemMediaTransportControlsButton::Pause => MediaControlEvent::Pause,
                    SystemMediaTransportControlsButton::Stop => MediaControlEvent::Stop,
                    SystemMediaTransportControlsButton::Next => MediaControlEvent::Next,
                    SystemMediaTransportControlsButton::Previous => MediaControlEvent::Previous,
                    SystemMediaTransportControlsButton::FastForward => {
                        MediaControlEvent::Seek(SeekDirection::Forward)
                    }
                    SystemMediaTransportControlsButton::Rewind => {
                        MediaControlEvent::Seek(SeekDirection::Backward)
                    }
                    _ => return Ok(()),
                };
                super::media_controls::forward_event(&active, &tx, &ctx, event.into());
                Ok(())
            }
        }))?);
        session.position = Some(session.controls.PlaybackPositionChangeRequested(
            &TypedEventHandler::new({
                let active = active.clone();
                let tx = tx.clone();
                let ctx = ctx.clone();
                move |_, args: windows::core::Ref<'_, PlaybackPositionChangeRequestedEventArgs>| {
                    let Some(args) = args.as_ref() else {
                        return Ok(());
                    };
                    let ticks = args.RequestedPlaybackPosition()?.Duration;
                    // WinRT ticks are signed: malformed/negative requests cannot wrap.
                    if let Some(position) = requested_position(ticks) {
                        let event = MediaControlEvent::SetPosition(MediaPosition(position));
                        super::media_controls::forward_event(&active, &tx, &ctx, event.into());
                    }
                    Ok(())
                }
            }),
        )?);
        session.rate = Some(session.controls.PlaybackRateChangeRequested(
            &TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, PlaybackRateChangeRequestedEventArgs>| {
                    if let Some(args) = args.as_ref() {
                        super::media_controls::forward_event(
                            &active,
                            &tx,
                            &ctx,
                            NativeMediaEvent::SetRate(args.RequestedPlaybackRate()?),
                        );
                    }
                    Ok(())
                },
            ),
        )?);
        session.capabilities(SessionCapabilities::default())?;
        Ok(session)
    }

    pub fn capabilities(&self, state: SessionCapabilities) -> windows::core::Result<()> {
        self.controls.SetIsEnabled(state.can_stop)?;
        self.controls.SetIsPlayEnabled(state.can_play)?;
        self.controls.SetIsPauseEnabled(state.can_pause)?;
        self.controls.SetIsStopEnabled(state.can_stop)?;
        self.controls.SetIsNextEnabled(state.can_next)?;
        self.controls.SetIsPreviousEnabled(state.can_previous)?;
        self.controls.SetIsFastForwardEnabled(state.can_seek)?;
        self.controls.SetIsRewindEnabled(state.can_seek)?;
        self.controls.SetIsRecordEnabled(false)?;
        self.controls.SetIsChannelUpEnabled(false)?;
        self.controls.SetIsChannelDownEnabled(false)
    }

    pub fn metadata(
        &mut self,
        metadata: &SessionMetadata,
        duration: Option<Duration>,
        artwork: Option<&[u8]>,
        artwork_revision: u64,
    ) -> windows::core::Result<()> {
        let updater = self.controls.DisplayUpdater()?;
        // Music/video properties and old artwork must not leak across files/Stop.
        updater.ClearAll()?;
        updater.SetType(if metadata.video {
            MediaPlaybackType::Video
        } else {
            MediaPlaybackType::Music
        })?;
        if metadata.video {
            let properties = updater.VideoProperties()?;
            properties.SetTitle(&HSTRING::from(
                metadata.title.as_deref().unwrap_or_default(),
            ))?;
            properties.SetSubtitle(&HSTRING::from(
                metadata.artist.as_deref().unwrap_or_default(),
            ))?;
        } else {
            let properties = updater.MusicProperties()?;
            properties.SetTitle(&HSTRING::from(
                metadata.title.as_deref().unwrap_or_default(),
            ))?;
            properties.SetArtist(&HSTRING::from(
                metadata.artist.as_deref().unwrap_or_default(),
            ))?;
            properties.SetAlbumTitle(&HSTRING::from(
                metadata.album.as_deref().unwrap_or_default(),
            ))?;
        }
        if let Some(bytes) = artwork {
            if self
                .artwork
                .as_ref()
                .is_none_or(|(revision, _)| *revision != artwork_revision)
            {
                self.artwork = Some((artwork_revision, artwork_reference(bytes)?));
            }
            updater.SetThumbnail(&self.artwork.as_ref().expect("cached artwork").1)?;
        } else {
            self.artwork = None;
        }
        updater.Update()?;
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        timeline.SetStartTime(TimeSpan { Duration: 0 })?;
        timeline.SetMinSeekTime(TimeSpan { Duration: 0 })?;
        let end = timespan(duration.unwrap_or_default());
        timeline.SetEndTime(end)?;
        timeline.SetMaxSeekTime(end)?;
        self.controls.UpdateTimelineProperties(&timeline)
    }

    pub fn playback(
        &self,
        playback: &MediaPlayback,
        duration: Option<Duration>,
    ) -> windows::core::Result<()> {
        let (status, progress) = match playback {
            MediaPlayback::Stopped => (MediaPlaybackStatus::Stopped, None),
            MediaPlayback::Paused { progress } => (MediaPlaybackStatus::Paused, *progress),
            MediaPlayback::Playing { progress } => (MediaPlaybackStatus::Playing, *progress),
        };
        self.controls.SetPlaybackStatus(status)?;
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        timeline.SetStartTime(TimeSpan { Duration: 0 })?;
        timeline.SetMinSeekTime(TimeSpan { Duration: 0 })?;
        let end = timespan(duration.unwrap_or_default());
        timeline.SetEndTime(end)?;
        timeline.SetMaxSeekTime(end)?;
        let position = progress.map(|p| p.0).unwrap_or_default();
        timeline.SetPosition(timespan(
            duration.map(|end| position.min(end)).unwrap_or(position),
        ))?;
        self.controls.UpdateTimelineProperties(&timeline)
    }

    pub fn playback_rate(&self, rate: f64) -> windows::core::Result<()> {
        self.controls.SetPlaybackRate(rate)
    }
}

fn timespan(duration: Duration) -> TimeSpan {
    TimeSpan {
        Duration: (duration.as_nanos() / 100).min(i64::MAX as u128) as i64,
    }
}

fn artwork_reference(bytes: &[u8]) -> windows::core::Result<RandomAccessStreamReference> {
    // Reused in-memory PNG, never a temporary file or HTTP artwork fetch.
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(bytes)?;
    writer.StoreAsync()?.join()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    RandomAccessStreamReference::CreateFromStream(&stream)
}

fn requested_position(ticks: i64) -> Option<Duration> {
    let ticks = u64::try_from(ticks).ok()?;
    Some(Duration::new(
        ticks / 10_000_000,
        ((ticks % 10_000_000) * 100) as u32,
    ))
}

impl Drop for WindowsMediaSession {
    fn drop(&mut self) {
        // Also covers partially attached sessions whose constructor failed.
        self.active
            .store(false, std::sync::atomic::Ordering::Release);
        if let Some(token) = self.buttons.take() {
            let _ = self.controls.RemoveButtonPressed(token);
        }
        if let Some(token) = self.position.take() {
            let _ = self.controls.RemovePlaybackPositionChangeRequested(token);
        }
        if let Some(token) = self.rate.take() {
            let _ = self.controls.RemovePlaybackRateChangeRequested(token);
        }
        if let Ok(updater) = self.controls.DisplayUpdater() {
            let _ = updater.ClearAll();
            let _ = updater.Update();
        }
        let _ = self
            .controls
            .SetPlaybackStatus(MediaPlaybackStatus::Stopped);
        let _ = self.controls.SetIsEnabled(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn winrt_timeline_ticks_preserve_fractional_positions_and_bound_overflow() {
        assert_eq!(timespan(Duration::from_millis(3429)).Duration, 34_290_000);
        assert_eq!(timespan(Duration::MAX).Duration, i64::MAX);
        assert!(requested_position(-1).is_none());
        assert_eq!(
            requested_position(34_290_000),
            Some(Duration::from_millis(3429))
        );
        assert_eq!(
            timespan(requested_position(i64::MAX).unwrap()).Duration,
            i64::MAX
        );
    }
    #[test]
    fn every_registered_callback_has_an_explicit_removal_path() {
        let source = include_str!("windows_media_session.rs");
        for event in [
            "ButtonPressed",
            "PlaybackPositionChangeRequested",
            "PlaybackRateChangeRequested",
        ] {
            assert!(source.contains(&format!("Remove{event}(token)")));
        }
        assert!(source.contains("self.active.store(false,"));
    }

    #[test]
    fn cached_artwork_reference_roundtrips_bytes_without_file_or_session_registration() {
        let _apartment = Apartment::new().unwrap();
        let bytes = b"In-memory artwork fixture";
        let reference = artwork_reference(bytes).unwrap();
        let stream = reference.OpenReadAsync().unwrap().join().unwrap();
        assert_eq!(stream.Size().unwrap(), bytes.len() as u64);
        let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream).unwrap();
        reader
            .LoadAsync(bytes.len() as u32)
            .unwrap()
            .join()
            .unwrap();
        let mut received = vec![0; bytes.len()];
        reader.ReadBytes(&mut received).unwrap();
        assert_eq!(received, bytes);
    }
}
