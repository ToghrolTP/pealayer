pub mod association;
pub mod clipboard_urls;
pub mod associations;
pub mod interop;
#[cfg(target_os = "windows")]
pub(crate) mod copydata;
pub mod linux;
pub mod media_controls;
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
