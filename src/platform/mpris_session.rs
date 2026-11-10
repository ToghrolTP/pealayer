//! Application-owned MPRIS, as consumed by KDE Connect/Plasma/playerctl.
//! OS I/O stays on a worker; a one-slot mailbox coalesces factual snapshots.
use super::media_controls::{NativeMediaEvent, SessionCapabilities, SessionMetadata};
use souvlaki::{MediaControlEvent, MediaPlayback, MediaPosition, SeekDirection};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use zbus::{
    blocking::Connection,
    zvariant::{ObjectPath, OwnedValue, Value},
};

const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Snapshot {
    pub metadata: SessionMetadata,
    pub duration: Option<Duration>,
    pub artwork: Option<String>,
    pub playback: MediaPlayback,
    pub capabilities: SessionCapabilities,
    pub rate: f64,
    pub volume: f64,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            metadata: Default::default(),
            duration: None,
            artwork: None,
            playback: MediaPlayback::Stopped,
            capabilities: Default::default(),
            rate: 1.0,
            volume: 1.0,
        }
    }
}
impl Snapshot {
    fn status(&self) -> &'static str {
        match self.playback {
            MediaPlayback::Playing { .. } => "Playing",
            MediaPlayback::Paused { .. } => "Paused",
            MediaPlayback::Stopped => "Stopped",
        }
    }
    fn position(&self, elapsed: Duration) -> i64 {
        let (position, playing) = match self.playback {
            MediaPlayback::Playing { progress } => (progress, true),
            MediaPlayback::Paused { progress } => (progress, false),
            MediaPlayback::Stopped => return 0,
        };
        let base = position.map(|p| p.0).unwrap_or_default();
        let advance = if playing {
            Duration::try_from_secs_f64(elapsed.as_secs_f64() * self.rate).unwrap_or(Duration::MAX)
        } else {
            Duration::ZERO
        };
        let position = base.saturating_add(advance);
        micros(
            self.duration
                .map(|end| position.min(end))
                .unwrap_or(position),
        )
    }
    fn metadata_map(&self) -> HashMap<&str, OwnedValue> {
        let mut map = HashMap::new();
        if let Some(track) = &self.metadata.track_id {
            if let Ok(path) = ObjectPath::try_from(track.as_str()) {
                map.insert("mpris:trackid", Value::from(path).try_into().unwrap());
            }
            if let Some(duration) = self.duration {
                map.insert("mpris:length", OwnedValue::from(micros(duration)));
            }
            if let Some(art) = &self.artwork {
                map.insert(
                    "mpris:artUrl",
                    Value::from(art.as_str()).try_into().unwrap(),
                );
            }
        }
        for (key, text) in [
            ("xesam:title", &self.metadata.title),
            ("xesam:album", &self.metadata.album),
        ] {
            if let Some(text) = text {
                map.insert(key, Value::from(text.as_str()).try_into().unwrap());
            }
        }
        if let Some(artist) = &self.metadata.artist {
            map.insert(
                "xesam:artist",
                Value::from(vec![artist.as_str()]).try_into().unwrap(),
            );
        }
        map
    }
}
fn micros(duration: Duration) -> i64 {
    duration.as_micros().min(i64::MAX as u128) as i64
}

