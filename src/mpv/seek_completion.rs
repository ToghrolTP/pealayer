//! Ordered completion for a native seek to a file's first displayable frame.
//! A decoder can start after timestamp zero; widening the ordinary position
//! tolerance would also admit stale preview restarts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeSeekEvent {
    Reply { request_id: u64, error: i32 },
    Seek,
    Restart,
    MediaBoundary,
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ZeroSeekFence {
    request_id: u64,
    epoch: u64,
    accepted: bool,
    seek_seen: bool,
    restarted: bool,
    failed: bool,
}

impl ZeroSeekFence {
    pub(crate) fn new(request_id: u64, epoch: u64) -> Self {
        Self {
            request_id,
            epoch,
            accepted: false,
            seek_seen: false,
            restarted: false,
            failed: false,
        }
    }

    /// Return a matching command failure without consuming another request's
    /// reply. Seek/restart events before the matching command reply are stale.
    pub(crate) fn observe(&mut self, event: NativeSeekEvent, epoch: u64) -> Option<i32> {
        if self.epoch != epoch || self.failed {
            return None;
        }
        match event {
            NativeSeekEvent::Reply { request_id, error } if request_id == self.request_id => {
                if error < 0 {
                    self.failed = true;
                    return Some(error);
                }
                self.accepted = true;
            }
            NativeSeekEvent::Seek if self.accepted => {
                self.seek_seen = true;
                self.restarted = false;
            }
            NativeSeekEvent::Restart if self.accepted && self.seek_seen => self.restarted = true,
            NativeSeekEvent::MediaBoundary | NativeSeekEvent::Overflow => self.failed = true,
            _ => {}
        }
        None
    }

    pub(crate) fn ready(&self, epoch: u64) -> bool {
        self.epoch == epoch && !self.failed && self.accepted && self.seek_seen && self.restarted
    }
}

/// This client has one event consumer (the UI thread). The seek worker only
/// queues commands. A separate client retains error userdata and gives the
/// command reply and subsequent seek/restart events one ordered queue.
pub(crate) struct NativeSeekClient {
    mpv: libmpv2::Mpv,
    active: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl NativeSeekClient {
    pub(crate) fn new(core: &libmpv2::Mpv) -> libmpv2::Result<Self> {
        Ok(Self {
            mpv: core.create_client(None)?,
            active: Default::default(),
        })
    }

    pub(crate) fn set_wakeup_callback<F: Fn() + Send + 'static>(&mut self, callback: F) {
        let active = self.active.clone();
        self.mpv.set_wakeup_callback(move || {
            if active.load(std::sync::atomic::Ordering::Acquire) {
                callback();
            }
        });
    }

