//! Process-local, observed window state. Never substitute the remote authority's
//! desktop placement, or persist a minimized rectangle as normal restore bounds.
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Placement {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub state: Option<WindowMode>,
}
impl Placement {
    pub fn has_bounds(&self) -> bool {
        self.x.is_some() || self.y.is_some() || self.width.is_some() || self.height.is_some()
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.has_bounds() && self.state.is_none() {
            return Err("Window placement needs bounds or a state".into());
        }
        for value in [self.x, self.y].into_iter().flatten() {
            if !value.is_finite() || value.abs() > 131_072.0 {
                return Err(
                    "Window coordinates must be finite logical points within ±131072".into(),
                );
            }
        }
        for (value, minimum) in [(self.width, 420.0), (self.height, 300.0)] {
            if value
                .is_some_and(|value| !value.is_finite() || !(minimum..=16_384.0).contains(&value))
            {
                return Err("Window width/height must be finite, at least 420×300 and at most 16384 logical points".into());
            }
        }
        Ok(())
    }
    pub fn resolve(
        &self,
        previous: crate::config::WindowGeometry,
    ) -> crate::config::WindowGeometry {
        crate::config::WindowGeometry {
            x: self.x.unwrap_or(previous.x),
            y: self.y.unwrap_or(previous.y),
            width: self.width.unwrap_or(previous.width),
            height: self.height.unwrap_or(previous.height),
            maximized: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl From<egui::Rect> for Bounds {
    fn from(rect: egui::Rect) -> Self {
        Self {
            x: rect.min.x,
            y: rect.min.y,
            width: rect.width(),
            height: rect.height(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowStatus {
    pub process_id: u32,
    pub available: bool,
    pub units: String,
    pub pixels_per_point: f32,
    pub outer_bounds: Option<Bounds>,
    pub inner_bounds: Option<Bounds>,
    pub normal_bounds: Option<crate::config::WindowGeometry>,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub focused: bool,
    pub revision: u64,
    pub error: Option<String>,
    /// Decimal HWND on Windows, null elsewhere. The receiving HWND is local.
    pub native_handle: Option<String>,
}
impl Default for WindowStatus {
    fn default() -> Self {
        Self {
            process_id: std::process::id(),
            available: false,
            units: "logical_points".into(),
            pixels_per_point: 1.0,
            outer_bounds: None,
            inner_bounds: None,
            normal_bounds: None,
            minimized: false,
            maximized: false,
            fullscreen: false,
            focused: false,
            revision: 0,
            error: None,
            native_handle: None,
        }
    }
}
static LIVE: Mutex<Option<WindowStatus>> = Mutex::new(None);
pub fn observe(
    viewport: &egui::ViewportInfo,
    normal: Option<crate::config::WindowGeometry>,
    pixels_per_point: f32,
    headless: bool,
) {
    if let Ok(mut guard) = LIVE.lock() {
        let old = guard.clone().unwrap_or_default();
        let mut next = WindowStatus {
            available: !headless,
            pixels_per_point,
            outer_bounds: viewport.outer_rect.map(Into::into),
            inner_bounds: viewport.inner_rect.map(Into::into),
            normal_bounds: normal,
            minimized: viewport.minimized.unwrap_or(false),
            maximized: viewport.maximized.unwrap_or(false),
            fullscreen: viewport.fullscreen.unwrap_or(false),
            focused: viewport.focused.unwrap_or(false),
            revision: old.revision,
            error: old.error.clone(),
            ..Default::default()
        };
        if next.minimized {
            next.outer_bounds = None;
            next.inner_bounds = None;
        }
        if next != old {
            next.revision = old.revision.saturating_add(1);
        }
        *guard = Some(next);
    }
}
pub fn set_error(error: Option<String>) {
    if let Ok(mut guard) = LIVE.lock() {
        let state = guard.get_or_insert_with(Default::default);
        state.error = error;
        state.revision = state.revision.saturating_add(1);
    }
}
pub fn snapshot() -> WindowStatus {
    let mut value = LIVE
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_default();
    #[cfg(windows)]
    if value.available {
        unsafe {
            use windows::Win32::{
                Foundation::{HWND, POINT, RECT},
                Graphics::Gdi::ClientToScreen,
                UI::WindowsAndMessaging::{GetClientRect, GetWindowRect, IsIconic, IsZoomed},
            };
            let raw = crate::platform::windows::get_registered_hwnd();
            if raw != 0 {
                let hwnd = HWND(raw as *mut _);
                value.native_handle = Some(raw.to_string());
                value.minimized = IsIconic(hwnd).as_bool();
                value.maximized = IsZoomed(hwnd).as_bool();
                if value.minimized {
                    value.outer_bounds = None;
                    value.inner_bounds = None;
                } else {
                    let scale = value.pixels_per_point.max(0.1);
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                        value.outer_bounds = Some(Bounds {
                            x: rect.left as f32 / scale,
                            y: rect.top as f32 / scale,
                            width: (rect.right - rect.left) as f32 / scale,
                            height: (rect.bottom - rect.top) as f32 / scale,
                        });
                    }
                    let mut origin = POINT::default();
                    if GetClientRect(hwnd, &mut rect).is_ok()
                        && ClientToScreen(hwnd, &mut origin).as_bool()
                    {
                        value.inner_bounds = Some(Bounds {
                            x: origin.x as f32 / scale,
                            y: origin.y as f32 / scale,
                            width: (rect.right - rect.left) as f32 / scale,
                            height: (rect.bottom - rect.top) as f32 / scale,
                        });
                    }
                }
            }
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placement_validates_and_preserves_unspecified_restore_bounds() {
        let original = crate::config::WindowGeometry {
            x: -100.0,
            y: 30.0,
            width: 900.0,
            height: 650.0,
            maximized: true,
        };
        let patch = Placement {
            x: Some(0.0),
            width: Some(1100.0),
            ..Default::default()
        };
        assert!(patch.validate().is_ok());
        let next = patch.resolve(original);
        assert_eq!(
            (next.x, next.y, next.width, next.height, next.maximized),
            (0.0, 30.0, 1100.0, 650.0, false)
        );
        assert!(Placement::default().validate().is_err());
        for value in [0.0, 100.0, f32::NAN, f32::INFINITY, 20000.0] {
            assert!(
                Placement {
                    width: Some(value),
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        for mode in [
            WindowMode::Normal,
            WindowMode::Minimized,
            WindowMode::Maximized,
            WindowMode::Fullscreen,
        ] {
            assert!(
                Placement {
                    state: Some(mode),
                    ..Default::default()
                }
                .validate()
                .is_ok()
            );
        }
        assert!(serde_json::from_value::<Placement>(serde_json::json!({"widht":900})).is_err());
    }
}
