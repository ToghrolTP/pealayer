//! Pealayer owns normal placement; framework widget persistence must not override it.
use crate::config::{AppConfig, WindowGeometry};
use eframe::egui;

pub const MINIMUM: egui::Vec2 = egui::vec2(420.0, 300.0);

pub fn initial(config: &AppConfig, remote: bool) -> Option<WindowGeometry> {
    if remote { return None; }
    config.workspace_session.window_geometry.filter(|geometry| geometry.is_valid())
        .or_else(|| config.window_geometry.filter(|geometry| geometry.is_valid()))
}

pub fn builder(mut builder: egui::ViewportBuilder, geometry: Option<WindowGeometry>, fullscreen: bool) -> egui::ViewportBuilder {
    // This runs after eframe's persisted WindowSettings restoration. Its window
    // key can contain a minimized zero-sized client, clamped by eframe to 64².
    // Keep widget memory, but always use this single validated placement owner.
    builder.position = geometry.map(|geometry| egui::pos2(geometry.x, geometry.y));
    builder.with_inner_size(geometry.map_or(egui::vec2(800.0, 600.0), |geometry| egui::vec2(geometry.width, geometry.height)))
        .with_min_inner_size(MINIMUM)
        .with_maximized(geometry.is_some_and(|geometry| geometry.maximized))
        .with_fullscreen(fullscreen)
}

pub fn observe(previous: Option<WindowGeometry>, viewport: &egui::ViewportInfo) -> Option<WindowGeometry> {
    if viewport.minimized.unwrap_or(false) || viewport.fullscreen.unwrap_or(false) { return previous; }
    if viewport.maximized.unwrap_or(false) {
        // A maximized client is not its Restore rectangle.
        return previous.map(|mut geometry| {geometry.maximized = true; geometry});
    }
    let (Some(inner), Some(outer)) = (viewport.inner_rect, viewport.outer_rect) else {return previous;};
    let geometry = WindowGeometry {x:outer.min.x,y:outer.min.y,width:inner.width(),height:inner.height(),maximized:false};
    if geometry.is_valid() {Some(geometry)} else {previous}
}

/// eframe applies its saved position/maximize once more after creating the
/// platform window. Reconcile it before the first app frame, without activation.
pub fn restore(window: &winit::window::Window, geometry: Option<WindowGeometry>, fullscreen: bool, zoom: f32) {
    let zoom=if zoom.is_finite() && zoom>0.0 {zoom}else{1.0};
    let mut size = geometry.map_or(egui::vec2(800.0,600.0), |value| egui::vec2(value.width,value.height))*zoom;
    window.set_min_inner_size(Some(winit::dpi::LogicalSize::new(MINIMUM.x*zoom,MINIMUM.y*zoom)));
    window.set_fullscreen(None);
    window.set_maximized(false);
    if let Some(value) = geometry {
        window.set_outer_position(winit::dpi::LogicalPosition::new(value.x*zoom,value.y*zoom));
    }
    if let Some(monitor)=window.current_monitor().or_else(||window.available_monitors().next()) {
        let extent=monitor.size().to_logical::<f32>(window.scale_factor());
        size=size.min(egui::vec2((extent.width-32.0).max(MINIMUM.x*zoom),(extent.height-80.0).max(MINIMUM.y*zoom)));
    }
    let _ = window.request_inner_size(winit::dpi::LogicalSize::new(size.x,size.y));
    // An unplugged monitor must not leave the titlebar unreachable. Work in
    // physical pixels here, so negative monitor origins and mixed DPI are real.
    if let (Ok(position), Some(monitor)) = (window.outer_position(), window.current_monitor().or_else(||window.available_monitors().next())) {
        let area = monitor.position(); let extent = monitor.size(); let outer = window.outer_size();
        let x = position.x.clamp(area.x, area.x.saturating_add(extent.width.saturating_sub(outer.width) as i32));
        let y = position.y.clamp(area.y, area.y.saturating_add(extent.height.saturating_sub(outer.height.min(80)) as i32));
        if x != position.x || y != position.y {window.set_outer_position(winit::dpi::PhysicalPosition::new(x,y));}
    }
    if fullscreen {window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));}
    else if geometry.is_some_and(|value| value.maximized) {window.set_maximized(true);}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn geometry() -> WindowGeometry {WindowGeometry{x:-900.0,y:70.0,width:900.0,height:650.0,maximized:false}}
    #[test]
    fn validated_application_placement_overrides_tiny_framework_state_every_launch() {
        let mut config=AppConfig::default();config.window_geometry=Some(geometry());
        config.workspace_session.window_geometry=Some(WindowGeometry{width:0.0,..geometry()});
        assert_eq!(initial(&config,false),Some(geometry()));
        for stored in [egui::Vec2::ZERO,egui::vec2(64.0,64.0),egui::vec2(5000.0,4000.0)] {
            let restored=builder(egui::ViewportBuilder::default().with_inner_size(stored).with_position([0.0,0.0]).with_maximized(true),initial(&config,false),false);
            assert_eq!(restored.inner_size,Some(egui::vec2(900.0,650.0)));
            assert_eq!(restored.position,Some(egui::pos2(-900.0,70.0)));
            assert_eq!(restored.min_inner_size,Some(MINIMUM));assert_eq!(restored.maximized,Some(false));
        }
        assert_eq!(initial(&config,true),None);
        assert_eq!(builder(egui::ViewportBuilder::default(),None,true).inner_size,Some(egui::vec2(800.0,600.0)));
    }
    #[test]
    fn minimize_fullscreen_and_maximize_never_replace_the_normal_restore_rectangle() {
        let normal=Some(geometry());
        let mut viewport=egui::ViewportInfo::default();viewport.minimized=Some(true);
        viewport.inner_rect=Some(egui::Rect::ZERO);assert_eq!(observe(normal,&viewport),normal);
        viewport.minimized=Some(false);viewport.maximized=Some(true);
        let maximized=observe(normal,&viewport).unwrap();assert!(maximized.maximized);
        assert_eq!(maximized.width,900.0);assert_eq!(maximized.x,-900.0);
        viewport.maximized=Some(false);viewport.fullscreen=Some(true);assert_eq!(observe(normal,&viewport),normal);
        viewport.fullscreen=Some(false);viewport.inner_rect=Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1200.0,700.0)));
        viewport.outer_rect=Some(egui::Rect::from_min_size(egui::pos2(40.0,60.0),egui::vec2(1216.0,740.0)));
        let next=observe(normal,&viewport).unwrap();assert_eq!((next.x,next.y,next.width,next.height),(40.0,60.0,1200.0,700.0));
    }
}
