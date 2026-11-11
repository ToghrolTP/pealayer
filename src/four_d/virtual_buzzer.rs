use std::sync::mpsc::{self, SyncSender};
use std::time::{Duration, Instant};

use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
struct Note {
    frequency_hz: u16,
    duration_ms: u16,
}

impl Note {
    fn from_event(event: &Value) -> Option<Self> {
        if event["kind"] != "buzzer.note" || event["source"] != "board" {
            return None;
        }
        let metadata = event.get("metadata")?;
        let integer = |name: &str| metadata.get(name)?.as_str()?.parse::<u16>().ok();
        let frequency_hz = integer("frequency_hz")?;
        let duration_ms = integer("duration_ms")?;
        if frequency_hz == 0 || metadata.get("muted")?.as_str()? == "true" {
            return Some(Self {
                frequency_hz: 0,
                duration_ms: 0,
            });
        }
        (duration_ms > 0 && frequency_hz < 22_050).then_some(Self {
            frequency_hz,
            duration_ms,
        })
    }

    fn source(&self) -> String {
        format!(
            "av://lavfi:sine=frequency={}:duration={:.3}",
            self.frequency_hz,
            f64::from(self.duration_ms) / 1000.0
        )
    }
}

/// The connection owns the sender. Closing or rebinding the private board
/// ends its audio along with the subscription. This renders firmware note
/// events through the normal audio device and never opens a PC-speaker driver.
pub(crate) struct VirtualBuzzer(SyncSender<(Note, Instant)>);

impl VirtualBuzzer {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel::<(Note, Instant)>(16);
        std::thread::spawn(move || {
            let mut player: Option<libmpv2::Mpv> = None;
            let mut device = String::new();
            while let Ok((note, received_at)) = rx.recv() {
                let config = crate::platform::interop::get_live_config();
                if note.frequency_hz == 0 || !config.virtual_buzzer_audio {
                    if let Some(player) = &player {
                        let _ = player.command("stop", &[]);
                    }
                    continue;
                }
                if received_at.elapsed() > Duration::from_millis(250) {
                    continue;
                }
                let requested_device = crate::mpv::sfx::resolved_device(
                    &Default::default(),
                    &config.audio_device,
                    &config.sfx_audio_device,
                );
                if device != requested_device || player.is_none() {
                    match libmpv2::Mpv::with_initializer(|options| {
                        options.set_option("vo", "null")?;
                        options.set_option("vid", "no")?;
                        options.set_option("idle", true)?;
                        options.set_option("audio-device", requested_device.as_str())?;
                        options.set_option("audio-buffer", 0.05)?;
                        options.set_option("gapless-audio", "yes")?;
                        options.set_option("volume", 25)?;
                        Ok(())
                    }) {
                        Ok(output) => {
                            player = Some(output);
                            device = requested_device;
                        }
                        Err(error) => {
                            log::warn!("Virtual Board buzzer output unavailable: {error}");
                            continue;
                        }
                    }
                }
                if let Some(player) = &player
                    && let Err(error) = player.command("loadfile", &[&note.source(), "replace"])
                {
                    log::warn!("Virtual Board buzzer note could not be played: {error}");
                }
            }
        });
        Self(tx)
    }

    pub(crate) fn play_event(&self, event: &Value) {
        if let Some(note) = Note::from_event(event) {
            let _ = self.0.try_send((note, Instant::now()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_buzzer_renders_only_real_board_note_events() {
        let mut event = serde_json::json!({"kind":"buzzer.note", "source":"board",
            "metadata":{"frequency_hz":"733", "duration_ms":"300", "muted":"false"}});
        assert_eq!(
            Note::from_event(&event).unwrap().source(),
            "av://lavfi:sine=frequency=733:duration=0.300"
        );
        event["metadata"]["muted"] = "true".into();
        assert_eq!(Note::from_event(&event).unwrap().frequency_hz, 0);
        event["source"] = "host".into();
        assert!(Note::from_event(&event).is_none());
    }
}