struct Published {
    snapshot: Snapshot,
    at: Instant,
}
fn seeked_position(before: &Published, next: &Published) -> Option<i64> {
    let position = next.snapshot.position(Duration::ZERO);
    let changed_track = before.snapshot.metadata.track_id != next.snapshot.metadata.track_id;
    let unexpected_start =
        changed_track && next.snapshot.metadata.track_id.is_some() && position > 0;
    let discrepancy = before
        .snapshot
        .position(next.at.saturating_duration_since(before.at))
        .abs_diff(position);
    let paused_seek = before.snapshot.status() == "Paused"
        && next.snapshot.status() == "Paused"
        && before.snapshot.playback != next.snapshot.playback;
    (unexpected_start || (!changed_track && (paused_seek || discrepancy > 100_000)))
        .then_some(position)
}
struct Shared {
    current: Mutex<Published>,
    pending: Mutex<Option<Published>>,
    failure: Mutex<Option<Instant>>,
    shutdown: AtomicBool,
    active: Arc<AtomicBool>,
    tx: mpsc::Sender<NativeMediaEvent>,
    ctx: eframe::egui::Context,
}
impl Shared {
    fn emit(&self, event: impl Into<NativeMediaEvent>) {
        super::media_controls::forward_event(&self.active, &self.tx, &self.ctx, event.into());
    }
    fn snapshot(&self) -> Snapshot {
        self.current.lock().unwrap().snapshot.clone()
    }
    fn position(&self) -> i64 {
        let value = self.current.lock().unwrap();
        value.snapshot.position(value.at.elapsed())
    }
    fn fail(&self, error: &dyn std::fmt::Display) {
        log::warn!("MPRIS session unavailable: {error}");
        self.active.store(false, Ordering::Release);
        *self.failure.lock().unwrap() = Some(Instant::now());
        self.ctx.request_repaint_after(Duration::from_secs(5));
    }
}
struct Root {
    shared: Arc<Shared>,
    name: String,
}
#[zbus::interface(name = "org.mpris.MediaPlayer2", spawn = false)]
impl Root {
    fn raise(&self) {
        self.shared.emit(MediaControlEvent::Raise);
    }
    fn quit(&self) {
        self.shared.emit(MediaControlEvent::Quit);
    }
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }
    #[zbus(property, name = "HasTrackList")]
    fn has_track_list(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn identity(&self) -> &str {
        &self.name
    }
    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "pealayer"
    }
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec!["file", "http", "https", "rtsp", "rtsps"]
    }
    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![]
    }
}
struct Player {
    shared: Arc<Shared>,
}
#[zbus::interface(name = "org.mpris.MediaPlayer2.Player", spawn = false)]
impl Player {
    #[zbus(signal)]
    async fn seeked(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        position: i64,
    ) -> zbus::Result<()>;

