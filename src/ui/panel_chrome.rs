//! Application-owned native panel glyphs. Painting never changes a hit target.
use eframe::egui;

pub(crate) fn paint_icon(painter: &egui::Painter, rect: egui::Rect, glyph: &str, color: egui::Color32) {
    let size = 14.0_f32.min((rect.width().min(rect.height()) - 4.0).max(1.0));
    let galley = painter.layout_no_wrap(glyph.into(), egui::FontId::proportional(size), color);
    let origin = rect.center() - galley.mesh_bounds.center().to_vec2();
    painter.with_clip_rect(rect.intersect(painter.clip_rect())).galley(origin, galley, color);
}

pub(crate) fn button(ui: &mut egui::Ui, rect: egui::Rect, id: egui::Id, glyph: &str, label: &str) -> egui::Response {
    let interactive = ui.ctx().memory(|memory| memory.is_above_modal_layer(ui.layer_id()));
    let response = ui.add_enabled_ui(interactive, |ui| ui.interact(rect, id, egui::Sense::click())).inner
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let tokens = super::craft_theme::current(ui.ctx());
    if response.hovered() || response.is_pointer_button_down_on() || response.has_focus() {
        ui.painter().rect_filled(rect, 3.0, if response.is_pointer_button_down_on() {tokens.pressed} else {tokens.hover});
    }
    if response.has_focus() {
        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, tokens.focus), egui::StrokeKind::Inside);
    }
    paint_icon(ui.painter(), rect, glyph, if !response.enabled() {tokens.text_faint}
        else if response.hovered() || response.has_focus() {tokens.tab_text_active} else {tokens.icon});
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, response.enabled(), label));
    response.on_hover_text(label)
}

pub(crate) fn craft_actions(rect: egui::Rect, rtl: bool) -> [egui::Rect; 2] {
    let side = 24.0_f32.min(rect.height()).min((rect.width() - 4.0).max(0.0) * 0.5);
    let last = rect.right() - 2.0 - side * 0.5;
    let slots = [last - side, last].map(|x| egui::Rect::from_center_size(egui::pos2(x, rect.center().y), egui::Vec2::splat(side)));
    slots.map(|slot| if rtl {egui::Rect::from_x_y_ranges((rect.left()+rect.right()-slot.right())..=(rect.left()+rect.right()-slot.left()), slot.y_range())} else {slot})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::PealayerApp, ui::layout::{PealayerTab as Tab, PealayerTabViewer}};

    fn frame(ctx: &egui::Context, app: &mut PealayerApp, events: Vec<egui::Event>, blocked: bool) -> egui::FullOutput {
        let layout = app.dock_state.layout.clone();
        let locked = app.dock_state.locked;
        let mut output = ctx.run_ui(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0,480.0))), events, ..Default::default()
        }, |ui| {
            let mut style = craft_ui::docking::DockStyle::from_ui(ui);
            super::super::craft_theme::apply_dock_style(&mut style, &super::super::craft_theme::current(ctx));
            style.right_to_left = app.rtl;
            craft_ui::docking::DockArea::new(egui::Id::new("panel-chrome-test")).locked(locked).input_blocked(blocked)
                .show_content(ui,&layout,&style,&mut PealayerTabViewer { app });
            if blocked { egui::Modal::new(egui::Id::new("panel-chrome-modal")).show(ctx, |ui| {ui.label("Modal");}); }
        });
        output.textures_delta.clear(); output
    }

    fn click(ctx: &egui::Context, app: &mut PealayerApp, pos: egui::Pos2, blocked: bool) -> egui::FullOutput {
        frame(ctx,app,vec![egui::Event::PointerMoved(pos)],blocked);
        frame(ctx,app,vec![egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}],blocked);
        frame(ctx,app,vec![egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}],blocked)
    }

    fn fixture(rtl: bool) -> (egui::Context,PealayerApp) {
        let ctx = egui::Context::default();
        crate::ui::i18n::configure_ui_fonts(&ctx,rtl);
        let mut app = PealayerApp::default();
        app.engine_handle = crate::four_d::engine::memory_only_test_engine();
        app.rtl=rtl;
        app.dock_state.layout=craft_ui::docking::Layout {root:Some(craft_ui::docking::Node::Tabs {
            panels:vec![Tab::EffectControls,Tab::MediaInspector],active:0}),floating:vec![]};
        app.dock_state.hidden.clear();
        (ctx,app)
    }

    fn control_centers(output: &egui::FullOutput) -> [egui::Pos2;2] {
        [crate::ui::icons::CARET_DOWN,crate::ui::icons::X].map(|glyph| output.shapes.iter().find_map(|shape|match &shape.shape {
            egui::Shape::Text(text) if text.galley.text()==glyph => Some(text.visual_bounding_rect().center()), _=>None
        }).expect("real CraftUI header glyph missing"))
    }

    #[test]
    fn craft_header_menu_and_close_are_real_stable_controls() {
        let _guard=crate::app::tests::lock_app_tests();
        for rtl in [false,true] { for scale in [1.0,1.25,1.5,2.0] { for dark in [false,true] {
            let (ctx,mut app)=fixture(rtl);
            ctx.set_pixels_per_point(scale);
            ctx.set_visuals(if dark {egui::Visuals::dark()} else {egui::Visuals::light()});
            frame(&ctx,&mut app,vec![],false);
            let output=frame(&ctx,&mut app,vec![],false);
            let [menu,close]=control_centers(&output);
            assert!((menu.y-close.y).abs()<0.1);
            assert_eq!(menu.x>close.x,rtl,"trailing controls must mirror with their header");
            let before=app.dock_state.clone();
            let hover=frame(&ctx,&mut app,vec![egui::Event::PointerMoved(menu)],false);
            assert_eq!(control_centers(&hover),[menu,close],"hover must not move glyphs");
            assert_eq!(hover.platform_output.cursor_icon,egui::CursorIcon::PointingHand);
            click(&ctx,&mut app,menu,false);
            let open=frame(&ctx,&mut app,vec![],false);
            assert!(open.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Preferences"))),"chevron failed to open the panel menu");
            assert_eq!(app.dock_state,before,"opening menu changed the workspace");
            frame(&ctx,&mut app,vec![egui::Event::Key {key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}],false);
            click(&ctx,&mut app,close,false);
            assert!(!app.dock_state.layout.contains(&Tab::EffectControls));
            assert!(app.dock_state.layout.contains(&Tab::MediaInspector));
            assert!(app.dock_state.hidden.iter().any(|(tab,_)|*tab==Tab::EffectControls));
            assert!(app.dock_state.validate().is_ok());
        } } }
    }

    #[test]
    fn craft_close_respects_lock_and_modal_without_hiding_its_glyph() {
        let _guard=crate::app::tests::lock_app_tests();
        for (locked,blocked) in [(true,false),(false,true)] {
            let (ctx,mut app)=fixture(false);app.dock_state.locked=locked;
            frame(&ctx,&mut app,vec![],blocked);
            let output=frame(&ctx,&mut app,vec![],blocked);
            let close=control_centers(&output)[1];
            let before=app.dock_state.clone();
            click(&ctx,&mut app,close,blocked);
            assert_eq!(app.dock_state,before,"locked/modal header changed workspace");
        }
    }
}
