use eframe::egui;
use egui_dock::TabViewer;
use crate::app::PealayerApp;
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

                            ui.horizontal(|ui| {
                                let play_icon = if self.app.is_paused { "▶" } else { "⏸" };
                                if ui.button(play_icon).clicked() {
                                    let _ = self.app.mpv.command("cycle", &["pause"]);
                                }
                                if ui.button("⏹").clicked() {
                                    let _ = self.app.mpv.command("seek", &["0", "absolute"]);
                                    let _ = self.app.mpv.set_property("pause", true);
                                }
                                ui.separator();

                                // timecode HH:MM:SS:FF at 24fps
                                let tc = format_timecode(self.app.playback_time);
                                ui.monospace(tc);

                                // seekbar
                                let mut current_pos = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                let slider = egui::Slider::new(&mut current_pos, 0.0..=self.app.duration)
                                    .show_value(false)
                                    .trailing_fill(true);

                                let seekbar_w = (ui.available_width() - 80.0).max(50.0);
                                let old_w = ui.spacing().slider_width;
                                ui.spacing_mut().slider_width = seekbar_w;
                                let response = ui.add(slider);
                                ui.spacing_mut().slider_width = old_w;

                                if response.dragged() {
                                    self.app.seek_pos = Some(current_pos);
                                }
                                if response.drag_stopped() {
                                    let _ = self.app.mpv.command("seek", &[&current_pos.to_string(), "absolute"]);
                                    self.app.seek_pos = None;
                                }
                            });
                        });
                    }
                    PealayerTab::EffectControls => {
                        let selected_id = self.app.selected_instance_id;
                        
                        if let Some(id) = selected_id {
                            let mut timeline_dirty = false;
                            
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
                                }
                            }
                            
                            if timeline_dirty {
                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline);
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
                        ui.label("List of pre-made templates to add to the timeline.");
                    }
                    PealayerTab::HardwareMonitor => {
                        ui.heading("Hardware Monitor Dashboard");
                        ui.add_space(8.0);
                        
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
                                    let is_timeline_active = evaluate_relay_state(&self.app.timeline, *id, (self.app.playback_time * 1000.0) as u64);
                                    let is_forced = self.app.relay_overrides[*id as usize] == Some(true);
                                    let active = is_forced || (self.app.relay_overrides[*id as usize].is_none() && is_timeline_active);
                                    
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
                            
                        ui.ctx().request_repaint();
                    }
                    PealayerTab::Timeline => {
                        ui.horizontal(|ui| {
                            // 1. Left column: Fixed Track Headers
                            ui.vertical(|ui| {
                                ui.set_width(100.0);
                                
                                let track_names = [
                                    "Video",
                                    "Audio",
                                    "Relay 1",
                                    "Relay 2",
                                    "Relay 3",
                                    "Relay 4",
                                    "Relay 5",
                                    "Relay 6",
                                    "Relay 7",
                                    "Relay 8",
                                ];
                                
                                for name in &track_names {
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(100.0, 32.0), egui::Sense::hover());
                                    // Draw background with dark Premiere aesthetics
                                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(26, 26, 26));
                                    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)), egui::StrokeKind::Inside);
                                    
                                    ui.painter().text(
                                        rect.left_center() + egui::vec2(8.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        name,
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_rgb(200, 200, 200),
                                    );
                                }
                            });
                            
                            // 2. Right column: Scrollable Timeline Grid
                            let total_seconds = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                            let total_width = (total_seconds * 100.0) as f32;
                            
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
                                    
                                    // Draw horizontal track separators
                                    for i in 0..=10 {
                                        let grid_y = rect.min.y + (i as f32 * 32.0);
                                        painter.line_segment(
                                            [egui::pos2(rect.min.x, grid_y), egui::pos2(rect.max.x, grid_y)],
                                            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)),
                                        );
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
                                    
                                    // Draw timeline instances from app.timeline
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
                                            
                                            let is_selected = self.app.selected_instance_id == Some(instance.id);
                                            let stroke_color = if is_selected {
                                                egui::Color32::from_rgb(255, 235, 59) // Selection Yellow outline
                                            } else {
                                                egui::Color32::WHITE
                                            };
                                            let stroke_width = if is_selected { 2.0 } else { 1.0 };
                                            
                                            // Draw clip box
                                            painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgb(142, 68, 173)); // Purple clip
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
                                    
                                    // Click/Drag logic to seek video OR select clips
                                    if response.clicked() || response.dragged() {
                                        if let Some(mouse_pos) = response.interact_pointer_pos() {
                                            let mut clicked_clip = false;
                                            if response.clicked() {
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
                                                        
                                                        if clip_rect.contains(mouse_pos) {
                                                            self.app.selected_instance_id = Some(instance.id);
                                                            clicked_clip = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            if !clicked_clip && !response.dragged() && response.clicked() {
                                                self.app.selected_instance_id = None;
                                            }
                                            
                                            if !clicked_clip {
                                                let relative_x = mouse_pos.x - rect.min.x;
                                                let seek_time = (relative_x / 100.0) as f64;
                                                let target_time = seek_time.clamp(0.0, total_seconds);
                                                let _ = self.app.mpv.command("seek", &[&target_time.to_string(), "absolute"]);
                                            }
                                        }
                                    }
                                });
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

