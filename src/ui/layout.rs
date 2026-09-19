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
                                        // Punch out on stop
                                        self.app.commit_recorded_samples();

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
                                        // Punch out on seek
                                        self.app.commit_recorded_samples();
                                        
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
                            let mut relocate_effect_id = None;
                            
                            ui.heading("Effect Controls");
                            ui.add_space(8.0);
                            
                            let mut instance_idx = None;
                            for (idx, inst) in self.app.timeline.instances.iter().enumerate() {
                                if inst.id == id {
                                    instance_idx = Some(idx);
                                    break;
                                }
                            }
                            
                            let mut push_undo = false;
                            let mut isolate_instance = false;
                            let mut update_start_to = None;
                            let mut update_duration_to = None;
                            let mut update_relay_to = None;

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
                                            if slider.drag_started() || (slider.changed() && !slider.dragged()) {
                                                push_undo = true;
                                            }
                                            if slider.changed() {
                                                update_start_to = Some((start_secs * 1000.0) as u64);
                                                timeline_dirty = true;
                                            }
                                        });
                                        
                                        let mut duration_ms = template.duration_ms as f64;
                                        ui.horizontal(|ui| {
                                            ui.label("Duration:");
                                            let slider = ui.add(egui::Slider::new(&mut duration_ms, 50.0..=60000.0).suffix("ms"));
                                            if slider.drag_started() || (slider.changed() && !slider.dragged()) {
                                                push_undo = true;
                                            }
                                            if slider.changed() {
                                                isolate_instance = true;
                                                update_duration_to = Some(duration_ms as u64);
                                                timeline_dirty = true;
                                            }
                                        });
                                    });
                                    
                                    ui.add_space(8.0);

                                    let current_relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                    let is_mismatched = !template.target.is_compatible_with_relay(current_relay_id);
                                    if is_mismatched {
                                        ui.group(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(245, 158, 11),
                                                format!("⚠️ Track Mismatch: Configured for {}, but placed on R{}", template.target.display_name(), current_relay_id),
                                            );
                                            if let Some(primary) = template.target.primary_relay_id() {
                                                if ui.button(format!("⚡ Relocate to R{}: {}", primary, template.target.display_name())).clicked() {
                                                    relocate_effect_id = Some(template.id);
                                                }
                                            }
                                        });
                                        ui.add_space(8.0);
                                    }
                                    
                                    ui.group(|ui| {
                                        ui.strong("Hardware Target");
                                        ui.add_space(4.0);
                                        
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
                                            push_undo = true;
                                            isolate_instance = true;
                                            update_relay_to = Some(selected_relay);
                                            timeline_dirty = true;
                                        }
                                    });
                                    
                                    ui.add_space(12.0);
                                    if ui.button(egui::RichText::new("🗑 Delete Cue").color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                        delete_cue = true;
                                    }
                                }
                            }
                            
                            if push_undo {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                            }

                            if isolate_instance {
                                self.app.isolate_template_for_instance(id);
                            }

                            if let Some(new_start) = update_start_to {
                                if let Some(inst) = self.app.timeline.instances.iter_mut().find(|i| i.id == id) {
                                    inst.start_time_ms = new_start;
                                }
                            }

                            if let Some(new_dur) = update_duration_to {
                                if let Some(inst) = self.app.timeline.instances.iter().find(|i| i.id == id) {
                                    let eff_id = inst.effect_id;
                                    if let Some(tmpl) = self.app.timeline.templates.iter_mut().find(|t| t.id == eff_id) {
                                        crate::app::update_effect_duration(tmpl, new_dur);
                                    }
                                }
                            }

                            if let Some(new_relay) = update_relay_to {
                                if let Some(inst) = self.app.timeline.instances.iter().find(|i| i.id == id) {
                                    let eff_id = inst.effect_id;
                                    if let Some(tmpl) = self.app.timeline.templates.iter_mut().find(|t| t.id == eff_id) {
                                        if tmpl.actions.is_empty() {
                                            tmpl.actions = crate::four_d::patterns::generate_constant(new_relay, true, tmpl.duration_ms);
                                        } else {
                                            for a in &mut tmpl.actions {
                                                a.relay_id = new_relay;
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(eff_id) = relocate_effect_id {
                                self.app.relocate_effect_to_primary(eff_id);
                                ui.ctx().request_repaint();
                            }

                            if delete_cue {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                                self.app.timeline.instances.retain(|inst| inst.id != id);
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }
                            
                            if timeline_dirty {
                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                ui.ctx().request_repaint();
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
                                                        target: preset.effect.target,
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
                                                            
                                                            egui::Tooltip::always_open(
                                                                ui.ctx().clone(),
                                                                ui.layer_id(),
                                                                egui::Id::new("dnd_tooltip"),
                                                                egui::PopupAnchor::Pointer,
                                                            )
                                                            .show(|ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(format!("{} {}", preset.effect.icon, preset.effect.name));
                                                                    ui.label(egui::RichText::new(format!("({}ms)", preset.effect.duration_ms)).weak());
                                                                });
                                                            });
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
                                ui.set_width(250.0);
                                
                                // 26px spacer to align with the right-side ruler
                                let (header_rect, _) = ui.allocate_exact_size(egui::vec2(250.0, 26.0), egui::Sense::hover());
                                ui.painter().rect_filled(header_rect, 0.0, egui::Color32::from_rgb(33, 33, 33));
                                ui.painter().line_segment(
                                    [egui::pos2(header_rect.min.x, header_rect.max.y), egui::pos2(header_rect.max.x, header_rect.max.y)],
                                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 45)),
                                );
                                ui.painter().text(
                                    header_rect.left_center() + egui::vec2(6.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    "TRACKS",
                                    egui::FontId::proportional(11.0),
                                    egui::Color32::from_rgb(150, 150, 150),
                                );
                                
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
                                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(250.0, 32.0), egui::Sense::hover());
                                    // Draw background with dark Premiere aesthetics
                                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(26, 26, 26));
                                    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 45)), egui::StrokeKind::Inside);
                                    
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
                                            let m_btn = ui.selectable_label(*muted, egui::RichText::new("M").strong().size(10.0))
                                                .on_hover_text("Mute Track (M)\nMutes relay physical output during playback.");
                                            if m_btn.clicked() {
                                                *muted = !*muted;
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }
                                            
                                            // Solo button (S)
                                            let soloed = &mut self.app.track_soloed[relay_id];
                                            let s_btn = ui.selectable_label(*soloed, egui::RichText::new("S").strong().size(10.0))
                                                .on_hover_text("Solo Track (S)\nSolos this relay track output during playback.");
                                            if s_btn.clicked() {
                                                *soloed = !*soloed;
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }
                                            
                                            // Lock button (L)
                                            let locked = &mut self.app.track_locked[relay_id];
                                            let l_btn = ui.selectable_label(*locked, egui::RichText::new("L").strong().size(10.0))
                                                .on_hover_text("Lock Track (L)\nPrevents moving or modifying effects on this track.");
                                            if l_btn.clicked() {
                                                *locked = !*locked;
                                            }
                                        }
                                    });
                                }

                                // Analog Curve Track Headers
                                let mut analog_tracks_changed = false;
                                for track in self.app.timeline.analog_tracks.iter_mut() {
                                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(250.0, 40.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(22, 28, 32));
                                    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 45)), egui::StrokeKind::Inside);

                                    // Amplitude Y-axis tick labels on track header right margin
                                    let painter = ui.painter();
                                    painter.text(
                                        egui::pos2(rect.max.x - 3.0, rect.min.y + 5.0),
                                        egui::Align2::RIGHT_TOP,
                                        "100%",
                                        egui::FontId::monospace(7.5),
                                        egui::Color32::from_rgb(90, 90, 90),
                                    );
                                    painter.text(
                                        egui::pos2(rect.max.x - 3.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        "50%",
                                        egui::FontId::monospace(7.5),
                                        egui::Color32::from_rgb(70, 70, 70),
                                    );
                                    painter.text(
                                        egui::pos2(rect.max.x - 3.0, rect.max.y - 5.0),
                                        egui::Align2::RIGHT_BOTTOM,
                                        "0%",
                                        egui::FontId::monospace(7.5),
                                        egui::Color32::from_rgb(90, 90, 90),
                                    );

                                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                                    child_ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        ui.allocate_ui(egui::vec2(85.0, 20.0), |ui| {
                                            ui.label(egui::RichText::new(format!("P{}: {}", track.channel, track.name)).size(10.5).strong().color(egui::Color32::from_rgb(0, 220, 255)))
                                                .on_hover_text(format!("Analog Track: {}\nPort/Channel: P{}", track.name, track.channel));
                                        });

                                        let m_btn = ui.selectable_label(track.muted, egui::RichText::new("M").strong().size(10.0))
                                            .on_hover_text("Mute Track (M)\nMutes actuator physical output during playback.");
                                        if m_btn.clicked() {
                                            track.muted = !track.muted;
                                            analog_tracks_changed = true;
                                        }

                                        // Record Arm Button [●]
                                        let arm_color = if track.armed {
                                            egui::Color32::from_rgb(255, 60, 60)
                                        } else {
                                            egui::Color32::from_rgb(120, 120, 120)
                                        };
                                        let arm_btn = ui.selectable_label(
                                            track.armed,
                                            egui::RichText::new("●").size(12.0).color(arm_color),
                                        )
                                        .on_hover_text("Record Arm (R)\nArms this track for real-time motion capture gesture recording.");
                                        if arm_btn.clicked() {
                                            track.armed = !track.armed;
                                            analog_tracks_changed = true;
                                            
                                            if !track.armed && self.app.recording_session.sample_count(track.id) > 0 {
                                                self.app.recording_session.commit_to_track(
                                                    track,
                                                    0.015,
                                                    crate::four_d::curve::Interpolation::Smooth,
                                                );
                                            }
                                        }
                                        
                                        if track.armed {
                                            let mut val = self.app.input_capture.current_throttle;
                                            let slider = egui::Slider::new(&mut val, 0.0..=1.0)
                                                .show_value(false)
                                                .text("");
                                            if ui.add_sized([65.0, 16.0], slider)
                                                .on_hover_text("Live Actuator Fader\nControl actuator intensity in real time (0% - 100%).")
                                                .changed() {
                                                self.app.input_capture.set_throttle(val);
                                                let byte_val = (val * 255.0).round() as u8;
                                                let _ = self.app.engine_handle.sender.send(
                                                    crate::four_d::engine::EngineMessage::LiveActuatorOverride {
                                                        channel: track.channel,
                                                        value: byte_val,
                                                    },
                                                );
                                            }
                                        }

                                        let add_btn = ui.button(egui::RichText::new("+").size(10.0))
                                            .on_hover_text("Add Keyframe\nInserts a keyframe at the current playhead position.");
                                        if add_btn.clicked() {
                                            let cur_ms = (self.app.playback_time * 1000.0) as u64;
                                            let cur_val = track.evaluate(cur_ms);
                                            track.add_keyframe(crate::four_d::curve::Keyframe::new(cur_ms, cur_val, crate::four_d::curve::Interpolation::Linear));
                                            analog_tracks_changed = true;
                                        }
                                    });
                                }
                                if analog_tracks_changed {
                                    let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                }
                            });
                            
                            // 2. Right column: Scrollable Timeline Grid
                            let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                            let zoom = self.app.timeline_zoom;
                            let px_per_ms = zoom / 1000.0;
                            let total_seconds = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                            let total_width = (total_seconds * zoom as f64) as f32;
                            let num_analog = self.app.timeline.analog_tracks.len();
                            let total_height = 26.0 + 320.0 + (num_analog as f32 * 40.0);
                            
                            // Define dropping target zone
                            let drop_res = ui.dnd_drop_zone::<EffectDragPayload, _>(egui::Frame::NONE, |ui| {
                                egui::ScrollArea::both()
                                    .id_salt("timeline_scroll")
                                    .show(ui, |ui| {
                                        let size = egui::vec2(total_width, total_height);
                                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                                        
                                        let painter = ui.painter();
                                        
                                        // Draw timeline tracks background
                                        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(33, 33, 33));
                                        
                                        let tracks_top = rect.min.y + 26.0;

                                        // Allocate top 26px band of the timeline grid canvas as dedicated time ruler
                                        let ruler_rect = egui::Rect::from_min_max(
                                            egui::pos2(rect.min.x, rect.min.y),
                                            egui::pos2(rect.max.x, tracks_top),
                                        );

                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        if let Some(pos) = pointer_pos {
                                            if rect.contains(pos) && (ui.input(|i| i.modifiers.ctrl || i.modifiers.command)) && scroll_delta.y != 0.0 {
                                                self.app.timeline_zoom = (self.app.timeline_zoom + scroll_delta.y * 0.2).clamp(20.0, 500.0);
                                            }
                                        }

                                        let ruler_response = ui.interact(ruler_rect, egui::Id::new("timeline_ruler"), egui::Sense::click_and_drag())
                                            .on_hover_text("Timeline Ruler\nClick or drag to scrub playhead. Ctrl+Scroll to zoom time.");

                                        if let Some(pos) = pointer_pos {
                                            if (ruler_rect.contains(pos) || ruler_response.dragged())
                                                && self.app.active_drag.is_none()
                                                && self.app.lasso_origin.is_none()
                                                && self.app.active_keyframe_drag.is_none()
                                            {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                if ui.input(|i| i.pointer.primary_down()) || ruler_response.dragged() {
                                                    let relative_x = (pos.x - rect.min.x).max(0.0);
                                                    let target_time = ((relative_x / zoom) as f64).clamp(0.0, total_seconds);
                                                    self.app.playback_time = target_time;
                                                    self.app.commit_recorded_samples();
                                                    let _ = self.app.mpv.command("seek", &[&target_time.to_string(), "absolute"]);
                                                }
                                            }
                                        }

                                        // Draw grid lines
                                        // Major grid lines every second (zoom px)
                                        for i in 0..=(total_seconds.ceil() as i32) {
                                            let grid_x = rect.min.x + (i as f32 * zoom);
                                            if grid_x <= rect.max.x {
                                                painter.line_segment(
                                                    [egui::pos2(grid_x, rect.min.y), egui::pos2(grid_x, rect.max.y)],
                                                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 50, 50)),
                                                );
                                            }
                                        }
                                        
                                        // Draw horizontal track separators and backgrounds
                                        for i in 0..=10 {
                                            let grid_y = tracks_top + (i as f32 * 32.0);
                                            
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
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 45)),
                                            );
                                        }

                                        // Horizontal separators for Analog tracks
                                        for (t_idx, _) in self.app.timeline.analog_tracks.iter().enumerate() {
                                            let grid_y = tracks_top + 320.0 + ((t_idx + 1) as f32 * 40.0);
                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, grid_y), egui::pos2(rect.max.x, grid_y)],
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 45, 45)),
                                            );
                                        }
                                        
                                        // Highlight target destination track row during active move drag
                                        if let Some(drag) = &self.app.active_drag {
                                            if drag.mode == crate::app::DragMode::Move {
                                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                    let relative_y = mouse_pos.y - tracks_top;
                                                    let track_index = (relative_y / 32.0).floor() as i32;
                                                    if track_index >= 2 && track_index <= 9 {
                                                        let target_r = (track_index - 1) as u8;
                                                        let target_compatible = self.app.timeline.instances.iter()
                                                            .find(|i| i.id == drag.instance_id)
                                                            .and_then(|inst| self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id))
                                                            .map(|tmpl| tmpl.target.is_compatible_with_relay(target_r))
                                                            .unwrap_or(true);

                                                        if !self.app.track_locked[target_r as usize] && target_compatible {
                                                            let row_y = tracks_top + (track_index as f32 * 32.0);
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(rect.max.x, row_y + 32.0),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(46, 204, 113, 25)); // Faint green highlight
                                                        } else if !target_compatible && !self.app.track_locked[target_r as usize] {
                                                            let row_y = tracks_top + (track_index as f32 * 32.0);
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(rect.max.x, row_y + 32.0),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(231, 76, 60, 30)); // Faint red warning highlight for incompatible track
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Render Video clip placeholder if active
                                        if self.app.duration > 0.0 {
                                            let video_clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, tracks_top + 4.0),
                                                egui::pos2(rect.min.x + total_width, tracks_top + 28.0),
                                            );
                                            painter.rect_filled(video_clip_rect, 4.0, egui::Color32::from_rgb(41, 128, 185)); // Blue clip
                                            painter.rect_stroke(video_clip_rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::WHITE), egui::StrokeKind::Inside);
                                            painter.text(
                                                video_clip_rect.left_center() + egui::vec2(10.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                "Active Video File",
                                                egui::FontId::proportional(11.0),
                                                egui::Color32::WHITE,
                                            );
                                            
                                            // Audio clip placeholder
                                            let audio_clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, tracks_top + 32.0 + 4.0),
                                                egui::pos2(rect.min.x + total_width, tracks_top + 32.0 + 28.0),
                                            );
                                            painter.rect_filled(audio_clip_rect, 4.0, egui::Color32::from_rgb(39, 174, 96)); // Green clip
                                            painter.rect_stroke(audio_clip_rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::WHITE), egui::StrokeKind::Inside);
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
                                        let mut relocate_to_primary = None;
                                        let mut delete_cue_id = None;
                                        
                                        // 1st Pass: Draw all non-dragged clips
                                        let active_drag_id = self.app.active_drag.as_ref().map(|d| d.instance_id);
                                        let mut dragged_clip_data = None;
                                        
                                        for instance in &self.app.timeline.instances {
                                            if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                                // Find the relay used by this template's actions
                                                let relay_id = effect.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                                let is_mismatched = !effect.target.is_compatible_with_relay(relay_id);
                                                
                                                // Determine Y range based on relay_id (1..8)
                                                let track_index = relay_id as f32 + 1.0;
                                                let track_y = tracks_top + (track_index * 32.0);
                                                
                                                let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                                let end_x = start_x + (effect.duration_ms as f32 * px_per_ms);
                                                
                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(start_x, track_y + 4.0),
                                                    egui::pos2(end_x, track_y + 28.0),
                                                );
                                                
                                                let clip_id = egui::Id::new(instance.id);
                                                let is_track_locked = self.app.track_locked[relay_id as usize];
                                                
                                                let mut clip_response = if is_track_locked {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click())
                                                } else {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click_and_drag())
                                                };
                                                
                                                if is_mismatched {
                                                    let warn_msg = format!(
                                                        "⚠️ HARDWARE MISMATCH DETECTED\n• Effect: {}\n• Requires: {}\n• Current Track: R{}\n• Hazard: Will trigger the wrong physical actuator in live show!\nRight-click to automatically relocate.",
                                                        effect.name, effect.target.display_name(), relay_id
                                                    );
                                                    clip_response = clip_response.on_hover_text(warn_msg);
                                                }

                                                clip_response.context_menu(|ui| {
                                                    if is_mismatched {
                                                        if let Some(primary) = effect.target.primary_relay_id() {
                                                            if ui.button(format!("⚡ Relocate to R{}: {}", primary, effect.target.display_name())).clicked() {
                                                                relocate_to_primary = Some(instance.effect_id);
                                                                ui.close();
                                                            }
                                                            ui.separator();
                                                        }
                                                    }
                                                    if ui.button(egui::RichText::new("🗑 Delete Cue").color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                                        delete_cue_id = Some(instance.id);
                                                        ui.close();
                                                    }
                                                });
                                                
                                                let is_hovered = clip_response.hovered() && !is_track_locked;
                                                let mut hovered_handle = None;

                                                if is_hovered && self.app.active_drag.is_none() {
                                                    if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                        let mode = crate::app::classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), mouse_pos.x);
                                                        match mode {
                                                            crate::app::DragMode::ResizeLeft => {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                                hovered_handle = Some(crate::app::DragMode::ResizeLeft);
                                                            }
                                                            crate::app::DragMode::ResizeRight => {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                                hovered_handle = Some(crate::app::DragMode::ResizeRight);
                                                            }
                                                            crate::app::DragMode::Move => {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                                            }
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
                                                        let press_x = ui.ctx().input(|i| i.pointer.press_origin())
                                                            .or_else(|| clip_response.interact_pointer_pos())
                                                            .map(|p| p.x)
                                                            .unwrap_or(mouse_pos.x);

                                                        let drag_mode = crate::app::classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), press_x);
                                                        started_drag = Some((instance.id, drag_mode, instance.start_time_ms, effect.duration_ms, mouse_pos.x, initial_positions));
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
                                                } else if is_mismatched {
                                                    egui::Color32::from_rgb(245, 158, 11) // Warning amber border
                                                } else {
                                                    egui::Color32::WHITE
                                                };
                                                let stroke_width = if is_selected { 2.0_f32 } else if is_mismatched { 1.5_f32 } else { 1.0_f32 };
                                                
                                                let is_muted = self.app.track_muted[relay_id as usize];
                                                let alpha = if is_muted { 128 } else { 255 };
                                                
                                                // Draw clip box
                                                painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgba_unmultiplied(142, 68, 173, alpha)); // Purple clip
                                                painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color), egui::StrokeKind::Inside);
                                                
                                                // Visual handle grips
                                                let left_active = hovered_handle == Some(crate::app::DragMode::ResizeLeft);
                                                let right_active = hovered_handle == Some(crate::app::DragMode::ResizeRight);
                                                render_clip_handles(&painter, clip_rect, left_active, right_active, alpha);

                                                // Clip name label
                                                let title = if is_mismatched {
                                                    format!("⚠️ {} {}", effect.icon, effect.name)
                                                } else {
                                                    format!("{} {}", effect.icon, effect.name)
                                                };
                                                painter.text(
                                                    clip_rect.left_center() + egui::vec2(12.0, 0.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    title,
                                                    egui::FontId::proportional(10.0),
                                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                                                );
                                            }
                                        }
                                        
                                        // 2nd Pass: Draw the actively dragged clip on top with a shadow and brighter color
                                        if let Some((clip_rect, instance_id, effect, relay_id)) = dragged_clip_data {
                                            let is_mismatched = !effect.target.is_compatible_with_relay(relay_id);
                                            let is_selected = self.app.selected_instance_ids.contains(&instance_id);
                                            let stroke_color = if is_selected {
                                                egui::Color32::from_rgb(255, 235, 59)
                                            } else if is_mismatched {
                                                egui::Color32::from_rgb(245, 158, 11)
                                            } else {
                                                egui::Color32::WHITE
                                            };
                                            let stroke_width = if is_selected { 2.0_f32 } else if is_mismatched { 1.5_f32 } else { 1.0_f32 };
                                            
                                            // Draw drop shadow
                                            let shadow_rect = clip_rect.translate(egui::vec2(2.0, 3.0));
                                            painter.rect_filled(shadow_rect, 4.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 80));
                                            
                                            // Draw bright purple clip
                                            painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgb(172, 98, 203)); // Brighter purple
                                            painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color), egui::StrokeKind::Inside);
                                            
                                            // Visual handle grips
                                            let left_active = self.app.active_drag.as_ref().map(|d| d.mode) == Some(crate::app::DragMode::ResizeLeft);
                                            let right_active = self.app.active_drag.as_ref().map(|d| d.mode) == Some(crate::app::DragMode::ResizeRight);
                                            render_clip_handles(&painter, clip_rect, left_active, right_active, 255);

                                            // Clip name label
                                            let title = if is_mismatched {
                                                format!("⚠️ {} {}", effect.icon, effect.name)
                                            } else {
                                                format!("{} {}", effect.icon, effect.name)
                                            };
                                            painter.text(
                                                clip_rect.left_center() + egui::vec2(12.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                title,
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        
                                        // Apply relocation or deletion from context menu outside borrow loop
                                        if let Some(effect_id) = relocate_to_primary {
                                            self.app.relocate_effect_to_primary(effect_id);
                                            ui.ctx().request_repaint();
                                        }
                                        if let Some(cue_id) = delete_cue_id {
                                            self.app.undo_stack.push(self.app.snapshot_timeline());
                                            self.app.timeline.instances.retain(|inst| inst.id != cue_id);
                                            self.app.selected_instance_ids.remove(&cue_id);
                                            let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                            let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            ui.ctx().request_repaint();
                                        }
                                        
                                        // Apply selection or drag start outside the borrow loop
                                        if let Some((drag_id, mode, init_start, init_dur, start_x, init_positions)) = started_drag {
                                            // Push undo snapshot before mutating timeline
                                            self.app.undo_stack.push(self.app.snapshot_timeline());

                                            // If resizing, isolate template if shared by multiple instances
                                            if mode == crate::app::DragMode::ResizeLeft || mode == crate::app::DragMode::ResizeRight {
                                                self.app.isolate_template_for_instance(drag_id);
                                            }

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
                                            match drag_state.mode {
                                                crate::app::DragMode::Move => {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                }
                                                crate::app::DragMode::ResizeLeft | crate::app::DragMode::ResizeRight => {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                }
                                            }

                                            if ui.ctx().input(|i| i.pointer.any_released()) {
                                                drag_ended = true;
                                            } else {
                                                let delta_x = ui.ctx().pointer_latest_pos().map(|p| p.x).unwrap_or(drag_state.drag_start_x) - drag_state.drag_start_x;
                                                let delta_time_ms = (delta_x / px_per_ms) as i64;
                                                
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
                                                
                                                let hud_text = match drag_state.mode {
                                                    crate::app::DragMode::Move => {
                                                        // Vertical track switching
                                                        let mut target_relay = None;
                                                        if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                            let relative_y = mouse_pos.y - tracks_top;
                                                            let track_index = (relative_y / 32.0).floor() as i32;
                                                            if track_index >= 2 && track_index <= 9 {
                                                                let target_r = (track_index - 1) as u8;
                                                                if !self.app.track_locked[target_r as usize] {
                                                                    let is_compatible = self.app.timeline.instances.iter()
                                                                        .find(|i| i.id == drag_state.instance_id)
                                                                        .and_then(|inst| self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id))
                                                                        .map(|tmpl| tmpl.target.is_compatible_with_relay(target_r))
                                                                        .unwrap_or(true);
                                                                    if is_compatible {
                                                                        target_relay = Some(target_r);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        
                                                        let mut primary_new_start = (drag_state.initial_start_time_ms as i64 + delta_time_ms).max(0) as u64;
                                                        
                                                        if snap_enabled {
                                                            // Check snap to start
                                                            for target in &snap_targets {
                                                                if (primary_new_start as i64 - *target as i64).abs() <= 100 {
                                                                    primary_new_start = *target;
                                                                    snap_line_x = Some(rect.min.x + (primary_new_start as f32 * px_per_ms));
                                                                    break;
                                                                }
                                                            }
                                                            // Check snap to end
                                                            if snap_line_x.is_none() {
                                                                let primary_new_end = primary_new_start + drag_state.initial_duration_ms;
                                                                for target in &snap_targets {
                                                                    if (primary_new_end as i64 - *target as i64).abs() <= 100 {
                                                                        primary_new_start = target.saturating_sub(drag_state.initial_duration_ms);
                                                                        snap_line_x = Some(rect.min.x + (*target as f32 * px_per_ms));
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

                                                        let start_secs = primary_new_start as f64 / 1000.0;
                                                        let track_name = target_relay.map(|r| format!("R{}", r)).unwrap_or_else(|| "Track".to_string());
                                                        format!("⏱ Start: {:.3}s | {}", start_secs, track_name)
                                                    }
                                                    crate::app::DragMode::ResizeRight => {
                                                        let mut new_end = (drag_state.initial_start_time_ms + drag_state.initial_duration_ms) as i64 + delta_time_ms;
                                                        if snap_enabled {
                                                            for target in &snap_targets {
                                                                if (new_end - *target as i64).abs() <= 100 {
                                                                    new_end = *target as i64;
                                                                    snap_line_x = Some(rect.min.x + (new_end as f32 * px_per_ms));
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        let new_dur = (new_end - drag_state.initial_start_time_ms as i64).max(100) as u64;
                                                        let instance_id = drag_state.instance_id;
                                                        if let Some(instance) = self.app.timeline.instances.iter_mut().find(|inst| inst.id == instance_id) {
                                                            if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                                                                crate::app::update_effect_duration(template, new_dur);
                                                            }
                                                        }

                                                        let delta_ms = (new_dur as i64) - (drag_state.initial_duration_ms as i64);
                                                        let delta_str = if delta_ms >= 0 { format!("+{}ms", delta_ms) } else { format!("{}ms", delta_ms) };
                                                        let dur_secs = new_dur as f64 / 1000.0;
                                                        format!("⏱ Dur: {:.2}s ({})", dur_secs, delta_str)
                                                    }
                                                    crate::app::DragMode::ResizeLeft => {
                                                        let right_anchor = drag_state.initial_start_time_ms + drag_state.initial_duration_ms;
                                                        let mut new_start = (drag_state.initial_start_time_ms as i64 + delta_time_ms).max(0) as u64;
                                                        if snap_enabled {
                                                            for target in &snap_targets {
                                                                if (new_start as i64 - *target as i64).abs() <= 100 {
                                                                    new_start = *target;
                                                                    snap_line_x = Some(rect.min.x + (new_start as f32 * px_per_ms));
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        new_start = new_start.min(right_anchor.saturating_sub(100));
                                                        let new_dur = right_anchor - new_start;
                                                        let instance_id = drag_state.instance_id;
                                                        if let Some(instance) = self.app.timeline.instances.iter_mut().find(|inst| inst.id == instance_id) {
                                                            instance.start_time_ms = new_start;
                                                            if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                                                                crate::app::update_effect_duration(template, new_dur);
                                                            }
                                                        }

                                                        let delta_ms = (new_dur as i64) - (drag_state.initial_duration_ms as i64);
                                                        let delta_str = if delta_ms >= 0 { format!("+{}ms", delta_ms) } else { format!("{}ms", delta_ms) };
                                                        let dur_secs = new_dur as f64 / 1000.0;
                                                        format!("⏱ Dur: {:.2}s ({})", dur_secs, delta_str)
                                                    }
                                                };

                                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                    let galley = painter.layout_no_wrap(
                                                        hud_text.clone(),
                                                        egui::FontId::monospace(11.0),
                                                        egui::Color32::WHITE,
                                                    );
                                                    let padding = egui::vec2(8.0, 4.0);
                                                    let badge_size = galley.size() + padding * 2.0;
                                                    let hud_center = egui::pos2(mouse_pos.x, mouse_pos.y - 25.0);
                                                    let half_w = badge_size.x / 2.0;
                                                    let half_h = badge_size.y / 2.0;
                                                    let clamped_x = hud_center.x.clamp(rect.min.x + half_w + 4.0, rect.max.x - half_w - 4.0);
                                                    let clamped_y = hud_center.y.clamp(rect.min.y + half_h + 4.0, rect.max.y - half_h - 4.0);
                                                    let hud_rect = egui::Rect::from_center_size(egui::pos2(clamped_x, clamped_y), badge_size);

                                                    painter.rect_filled(
                                                        hud_rect,
                                                        4.0,
                                                        egui::Color32::from_black_alpha(220),
                                                    );
                                                    painter.rect_stroke(
                                                        hud_rect,
                                                        4.0,
                                                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 220, 255)),
                                                        egui::StrokeKind::Inside,
                                                    );
                                                    painter.text(
                                                        hud_rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        hud_text,
                                                        egui::FontId::monospace(11.0),
                                                        egui::Color32::WHITE,
                                                    );
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

                                        // Keyboard throttle update for Live Fader
                                        let up = ui.input(|i| i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp));
                                        let down = ui.input(|i| i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown));
                                        let dt = ui.input(|i| i.unstable_dt).min(0.05);
                                        self.app.input_capture.update_from_keyboard(up, down, dt);

                                        let is_playing_now = !self.app.is_paused && self.app.duration > 0.0;
                                        let was_recording = self.app.is_recording;
                                        let cur_time_ms = (self.app.playback_time * 1000.0) as u64;
                                        self.app.is_recording = false;

                                        if is_playing_now {
                                            for track in self.app.timeline.analog_tracks.iter_mut() {
                                                if track.armed {
                                                    self.app.is_recording = true;
                                                    let last_time = self.app.recording_session.get_live_samples(track.id).and_then(|s| s.last()).map(|s| s.0);
                                                    if last_time != Some(cur_time_ms) {
                                                        self.app.recording_session.record_sample(track.id, cur_time_ms, self.app.input_capture.current_throttle);
                                                    }
                                                    let byte_val = (self.app.input_capture.current_throttle * 255.0).round() as u8;
                                                    let _ = self.app.engine_handle.sender.send(
                                                        crate::four_d::engine::EngineMessage::LiveActuatorOverride {
                                                            channel: track.channel,
                                                            value: byte_val,
                                                        },
                                                    );
                                                }
                                            }
                                        }

                                        if was_recording && !is_playing_now {
                                            self.app.commit_recorded_samples();
                                        }
                                        
                                        if self.app.is_recording || (is_playing_now && self.app.timeline.analog_tracks.iter().any(|t| t.armed)) {
                                            ui.ctx().request_repaint();
                                        }

                                        // Render Analog Curve Tracks
                                        let mut clicked_any_keyframe = false;
                                        let mut curve_updated = false;
                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        let mut started_drag_info = None;
                                        let mut kf_interp_change = None;
                                        let mut kf_to_remove = None;
                                        let mut pending_add_keyframe = None;

                                        for (t_idx, track) in self.app.timeline.analog_tracks.iter_mut().enumerate() {
                                            let row_y = tracks_top + 320.0 + (t_idx as f32 * 40.0);
                                            let row_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, row_y),
                                                egui::pos2(rect.max.x, row_y + 40.0),
                                            );

                                            // Row background shading
                                            painter.rect_filled(row_rect, 0.0, egui::Color32::from_rgb(20, 25, 29));

                                            // Centerline guide (50% intensity)
                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, row_y + 20.0), egui::pos2(rect.max.x, row_y + 20.0)],
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(34, 42, 48)),
                                            );

                                            // Sample continuous curve along timeline
                                            let step_px = 6.0_f32;
                                            let mut points = Vec::new();
                                            let mut curr_x = rect.min.x;
                                            while curr_x <= rect.max.x {
                                                let t_ms = (((curr_x - rect.min.x) / zoom) * 1000.0).max(0.0) as u64;
                                                let norm_val = track.evaluate(t_ms);
                                                let py = (row_y + 36.0) - (norm_val * 32.0);
                                                points.push(egui::pos2(curr_x, py));
                                                curr_x += step_px;
                                            }

                                            // Draw translucent fill under curve (Curve Gradient Underlay)
                                            let fill_col = if track.muted {
                                                egui::Color32::from_rgba_unmultiplied(100, 100, 100, 20)
                                            } else {
                                                egui::Color32::from_rgba_unmultiplied(0, 220, 255, 25)
                                            };
                                            for window in points.windows(2) {
                                                let p1 = window[0];
                                                let p2 = window[1];
                                                let b1 = egui::pos2(p1.x, row_y + 36.0);
                                                let b2 = egui::pos2(p2.x, row_y + 36.0);
                                                painter.add(egui::Shape::convex_polygon(
                                                    vec![b1, p1, p2, b2],
                                                    fill_col,
                                                    egui::Stroke::NONE,
                                                ));
                                            }

                                            // Draw curve line
                                            let curve_color = if track.muted {
                                                egui::Color32::from_rgb(110, 110, 110)
                                            } else {
                                                egui::Color32::from_rgb(0, 220, 255)
                                            };
                                            painter.add(egui::Shape::line(
                                                points,
                                                egui::Stroke::new(1.8_f32, curve_color),
                                            ));

                                            // Draw recording ghost trail
                                            if track.armed {
                                                if let Some(live_samples) = self.app.recording_session.get_live_samples(track.id) {
                                                    if !live_samples.is_empty() {
                                                        let mut ghost_points = Vec::with_capacity(live_samples.len());
                                                        for s in live_samples {
                                                            let gx = rect.min.x + (s.0 as f32 * px_per_ms);
                                                            let gy = (row_y + 36.0) - (s.1 * 32.0);
                                                            ghost_points.push(egui::pos2(gx, gy));
                                                        }
                                                        painter.add(egui::Shape::line(
                                                            ghost_points,
                                                            egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(255, 50, 50)),
                                                        ));
                                                    }
                                                }
                                            }

                                            // Keyframe markers and interactions
                                            for (k_idx, kf) in track.keyframes.iter().enumerate() {
                                                let kx = rect.min.x + (kf.time_ms as f32 * px_per_ms);
                                                let ky = (row_y + 36.0) - (kf.value * 32.0);
                                                let center = egui::pos2(kx, ky);
                                                let is_selected = self.app.selected_keyframes.contains(&(track.id, k_idx));

                                                // 16px Euclidean distance hitbox detection
                                                let hover_dist = 16.0;
                                                let is_hovered = pointer_pos.map_or(false, |pos| pos.distance(center) <= hover_dist);

                                                if is_hovered && self.app.active_keyframe_drag.is_none() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                                }

                                                let diamond = vec![
                                                    egui::pos2(center.x, center.y - 5.0),
                                                    egui::pos2(center.x + 5.0, center.y),
                                                    egui::pos2(center.x, center.y + 5.0),
                                                    egui::pos2(center.x - 5.0, center.y),
                                                ];
                                                let fill_diamond = if is_selected {
                                                    egui::Color32::from_rgb(255, 230, 0)
                                                } else {
                                                    curve_color
                                                };

                                                // Outline glows bright white/yellow on hover
                                                let stroke = if is_hovered {
                                                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 255, 180))
                                                } else {
                                                    egui::Stroke::new(1.2_f32, egui::Color32::WHITE)
                                                };

                                                if is_hovered {
                                                    let glow_diamond = vec![
                                                        egui::pos2(center.x, center.y - 7.5),
                                                        egui::pos2(center.x + 7.5, center.y),
                                                        egui::pos2(center.x, center.y + 7.5),
                                                        egui::pos2(center.x - 7.5, center.y),
                                                    ];
                                                    painter.add(egui::Shape::convex_polygon(
                                                        glow_diamond,
                                                        egui::Color32::from_rgba_unmultiplied(255, 255, 150, 45),
                                                        egui::Stroke::NONE,
                                                    ));
                                                }

                                                painter.add(egui::Shape::convex_polygon(
                                                    diamond,
                                                    fill_diamond,
                                                    stroke,
                                                ));

                                                // Keyframe interact widget for context menu and clicks
                                                let kf_rect = egui::Rect::from_center_size(center, egui::vec2(32.0, 32.0));
                                                let kf_id = egui::Id::new((track.id, k_idx, "kf_node"));
                                                let mut kf_response = ui.interact(kf_rect, kf_id, egui::Sense::click());

                                                // Native tooltip on hover showing timecode, value percentage, and interpolation mode
                                                if self.app.active_keyframe_drag.is_none() {
                                                    let interp_name = match kf.interpolation {
                                                        crate::four_d::curve::Interpolation::Step => "Step",
                                                        crate::four_d::curve::Interpolation::Linear => "Linear",
                                                        crate::four_d::curve::Interpolation::Smooth => "Smooth (Hermite)",
                                                    };
                                                    kf_response = kf_response.on_hover_ui(|ui| {
                                                        ui.label(format!("Time: {}", format_timecode(kf.time_ms as f64 / 1000.0)));
                                                        ui.label(format!("Value: {:.1}%", kf.value * 100.0));
                                                        ui.label(format!("Interpolation: {}", interp_name));
                                                    });
                                                }

                                                // Right-Click Context Menu
                                                kf_response.context_menu(|ui| {
                                                    ui.menu_button("Interpolation", |ui| {
                                                        if ui.button("Linear").clicked() {
                                                            kf_interp_change = Some((track.id, k_idx, crate::four_d::curve::Interpolation::Linear));
                                                            ui.close();
                                                        }
                                                        if ui.button("Smooth (Hermite)").clicked() {
                                                            kf_interp_change = Some((track.id, k_idx, crate::four_d::curve::Interpolation::Smooth));
                                                            ui.close();
                                                        }
                                                        if ui.button("Step").clicked() {
                                                            kf_interp_change = Some((track.id, k_idx, crate::four_d::curve::Interpolation::Step));
                                                            ui.close();
                                                        }
                                                    });
                                                    ui.separator();
                                                    if ui.button("Delete Keyframe").clicked() {
                                                        kf_to_remove = Some((track.id, k_idx));
                                                        ui.close();
                                                    }
                                                });

                                                if is_hovered && ui.input(|i| i.pointer.secondary_clicked()) {
                                                    clicked_any_keyframe = true;
                                                }

                                                // Primary click: Selection & Active Drag Lock initialization on mouse press/down
                                                if is_hovered
                                                    && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary) || i.pointer.primary_down())
                                                    && self.app.active_keyframe_drag.is_none()
                                                {
                                                    if let Some(pos) = pointer_pos {
                                                        started_drag_info = Some((track.id, k_idx, pos, kf.time_ms, kf.value));
                                                        clicked_any_keyframe = true;
                                                    }
                                                }
                                            }

                                            if response.double_clicked() && !clicked_any_keyframe {
                                                if let Some(pos) = response.interact_pointer_pos() {
                                                    if row_rect.contains(pos) {
                                                        let new_t = (((pos.x - rect.min.x) / zoom) * 1000.0).max(0.0) as u64;
                                                        let new_v = ((row_y + 36.0 - pos.y) / 32.0).clamp(0.0, 1.0);
                                                        pending_add_keyframe = Some((track.id, new_t, new_v));
                                                        clicked_any_keyframe = true;
                                                    }
                                                }
                                            }
                                        }

                                        if let Some((tid, new_t, new_v)) = pending_add_keyframe {
                                            let pre_snap = self.app.snapshot_timeline();
                                            self.app.undo_stack.push(pre_snap);
                                            if let Some(track) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                track.add_keyframe(crate::four_d::curve::Keyframe::new(
                                                    new_t,
                                                    new_v,
                                                    crate::four_d::curve::Interpolation::Linear,
                                                ));
                                            }
                                            curve_updated = true;
                                        }

                                        if let Some((tid, kid, new_interp)) = kf_interp_change {
                                            let pre_snap = self.app.snapshot_timeline();
                                            self.app.undo_stack.push(pre_snap);
                                            if let Some(track) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                if let Some(kf) = track.keyframes.get_mut(kid) {
                                                    kf.interpolation = new_interp;
                                                }
                                            }
                                            curve_updated = true;
                                        }

                                        if let Some((tid, kid)) = kf_to_remove {
                                            let pre_snap = self.app.snapshot_timeline();
                                            self.app.undo_stack.push(pre_snap);
                                            if let Some(track) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                if kid < track.keyframes.len() {
                                                    track.keyframes.remove(kid);
                                                }
                                            }
                                            self.app.selected_keyframes.clear();
                                            curve_updated = true;
                                        }

                                        if let Some((track_id, k_idx, pos, orig_t, orig_v)) = started_drag_info {
                                            if !self.app.selected_keyframes.contains(&(track_id, k_idx)) {
                                                if !ui.input(|i| i.modifiers.shift || i.modifiers.ctrl) {
                                                    self.app.selected_keyframes.clear();
                                                }
                                                self.app.selected_keyframes.insert((track_id, k_idx));
                                            }
                                            let mut group_originals = Vec::new();
                                            for &(tid, kid) in &self.app.selected_keyframes {
                                                if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
                                                    if let Some(k) = t.keyframes.get(kid) {
                                                        group_originals.push((tid, kid, k.time_ms, k.value));
                                                    }
                                                }
                                            }
                                            if !group_originals.iter().any(|&(tid, kid, _, _)| tid == track_id && kid == k_idx) {
                                                group_originals.push((track_id, k_idx, orig_t, orig_v));
                                            }
                                            self.app.active_keyframe_drag = Some(crate::app::KeyframeDragState {
                                                track_id,
                                                keyframe_index: k_idx,
                                                start_pointer_pos: pos,
                                                original_time_ms: orig_t,
                                                original_value: orig_v,
                                                group_originals,
                                            });
                                        }

                                        // Active Keyframe Drag Processing & Drag Lock
                                        if let Some(drag) = self.app.active_keyframe_drag.clone() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                            ui.ctx().request_repaint();

                                            let drag_ended = ui.ctx().input(|i| i.pointer.any_released());

                                            if drag_ended {
                                                // Reconstruct pre-drag snapshot
                                                let mut pre_snap = self.app.snapshot_timeline();
                                                for &(tid, kid, orig_t, orig_v) in &drag.group_originals {
                                                    if let Some(t) = pre_snap.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                        if let Some(k) = t.keyframes.get_mut(kid) {
                                                            k.time_ms = orig_t;
                                                            k.value = orig_v;
                                                        }
                                                    }
                                                }
                                                for t in &mut pre_snap.analog_tracks {
                                                    t.keyframes.sort_by_key(|k| k.time_ms);
                                                }
                                                let any_changed = drag.group_originals.iter().any(|&(tid, kid, orig_t, orig_v)| {
                                                    self.app.timeline.analog_tracks.iter().find(|t| t.id == tid)
                                                        .and_then(|t| t.keyframes.get(kid))
                                                        .map_or(false, |k| k.time_ms != orig_t || (k.value - orig_v).abs() > 1e-4)
                                                });
                                                if any_changed {
                                                    self.app.undo_stack.push(pre_snap);
                                                }

                                                let mut target_keyframes = Vec::new();
                                                for &(tid, kid) in &self.app.selected_keyframes {
                                                    if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
                                                        if let Some(k) = t.keyframes.get(kid) {
                                                            target_keyframes.push((tid, k.time_ms, (k.value * 10000.0).round() as i64));
                                                        }
                                                    }
                                                }

                                                // Sort keyframes on mouse release
                                                for track in self.app.timeline.analog_tracks.iter_mut() {
                                                    track.keyframes.sort_by_key(|k| k.time_ms);
                                                }

                                                // Restore selected_keyframes with updated indices
                                                let mut new_selection = std::collections::HashSet::new();
                                                for (tid, target_t, target_v) in target_keyframes {
                                                    if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
                                                        if let Some(new_idx) = t.keyframes.iter().enumerate().position(|(idx, k)| {
                                                            !new_selection.contains(&(tid, idx))
                                                                && k.time_ms == target_t
                                                                && ((k.value * 10000.0).round() as i64 - target_v).abs() <= 1
                                                        }) {
                                                            new_selection.insert((tid, new_idx));
                                                        }
                                                    }
                                                }
                                                self.app.selected_keyframes = new_selection;

                                                self.app.active_keyframe_drag = None;
                                                curve_updated = true;
                                            } else if let Some(pos) = pointer_pos {
                                                let delta_x = pos.x - drag.start_pointer_pos.x;
                                                let delta_time_ms = (delta_x / px_per_ms) as i64;
                                                let raw_new_t = (drag.original_time_ms as i64 + delta_time_ms).max(0) as u64;

                                                // Magnetic Snapping (within 5px of playhead or 1s grid mark)
                                                let playhead_ms = (self.app.playback_time * 1000.0).round() as u64;
                                                let nearest_sec = ((raw_new_t as f64 / 1000.0).round() as u64) * 1000;

                                                let play_x = rect.min.x + (playhead_ms as f32 * px_per_ms);
                                                let grid_x = rect.min.x + (nearest_sec as f32 * px_per_ms);
                                                let kf_x = rect.min.x + (raw_new_t as f32 * px_per_ms);

                                                let snap_enabled = !ui.input(|i| i.modifiers.shift || i.modifiers.alt);
                                                let mut new_t = raw_new_t;
                                                if snap_enabled {
                                                    let dist_play = (kf_x - play_x).abs();
                                                    let dist_grid = (kf_x - grid_x).abs();
                                                    if dist_play <= 5.0 && dist_play <= dist_grid {
                                                        new_t = playhead_ms;
                                                        snap_line_x = Some(play_x);
                                                    } else if dist_grid <= 5.0 {
                                                        new_t = nearest_sec;
                                                        snap_line_x = Some(grid_x);
                                                    }
                                                }

                                                let delta_y = pos.y - drag.start_pointer_pos.y;
                                                let val_delta = -delta_y / 32.0;
                                                let new_v = (drag.original_value + val_delta).clamp(0.0, 1.0);

                                                let time_delta = new_t as i64 - drag.original_time_ms as i64;

                                                for &(tid, kid, orig_t, orig_v) in &drag.group_originals {
                                                    let k_new_t = (orig_t as i64 + time_delta).max(0) as u64;
                                                    let k_new_v = (orig_v + val_delta).clamp(0.0, 1.0);
                                                    if let Some(t) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                        if let Some(k) = t.keyframes.get_mut(kid) {
                                                            k.time_ms = k_new_t;
                                                            k.value = k_new_v;
                                                        }
                                                    }
                                                }
                                                curve_updated = true;

                                                // Floating HUD Badge
                                                let hud_pos = egui::pos2(pos.x, pos.y - 20.0);
                                                let hud_text = format!("{} | {:.1}%", format_timecode(new_t as f64 / 1000.0), new_v * 100.0);
                                                painter.rect_filled(
                                                    egui::Rect::from_center_size(hud_pos, egui::vec2(100.0, 18.0)),
                                                    4.0,
                                                    egui::Color32::from_black_alpha(200),
                                                );
                                                painter.text(hud_pos, egui::Align2::CENTER_CENTER, hud_text, egui::FontId::monospace(10.0), egui::Color32::WHITE);
                                            }
                                        }

                                        if curve_updated {
                                            let _ = self.app.engine_handle.sender.send(
                                                crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                                                    self.app.timeline.analog_tracks.clone(),
                                                ),
                                            );
                                        }

                                        // Render Dedicated Time Ruler Bar Header
                                        painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_rgb(25, 25, 25));
                                        painter.line_segment(
                                            [egui::pos2(ruler_rect.min.x, ruler_rect.max.y), egui::pos2(ruler_rect.max.x, ruler_rect.max.y)],
                                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 50, 50)),
                                        );

                                        let label_step = if zoom < 30.0 {
                                            5
                                        } else if zoom < 60.0 {
                                            2
                                        } else {
                                            1
                                        };

                                        for i in 0..=(total_seconds.ceil() as i32) {
                                            let grid_x = rect.min.x + (i as f32 * zoom);
                                            if grid_x <= rect.max.x - 8.0 {
                                                // Major second tick
                                                painter.line_segment(
                                                    [egui::pos2(grid_x, ruler_rect.max.y - 8.0), egui::pos2(grid_x, ruler_rect.max.y)],
                                                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 100, 100)),
                                                );
                                                // Sub-second frame notches
                                                if zoom >= 50.0 {
                                                    for sub in 1..10 {
                                                        let sub_x = grid_x + (sub as f32 * (zoom / 10.0));
                                                        if sub_x <= rect.max.x - 8.0 {
                                                            let notch_h = if sub == 5 { 5.0 } else { 3.0 };
                                                            painter.line_segment(
                                                                [egui::pos2(sub_x, ruler_rect.max.y - notch_h), egui::pos2(sub_x, ruler_rect.max.y)],
                                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 60, 60)),
                                                            );
                                                        }
                                                    }
                                                }
                                                // Time label inside ruler
                                                if i % label_step == 0 {
                                                    let mut label_x = grid_x + 4.0;
                                                    // Ensure label doesn't clip against right margin
                                                    if label_x + 20.0 > rect.max.x - 8.0 {
                                                        label_x = rect.max.x - 28.0;
                                                    }
                                                    painter.text(
                                                        egui::pos2(label_x, ruler_rect.min.y + 13.0),
                                                        egui::Align2::LEFT_CENTER,
                                                        format!("{}s", i),
                                                        egui::FontId::monospace(9.0),
                                                        egui::Color32::from_rgb(140, 140, 140),
                                                    );
                                                }
                                            }
                                        }

                                        // Draw Playhead
                                        let playhead_x = rect.min.x + (self.app.playback_time as f32 * zoom);
                                        if playhead_x <= rect.max.x {
                                            // Vertical line
                                            painter.line_segment(
                                                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                                                egui::Stroke::new(1.5_f32, egui::Color32::RED),
                                            );
                                            // Playhead handle (triangle at top in ruler bar, 14px width)
                                            let points = vec![
                                                egui::pos2(playhead_x - 7.0, rect.min.y),
                                                egui::pos2(playhead_x + 7.0, rect.min.y),
                                                egui::pos2(playhead_x, rect.min.y + 12.0),
                                            ];
                                            painter.add(egui::Shape::convex_polygon(points, egui::Color32::RED, egui::Stroke::NONE));
                                        }
                                        
                                        // Draw Snap line if active
                                        if let Some(x) = snap_line_x {
                                            painter.line_segment(
                                                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 255, 255)), // Cyan snap line
                                            );
                                        }
                                        
                                        // Target highlighting during active drag
                                        if let Some(payload) = egui::DragAndDrop::payload::<EffectDragPayload>(ui.ctx()) {
                                            if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
                                                if rect.contains(mouse_pos) {
                                                    let relative_y = mouse_pos.y - tracks_top;
                                                    let hovered_track_index = (relative_y / 32.0).floor() as i32;

                                                    // Loop through visible tracks (0..=1 video/audio, 2..=9 relays 1..=8, 10.. analog)
                                                    // For relay tracks (2..=9):
                                                    for i in 2..=9 {
                                                        let relay_id = (i - 1) as u8;
                                                        let is_compatible = payload.target.is_compatible_with_relay(relay_id);
                                                        let is_locked = self.app.track_locked[relay_id as usize];
                                                        let is_primary = payload.target.primary_relay_id() == Some(relay_id);
                                                        let row_y = tracks_top + (i as f32 * 32.0);
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(rect.max.x, row_y + 32.0),
                                                        );

                                                        if hovered_track_index == i {
                                                            if is_locked || !is_compatible {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                                painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 45));
                                                                painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 70, 70)), egui::StrokeKind::Inside);

                                                                let reason = if is_locked {
                                                                    "Track is locked".to_string()
                                                                } else {
                                                                    format!("⊘ Incompatible Track: '{}' requires {} or Aux", payload.name, payload.target.display_name())
                                                                };
                                                                egui::Tooltip::always_open(
                                                                    ui.ctx().clone(),
                                                                    ui.layer_id(),
                                                                    egui::Id::new("drag_incompat_tip"),
                                                                    egui::PopupAnchor::Pointer,
                                                                )
                                                                .show(|ui| {
                                                                    ui.label(reason);
                                                                });
                                                            } else {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
                                                                painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 255, 136, 45));
                                                                painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0, 255, 136)), egui::StrokeKind::Inside);

                                                                let tooltip_text = format!("Drop to place '{}' on R{}: {}", payload.name, relay_id, payload.target.display_name());
                                                                egui::Tooltip::always_open(
                                                                    ui.ctx().clone(),
                                                                    ui.layer_id(),
                                                                    egui::Id::new("drag_compat_tip"),
                                                                    egui::PopupAnchor::Pointer,
                                                                )
                                                                .show(|ui| {
                                                                    ui.label(tooltip_text);
                                                                });
                                                            }
                                                        } else if is_primary && !is_locked {
                                                            // Subtle beacon highlight on primary track
                                                            painter.rect_stroke(
                                                                track_rect,
                                                                0.0,
                                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(0, 255, 136, 120)),
                                                                egui::StrokeKind::Inside,
                                                            );
                                                        }
                                                    }

                                                    // Video / Audio tracks (0 and 1) or past relays: NotAllowed
                                                    if hovered_track_index == 0 || hovered_track_index == 1 {
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                        let row_y = tracks_top + (hovered_track_index as f32 * 32.0);
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(rect.max.x, row_y + 32.0),
                                                        );
                                                        painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 45));
                                                        painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 70, 70)), egui::StrokeKind::Inside);
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
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(52, 152, 219)),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                        
                                        ((rect, response), clicked_any_clip, clicked_any_keyframe)
                                    })
                            });
                            
                            let ((rect, response), clicked_any_clip, clicked_any_keyframe) = drop_res.0.inner.inner;
                            
                            let tracks_top = rect.min.y + 26.0;

                            // Successful drop logic
                            if let Some(payload) = &drop_res.1 {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if rect.contains(mouse_pos) {
                                        let relative_y = mouse_pos.y - tracks_top;
                                        let track_index = (relative_y / 32.0).floor() as i32;
                                        let relative_x = mouse_pos.x - rect.min.x;
                                        let drop_time_secs = (relative_x / zoom) as f64;

                                        self.app.handle_effect_drop(payload, track_index, drop_time_secs);
                                    }
                                }
                            }
                            
                            // Background click, seek, or lasso selection logic
                            let ruler_bottom = tracks_top;

                            if response.drag_started() && !clicked_any_clip && !clicked_any_keyframe && self.app.active_drag.is_none() && self.app.active_keyframe_drag.is_none() {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if mouse_pos.y >= ruler_bottom {
                                        self.app.lasso_origin = Some(mouse_pos);
                                    }
                                }
                            }
                            
                            if response.dragged() && self.app.lasso_origin.is_some() {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    let origin = self.app.lasso_origin.unwrap();
                                    let lasso_rect = egui::Rect::from_two_pos(origin, mouse_pos);
                                    self.app.lasso_rect = Some(lasso_rect);
                                    
                                    let shift_held = ui.ctx().input(|i| i.modifiers.shift);
                                    let mut new_instance_selection = if shift_held { self.app.selected_instance_ids.clone() } else { std::collections::HashSet::new() };
                                    let mut new_keyframe_selection = if shift_held { self.app.selected_keyframes.clone() } else { std::collections::HashSet::new() };
                                    
                                    // Instances
                                    for instance in &self.app.timeline.instances {
                                        if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                            let relay_id = effect.actions.first().map(|a| a.relay_id).unwrap_or(1);
                                            let track_index = relay_id as f32 + 1.0;
                                            let track_y = tracks_top + (track_index * 32.0);
                                            let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                            let end_x = start_x + (effect.duration_ms as f32 * px_per_ms);
                                            let clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(start_x, track_y + 4.0),
                                                egui::pos2(end_x, track_y + 28.0),
                                            );
                                            if lasso_rect.intersects(clip_rect) {
                                                new_instance_selection.insert(instance.id);
                                            }
                                        }
                                    }
                                    
                                    // Keyframes
                                    for (t_idx, track) in self.app.timeline.analog_tracks.iter().enumerate() {
                                        let t_y = tracks_top + 320.0 + (t_idx as f32 * 40.0);
                                        for (k_idx, kf) in track.keyframes.iter().enumerate() {
                                            let k_x = rect.min.x + (kf.time_ms as f32 * px_per_ms);
                                            let k_y = (t_y + 36.0) - (kf.value * 32.0);
                                            let k_pos = egui::pos2(k_x, k_y);
                                            if lasso_rect.contains(k_pos) {
                                                new_keyframe_selection.insert((track.id, k_idx));
                                            }
                                        }
                                    }
                                    
                                    self.app.selected_instance_ids = new_instance_selection;
                                    self.app.selected_keyframes = new_keyframe_selection;
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
                            
                            if response.clicked() && !clicked_any_clip && !clicked_any_keyframe && self.app.active_drag.is_none() && self.app.active_keyframe_drag.is_none() {
                                if let Some(mouse_pos) = response.interact_pointer_pos() {
                                    if mouse_pos.y >= tracks_top && mouse_pos.y < tracks_top + 320.0 {
                                        self.app.selected_instance_ids.clear();
                                        let relative_x = mouse_pos.x - rect.min.x;
                                        let seek_time = (relative_x / zoom) as f64;
                                        let target_time = seek_time.clamp(0.0, total_seconds);
                                        // Punch out on seek
                                        self.app.commit_recorded_samples();
                                        
                                        let _ = self.app.mpv.command("seek", &[&target_time.to_string(), "absolute"]);
                                    }
                                }
                            }

                            if !ui.ctx().egui_wants_keyboard_input() {
                                let delete_pressed = ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
                                let undo_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift);
                                let redo_pressed = ui.input(|i| (i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)));
                                let select_all_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A));
                                let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                
                                let num_modifier_free = ui.input(|i| !i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.command && !i.modifiers.shift);
                                let key_1_pressed = ui.input(|i| i.key_pressed(egui::Key::Num1)) && num_modifier_free;
                                let key_2_pressed = ui.input(|i| i.key_pressed(egui::Key::Num2)) && num_modifier_free;
                                let key_3_pressed = ui.input(|i| i.key_pressed(egui::Key::Num3)) && num_modifier_free;

                                if undo_pressed {
                                    let current = self.app.snapshot_timeline();
                                    if let Some(prev) = self.app.undo_stack.undo(current) {
                                        self.app.restore_timeline_snapshot(prev);
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                } else if redo_pressed {
                                    let current = self.app.snapshot_timeline();
                                    if let Some(next) = self.app.undo_stack.redo(current) {
                                        self.app.restore_timeline_snapshot(next);
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                } else if select_all_pressed {
                                    self.app.selected_instance_ids = self.app.timeline.instances.iter().map(|i| i.id).collect();
                                    self.app.selected_keyframes.clear();
                                    for track in &self.app.timeline.analog_tracks {
                                        for (k_idx, _) in track.keyframes.iter().enumerate() {
                                            self.app.selected_keyframes.insert((track.id, k_idx));
                                        }
                                    }
                                } else if escape_pressed {
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_keyframes.clear();
                                } else if delete_pressed {
                                    if !self.app.selected_instance_ids.is_empty() || !self.app.selected_keyframes.is_empty() {
                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                        
                                        self.app.timeline.instances.retain(|inst| !self.app.selected_instance_ids.contains(&inst.id));
                                        
                                        for track in &mut self.app.timeline.analog_tracks {
                                            let mut k_indices: Vec<usize> = self.app.selected_keyframes.iter()
                                                .filter(|(t_id, _)| *t_id == track.id)
                                                .map(|(_, k_idx)| *k_idx)
                                                .collect();
                                            k_indices.sort_unstable();
                                            for idx in k_indices.into_iter().rev() {
                                                if idx < track.keyframes.len() {
                                                    track.keyframes.remove(idx);
                                                }
                                            }
                                        }
                                        
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        
                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                } else if key_1_pressed || key_2_pressed || key_3_pressed {
                                    if !self.app.selected_keyframes.is_empty() {
                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                        let interp = if key_1_pressed {
                                            crate::four_d::curve::Interpolation::Step
                                        } else if key_2_pressed {
                                            crate::four_d::curve::Interpolation::Linear
                                        } else {
                                            crate::four_d::curve::Interpolation::Smooth
                                        };
                                        for track in &mut self.app.timeline.analog_tracks {
                                            for (k_idx, kf) in track.keyframes.iter_mut().enumerate() {
                                                if self.app.selected_keyframes.contains(&(track.id, k_idx)) {
                                                    kf.interpolation = interp.clone();
                                                }
                                            }
                                        }
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
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
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 100, 100)),
    );
    
    // Emissive filled circle
    let fill_color = if active {
        egui::Color32::from_rgb(0, 255, 136) // Neon Green
    } else {
        egui::Color32::from_rgb(50, 50, 50) // Dark Grey
    };
    painter.circle_filled(center, inner_radius, fill_color);
}

impl PealayerApp {
    /// Locks or unlocks a track by relay ID (1..=8).
    pub fn lock_track(&mut self, relay_id: u8, locked: bool) {
        if (relay_id as usize) < self.track_locked.len() {
            self.track_locked[relay_id as usize] = locked;
        }
    }

    /// Returns the latest OSD message text, if any.
    pub fn last_osd_message(&self) -> Option<String> {
        self.osd_message.as_ref().map(|(msg, _)| msg.clone())
    }

    /// Handles dropping an effect payload onto the timeline canvas.
    ///
    /// - Case A: Dropped on a Relay Track (track_index 2..=9):
    ///   Validates track lock and target compatibility with `payload.target`.
    ///   Rejects drop with warning message and OSD update if incompatible or locked.
    /// - Case B: Dropped on Empty Grid Space or Video/Audio Header (track_index < 2 || track_index > 9):
    ///   Smart auto-routes directly to the primary hardware track (`payload.target.primary_relay_id()`)
    ///   if defined and unlocked.
    ///
    /// Returns true if a clip instance was successfully created.
    pub fn handle_effect_drop(
        &mut self,
        payload: &EffectDragPayload,
        track_index: i32,
        drop_time_secs: f64,
    ) -> bool {
        let target_relay = if track_index >= 2 && track_index <= 9 {
            let relay = (track_index - 1) as u8;
            if self.track_locked[relay as usize] {
                println!("[Timeline] Drop blocked: track R{} is locked", relay);
                self.set_osd(format!("Cannot place '{}': track R{} is locked", payload.name, relay));
                return false;
            }
            if !payload.target.is_compatible_with_relay(relay) {
                println!("[Timeline] Rejected incompatible drop: '{}' on R{}", payload.name, relay);
                self.set_osd(format!("Placement Rejected: '{}' cannot be placed on R{}", payload.name, relay));
                return false;
            }
            relay
        } else if track_index == 0 || track_index == 1 {
            // Explicitly reject drops onto video/audio tracks to match visual NotAllowed affordance
            self.set_osd(format!("Cannot place effect '{}' on media track", payload.name));
            return false;
        } else {
            // Case B: Dropped on empty grid space or neutral ruler -> Smart Auto-Routing
            if let Some(primary) = payload.target.primary_relay_id() {
                if self.track_locked[primary as usize] {
                    println!("[Timeline] Auto-routing for '{}' blocked: primary track R{} is locked", payload.name, primary);
                    self.set_osd(format!("Cannot place '{}': primary track R{} is locked", payload.name, primary));
                    return false;
                }
                println!("[Timeline] Auto-routed drop of '{}' to primary track R{}", payload.name, primary);
                self.set_osd(format!("Auto-routed '{}' to R{}: {}", payload.name, primary, payload.target.display_name()));
                primary
            } else {
                return false;
            }
        };

        let mut snapped_secs = drop_time_secs;
        // Playhead/grid snapping
        if (snapped_secs - self.playback_time).abs() < 0.15 {
            snapped_secs = self.playback_time;
        } else {
            snapped_secs = (snapped_secs * 10.0).round() / 10.0;
        }
        let start_time_ms = (snapped_secs.max(0.0) * 1000.0) as u64;

        // Record undo snapshot before mutating timeline
        self.undo_stack.push(self.snapshot_timeline());

        // Check or create template targeting target_relay with payload.target
        let template_id = if let Some(existing) = self.timeline.templates.iter().find(|t| {
            t.name == payload.name
                && t.duration_ms == payload.duration_ms
                && t.target == payload.target
                && t.actions.first().map(|a| a.relay_id).unwrap_or(1) == target_relay
        }) {
            existing.id
        } else {
            let actions = if !payload.actions.is_empty() {
                let mut acts = payload.actions.clone();
                for a in &mut acts {
                    a.relay_id = target_relay;
                }
                acts
            } else {
                crate::four_d::patterns::generate_constant(target_relay, true, payload.duration_ms)
            };
            let new_effect = crate::four_d::models::Effect::with_target(
                payload.name.clone(),
                payload.icon.clone(),
                payload.duration_ms,
                payload.target,
                actions,
            );
            let id = new_effect.id;
            self.timeline.templates.push(new_effect);
            id
        };

        // Instantiate and select
        let new_instance = crate::four_d::models::EffectInstance::new(template_id, start_time_ms);
        let new_instance_id = new_instance.id;
        self.timeline.instances.push(new_instance);
        self.selected_instance_ids.clear();
        self.selected_instance_ids.insert(new_instance_id);
        self.selected_keyframes.clear();

        // Recompile timeline
        let compiled = crate::four_d::engine::compile_timeline(
            &self.timeline,
            &self.track_muted,
            &self.track_soloed,
        );
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateQueue(compiled),
        );

        true
    }

    /// 1-Click Relocation: Automatically reassigns an effect's template actions to its primary relay track,
    /// pushes an undo snapshot to the undo stack, recompiles the timeline, and updates the engine queue.
    /// Preserves internal pattern sequences (e.g. pulsing) while updating the target relay.
    /// Returns true if relocation was successfully performed.
    pub fn relocate_effect_to_primary(&mut self, effect_id: uuid::Uuid) -> bool {
        let (primary, duration_ms, display_name, effect_name) = if let Some(tmpl) = self.timeline.templates.iter().find(|t| t.id == effect_id) {
            if let Some(primary) = tmpl.target.primary_relay_id() {
                (primary, tmpl.duration_ms, tmpl.target.display_name(), tmpl.name.clone())
            } else {
                return false;
            }
        } else {
            return false;
        };

        // Record undo snapshot before mutating timeline
        self.undo_stack.push(self.snapshot_timeline());

        if let Some(t) = self.timeline.templates.iter_mut().find(|t| t.id == effect_id) {
            if t.actions.is_empty() {
                t.actions = crate::four_d::patterns::generate_constant(primary, true, duration_ms);
            } else {
                for a in &mut t.actions {
                    a.relay_id = primary;
                }
            }
        }

        let compiled = crate::four_d::engine::compile_timeline(
            &self.timeline,
            &self.track_muted,
            &self.track_soloed,
        );
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateQueue(compiled),
        );

        self.set_osd(format!("Relocated '{}' to R{}: {}", effect_name, primary, display_name));
        true
    }
}

