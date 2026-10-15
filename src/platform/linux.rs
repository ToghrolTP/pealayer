//! Linux and Wayland runtime operational environment helpers.

/// Resolves whether a process is running within a native Wayland session
/// using the supplied environment lookup function.
pub fn is_wayland_session_from_env<F: Fn(&str) -> Option<std::ffi::OsString>>(env_lookup: F) -> bool {
    #[cfg(target_os = "linux")]
    {
        if env_lookup("WINIT_UNIX_BACKEND").as_deref() == Some(std::ffi::OsStr::new("x11")) {
            return false;
        }
        env_lookup("WAYLAND_DISPLAY").is_some() || env_lookup("WAYLAND_SOCKET").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = env_lookup;
        false
    }
}

/// Returns true if the current process is running in a native Wayland session
/// without an explicit X11 backend override.
pub fn is_wayland_session() -> bool {
    is_wayland_session_from_env(|k| std::env::var_os(k))
}

/// Resolves the effective OpenGL VSync setting for eframe's Glow backend.
///
/// On native Wayland, blocking swap intervals (`SwapInterval::Wait`) cause `eglSwapBuffers`
/// to block indefinitely when the window is occluded or mapped to an inactive workspace
/// because the compositor halts `wl_surface.frame` presentation callbacks.
/// This starves the main UI event loop and triggers compositor ANR watchdogs (such as
/// Hyprland's `ANRManager`).
///
/// Wayland compositors already composite and page-flip double-buffered surfaces atomically
/// and tear-free by design, and `winit` handles `wl_surface.frame` pacing at the event loop level.
/// Therefore, on native Wayland, `glow_options.vsync` must be disabled (`SwapInterval::DontWait`),
/// while preserving the user's configured preference on Windows and X11.
pub fn resolve_effective_opengl_vsync(configured_vsync: bool, is_wayland: bool) -> bool {
    if is_wayland {
        false
    } else {
        configured_vsync
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::OsString;

    #[test]
    fn test_is_wayland_session_from_env() {
        let mut env = HashMap::new();

        // 1. Neither variable set -> false
        assert!(!is_wayland_session_from_env(|k| env.get(k).cloned()));

        // 2. WAYLAND_DISPLAY set -> true on linux, false on other OS
        env.insert("WAYLAND_DISPLAY".to_string(), OsString::from("wayland-1"));
        let is_linux = cfg!(target_os = "linux");
        assert_eq!(
            is_wayland_session_from_env(|k| env.get(k).cloned()),
            is_linux
        );

        // 3. WINIT_UNIX_BACKEND=x11 override -> false even if WAYLAND_DISPLAY is set
        env.insert("WINIT_UNIX_BACKEND".to_string(), OsString::from("x11"));
        assert!(!is_wayland_session_from_env(|k| env.get(k).cloned()));

        // 4. WAYLAND_SOCKET set without X11 override
        env.remove("WINIT_UNIX_BACKEND");
        env.remove("WAYLAND_DISPLAY");
        env.insert("WAYLAND_SOCKET".to_string(), OsString::from("3"));
        assert_eq!(
            is_wayland_session_from_env(|k| env.get(k).cloned()),
            is_linux
        );
    }

    #[test]
    fn test_resolve_effective_opengl_vsync() {
        // On non-Wayland environments, configured value is preserved
        assert!(resolve_effective_opengl_vsync(true, false));
        assert!(!resolve_effective_opengl_vsync(false, false));

        // On Wayland, hardware swap wait is disabled to prevent occlusion deadlocks
        assert!(!resolve_effective_opengl_vsync(true, true));
        assert!(!resolve_effective_opengl_vsync(false, true));
    }
}