    fn play(&self) {
        if self.shared.snapshot().capabilities.can_stop {
            self.shared.emit(MediaControlEvent::Play);
        }
    }
    fn pause(&self) {
        if self.shared.snapshot().capabilities.can_stop {
            self.shared.emit(MediaControlEvent::Pause);
        }
    }
    fn play_pause(&self) {
        if self.shared.snapshot().capabilities.can_stop {
            self.shared.emit(MediaControlEvent::Toggle);
        }
    }
    fn stop(&self) {
        if self.shared.snapshot().capabilities.can_stop {
            self.shared.emit(MediaControlEvent::Stop);
        }
    }
    fn next(&self) {
        if self.shared.snapshot().capabilities.can_next {
            self.shared.emit(MediaControlEvent::Next);
        }
    }
    fn previous(&self) {
        if self.shared.snapshot().capabilities.can_previous {
            self.shared.emit(MediaControlEvent::Previous);
        }
    }
    fn seek(&self, offset: i64) {
        if self.shared.snapshot().capabilities.can_seek {
            self.shared.emit(MediaControlEvent::SeekBy(
                if offset >= 0 {
                    SeekDirection::Forward
                } else {
                    SeekDirection::Backward
                },
                Duration::from_micros(offset.unsigned_abs()),
            ));
        }
    }
    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        let state = self.shared.snapshot();
        if !state.capabilities.can_seek
            || state.metadata.track_id.as_deref() != Some(track_id.as_str())
            || position < 0
            || state.duration.is_some_and(|d| position > micros(d))
        {
            return;
        }
        self.shared
            .emit(MediaControlEvent::SetPosition(MediaPosition(
                Duration::from_micros(position as u64),
            )));
    }
    fn open_uri(&self, uri: &str) -> zbus::fdo::Result<()> {
        let url = url::Url::parse(uri).map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        if !["file", "http", "https", "rtsp", "rtsps"].contains(&url.scheme()) {
            return Err(zbus::fdo::Error::NotSupported(
                "Unsupported URI scheme".into(),
            ));
        }
        self.shared.emit(MediaControlEvent::OpenUri(uri.into()));
        Ok(())
    }
    #[zbus(property)]
    fn playback_status(&self) -> &str {
        self.shared.snapshot().status()
    }
    #[zbus(property)]
    fn rate(&self) -> f64 {
        self.shared.snapshot().rate
    }
    #[zbus(property)]
    fn set_rate(&self, rate: f64) -> zbus::fdo::Result<()> {
        if !rate.is_finite() || (rate != 0.0 && !(0.05..=16.0).contains(&rate)) {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Unsupported playback rate".into(),
            ));
        }
        if self.shared.snapshot().capabilities.can_stop {
            if rate == 0.0 {
                self.shared.emit(MediaControlEvent::Pause);
            } else {
                self.shared.emit(NativeMediaEvent::SetRate(rate));
            }
        }
        Ok(())
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.shared
            .snapshot()
            .metadata_map()
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect()
    }
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.shared.snapshot().volume
    }
    #[zbus(property)]
    fn set_volume(&self, volume: f64) -> zbus::fdo::Result<()> {
        if !volume.is_finite() {
            return Err(zbus::fdo::Error::InvalidArgs("Invalid volume".into()));
        }
        if self.shared.snapshot().capabilities.can_stop {
            self.shared
                .emit(MediaControlEvent::SetVolume(volume.clamp(0.0, 1.3)));
        }
        Ok(())
    }
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        self.shared.position()
    }
    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        0.05
    }
    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        16.0
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.shared.snapshot().capabilities.can_next
    }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.shared.snapshot().capabilities.can_previous
    }
    // MPRIS expresses abilities, not which single button is currently useful.
    // In particular CanPause must stay true while paused so PlayPause resumes.
    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.shared.snapshot().capabilities.can_stop
    }
    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.shared.snapshot().capabilities.can_stop
    }
    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.shared.snapshot().capabilities.can_seek
    }
    #[zbus(property)]
    fn can_control(&self) -> bool {
        self.shared.snapshot().capabilities.can_stop
    }
}