    pub(crate) fn finish(&self) {
        self.active
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub(crate) fn queue_zero(&self, request_id: u64) -> bool {
        if request_id == 0 {
            return false;
        }
        let mut args = [
            c"seek".as_ptr(),
            c"0".as_ptr(),
            c"absolute+exact".as_ptr(),
            std::ptr::null(),
        ];
        // libmpv copies command arguments before returning; queuing is safe
        // concurrently with the sole event consumer on this client.
        self.active
            .store(true, std::sync::atomic::Ordering::Release);
        let accepted = unsafe {
            libmpv2_sys::mpv_command_async(self.mpv.ctx.as_ptr(), request_id, args.as_mut_ptr())
                >= 0
        };
        if !accepted {
            self.finish();
        }
        accepted
    }

    pub(crate) fn take_events(&self) -> Vec<NativeSeekEvent> {
        let mut events = Vec::new();
        for _ in 0..1024 {
            // Copy only scalar header fields while the event remains valid.
            // Never retain mpv-owned event data across another wait_event.
            let Some(event) =
                (unsafe { libmpv2_sys::mpv_wait_event(self.mpv.ctx.as_ptr(), 0.0).as_ref() })
            else {
                events.push(NativeSeekEvent::Overflow);
                break;
            };
            match event.event_id {
                libmpv2_sys::mpv_event_id_MPV_EVENT_NONE => break,
                libmpv2_sys::mpv_event_id_MPV_EVENT_COMMAND_REPLY => {
                    events.push(NativeSeekEvent::Reply {
                        request_id: event.reply_userdata,
                        error: event.error,
                    })
                }
                libmpv2_sys::mpv_event_id_MPV_EVENT_SEEK => events.push(NativeSeekEvent::Seek),
                libmpv2_sys::mpv_event_id_MPV_EVENT_PLAYBACK_RESTART => {
                    events.push(NativeSeekEvent::Restart)
                }
                libmpv2_sys::mpv_event_id_MPV_EVENT_START_FILE
                | libmpv2_sys::mpv_event_id_MPV_EVENT_END_FILE
                | libmpv2_sys::mpv_event_id_MPV_EVENT_SHUTDOWN => {
                    events.push(NativeSeekEvent::MediaBoundary)
                }
                libmpv2_sys::mpv_event_id_MPV_EVENT_QUEUE_OVERFLOW => {
                    events.push(NativeSeekEvent::Overflow)
                }
                _ => {}
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_seek_requires_matching_reply_then_seek_then_restart() {
        let mut fence = ZeroSeekFence::new(12, 3);
        for event in [
            NativeSeekEvent::Seek,
            NativeSeekEvent::Restart,
            NativeSeekEvent::Reply {
                request_id: 11,
                error: 0,
            },
            NativeSeekEvent::Restart,
        ] {
            assert_eq!(fence.observe(event, 3), None);
            assert!(!fence.ready(3));
        }
        fence.observe(
            NativeSeekEvent::Reply {
                request_id: 12,
                error: 0,
            },
            3,
        );
        fence.observe(NativeSeekEvent::Restart, 3);
        assert!(!fence.ready(3));
        fence.observe(NativeSeekEvent::Seek, 3);
        fence.observe(NativeSeekEvent::Restart, 3);
        assert!(fence.ready(3));
        assert!(!fence.ready(4));
        fence.observe(NativeSeekEvent::Seek, 3);
        assert!(!fence.ready(3));
        fence.observe(NativeSeekEvent::Restart, 3);
        assert!(fence.ready(3));
    }

    #[test]
    fn zero_seek_rejects_old_media_replies_and_retains_matching_errors() {
        let mut fence = ZeroSeekFence::new(12, 3);
        assert_eq!(
            fence.observe(
                NativeSeekEvent::Reply {
                    request_id: 12,
                    error: -12
                },
                4
            ),
            None
        );
        assert_eq!(
            fence.observe(
                NativeSeekEvent::Reply {
                    request_id: 11,
                    error: -12
                },
                3
            ),
            None
        );
        assert_eq!(
            fence.observe(
                NativeSeekEvent::Reply {
                    request_id: 12,
                    error: -12
                },
                3
            ),
            Some(-12)
        );
        assert!(!fence.ready(3));
        for event in [
            NativeSeekEvent::Reply {
                request_id: 12,
                error: 0,
            },
            NativeSeekEvent::Seek,
            NativeSeekEvent::Restart,
        ] {
            fence.observe(event, 3);
        }
        assert!(!fence.ready(3));
    }

    #[test]
    fn replacement_and_queue_loss_cannot_complete_a_zero_seek() {
        for boundary in [NativeSeekEvent::MediaBoundary, NativeSeekEvent::Overflow] {
            let mut fence = ZeroSeekFence::new(12, 3);
            fence.observe(
                NativeSeekEvent::Reply {
                    request_id: 12,
                    error: 0,
                },
                3,
            );
            fence.observe(NativeSeekEvent::Seek, 3);
            fence.observe(boundary, 3);
            fence.observe(NativeSeekEvent::Restart, 3);
            assert!(!fence.ready(3));
        }
    }

    #[test]
    fn first_zero_seek_after_load_drains_old_boundaries_and_observes_its_own_reply() {
        // Keep these isolated test handles alive for the process, as with the
        // app's shared test clients: Windows teardown must not race a new core.
        static CORE: std::sync::OnceLock<libmpv2::Mpv> = std::sync::OnceLock::new();
        static CLIENT: std::sync::OnceLock<NativeSeekClient> = std::sync::OnceLock::new();
        let core = CORE.get_or_init(|| {
            libmpv2::Mpv::with_initializer(|init| {
                for (key, value) in [
                    ("config", "no"),
                    ("load-scripts", "no"),
                    ("vo", "null"),
                    ("ao", "null"),
                    ("pause", "yes"),
                    ("keep-open", "always"),
                ] {
                    init.set_option(key, value)?;
                }
                Ok(())
            })
            .unwrap()
        });
        let client = CLIENT.get_or_init(|| NativeSeekClient::new(core).unwrap());
        let path =
            std::env::temp_dir().join(format!("pealayer-zero-fence-{}.y4m", uuid::Uuid::new_v4()));
        struct Cleanup(&'static libmpv2::Mpv, std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.command("stop", &[]);
                for _ in 0..20 {
                    if std::fs::remove_file(&self.1).is_ok() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        let _cleanup = Cleanup(core, path.clone());
        let mut video = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\n".to_vec();
        for _ in 0..3 {
            video.extend_from_slice(b"FRAME\n");
            video.extend_from_slice(&[128; 6]);
        }
        std::fs::write(&path, video).unwrap();
        core.command("loadfile", &[path.to_str().unwrap()]).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while core.get_property::<f64>("time-pos").is_err()
            || core.get_property::<bool>("seeking").unwrap_or(true)
        {
            assert!(std::time::Instant::now() < deadline, "fixture did not load");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let old_events = client.take_events();
        assert!(old_events.contains(&NativeSeekEvent::MediaBoundary));
        let request_id = 7;
        assert!(client.queue_zero(request_id));
        let mut fence = ZeroSeekFence::new(request_id, 1);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            for event in client.take_events() {
                assert_eq!(fence.observe(event, 1), None);
            }
            if fence.ready(1) && !core.get_property::<bool>("seeking").unwrap_or(true) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "zero seek did not complete after old-boundary drain"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(core.get_property::<f64>("time-pos").unwrap().is_finite());
        client.finish();
    }
}
