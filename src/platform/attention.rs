//! Bounded native attention. Never activate, restore or steal foreground focus.
pub fn important_change(
    previous: &crate::messaging::MessageSnapshot,
    next: &crate::messaging::MessageSnapshot,
) -> bool {
    next.toasts.iter().any(|toast| {
        matches!(
            toast.severity,
            crate::messaging::Severity::Warning | crate::messaging::Severity::Error
        ) && (previous.instance_id != next.instance_id
            || !previous
                .toasts
                .iter()
                .any(|old| old.id == toast.id && old.created_at_ms == toast.created_at_ms))
    })
}
pub fn request() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{
                FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GetForegroundWindow,
            },
        };
        let hwnd = HWND(super::windows::get_registered_hwnd() as *mut _);
        if !hwnd.0.is_null() && GetForegroundWindow() != hwnd {
            use std::sync::atomic::{AtomicU64, Ordering};
            static LAST: AtomicU64 = AtomicU64::new(0);
            let now = crate::messaging::now_ms();
            let last = LAST.load(Ordering::Relaxed);
            if now.saturating_sub(last) < 5000 {
                return;
            }
            LAST.store(now, Ordering::Relaxed);
            let info = FLASHWINFO {
                cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                hwnd,
                dwFlags: FLASHW_TRAY,
                uCount: 3,
                dwTimeout: 0,
            };
            let _ = FlashWindowEx(&info);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn important_local_or_remote_messages_only_alert_once_per_event() {
        let mut hub = crate::messaging::MessageHub::default();
        let previous = hub.snapshot(0);
        let make = |severity| crate::messaging::ToastRequest {
            id: Some("event".into()),
            title: String::new(),
            message: "Attention".into(),
            severity,
            timeout_ms: 5000,
            actions: Vec::new(),
            default_action: None,
            details: None,
        };
        hub.publish(make(crate::messaging::Severity::Info), "remote", 1000)
            .unwrap();
        let info = hub.snapshot(1000);
        assert!(!super::important_change(&previous, &info));
        hub.publish(make(crate::messaging::Severity::Warning), "remote", 2000)
            .unwrap();
        let warning = hub.snapshot(2000);
        assert!(super::important_change(&info, &warning));
        assert!(!super::important_change(&warning, &warning));
        hub.dismiss("event");
        assert!(!super::important_change(&warning, &hub.snapshot(2001)));
    }
}
pub fn stop(hwnd: isize) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{FLASHW_STOP, FLASHWINFO, FlashWindowEx},
        };
        let info = FLASHWINFO {
            cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
            hwnd: HWND(hwnd as *mut _),
            dwFlags: FLASHW_STOP,
            uCount: 0,
            dwTimeout: 0,
        };
        let _ = FlashWindowEx(&info);
    }
    #[cfg(not(windows))]
    let _ = hwnd;
}