pub(super) struct MprisSession {
    shared: Arc<Shared>,
    wake: mpsc::SyncSender<()>,
    last: Option<Snapshot>,
}
impl MprisSession {
    pub fn new(
        name: &str,
        active: Arc<AtomicBool>,
        tx: mpsc::Sender<NativeMediaEvent>,
        ctx: eframe::egui::Context,
    ) -> Self {
        let shared = Arc::new(Shared {
            current: Mutex::new(Published {
                snapshot: Snapshot::default(),
                at: Instant::now(),
            }),
            pending: Mutex::new(None),
            failure: Mutex::new(None),
            shutdown: AtomicBool::new(false),
            active,
            tx,
            ctx,
        });
        let (wake, receive) = mpsc::sync_channel(1);
        let worker = shared.clone();
        let name = name.to_owned();
        std::thread::spawn(move || {
            if worker.shutdown.load(Ordering::Acquire) {
                return;
            }
            let connection = zbus::blocking::connection::Builder::session()
                .and_then(|builder| {
                    builder.name(format!(
                        "org.mpris.MediaPlayer2.pealayer.instance{}",
                        std::process::id()
                    ))
                })
                .and_then(|builder| {
                    builder.serve_at(
                        PATH,
                        Root {
                            shared: worker.clone(),
                            name,
                        },
                    )
                })
                .and_then(|builder| {
                    builder.serve_at(
                        PATH,
                        Player {
                            shared: worker.clone(),
                        },
                    )
                })
                .and_then(|builder| builder.build());
            let connection = match connection {
                Ok(c) => c,
                Err(e) => {
                    worker.fail(&e);
                    return;
                }
            };
            while receive.recv().is_ok() {
                if worker.shutdown.load(Ordering::Acquire) {
                    break;
                }
                let Some(next) = worker.pending.lock().unwrap().take() else {
                    continue;
                };
                let old = {
                    let mut current = worker.current.lock().unwrap();
                    std::mem::replace(&mut *current, next)
                };
                if let Err(error) = publish_changes(&connection, &worker, &old) {
                    worker.fail(&error);
                    break;
                }
            }
            // Dropping this sole connection relinquishes the bus name. No timer
            // or polling survives shutdown and callback guards fail closed.
        });
        Self {
            shared,
            wake,
            last: None,
        }
    }
    pub fn failed_for_retry(&self) -> bool {
        self.shared
            .failure
            .lock()
            .unwrap()
            .is_some_and(|at| at.elapsed() >= Duration::from_secs(5))
    }
    pub fn publish(&mut self, snapshot: Snapshot, observed: Instant) {
        if self.last.as_ref() == Some(&snapshot) {
            return;
        }
        self.last = Some(snapshot.clone());
        *self.shared.pending.lock().unwrap() = Some(Published {
            snapshot,
            at: observed,
        });
        let _ = self.wake.try_send(());
    }
}
impl Drop for MprisSession {
    fn drop(&mut self) {
        self.shared.active.store(false, Ordering::Release);
        self.shared.shutdown.store(true, Ordering::Release);
        let _ = self.wake.try_send(());
    }
}