fn render_clip_handles(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    left_active: bool,
    right_active: bool,
    alpha: u8,
) {
    if clip_rect.width() < 14.0 {
        return;
    }
    let handle_w = (clip_rect.width() * 0.35).min(10.0);
    let left_handle_rect = egui::Rect::from_min_max(
        clip_rect.left_top(),
        egui::pos2(clip_rect.left() + handle_w, clip_rect.bottom()),
    );
    let right_handle_rect = egui::Rect::from_min_max(
        egui::pos2(clip_rect.right() - handle_w, clip_rect.top()),
        clip_rect.right_bottom(),
    );

    let alpha_scale = alpha as f32 / 255.0;
    let cyan = egui::Color32::from_rgb(0, 220, 255);

    // Left handle highlight & edge
    if left_active {
        painter.rect_filled(
            left_handle_rect,
            egui::CornerRadius { nw: 4, sw: 4, ne: 0, se: 0 },
            egui::Color32::from_rgba_unmultiplied(0, 220, 255, (50.0 * alpha_scale) as u8),
        );
        painter.line_segment(
            [
                clip_rect.left_top() + egui::vec2(1.0, 1.0),
                egui::pos2(clip_rect.left() + 1.0, clip_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(
                2.0_f32,
                egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha),
            ),
        );
    }

    // Right handle highlight & edge
    if right_active {
        painter.rect_filled(
            right_handle_rect,
            egui::CornerRadius { nw: 0, sw: 0, ne: 4, se: 4 },
            egui::Color32::from_rgba_unmultiplied(0, 220, 255, (50.0 * alpha_scale) as u8),
        );
        painter.line_segment(
            [
                egui::pos2(clip_rect.right() - 1.0, clip_rect.top() + 1.0),
                egui::pos2(clip_rect.right() - 1.0, clip_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(
                2.0_f32,
                egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha),
            ),
        );
    }

    // Draw grip affordance notches (2 subtle vertical lines in center of each handle)
    let notch_y_top = clip_rect.top() + 5.0;
    let notch_y_bot = clip_rect.bottom() - 5.0;
    if notch_y_bot > notch_y_top {
        // Left handle grip notches
        let left_cx = left_handle_rect.center().x;
        let left_notch_color = if left_active {
            egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (70.0 * alpha_scale) as u8)
        };
        painter.line_segment(
            [egui::pos2(left_cx - 1.5, notch_y_top), egui::pos2(left_cx - 1.5, notch_y_bot)],
            egui::Stroke::new(1.0_f32, left_notch_color),
        );
        painter.line_segment(
            [egui::pos2(left_cx + 1.5, notch_y_top), egui::pos2(left_cx + 1.5, notch_y_bot)],
            egui::Stroke::new(1.0_f32, left_notch_color),
        );

        // Right handle grip notches
        let right_cx = right_handle_rect.center().x;
        let right_notch_color = if right_active {
            egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (70.0 * alpha_scale) as u8)
        };
        painter.line_segment(
            [egui::pos2(right_cx - 1.5, notch_y_top), egui::pos2(right_cx - 1.5, notch_y_bot)],
            egui::Stroke::new(1.0_f32, right_notch_color),
        );
        painter.line_segment(
            [egui::pos2(right_cx + 1.5, notch_y_top), egui::pos2(right_cx + 1.5, notch_y_bot)],
            egui::Stroke::new(1.0_f32, right_notch_color),
        );
    }
}



