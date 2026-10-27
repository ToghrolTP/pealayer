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
}
