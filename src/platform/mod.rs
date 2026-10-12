pub mod association;
pub mod clipboard_urls;
pub mod associations;
pub mod interop;
pub mod media_controls;
#[cfg(target_os = "windows")]
mod windows_media_session;
pub mod registry;
pub mod taskbar_preview;
pub mod tray_board;
pub mod windows;
#[cfg(target_os = "windows")]
pub(crate) mod window_icons;
pub(crate) mod windows_quick_actions;
#[cfg(test)]
mod windows_shell_icons;
