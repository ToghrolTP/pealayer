pub mod association;
pub mod clipboard_urls;
pub mod clipboard;
pub mod attention;
#[cfg(windows)]
pub mod drop_target;
pub mod associations;
pub mod interop;
#[cfg(target_os = "windows")]
pub(crate) mod copydata;
pub mod linux;
pub mod media_controls;
#[cfg(any(test, all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android")))))]
mod mpris_session;
#[cfg(target_os = "windows")]
mod windows_media_session;
pub mod registry;
pub mod taskbar_preview;
#[cfg(target_os = "windows")]
pub mod taskbar_branding;
pub mod tray_board;
pub mod windows;
pub(crate) mod window_placement;
pub mod window_control;
#[cfg(target_os = "windows")]
pub(crate) mod window_icons;
pub(crate) mod windows_quick_actions;
#[cfg(test)]
mod windows_shell_icons;
