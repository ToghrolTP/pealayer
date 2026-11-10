//! Host-owned prompt data and decoder-confirmed command results.
use super::portable_shell_host::Action;
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub loaded: bool,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub muted: bool,
    pub controller: bool,
}
impl Status {
    pub fn state(&self) -> &'static str {
        if !self.loaded {
            "■ Idle"
        } else if self.playing {
            "▶ Playing"
        } else {
            "Ⅱ Paused"
        }
    }
    pub fn summary(&self) -> String {
        format!(
            "{}{} · {} · {} Hardware",
            self.state(),
            if self.loaded {
                format!(" {} / {}", time(self.position), time(self.duration))
            } else {
                String::new()
            },
            self.audio(),
            if self.controller { "●" } else { "○" }
        )
    }
    pub fn audio(&self) -> String {
        if self.muted {
            format!("⊘ Muted ({:.0}%)", self.volume)
        } else {
            format!("♪ {:.0}%", self.volume)
        }
    }
}
pub fn time(seconds: f64) -> String {
    let seconds = if seconds.is_finite() {
        seconds.max(0.0).floor() as u64
    } else {
        0
    };
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
pub fn precise_time(seconds: f64) -> String {
    let milliseconds = if seconds.is_finite() {
        (seconds.max(0.0) * 1000.0).round() as u64
    } else {
        0
    };
    format!(
        "{}.{:03}",
        time((milliseconds / 1000) as f64),
        milliseconds % 1000
    )
}
#[derive(Default)]
pub struct Observed {
    pub idle: Option<bool>,
    pub paused: Option<bool>,
    pub position: Option<f64>,
    pub seeking: Option<bool>,
    pub volume: Option<f64>,
    pub muted: Option<bool>,
}
pub struct Pending {
    pub action: Action,
    pub started: std::time::Instant,
}
impl Pending {
    pub fn new(action: Action) -> Self {
        Self {
            action,
            started: std::time::Instant::now(),
        }
    }
    pub fn label(&self) -> &'static str {
        match self.action {
            Action::Play => "Starting playback",
            Action::Pause => "Pausing",
            Action::Stop => "Stopping",
            Action::Seek(_) => "Seeking",
            Action::Volume(_) => "Changing volume",
            Action::Mute(_) => "Changing mute",
            _ => "Applying command",
        }
    }
    pub fn confirmed(&self, observed: &Observed) -> Option<String> {
        match self.action {
            Action::Play if observed.idle == Some(false) && observed.paused == Some(false) => {
                Some("Playing".into())
            }
            Action::Pause if observed.idle == Some(false) && observed.paused == Some(true) => {
                Some("Paused".into())
            }
            Action::Stop if observed.idle == Some(true) => Some("Stopped".into()),
            Action::Seek(target)
                if observed.seeking == Some(false)
                    && observed.position.is_some_and(|position| {
                        let advance = if observed.paused == Some(false) {
                            self.started.elapsed().as_secs_f64()
                        } else {
                            0.0
                        };
                        (position - target).abs() <= 0.100 + advance
                    }) =>
            {
                Some(format!("Position {}", time(observed.position.unwrap())))
            }
            Action::Volume(value)
                if observed
                    .volume
                    .is_some_and(|actual| (actual - value).abs() < 0.05) =>
            {
                Some(format!("Volume {value:.0}%"))
            }
            Action::Mute(value) if observed.muted == Some(value) => {
                Some(if value { "Muted" } else { "Unmuted" }.into())
            }
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatched_or_optimistic_state_is_not_confirmation() {
        let pending = Pending::new(Action::Play);
        assert_eq!(pending.confirmed(&Observed::default()), None);
        assert_eq!(
            pending.confirmed(&Observed {
                idle: Some(true),
                paused: Some(false),
                ..Default::default()
            }),
            None
        );
        assert_eq!(
            pending.confirmed(&Observed {
                idle: Some(false),
                paused: Some(false),
                ..Default::default()
            }),
            Some("Playing".into())
        );
        assert!(
            Pending::new(Action::Seek(12.0))
                .confirmed(&Observed {
                    position: Some(12.0),
                    seeking: Some(true),
                    ..Default::default()
                })
                .is_none()
        );
        assert!(
            Pending::new(Action::Volume(57.0))
                .confirmed(&Observed {
                    volume: Some(57.0),
                    ..Default::default()
                })
                .is_some()
        );
    }
    #[test]
    fn prompt_summarizes_real_states_without_static_narration() {
        let mut status = Status {
            loaded: true,
            position: 578.9,
            duration: 3600.0,
            volume: 57.0,
            ..Default::default()
        };
        assert!(status.summary().contains("Ⅱ Paused 00:09:38 / 01:00:00"));
        status.playing = true;
        status.muted = true;
        status.controller = true;
        assert!(status.summary().contains("▶ Playing"));
        assert!(status.summary().contains("⊘ Muted (57%)"));
        assert!(status.summary().contains("● Hardware"));
    }
}
