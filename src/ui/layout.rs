use eframe::egui;
use egui_dock::TabViewer;
use crate::app::{PealayerApp, EffectDragPayload};
use crate::four_d::engine::evaluate_relay_state;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PealayerTab {
    ProgramMonitor,
    EffectControls,
    EffectsLibrary,
    HardwareMonitor,
    Timeline,
}

pub struct PealayerTabViewer<'a> {
    pub app: &'a mut PealayerApp,
}

impl<'a> TabViewer for PealayerTabViewer<'a> {
    type Tab = PealayerTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            PealayerTab::ProgramMonitor => "Program Monitor 🎬".into(),
            PealayerTab::EffectControls => "Effect Controls ⚙".into(),
            PealayerTab::EffectsLibrary => "Effects Library 📚".into(),
            PealayerTab::HardwareMonitor => "Hardware Monitor 🖥".into(),
            PealayerTab::Timeline => "Timeline ⏱".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // High density styling for text elements
        ui.style_mut().override_text_style = Some(egui::TextStyle::Body);

        egui::Frame::NONE
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                match tab {
                    PealayerTab::ProgramMonitor => {
                        ui.vertical(|ui| {
                            // ponytail: reserve 35px at bottom for inline transport controls
                            let video_h = (ui.available_height() - 35.0).max(0.0);
                            let video_size = egui::vec2(ui.available_width(), video_h);
                            ui.allocate_ui_with_layout(video_size, egui::Layout::top_down(egui::Align::Center), |ui| {
                                crate::ui::video::draw(self.app, ui);
                            });

                            ui.add_space(5.0);

                            let has_video = self.app.current_video_path.is_some();
                            ui.add_enabled_ui(has_video, |ui| {
                                ui.horizontal(|ui| {
                                    let play_icon = if self.app.is_paused { "▶" } else { "⏸" };
                                    if ui.add_sized([30.0, 22.0], egui::Button::new(play_icon)).clicked() {
                                        let _ = self.app.mpv.command("cycle", &["pause"]);
                                    }
                                    if ui.add_sized([30.0, 22.0], egui::Button::new("⏹")).clicked() {
                                        let _ = self.app.mpv.command("seek", &["0", "absolute"]);
                                        let _ = self.app.mpv.set_property("pause", true);
                                    }
                                    ui.separator();

                                    // timecode HH:MM:SS:FF at 24fps
                                    let tc = format_timecode(if has_video { self.app.playback_time } else { 0.0 });
                                    ui.monospace(tc);

                                    // seekbar
                                    let mut current_pos = if has_video {
                                        self.app.seek_pos.unwrap_or(self.app.playback_time)
                                    } else {
                                        0.0
                                    };
                                    let max_dur = if has_video && self.app.duration > 0.0 {
                                        self.app.duration
                                    } else {
                                        1.0
                                    };
                                    let slider = egui::Slider::new(&mut current_pos, 0.0..=max_dur)
                                        .show_value(false)
                                        .trailing_fill(true);

                                    let seekbar_w = (ui.available_width() - 80.0).max(50.0);
                                    let old_w = ui.spacing().slider_width;
                                    ui.spacing_mut().slider_width = seekbar_w;
                                    let response = ui.add(slider);
                                    ui.spacing_mut().slider_width = old_w;

                                    if has_video && response.dragged() {
                                        self.app.seek_pos = Some(current_pos);
                                    }
                                    if has_video && response.drag_stopped() {
                                        let _ = self.app.mpv.command("seek", &[&current_pos.to_string(), "absolute"]);
                                        self.app.seek_pos = None;
                                    }
                                });
                            });
                        });
                    }
                    PealayerTab::EffectControls => {
                        let selected_count = self.app.selected_instance_ids.len();
                        
                        if selected_count == 1 {
                            let id = *self.app.selected_instance_ids.iter().next().unwrap();
                            let mut timeline_dirty = false;
                            let mut delete_cue = false;
                            
                            ui.heading("Effect Controls");
                            ui.add_space(8.0);
                            
                            let mut instance_idx = None;
                            for (idx, inst) in self.app.timeline.instances.iter().enumerate() {
                                if inst.id == id {
                                    instance_idx = Some(idx);
                                    break;
                                }
                            }
                            
                            if let Some(idx) = instance_idx {
                                let instance = &mut self.app.timeline.instances[idx];
                                
                                let mut template_idx = None;
                                for (t_idx, tmpl) in self.app.timeline.templates.iter().enumerate() {
                                    if tmpl.id == instance.effect_id {
                                        template_idx = Some(t_idx);
                                        break;
                                    }
                                }
                                
                                if let Some(t_idx) = template_idx {
                                    let template = &mut self.app.timeline.templates[t_idx];
                                    
                                    ui.group(|ui| {
                                        ui.strong("Identity");
                                        ui.add_space(4.0);
                                        
                                        ui.horizontal(|ui| {
                                            ui.label("Name: ");
                                            if ui.text_edit_singleline(&mut template.name).changed() {
                                                timeline_dirty = true;
                                            }
                                        });
                                        
                                        ui.horizontal(|ui| {
                                            ui.label("Icon: ");
                                            if ui.text_edit_singleline(&mut template.icon).changed() {
                                                timeline_dirty = true;
                                            }
                                        });
                                        
                                        ui.label(format!("Cue ID: {}", id));
                                        ui.label(format!("Template ID: {}", template.id));
                                    });
                                    
                                    ui.add_space(8.0);
                                    
                                    ui.group(|ui| {
                                        ui.strong("Timing Constraints");
                                        ui.add_space(4.0);
                                        
                                        let mut start_secs = instance.start_time_ms as f64 / 1000.0;
                                        ui.horizontal(|ui| {
                                            ui.label("Start Time:");
                                            let max_secs = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                                            let slider = ui.add(egui::Slider::new(&mut start_secs, 0.0..=max_secs).suffix("s"));
                                            if slider.changed() {
                                                instance.start_time_ms = (start_secs * 1000.0) as u64;
                                                timeline_dirty = true;
                                            }
                                        });
                                        
                                        let mut duration_ms = template.duration_ms as f64;
                                        ui.horizontal(|ui| {
                                            ui.label("Duration:");
                                            let slider = ui.add(egui::Slider::new(&mut duration_ms, 100.0..=10000.0).suffix("ms"));
                                            if slider.changed() {
                                                template.duration_ms = duration_ms as u64;
                                                let relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                                template.actions = crate::four_d::patterns::generate_constant(relay_id, true, template.duration_ms);
                                                timeline_dirty = true;
                                            }
                                        });
                                    });
                                    
                                    ui.add_space(8.0);
                                    
                                    ui.group(|ui| {
                                        ui.strong("Hardware Target");
                                        ui.add_space(4.0);
                                        
                                        let current_relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                        let mut selected_relay = current_relay_id;
                                        
                                        ui.horizontal(|ui| {
                                            ui.label("Target Relay:");
                                            egui::ComboBox::from_id_salt("relay_combo")
                                                .selected_text(format!("Relay {}", selected_relay))
                                                .show_ui(ui, |ui| {
                                                    for r in 1..=8 {
                                                        ui.selectable_value(&mut selected_relay, r, format!("Relay {}", r));
                                                    }
                                                });
                                        });
                                        
                                        if selected_relay != current_relay_id {
                                            template.actions = crate::four_d::patterns::generate_constant(selected_relay, true, template.duration_ms);
                                            timeline_dirty = true;
                                        }
                                    });
                                    
                                    ui.add_space(12.0);
                                    if ui.button(egui::RichText::new("🗑 Delete Cue").color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                        delete_cue = true;
                                    }
                                }
                            }
                            
                            if delete_cue {
                                self.app.timeline.instances.retain(|inst| inst.id != id);
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }
                            
                            if timeline_dirty {
                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                            }
                        } else if selected_count > 1 {
                            ui.heading("Bulk Effect Controls");
                            ui.add_space(8.0);
                            
                            ui.label(format!("Selected Cues: {}", selected_count));
                            ui.add_space(8.0);
                            
                            let mut timeline_dirty = false;
                            let mut delete_all = false;
                            
                            ui.group(|ui| {
                                ui.strong("Bulk Hardware Target Override");
                                ui.add_space(8.0);
                                
                                ui.horizontal(|ui| {
                                    for r in 1..=8 {
                                        if ui.button(format!("Set R{}", r)).clicked() {
                                            // Apply target relay r to all selected instances' templates
                                            let selected_ids = &self.app.selected_instance_ids;
                                            for inst in &mut self.app.timeline.instances {
                                                if selected_ids.contains(&inst.id) {
                                                    if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == inst.effect_id) {
                                                        template.actions = crate::four_d::patterns::generate_constant(r, true, template.duration_ms);
                                                    }
                                                }
                                            }
                                            timeline_dirty = true;
                                        }
                                    }
                                });
                            });
                            
                            ui.add_space(8.0);
                            
                            ui.group(|ui| {
                                ui.strong("Bulk Actions");
                                ui.add_space(8.0);
                                
                                if ui.button(egui::RichText::new("🗑 Delete All Selected").color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                    delete_all = true;
                                }
                            });
                            
                            if delete_all {
                                let selected_ids = &self.app.selected_instance_ids;
                                self.app.timeline.instances.retain(|inst| !selected_ids.contains(&inst.id));
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }
                            
                            if timeline_dirty {
                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label(egui::RichText::new("No Cue Selected").weak().size(14.0));
                            });
                        }
                    }
                    PealayerTab::EffectsLibrary => {
                        ui.heading("Effects Library");
                        ui.add_space(4.0);
                        
                        // 1. Instant search edit field
                        ui.horizontal(|ui| {
                            ui.label("🔍");
                            let res = ui.add(
                                egui::TextEdit::singleline(&mut self.app.effects_search_query)
                                    .hint_text("Search effects...")
                            );
                            if res.changed() {
                                // Request repaint to filter instantly
                                ui.ctx().request_repaint();
                            }
                        });
                        
                        ui.add_space(8.0);
                        
                        // Filter presets based on query
                        let query = self.app.effects_search_query.trim().to_lowercase();
                        let mut categorized: std::collections::BTreeMap<String, Vec<&crate::app::EffectPreset>> = std::collections::BTreeMap::new();
                        
                        for preset in &self.app.preset_library {
                            if query.is_empty() || preset.effect.name.to_lowercase().contains(&query) {
                                categorized.entry(preset.category.clone()).or_default().push(preset);
                            }
                        }
                        
                        let force_open = !query.is_empty();
                        
                        if categorized.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label(egui::RichText::new("No effects found").weak().size(12.0));
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("effects_scroll")
                                .show(ui, |ui| {
                                    for (category, presets) in categorized {
                                        let icon = if force_open { "📂" } else { "📁" };
                                        let header = egui::CollapsingHeader::new(format!("{} {}", icon, category))
                                            .default_open(true)
                                            .open(if force_open { Some(true) } else { None });
                                            
                                        header.show(ui, |ui| {
                                            ui.indent("preset_indent", |ui| {
                                                for preset in presets {
                                                    let item_id = egui::Id::new(&preset.effect.name);
                                                    let payload = EffectDragPayload {
                                                        name: preset.effect.name.clone(),
                                                        icon: preset.effect.icon.clone(),
                                                        duration_ms: preset.effect.duration_ms,
                                                        actions: preset.effect.actions.clone(),
                                                    };
                                                    
                                                    // Wrap item in drag source
                                                    ui.dnd_drag_source(item_id, payload, |ui| {
                                                        let (rect, response) = ui.allocate_exact_size(
                                                            egui::vec2(ui.available_width(), 26.0),
                                                            egui::Sense::click_and_drag(),
                                                        );
                                                        
                                                        let hovered = response.hovered();
                                                        let is_dragged = ui.ctx().is_being_dragged(response.id);
                                                        
                                                        // Hover state background
                                                        let bg_color = if is_dragged {
                                                            egui::Color32::from_rgb(55, 55, 55)
                                                        } else if hovered {
                                                            egui::Color32::from_rgb(45, 45, 45)
                                                        } else {
                                                            egui::Color32::TRANSPARENT
                                                        };
                                                        
                                                        ui.painter().rect_filled(rect, 4.0, bg_color);
                                                        
                                                        // Render Name and Icon
                                                        ui.painter().text(
                                                            rect.left_center() + egui::vec2(8.0, 0.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            format!("{} {}", preset.effect.icon, preset.effect.name),
                                                            egui::FontId::proportional(11.0),
                                                            egui::Color32::WHITE,
                                                        );
                                                        
                                                        // Change cursor to Grab on hover, Grabbing on active drag
                                                        if hovered {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                                        }
                                                        if is_dragged {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                            
                                                            #[allow(deprecated)]
                                                            egui::show_tooltip_at_pointer(
                                                                ui.ctx(),
                                                                ui.layer_id(),
                                                                egui::Id::new("dnd_tooltip"),
                                                                |ui: &mut egui::Ui| {
                                                                    ui.horizontal(|ui| {
                                                                        ui.label(format!("{} {}", preset.effect.icon, preset.effect.name));
                                                                        ui.label(egui::RichText::new(format!("({}ms)", preset.effect.duration_ms)).weak());
                                                                    });
                                                                }
                                                            );
                                                        }
                                                    });
                                                    ui.add_space(2.0);
                                                }
                                            });
                                        });
                                    }
                                });
                        }
                    }
                    PealayerTab::HardwareMonitor => {
                        ui.heading("Hardware Monitor Dashboard");
                        ui.add_space(8.0);
                        
                        if self.app.estop_active {
                            ui.horizontal(|ui| {
                                let time = ui.input(|i| i.time);
                                let is_flash = (time * 4.0).sin() > 0.0;
                                let color = if is_flash {
                                    egui::Color32::from_rgb(231, 76, 60) // Red
                                } else {
                                    egui::Color32::from_rgb(241, 196, 15) // Yellow
                                };
                                ui.colored_label(
                                    color,
                                    egui::RichText::new("⚠️ EMERGENCY STOP ACTIVE - ALL HARDWARE RELAYS DISABLED ⚠️")
                                        .strong()
                                        .size(13.0)
                                );
                            });
                            ui.add_space(8.0);
                        }
                        
                        egui::Grid::new("hardware_monitor_grid")
                            .num_columns(4)
                            .spacing([16.0, 12.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let relay_names = [
                                    (1, "R1: Water Valve"),
                                    (2, "R2: Wind Fan"),
                                    (3, "R3: Seat Vibration"),
                                    (4, "R4: Smoke Machine"),
                                    (5, "R5: Aux Relay"),
                                    (6, "R6: Aux Relay"),
                                    (7, "R7: Aux Relay"),
                                    (8, "R8: Aux Relay"),
                                ];
                                
                                for (id, name) in &relay_names {
                                    let is_timeline_active = evaluate_relay_state(&self.app.timeline, *id, (self.app.playback_time * 1000.0) as u64, &self.app.track_muted, &self.app.track_soloed);
                                    let is_forced = self.app.relay_overrides[*id as usize] == Some(true);
                                    let active = if self.app.estop_active {
                                        false
                                    } else {
                                        is_forced || (self.app.relay_overrides[*id as usize].is_none() && is_timeline_active)
                                    };
                                    
                                    // Draw LED
                                    ui.horizontal(|ui| {
                                        draw_led(ui, active);
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(*name).monospace());
                                    });
                                    
                                    // Force ON button
                                    let is_overridden = self.app.relay_overrides[*id as usize] == Some(true);
                                    let btn_text = if is_overridden { "🔴 FORCED" } else { "Force ON" };
                                    let btn = ui.selectable_label(is_overridden, btn_text);
                                    if btn.clicked() {
                                        if is_overridden {
                                            self.app.relay_overrides[*id as usize] = None;
                                            println!("{}:OFF", id);
                                        } else {
                                            self.app.relay_overrides[*id as usize] = Some(true);
                                            println!("{}:ON", id);
                                        }
                                    }
                                    
                                    // Status text label
                                    let status_text = if is_overridden {
                                        "Override ON"
                                    } else if is_timeline_active {
                                        "Timeline ON"
                                    } else {
                                        "Idle (OFF)"
                                    };
                                    ui.label(status_text);
                                    
                                    ui.end_row();
                                }
                            });
                            
                        if !self.app.is_paused {
                            ui.ctx().request_repaint();
                        }
                    }
                    PealayerTab::Timeline => {
                        ui.horizontal(|ui| {
                            // 1. Left column: Fixed Track Headers
                            ui.vertical(|ui| {
                                ui.set_width(180.0);
                                
                                let track_names = [
                                    "Video",
                                    "Audio",
                                    "R1: Water Valve",
                                    "R2: Wind Fan",
                                    "R3: Seat Vib.",
                                    "R4: Smoke Mac.",
                                    "R5: Aux Relay",
                                    "R6: Aux Relay",
                                    "R7: Aux Relay",
                                    "R8: Aux Relay",
                                ];
                                
                                for (idx, name) in track_names.iter().enumerate() {
                                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(180.0, 32.0), egui::Sense::hover());
                                    // Draw background with dark Premiere aesthetics
                                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(26, 26, 26));
                                    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)), egui::StrokeKind::Inside);
                                    
                                    // Create a nested UI at this rect to place buttons
                                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                                    child_ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        // Limit the track name label width
                                        ui.allocate_ui(egui::vec2(85.0, 20.0), |ui| {
                                            ui.label(egui::RichText::new(*name).size(11.0).strong());
                                        });
                                        
                                        // Render M, S, L buttons only for Relay tracks (idx >= 2)
                                        if idx >= 2 {
                                            let relay_id = idx - 1; // 1..=8
                                            
                                            // Mute button (M)
                                            let muted = &mut self.app.track_muted[relay_id];
                                            let m_btn = ui.selectable_label(*muted, egui::RichText::new("M").strong().size(10.0));
                                            if m_btn.clicked() {
                                                *muted = !*muted;
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }
                                            
                                            // Solo button (S)
                                            let soloed = &mut self.app.track_soloed[relay_id];
                                            let s_btn = ui.selectable_label(*soloed, egui::RichText::new("S").strong().size(10.0));
                                            if s_btn.clicked() {
                                                *soloed = !*soloed;
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }
                                            
                                            // Lock button (L)
                                            let locked = &mut self.app.track_locked[relay_id];
                                            let l_btn = ui.selectable_label(*locked, egui::RichText::new("L").strong().size(10.0));
                                            if l_btn.clicked() {
                                                *locked = !*locked;
                                            }
                                        }
                                    });
                                }
                            });
                            
                            // 2. Right column: Scrollable Timeline Grid
                            let total_seconds = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                            let total_width = (total_seconds * 100.0) as f32;
                            
                            // Define dropping target zone
                            let drop_res = ui.dnd_drop_zone::<EffectDragPayload, _>(egui::Frame::NONE, |ui| {
                                egui::ScrollArea::both()
                                    .id_salt("timeline_scroll")
                                    .show(ui, |ui| {
                                        let size = egui::vec2(total_width, 320.0);
                                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                                        
                                        let painter = ui.painter();
                                        
                                        // Draw timeline tracks background
                                        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(33, 33, 33));
                                        
                                        // Draw grid lines
                                        // Major grid lines every second (100 px), minor every 100ms (10 px)
                                        for i in 0..=(total_seconds.ceil() as i32) {
                                            let grid_x = rect.min.x + (i as f32 * 100.0);
                                            if grid_x <= rect.max.x {
                                                painter.line_segment(
                                                    [egui::pos2(grid_x, rect.min.y), egui::pos2(grid_x, rect.max.y)],
                                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)),
                                                );
                                                // Label time at top
                                                painter.text(
                                                    egui::pos2(grid_x + 4.0, rect.min.y + 10.0),
                                                    egui::Align2::LEFT_BOTTOM,
                                                    format!("{}s", i),
                                                    egui::FontId::monospace(9.0),
                                                    egui::Color32::from_rgb(120, 120, 120),
                                                );
                                            }
                                        }
                                        
                                        // Draw horizontal track separators and backgrounds
                                        for i in 0..=10 {
                                            let grid_y = rect.min.y + (i as f32 * 32.0);
                                            
                                            // Lock row background darkening
                                            if i >= 2 && i <= 9 {
                                                let relay_id = i - 1;
                                                if self.app.track_locked[relay_id as usize] {
                                                    let track_rect = egui::Rect::from_min_max(
                                                        egui::pos2(rect.min.x, grid_y),
                                                        egui::pos2(rect.max.x, grid_y + 32.0),
                                                    );
                                                    painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgb(26, 26, 26));
                                                }
                                            }
                                            
                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, grid_y), egui::pos2(rect.max.x, grid_y)],
                                                egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)),
                                            );
                                        }
                                        
                                        // Highlight target destination track row during active move drag
                                        if let Some(drag) = &self.app.active_drag {
                                            if drag.mode == crate::app::DragMode::Move {
                                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                    let relative_y = mouse_pos.y - rect.min.y;
                                                    let track_index = (relative_y / 32.0).floor() as i32;
                                                    if track_index >= 2 && track_index <= 9 {
                                                        let target_r = (track_index - 1) as u8;
                                                        if !self.app.track_locked[target_r as usize] {
                                                            let row_y = rect.min.y + (track_index as f32 * 32.0);
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(rect.max.x, row_y + 32.0),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(46, 204, 113, 25)); // Faint green highlight
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Render Video clip placeholder if active
                                        if self.app.duration > 0.0 {
                                            let video_clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, rect.min.y + 4.0),
                                                egui::pos2(rect.min.x + total_width, rect.min.y + 28.0),
                                            );
                                            painter.rect_filled(video_clip_rect, 4.0, egui::Color32::from_rgb(41, 128, 185)); // Blue clip
                                            painter.rect_stroke(video_clip_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Inside);
                                            painter.text(
                                                video_clip_rect.left_center() + egui::vec2(10.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                "Active Video File",
                                                egui::FontId::proportional(11.0),
                                                egui::Color32::WHITE,
                                            );
                                            
                                            // Audio clip placeholder
                                            let audio_clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, rect.min.y + 32.0 + 4.0),
                                                egui::pos2(rect.min.x + total_width, rect.min.y + 32.0 + 28.0),
                                            );
                                            painter.rect_filled(audio_clip_rect, 4.0, egui::Color32::from_rgb(39, 174, 96)); // Green clip
                                            painter.rect_stroke(audio_clip_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Inside);
                                            painter.text(
                                                audio_clip_rect.left_center() + egui::vec2(10.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                "Active Audio Track",
                                                egui::FontId::proportional(11.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        
                                        let mut clicked_any_clip = false;
                                        let mut started_drag = None;
                                        
                                        // 1st Pass: Draw all non-dragged clips
                                        let active_drag_id = self.app.active_drag.as_ref().map(|d| d.instance_id);
                                        let mut dragged_clip_data = None;
                                        
                                        for instance in &self.app.timeline.instances {
                                            if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                                // Find the relay used by this template's actions
                                                let relay_id = effect.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                                
                                                // Determine Y range based on relay_id (1..8)
                                                let track_index = relay_id as f32 + 1.0;
                                                let track_y = rect.min.y + (track_index * 32.0);
                                                
                                                let start_x = rect.min.x + (instance.start_time_ms as f32 * 0.1);
                                                let end_x = start_x + (effect.duration_ms as f32 * 0.1);
                                                
                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(start_x, track_y + 4.0),
                                                    egui::pos2(end_x, track_y + 28.0),
                                                );
                                                
                                                let clip_id = egui::Id::new(instance.id);
                                                let is_track_locked = self.app.track_locked[relay_id as usize];
                                                
                                                let clip_response = if is_track_locked {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click())
                                                } else {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click_and_drag())
                                                };
                                                
                                                let mut hover_mode = crate::app::DragMode::Move;
                                                if clip_response.hovered() && !is_track_locked {
                                                    if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
                                                        let left_dist = (mouse_pos.x - clip_rect.left()).abs();
                                                        let right_dist = (mouse_pos.x - clip_rect.right()).abs();
                                                        
                                                        if left_dist <= 6.0 {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                            hover_mode = crate::app::DragMode::ResizeLeft;
                                                        } else if right_dist <= 6.0 {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                            hover_mode = crate::app::DragMode::ResizeRight;
                                                        } else {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                                        }
                                                    }
                                                }
                                                
                                                if clip_response.clicked() {
                                                    let is_ctrl = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    clicked_any_clip = true;
                                                    if is_ctrl {
                                                        if self.app.selected_instance_ids.contains(&instance.id) {
                                                            self.app.selected_instance_ids.remove(&instance.id);
                                                        } else {
                                                            self.app.selected_instance_ids.insert(instance.id);
                                                        }
                                                    } else {
                                                        if !self.app.selected_instance_ids.contains(&instance.id) {
                                                            self.app.selected_instance_ids.clear();
                                                            self.app.selected_instance_ids.insert(instance.id);
                                                        }
                                                    }
                                                }
                                                
                                                if clip_response.drag_started() && !is_track_locked {
                                                    clicked_any_clip = true;
                                                    let is_ctrl = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    if !self.app.selected_instance_ids.contains(&instance.id) {
                                                        if !is_ctrl {
                                                            self.app.selected_instance_ids.clear();
                                                        }
                                                        self.app.selected_instance_ids.insert(instance.id);
                                                    }
                                                    
                                                    // Collect initial positions
                                                    let initial_positions: Vec<(uuid::Uuid, u64)> = self.app.timeline.instances.iter()
                                                        .filter(|inst| self.app.selected_instance_ids.contains(&inst.id))
                                                        .map(|inst| (inst.id, inst.start_time_ms))
                                                        .collect();
                                                    
                                                    if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                        started_drag = Some((instance.id, hover_mode, instance.start_time_ms, effect.duration_ms, mouse_pos.x, initial_positions));
                                                    }
                                                }
                                                
                                                if active_drag_id == Some(instance.id) {
                                                    // Save for 2nd pass
                                                    dragged_clip_data = Some((clip_rect, instance.id, effect.clone(), relay_id));
                                                    continue;
                                                }
                                                
                                                let is_selected = self.app.selected_instance_ids.contains(&instance.id);
                                                let stroke_color = if is_selected {
                                                    egui::Color32::from_rgb(255, 235, 59) // Selection Yellow outline
                                                } else {
                                                    egui::Color32::WHITE
                                                };
                                                let stroke_width = if is_selected { 2.0 } else { 1.0 };
                                                
                                                let is_muted = self.app.track_muted[relay_id as usize];
                                                let alpha = if is_muted { 128 } else { 255 };
                                                
                                                // Draw clip box
                                                painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgba_unmultiplied(142, 68, 173, alpha)); // Purple clip
                                                painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color), egui::StrokeKind::Inside);
                                                
                                                // Clip name label
                                                painter.text(
                                                    clip_rect.left_center() + egui::vec2(8.0, 0.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    format!("{} {}", effect.icon, effect.name),
                                                    egui::FontId::proportional(10.0),
                                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                                                );
                                            }
                                        }
                                        
                                        // 2nd Pass: Draw the actively dragged clip on top with a shadow and brighter color
                                        if let Some((clip_rect, instance_id, effect, _relay_id)) = dragged_clip_data {
                                            let is_selected = self.app.selected_instance_ids.contains(&instance_id);
                                            let stroke_color = if is_selected {
                                                egui::Color32::from_rgb(255, 235, 59)
                                            } else {
                                                egui::Color32::WHITE
                                            };
                                            let stroke_width = if is_selected { 2.0 } else { 1.0 };
                                            
                                            // Draw drop shadow
                                            let shadow_rect = clip_rect.translate(egui::vec2(2.0, 3.0));
                                            painter.rect_filled(shadow_rect, 4.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 80));
                                            
                                            // Draw bright purple clip
                                            painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgb(172, 98, 203)); // Brighter purple
                                            painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color), egui::StrokeKind::Inside);
                                            
                                            // Clip name label
                                            painter.text(
                                                clip_rect.left_center() + egui::vec2(8.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                format!("{} {}", effect.icon, effect.name),
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        
                                        // Apply selection or drag start outside the borrow loop
                                        if let Some((drag_id, mode, init_start, init_dur, start_x, init_positions)) = started_drag {
                                            self.app.active_drag = Some(crate::app::ActiveDragState {
                                                instance_id: drag_id,
                                                mode,
                                                initial_start_time_ms: init_start,
                                                initial_duration_ms: init_dur,
                                                drag_start_x: start_x,
                                                initial_positions: init_positions,
                                            });
                                        }
                                        
                                        // Process active drag logic
                                        let mut drag_ended = false;
                                        let mut snap_line_x = None;
                                        
                                        if let Some(drag_state) = &self.app.active_drag {
                                            if ui.ctx().input(|i| i.pointer.any_released()) {
                                                drag_ended = true;
                                            } else {
                                                let delta_x = ui.ctx().pointer_latest_pos().map(|p| p.x).unwrap_or(drag_state.drag_start_x) - drag_state.drag_start_x;
                                                let delta_time_ms = (delta_x / 0.1) as i64;
                                                
                                                let snap_enabled = !ui.ctx().input(|i| i.modifiers.shift || i.modifiers.alt);
                                                
                                                // Build snap targets list
                                                let mut snap_targets = vec![0, (self.app.playback_time * 1000.0) as u64];
                                                for inst in &self.app.timeline.instances {
                                                    if inst.id == drag_state.instance_id || self.app.selected_instance_ids.contains(&inst.id) {
                                                        continue;
                                                    }
                                                    if let Some(tmpl) = self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id) {
                                                        snap_targets.push(inst.start_time_ms);
                                                        snap_targets.push(inst.start_time_ms + tmpl.duration_ms);
                                                    }
                                                }
                                                
                                                match drag_state.mode {
                                                    crate::app::DragMode::Move => {
                                                        // Vertical track switching
                                                        let mut target_relay = None;
                                                        if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                            let relative_y = mouse_pos.y - rect.min.y;
                                                            let track_index = (relative_y / 32.0).floor() as i32;
                                                            if track_index >= 2 && track_index <= 9 {
                                                                let target_r = (track_index - 1) as u8;
                                                                if !self.app.track_locked[target_r as usize] {
                                                                    target_relay = Some(target_r);
                                                                }
                                                            }
                                                        }
                                                        
                                                        let mut primary_new_start = (drag_state.initial_start_time_ms as i64 + delta_time_ms).max(0) as u64;
                                                        
                                                        if snap_enabled {
                                                            // Check snap to start
                                                            for target in &snap_targets {
                                                                if (primary_new_start as i64 - *target as i64).abs() <= 100 {
                                                                    primary_new_start = *target;
                                                                    snap_line_x = Some(rect.min.x + (primary_new_start as f32 * 0.1));
                                                                    break;
                                                                }
                                                            }
                                                            // Check snap to end
                                                            if snap_line_x.is_none() {
                                                                let primary_new_end = primary_new_start + drag_state.initial_duration_ms;
                                                                for target in &snap_targets {
                                                                    if (primary_new_end as i64 - *target as i64).abs() <= 100 {
                                                                        primary_new_start = target.saturating_sub(drag_state.initial_duration_ms);
                                                                        snap_line_x = Some(rect.min.x + (*target as f32 * 0.1));
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        
                                                        let actual_delta_ms = primary_new_start as i64 - drag_state.initial_start_time_ms as i64;
                                                        
                                                        for &(inst_id, init_start) in &drag_state.initial_positions {
                                                            if let Some(inst) = self.app.timeline.instances.iter_mut().find(|i| i.id == inst_id) {
                                                                let new_start = (init_start as i64 + actual_delta_ms).max(0) as u64;
                                                                inst.start_time_ms = new_start;
                                                                
                                                                if inst_id == drag_state.instance_id {
                                                                    if let Some(r) = target_relay {
                                                                        if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == inst.effect_id) {
                                                                            template.actions = crate::four_d::patterns::generate_constant(r, true, template.duration_ms);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    crate::app::DragMode::ResizeRight => {
                                                        let mut new_end = (drag_state.initial_start_time_ms + drag_state.initial_duration_ms) as i64 + delta_time_ms;
                                                        if snap_enabled {
                                                            for target in &snap_targets {
                                                                if (new_end - *target as i64).abs() <= 100 {
                                                                    new_end = *target as i64;
                                                                    snap_line_x = Some(rect.min.x + (new_end as f32 * 0.1));
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        let new_dur = (new_end - drag_state.initial_start_time_ms as i64).max(100) as u64;
                                                        let instance_id = drag_state.instance_id;
                                                        if let Some(instance) = self.app.timeline.instances.iter_mut().find(|inst| inst.id == instance_id) {
                                                            if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                                                                let relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                                                template.duration_ms = new_dur;
                                                                template.actions = crate::four_d::patterns::generate_constant(relay_id, true, new_dur);
                                                            }
                                                        }
                                                    }
                                                    crate::app::DragMode::ResizeLeft => {
                                                        let right_anchor = drag_state.initial_start_time_ms + drag_state.initial_duration_ms;
                                                        let mut new_start = (drag_state.initial_start_time_ms as i64 + delta_time_ms).max(0) as u64;
                                                        if snap_enabled {
                                                            for target in &snap_targets {
                                                                if (new_start as i64 - *target as i64).abs() <= 100 {
                                                                    new_start = *target;
                                                                    snap_line_x = Some(rect.min.x + (new_start as f32 * 0.1));
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        new_start = new_start.min(right_anchor.saturating_sub(100));
                                                        let new_dur = right_anchor - new_start;
                                                        let instance_id = drag_state.instance_id;
                                                        if let Some(instance) = self.app.timeline.instances.iter_mut().find(|inst| inst.id == instance_id) {
                                                            if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                                                                let relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                                                instance.start_time_ms = new_start;
                                                                template.duration_ms = new_dur;
                                                                template.actions = crate::four_d::patterns::generate_constant(relay_id, true, new_dur);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        if drag_ended {
                                            self.app.active_drag = None;
                                            let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                            let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        }
                                        
                                        if self.app.active_drag.is_some() {
                                            ui.ctx().request_repaint();
                                        }

                                        // Draw Playhead
                                        let playhead_x = rect.min.x + (self.app.playback_time as f32 * 100.0);
                                        if playhead_x <= rect.max.x {
                                            // Vertical line
                                            painter.line_segment(
                                                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                                                egui::Stroke::new(1.5, egui::Color32::RED),
                                            );
                                            // Playhead handle (triangle at top)
                                            let points = vec![
                                                egui::pos2(playhead_x - 6.0, rect.min.y),
                                                egui::pos2(playhead_x + 6.0, rect.min.y),
                                                egui::pos2(playhead_x, rect.min.y + 8.0),
                                            ];
                                            painter.add(egui::Shape::convex_polygon(points, egui::Color32::RED, egui::Stroke::NONE));
                                        }
                                        
                                        // Draw Snap line if active
                                        if let Some(x) = snap_line_x {
                                            painter.line_segment(
                                                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                                                egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 255)), // Cyan snap line
                                            );
                                        }
                                        
                                        // Target highlighting during active drag
                                        if let Some(_payload) = egui::DragAndDrop::payload::<EffectDragPayload>(ui.ctx()) {
                                            if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
                                                if rect.contains(mouse_pos) {
                                                    let relative_y = mouse_pos.y - rect.min.y;
                                                    let track_index = (relative_y / 32.0).floor() as i32;
                                                    
                                                    if track_index >= 2 && track_index <= 9 {
                                                        let relay_id = track_index - 1;
                                                        let row_y = rect.min.y + (track_index as f32 * 32.0);
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(rect.max.x, row_y + 32.0),
                                                        );
                                                        
                                                        if self.app.track_locked[relay_id as usize] {
                                                            // Locked: incompatible highlight
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                            painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 40));
                                                        } else {
                                                            // Valid relay track: faint green highlight
                                                            painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 255, 136, 40));
                                                        }
                                                    } else if track_index == 0 || track_index == 1 {
                                                        // Incompatible track: faint red highlight & NotAllowed cursor
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                        let row_y = rect.min.y + (track_index as f32 * 32.0);
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(rect.max.x, row_y + 32.0),
                                                        );
                                                        painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 40));
                                                    }
                                                }
                                            }
                                        }

                                        // Lasso selection drawing
                                        if let Some(lasso_rect) = self.app.lasso_rect {
                                            painter.rect(
                                                lasso_rect,
                                                2.0,
                                                egui::Color32::from_rgba_unmultiplied(52, 152, 219, 30),
                                                egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 152, 219)),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                        
                                        ((rect, response), clicked_any_clip)
                                    })
                            });
                            
                            let ((rect, response), clicked_any_clip) = drop_res.0.inner.inner;
                            
                            // Successful drop logic
                            if let Some(payload) = &drop_res.1 {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if rect.contains(mouse_pos) {
                                        let relative_y = mouse_pos.y - rect.min.y;
                                        let track_index = (relative_y / 32.0).floor() as i32;
                                        
                                        if track_index >= 2 && track_index <= 9 {
                                            let target_relay = (track_index - 1) as u8;
                                            if !self.app.track_locked[target_relay as usize] {
                                                let relative_x = mouse_pos.x - rect.min.x;
                                                let mut drop_time_secs = (relative_x / 100.0) as f64;
                                                
                                                // Playhead/grid snapping
                                                if (drop_time_secs - self.app.playback_time).abs() < 0.15 {
                                                    drop_time_secs = self.app.playback_time;
                                                } else {
                                                    drop_time_secs = (drop_time_secs * 10.0).round() / 10.0;
                                                }
                                                
                                                let start_time_ms = (drop_time_secs.max(0.0) * 1000.0) as u64;
                                            
                                            // Check or create template
                                            let template_id = if let Some(existing) = self.app.timeline.templates.iter().find(|t| {
                                                t.name == payload.name && t.duration_ms == payload.duration_ms
                                            }) {
                                                existing.id
                                            } else {
                                                let new_effect = crate::four_d::models::Effect::new(
                                                    payload.name.clone(),
                                                    payload.icon.clone(),
                                                    payload.duration_ms,
                                                    crate::four_d::patterns::generate_constant(target_relay, true, payload.duration_ms),
                                                );
                                                let id = new_effect.id;
                                                self.app.timeline.templates.push(new_effect);
                                                id
                                            };
                                            
                                            // Instantiate and select
                                            let new_instance = crate::four_d::models::EffectInstance::new(template_id, start_time_ms);
                                            let new_instance_id = new_instance.id;
                                            self.app.timeline.instances.push(new_instance);
                                            self.app.selected_instance_ids.clear();
                                            self.app.selected_instance_ids.insert(new_instance_id);
                                            
                                            // Recompile timeline
                                            let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                            let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Background click, seek, or lasso selection logic
                            if response.drag_started() && !clicked_any_clip && self.app.active_drag.is_none() {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    self.app.lasso_origin = Some(mouse_pos);
                                }
                            }
                            
                            if response.dragged() && self.app.lasso_origin.is_some() {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    let origin = self.app.lasso_origin.unwrap();
                                    let lasso_rect = egui::Rect::from_two_pos(origin, mouse_pos);
                                    self.app.lasso_rect = Some(lasso_rect);
                                    
                                    let mut new_selection = std::collections::HashSet::new();
                                    for instance in &self.app.timeline.instances {
                                        if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                            let relay_id = effect.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                            let track_index = relay_id as f32 + 1.0;
                                            let track_y = rect.min.y + (track_index * 32.0);
                                            let start_x = rect.min.x + (instance.start_time_ms as f32 * 0.1);
                                            let end_x = start_x + (effect.duration_ms as f32 * 0.1);
                                            let clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(start_x, track_y + 4.0),
                                                egui::pos2(end_x, track_y + 28.0),
                                            );
                                            if lasso_rect.intersects(clip_rect) {
                                                new_selection.insert(instance.id);
                                            }
                                        }
                                    }
                                    self.app.selected_instance_ids = new_selection;
                                }
                            }
                            
                            let mut lasso_ended = false;
                            if ui.ctx().input(|i| i.pointer.any_released()) {
                                lasso_ended = true;
                            }
                            
                            if lasso_ended {
                                self.app.lasso_origin = None;
                                self.app.lasso_rect = None;
                            }
                            
                            if response.clicked() && !clicked_any_clip && self.app.active_drag.is_none() {
                                if let Some(mouse_pos) = response.interact_pointer_pos() {
                                    self.app.selected_instance_ids.clear();
                                    let relative_x = mouse_pos.x - rect.min.x;
                                    let seek_time = (relative_x / 100.0) as f64;
                                    let target_time = seek_time.clamp(0.0, total_seconds);
                                    let _ = self.app.mpv.command("seek", &[&target_time.to_string(), "absolute"]);
                                }
                            }
                        });
                    }
                }
            });
    }
}