fn publish_changes(
    connection: &Connection,
    shared: &Shared,
    before: &Published,
) -> zbus::Result<()> {
    let current = shared.current.lock().unwrap();
    let next = &current.snapshot;
    let old = &before.snapshot;
    let mut changed = HashMap::<&str, OwnedValue>::new();
    if old.metadata != next.metadata || old.duration != next.duration || old.artwork != next.artwork
    {
        changed.insert(
            "Metadata",
            Value::from(next.metadata_map()).try_into().unwrap(),
        );
    }
    if old.status() != next.status() {
        changed.insert(
            "PlaybackStatus",
            Value::from(next.status()).try_into().unwrap(),
        );
    }
    if old.rate != next.rate {
        changed.insert("Rate", OwnedValue::from(next.rate));
    }
    if old.volume != next.volume {
        changed.insert("Volume", OwnedValue::from(next.volume));
    }
    if old.capabilities != next.capabilities {
        for (name, value) in [
            ("CanPlay", next.capabilities.can_stop),
            ("CanPause", next.capabilities.can_stop),
            ("CanControl", next.capabilities.can_stop),
            ("CanGoNext", next.capabilities.can_next),
            ("CanGoPrevious", next.capabilities.can_previous),
            ("CanSeek", next.capabilities.can_seek),
        ] {
            changed.insert(name, OwnedValue::from(value));
        }
    }
    let seeked = seeked_position(before, &current);
    drop(current); // Never hold our state lock across D-Bus I/O.
    if !changed.is_empty() {
        connection.emit_signal(
            None::<&str>,
            PATH,
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            &(PLAYER, changed, Vec::<&str>::new()),
        )?;
    }
    if let Some(position) = seeked {
        connection.emit_signal(None::<&str>, PATH, PLAYER, "Seeked", &(position,))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        Snapshot {
            metadata: SessionMetadata {
                track_id: Some("/org/mpris/MediaPlayer2/track/fixture".into()),
                title: Some("Session fixture".into()),
                artist: Some("Fixture artist".into()),
                album: Some("Fixture album".into()),
                video: false,
            },
            duration: Some(Duration::from_secs(100)),
            artwork: Some("file:///tmp/fixture.png".into()),
            playback: MediaPlayback::Paused {
                progress: Some(MediaPosition(Duration::from_millis(3429))),
            },
            capabilities: SessionCapabilities::for_player(true, true, false, true, false, false),
            rate: 1.5,
            volume: 0.6,
        }
    }
    fn shared(snapshot: Snapshot) -> (Arc<Shared>, mpsc::Receiver<NativeMediaEvent>) {
        let (tx, rx) = mpsc::channel();
        (
            Arc::new(Shared {
                current: Mutex::new(Published {
                    snapshot,
                    at: Instant::now(),
                }),
                pending: Mutex::new(None),
                failure: Mutex::new(None),
                shutdown: AtomicBool::new(false),
                active: Arc::new(AtomicBool::new(true)),
                tx,
                ctx: eframe::egui::Context::default(),
            }),
            rx,
        )
    }
    #[test]
    fn metadata_units_and_elapsed_position_are_factual_and_bounded() {
        let mut state = fixture();
        assert_eq!(state.position(Duration::from_secs(5)), 3_429_000);
        state.playback = MediaPlayback::Playing {
            progress: Some(MediaPosition(Duration::from_millis(3429))),
        };
        assert_eq!(state.position(Duration::from_secs(2)), 6_429_000);
        assert_eq!(state.position(Duration::from_secs(1000)), 100_000_000);
        assert_eq!(
            i64::try_from(state.metadata_map().remove("mpris:length").unwrap()).unwrap(),
            100_000_000
        );
        assert_eq!(micros(Duration::MAX), i64::MAX);
        assert!(Snapshot::default().metadata_map().is_empty());
        state.duration = None;
        for position in [1, 3_429_001, 9_007_199_254_740_993] {
            state.playback = MediaPlayback::Paused {
                progress: Some(MediaPosition(Duration::from_micros(position))),
            };
            assert_eq!(
                state.position(Duration::from_secs(50)),
                position as i64,
                "paused positions must not round-trip through f64"
            );
        }
    }

    #[test]
    fn introspection_advertises_the_real_seek_signal_and_property_contract() {
        use zbus::object_server::Interface;
        let (shared, _) = shared(fixture());
        let mut xml = String::new();
        Player { shared }.introspect_to_writer(&mut xml, 0);
        assert!(xml.contains("signal name=\"Seeked\""));
        for property in [
            "Metadata",
            "PlaybackStatus",
            "Rate",
            "Volume",
            "CanPlay",
            "CanPause",
            "CanSeek",
        ] {
            assert!(
                xml.contains(&format!("property name=\"{property}\"")),
                "missing {property}"
            );
        }
        assert!(!xml.contains("LoopStatus") && !xml.contains("Shuffle"));
    }

    #[test]
    fn restored_nonzero_positions_and_exact_paused_seeks_emit_without_tick_spam() {
        let at = Instant::now();
        let stopped = Published {
            snapshot: Snapshot::default(),
            at,
        };
        let mut first = Published {
            snapshot: fixture(),
            at,
        };
        assert_eq!(seeked_position(&stopped, &first), Some(3_429_000));
        let mut second = Published {
            snapshot: first.snapshot.clone(),
            at,
        };
        second.snapshot.playback = MediaPlayback::Paused {
            progress: Some(MediaPosition(Duration::from_micros(3_429_001))),
        };
        assert_eq!(seeked_position(&first, &second), Some(3_429_001));
        first.snapshot.playback = MediaPlayback::Playing {
            progress: Some(MediaPosition(Duration::from_micros(3_429_000))),
        };
        second.at = at + Duration::from_millis(250);
        second.snapshot.playback = MediaPlayback::Playing {
            progress: Some(MediaPosition(Duration::from_micros(3_804_000))),
        };
        assert_eq!(
            seeked_position(&first, &second),
            None,
            "ordinary 1.5x advancement is not a seek"
        );
        assert_eq!(
            seeked_position(&first, &stopped),
            None,
            "Stop clears metadata, not a fake new seek"
        );
    }
    #[test]
    fn paused_toggle_and_stale_seek_use_real_capabilities_and_track_identity() {
        let (shared, receive) = shared(fixture());
        let player = Player {
            shared: shared.clone(),
        };
        assert!(player.can_play() && player.can_pause() && player.can_seek());
        assert!(!player.can_go_next() && !player.can_go_previous());
        player.next();
        player.previous();
        player.set_position(
            ObjectPath::try_from("/org/mpris/MediaPlayer2/track/stale").unwrap(),
            50_000,
        );
        player.set_position(
            ObjectPath::try_from("/org/mpris/MediaPlayer2/track/fixture").unwrap(),
            -1,
        );
        assert!(receive.try_recv().is_err());
        player.play_pause();
        assert!(matches!(
            receive.try_recv(),
            Ok(NativeMediaEvent::Control(MediaControlEvent::Toggle))
        ));
        player.set_position(
            ObjectPath::try_from("/org/mpris/MediaPlayer2/track/fixture").unwrap(),
            42_001,
        );
        assert!(
            matches!(receive.try_recv(), Ok(NativeMediaEvent::Control(MediaControlEvent::SetPosition(MediaPosition(p)))) if p.as_micros() == 42_001)
        );
        *shared.current.lock().unwrap() = Published {
            snapshot: Snapshot::default(),
            at: Instant::now(),
        };
        player.play();
        player.play_pause();
        player.pause();
        player.stop();
        player.seek(1);
        assert!(receive.try_recv().is_err());
    }
    #[test]
    fn property_commands_wait_for_observed_state_and_reject_nonfinite_values() {
        let (shared, receive) = shared(fixture());
        let player = Player {
            shared: shared.clone(),
        };
        assert!(player.set_rate(f64::NAN).is_err());
        assert!(player.set_volume(f64::INFINITY).is_err());
        player.set_rate(2.0).unwrap();
        player.set_volume(-1.0).unwrap();
        assert_eq!(player.rate(), 1.5);
        assert_eq!(player.volume(), 0.6);
        assert!(matches!(
            receive.try_recv(),
            Ok(NativeMediaEvent::SetRate(2.0))
        ));
        assert!(matches!(
            receive.try_recv(),
            Ok(NativeMediaEvent::Control(MediaControlEvent::SetVolume(0.0)))
        ));
        player.set_rate(0.0).unwrap();
        assert!(matches!(
            receive.try_recv(),
            Ok(NativeMediaEvent::Control(MediaControlEvent::Pause))
        ));
        shared.active.store(false, Ordering::Release);
        player.play_pause();
        assert!(
            receive.try_recv().is_err(),
            "retired callbacks must fail closed"
        );
    }
    #[cfg(unix)]
    #[test]
    #[ignore = "Requires an explicitly isolated dbus-run-session; CI runs this gate separately"]
    fn private_bus_projection_and_controls() {
        assert_eq!(
            std::env::var("PEALAYER_TEST_MPRIS_BUS").as_deref(),
            Ok("1"),
            "Never register on a user's normal session bus"
        );
        let (tx, receive) = mpsc::channel();
        let mut session = MprisSession::new(
            "Pealayer session fixture",
            Arc::new(AtomicBool::new(true)),
            tx,
            eframe::egui::Context::default(),
        );
        let state = fixture();
        session.publish(state.clone(), Instant::now());
        let connection = Connection::session().unwrap();
        let name = format!(
            "org.mpris.MediaPlayer2.pealayer.instance{}",
            std::process::id()
        );
        let proxy = zbus::blocking::Proxy::new(&connection, name.as_str(), PATH, PLAYER).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if proxy
                .get_property::<String>("PlaybackStatus")
                .ok()
                .as_deref()
                == Some("Paused")
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "MPRIS did not publish its snapshot"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(proxy.get_property::<i64>("Position").unwrap(), 3_429_000);
        assert_eq!(proxy.get_property::<f64>("Rate").unwrap(), 1.5);
        assert_eq!(proxy.get_property::<f64>("Volume").unwrap(), 0.6);
        assert!(proxy.get_property::<bool>("CanPause").unwrap());
        assert!(!proxy.get_property::<bool>("CanGoNext").unwrap());
        let metadata: HashMap<String, OwnedValue> = proxy.get_property("Metadata").unwrap();
        assert_eq!(
            i64::try_from(metadata.get("mpris:length").unwrap()).unwrap(),
            100_000_000
        );
        let root =
            zbus::blocking::Proxy::new(&connection, name.as_str(), PATH, "org.mpris.MediaPlayer2")
                .unwrap();
        assert_eq!(
            root.get_property::<String>("Identity").unwrap(),
            "Pealayer session fixture"
        );
        let _: () = proxy.call("PlayPause", &()).unwrap();
        assert!(matches!(
            receive.recv_timeout(Duration::from_secs(2)),
            Ok(NativeMediaEvent::Control(MediaControlEvent::Toggle))
        ));
        let _: () = proxy
            .call(
                "SetPosition",
                &(ObjectPath::try_from("/stale").unwrap(), 5_000_i64),
            )
            .unwrap();
        assert!(receive.try_recv().is_err());
        proxy.set_property("Rate", 2.0_f64).unwrap();
        assert!(matches!(
            receive.recv_timeout(Duration::from_secs(2)),
            Ok(NativeMediaEvent::SetRate(2.0))
        ));
        assert_eq!(
            proxy.get_property::<f64>("Rate").unwrap(),
            1.5,
            "a queued request is not decoder acknowledgement"
        );
        // Subscribe before the replacement, just like KDE Connect's event path.
        let properties = zbus::blocking::fdo::PropertiesProxy::builder(&connection)
            .destination(name.as_str())
            .unwrap()
            .path(PATH)
            .unwrap()
            .build()
            .unwrap();
        let mut signals = properties.receive_properties_changed().unwrap();
        let (seen_tx, seen_rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Some(signal) = signals.next() {
                let _ = seen_tx.send(
                    signal
                        .args()
                        .map(|args| args.changed_properties().contains_key("Rate"))
                        .unwrap_or(false),
                );
            }
        });
        let mut next = state.clone();
        next.rate = 2.0;
        next.volume = 0.0;
        session.publish(next.clone(), Instant::now());
        assert_eq!(
            seen_rx.recv_timeout(Duration::from_secs(3)),
            Ok(true),
            "Rate/Volume change signal missing"
        );
        assert_eq!(proxy.get_property::<f64>("Rate").unwrap(), 2.0);
        assert_eq!(proxy.get_property::<f64>("Volume").unwrap(), 0.0);
        let mut seeks = proxy.receive_signal("Seeked").unwrap();
        let (seek_tx, seek_rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Some(message) = seeks.next() {
                let _ = seek_tx.send(message.body().deserialize::<(i64,)>().unwrap().0);
            }
        });
        next.playback = MediaPlayback::Paused {
            progress: Some(MediaPosition(Duration::from_micros(3_429_001))),
        };
        session.publish(next, Instant::now());
        assert_eq!(
            seek_rx.recv_timeout(Duration::from_secs(3)),
            Ok(3_429_001),
            "exact paused Seeked signal missing"
        );
        let mut cleared = Snapshot::default();
        cleared.playback = MediaPlayback::Stopped;
        session.publish(cleared, Instant::now());
        let deadline = Instant::now() + Duration::from_secs(3);
        while proxy.get_property::<bool>("CanControl").unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            proxy
                .get_property::<HashMap<String, OwnedValue>>("Metadata")
                .unwrap()
                .is_empty()
        );
        drop(session);
    }
}