/// Helper function to build the initial DockState layout
pub fn create_initial_layout() -> egui_dock::DockState<PealayerTab> {
    use egui_dock::NodeIndex;
    
    // Start with a Timeline tab as the initial tab in the root node
    let mut dock_state = egui_dock::DockState::new(vec![PealayerTab::Timeline]);
    
    // Split the root node: split_above creates a top node (70% height) for monitors,
    // leaving the Timeline at the bottom (30% height).
    let [_timeline_node, top_node] = dock_state.main_surface_mut().split_above(
        NodeIndex::root(),
        0.7,
        vec![PealayerTab::ProgramMonitor],
    );
    
    // Split the top node horizontally: split_left creates a left column (25% width)
    // for Effect Controls and Hardware Monitor, leaving Program Monitor in the center/right.
    let [center_node, _left_node] = dock_state.main_surface_mut().split_left(
        top_node,
        0.25,
        vec![PealayerTab::EffectControls, PealayerTab::HardwareMonitor],
    );
    
    // Split the remaining center node horizontally: split_right creates a right column
    // (30% of the remaining width, ~22.5% of total width) for Effects Library,
    // leaving the Program Monitor in the center (~52.5% width).
    let [_center_node, _right_node] = dock_state.main_surface_mut().split_right(
        center_node,
        0.7,
        vec![PealayerTab::EffectsLibrary],
    );
    
    dock_state
}

fn format_timecode(t: f64) -> String {
    let secs = t.floor() as i64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let f = ((t - t.floor()) * 24.0).round() as i64;
    format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
}

fn draw_led(ui: &mut egui::Ui, active: bool) {
    let size = egui::vec2(14.0, 14.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    
    let center = rect.center();
    let outer_radius = 7.0;
    let inner_radius = 4.5;
    
    let painter = ui.painter();
    
    // Outer housing
    painter.circle(
        center,
        outer_radius,
        egui::Color32::TRANSPARENT,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
    );
    
    // Emissive filled circle
    let fill_color = if active {
        egui::Color32::from_rgb(0, 255, 136) // Neon Green
    } else {
        egui::Color32::from_rgb(50, 50, 50) // Dark Grey
    };
    painter.circle_filled(center, inner_radius, fill_color);
}

