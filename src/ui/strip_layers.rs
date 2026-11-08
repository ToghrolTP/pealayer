//! Segmented strip authoring. Every interaction changes a draft, never hardware.
use super::{effects_library::designer_tr as tr, icons};
use crate::{
    app::ControllerEffectDraft,
    four_d::strip_program::{Color, Layer, Program, Segment},
};
use eframe::egui;

fn next_id(prefix: &str, ids: impl Iterator<Item = String>) -> String {
    let ids: Vec<_> = ids.collect();
    (1..)
        .map(|i| format!("{prefix}-{i}"))
        .find(|id| !ids.contains(id))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn strip_layer_paste_retargets_segment_and_rejects_invalid_values_atomically(){
        let mut p=Program::new(20);let mut layer=p.layers[0].clone();layer.name="Breathing".into();
        let text=serde_json::json!({"format":"pealayer.strip-layer","layer":layer}).to_string();
        paste_layer(&mut p,&text,"all",20).unwrap();assert_eq!(p.layers.len(),2);assert_ne!(p.layers[0].id,p.layers[1].id);
        let before=p.clone();assert!(paste_layer(&mut p,&text,"missing",20).is_err());assert_eq!(p,before);
        assert!(paste_layer(&mut p,"{}","all",20).is_err());assert_eq!(p,before);
    }
    #[test]
    fn strip_editor_fits_phone_tablet_and_desktop_without_mutating_draft() {
        for width in [340., 768., 1200.] {
            let context = egui::Context::default();
            let mut draft = ControllerEffectDraft::default();
            draft.default_pixels = 100;
            draft.program_json = serde_json::to_string_pretty(&Program::new(100)).unwrap();
            let original = draft.clone();
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 1200.),
                    )),
                    ..Default::default()
                },
                |ui| {
                    draw(ui, &mut draft);
                    assert!(
                        ui.min_rect().width() <= width + 1.,
                        "strip controls overflow {width}: {:?}",
                        ui.min_rect()
                    );
                },
            );
            output.textures_delta.clear();
            assert_eq!(draft, original);
        }
    }
}
fn color(ui: &mut egui::Ui, value: &mut Color) -> bool {
    let mut rgb = [value.red, value.green, value.blue];
    let changed = super::color_picker::color_button_srgb(ui, &mut rgb).changed();
    if changed {
        *value = Color {
            red: rgb[0],
            green: rgb[1],
            blue: rgb[2],
        };
    }
    changed
}
pub(crate) fn draw(ui: &mut egui::Ui, draft: &mut ControllerEffectDraft) {
    let Ok(mut p) = serde_json::from_str::<Program>(&draft.program_json) else {
        ui.colored_label(
            ui.visuals().error_fg_color,
            tr(
                ui,
                "Invalid segmented program; correct the Technical fields",
            ),
        );
        return;
    };
    // Malformed but parseable technical JSON stays intact, rather than indexing
    // empty arrays or silently replacing it with defaults.
    if p.segments.is_empty() || p.layers.is_empty() {
        ui.colored_label(
            ui.visuals().error_fg_color,
            tr(ui, "Use 1–64 segments and layers"),
        );
        return;
    }
    let original = p.clone();
    let count = draft.default_pixels.clamp(1, 100) as usize;
    let root = ui.scope_id().with(("strip-layers", &draft.reference, &draft.id));
    let mut selected = ui
        .data(|d| d.get_temp::<usize>(root.with("segment")))
        .unwrap_or(0)
        .min(p.segments.len() - 1);
    let mut layer = ui
        .data(|d| d.get_temp::<usize>(root.with("layer")))
        .unwrap_or(0)
        .min(p.layers.len() - 1);
    let mut clipboard_action=None;
    ui.horizontal_wrapped(|ui|{
        for (action,icon,label) in [(0,icons::COPY,"Copy layer"),(1,egui_phosphor::regular::SCISSORS,"Cut layer"),(2,egui_phosphor::regular::CLIPBOARD,"Paste layer")] {
            if ui.add_enabled(action!=1 || p.layers.len()>1,egui::Button::new(format!("{icon} {}",tr(ui,label)))).clicked(){clipboard_action=Some(action);}
        }
    });
    if !ui.ctx().text_edit_focused() && !egui::Popup::is_any_open(ui.ctx()) && ui.rect_contains_pointer(ui.max_rect()) {
        let keyboard=ui.input(|i|{
            i.events.iter().find_map(|e|match e {egui::Event::Copy=>Some(0),egui::Event::Cut=>Some(1),egui::Event::Paste(_)=>Some(2),_=>None})
        });
        if keyboard.is_some(){clipboard_action=keyboard;ui.input_mut(|i|i.events.retain(|e|!matches!(e,egui::Event::Copy|egui::Event::Cut|egui::Event::Paste(_))));}
    }
    if let Some(action)=clipboard_action {
        let result=(||->Result<(),String>{
            if action==2 {
                let text=crate::platform::clipboard::text()?;
                paste_layer(&mut p,&text,&original.segments[selected].id,count)?;
                layer=p.layers.len()-1;
            } else {
                if action==1 && p.layers.len()==1{return Err("Keep at least one strip layer".into());}
                let text=serde_json::to_string(&serde_json::json!({"format":"pealayer.strip-layer","layer":p.layers[layer]})).map_err(|e|e.to_string())?;
                crate::platform::clipboard::write(&text)?;
                if action==1 {p.layers.remove(layer);layer=layer.min(p.layers.len()-1);}
            }Ok(())
        })();
        if let Err(error)=result{ui.colored_label(ui.visuals().error_fg_color,error);}
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(ui, "Density"));
        ui.add(
            egui::DragValue::new(&mut p.pixels_per_meter)
                .range(1..=1000)
                .suffix(" px/m"),
        );
        if p.pixels_per_meter > 0 {
            ui.weak(format!(
                "{:.2} m",
                count as f64 / f64::from(p.pixels_per_meter)
            ));
        }
    });
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.strong(format!("{} {}", icons::GRID_FOUR, tr(ui, "Segments")));
        super::dropdown::ComboBox::from_id_salt(root.with("segments"))
            .selected_text(&p.segments[selected].name)
            .show_ui(ui, |ui| {
                for (i, s) in p.segments.iter().enumerate() {
                    ui.selectable_value(&mut selected, i, &s.name);
                }
            });
        if ui
            .add_enabled(p.segments.len() < 64, egui::Button::new(icons::PLUS))
            .on_hover_text(tr(ui, "Add segment"))
            .clicked()
        {
            let id = next_id("segment", p.segments.iter().map(|s| s.id.clone()));
            p.segments.push(Segment {
                id,
                name: tr(ui, "Segment"),
                pixels: vec![0],
            });
            selected = p.segments.len() - 1;
        }
        let removable = p.segments.len() > 1
            && !p
                .layers
                .iter()
                .any(|l| l.segment_id == p.segments[selected].id);
        if ui
            .add_enabled(removable, egui::Button::new(icons::TRASH))
            .on_hover_text(tr(ui, "Remove layers using this segment first"))
            .clicked()
        {
            p.segments.remove(selected);
            selected = selected.min(p.segments.len() - 1);
        }
    });
    ui.add(
        super::dialog::singleline_text_edit(ui, &mut p.segments[selected].name)
            .desired_width(ui.available_width()),
    );
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr(ui, "All pixels")).clicked() {
            p.segments[selected].pixels = (0..count).collect();
        }
        if ui.button(tr(ui, "Clear selection")).clicked() {
            p.segments[selected].pixels.clear();
        }
        if ui.button(tr(ui, "Invert selection")).clicked() {
            p.segments[selected].pixels = (0..count)
                .filter(|i| !p.segments[selected].pixels.contains(i))
                .collect();
        }
        if ui.button(tr(ui, "Reverse order")).clicked() {
            p.segments[selected].pixels.reverse();
        }
        ui.weak(format!("{} / {}", p.segments[selected].pixels.len(), count));
    });
    let now = ui.input(|i| i.time);
    let mut playing = ui
        .data(|d| d.get_temp::<bool>(root.with("play")))
        .unwrap_or(false);
    let mut phase = ui
        .data(|d| d.get_temp::<u64>(root.with("phase")))
        .unwrap_or(0);
    let mut start = ui
        .data(|d| d.get_temp::<f64>(root.with("start")))
        .unwrap_or(now);
    let period = p
        .layers
        .iter()
        .map(|l| l.period_ms)
        .max()
        .unwrap_or(3000)
        .max(50);
    if playing {
        phase = ((now - start).max(0.) * 1000.) as u64 % period;
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
    }
    let colors = p.frame(count, phase);
    ui.horizontal_wrapped(|ui| {
        for index in 0..count {
            let c = colors.get(index).cloned().unwrap_or_default();
            let rgb = egui::Color32::from_rgb(c.red, c.green, c.blue);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(27., 27.), egui::Sense::click_and_drag());
            let chosen = p.segments[selected].pixels.contains(&index);
            ui.painter().rect_filled(rect.shrink(2.), 4., rgb);
            ui.painter().rect_stroke(
                rect.shrink(1.),
                4.,
                egui::Stroke::new(
                    if chosen { 2. } else { 1. },
                    if chosen {
                        ui.visuals().selection.stroke.color
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke.color
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let text =
                if u32::from(c.red) * 299 + u32::from(c.green) * 587 + u32::from(c.blue) * 114
                    > 128000
                {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::WHITE
                };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                (index + 1).to_string(),
                egui::FontId::proportional(10.),
                text,
            );
            if response.clicked() {
                if chosen {
                    p.segments[selected].pixels.retain(|v| *v != index);
                } else {
                    p.segments[selected].pixels.push(index);
                }
            }
            response.on_hover_text(format!(
                "{} · #{:02X}{:02X}{:02X}",
                index + 1,
                c.red,
                c.green,
                c.blue
            ));
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(format!(
                "{} {}",
                if playing { icons::PAUSE } else { icons::PLAY },
                tr(ui, "Offline preview")
            ))
            .clicked()
        {
            playing = !playing;
            start = now - phase as f64 / 1000.;
        }
        if ui
            .add(egui::Slider::new(&mut phase, 0..=period - 1).suffix(" ms"))
            .changed()
        {
            playing = false;
        }
    });
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.strong(format!("{} {}", icons::TABS, tr(ui, "Layers")));
        super::dropdown::ComboBox::from_id_salt(root.with("layers"))
            .selected_text(&p.layers[layer].name)
            .show_ui(ui, |ui| {
                for (i, l) in p.layers.iter().enumerate() {
                    ui.selectable_value(&mut layer, i, &l.name);
                }
            });
        if ui
            .add_enabled(p.layers.len() < 64, egui::Button::new(icons::PLUS))
            .on_hover_text(tr(ui, "Add layer"))
            .clicked()
        {
            let id = next_id("layer", p.layers.iter().map(|l| l.id.clone()));
            p.layers
                .push(Layer::new(id, p.segments[selected].id.clone()));
            layer = p.layers.len() - 1;
        }
        if ui
            .add_enabled(p.layers.len() < 64, egui::Button::new(icons::COPY))
            .on_hover_text(tr(ui, "Duplicate"))
            .clicked()
        {
            let mut next = p.layers[layer].clone();
            next.id = next_id("layer", p.layers.iter().map(|l| l.id.clone()));
            p.layers.insert(layer + 1, next);
            layer += 1;
        }
        if ui
            .add_enabled(layer > 0, egui::Button::new(icons::ARROW_UP))
            .clicked()
        {
            p.layers.swap(layer, layer - 1);
            layer -= 1;
        }
        if ui
            .add_enabled(
                layer + 1 < p.layers.len(),
                egui::Button::new(icons::ARROW_DOWN),
            )
            .clicked()
        {
            p.layers.swap(layer, layer + 1);
            layer += 1;
        }
        if ui
            .add_enabled(p.layers.len() > 1, egui::Button::new(icons::TRASH))
            .clicked()
        {
            p.layers.remove(layer);
            layer = layer.min(p.layers.len() - 1);
        }
    });
    let l = &mut p.layers[layer];
    ui.horizontal(|ui| {
        ui.checkbox(&mut l.enabled, tr(ui, "Enabled"));
        ui.add(
            super::dialog::singleline_text_edit(ui, &mut l.name)
                .desired_width(ui.available_width()),
        );
    });
    egui::Grid::new(root.with("fields"))
        .num_columns(2)
        .spacing([14., 9.])
        .show(ui, |ui| {
            ui.label(tr(ui, "Segment"));
            super::dropdown::ComboBox::from_id_salt("layer-segment")
                .selected_text(
                    p.segments
                        .iter()
                        .find(|s| s.id == l.segment_id)
                        .map(|s| s.name.as_str())
                        .unwrap_or("—"),
                )
                .show_ui(ui, |ui| {
                    for s in &p.segments {
                        ui.selectable_value(&mut l.segment_id, s.id.clone(), &s.name);
                    }
                });
            ui.end_row();
            ui.label(tr(ui, "Pattern"));
            super::dropdown::ComboBox::from_id_salt("layer-pattern")
                .selected_text(&l.primitive)
                .show_ui(ui, |ui| {
                    for (id, label) in [
                        ("solid", "Solid"),
                        ("gradient", "Gradient"),
                        ("ramp", "Fade"),
                        ("blink", "Blink / Strobe"),
                        ("breathe", "Breathing"),
                    ] {
                        ui.selectable_value(&mut l.primitive, id.into(), tr(ui, label));
                    }
                });
            ui.end_row();
            ui.label(tr(ui, "Color"));
            color(ui, &mut l.primary);
            ui.end_row();
            if l.primitive != "solid" {
                ui.label(tr(ui, "Secondary color"));
                color(ui, &mut l.secondary);
                ui.end_row();
            }
            if !matches!(l.primitive.as_str(), "solid" | "gradient") {
                ui.label(tr(ui, "Cycle"));
                ui.add(crate::duration::time_value_drag(
                    &mut l.period_ms,
                    50..=3_600_000,
                    10.,
                    true,
                ));
                ui.end_row();
                ui.label(tr(ui, "Phase offset"));
                ui.add(crate::duration::time_value_drag(
                    &mut l.phase_ms,
                    0..=l.period_ms.saturating_sub(1),
                    10.,
                    true,
                ));
                ui.end_row();
            }
            if l.primitive == "blink" {
                ui.label(tr(ui, "Duty cycle"));
                ui.add(egui::Slider::new(&mut l.duty_percent, 0..=100).suffix("%"));
                ui.end_row();
            }
            if l.primitive != "solid" {
                ui.label(tr(ui, "Easing"));
                super::dropdown::ComboBox::from_id_salt("layer-easing")
                    .selected_text(&l.easing)
                    .show_ui(ui, |ui| {
                        for id in ["linear", "ease-in", "ease-out", "ease-in-out"] {
                            ui.selectable_value(&mut l.easing, id.into(), id);
                        }
                    });
                ui.end_row();
            }
            ui.label(tr(ui, "Reverse"));
            ui.checkbox(&mut l.reverse, "");
            ui.end_row();
        });
    if let Err(error) = p.validate(count) {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    if original != p {
        draft.program_json = serde_json::to_string_pretty(&p).unwrap();
    }
    ui.data_mut(|d| {
        d.insert_temp(root.with("segment"), selected);
        d.insert_temp(root.with("layer"), layer);
        d.insert_temp(root.with("play"), playing);
        d.insert_temp(root.with("phase"), phase);
        d.insert_temp(root.with("start"), start);
    });
}

fn paste_layer(program:&mut Program,text:&str,segment:&str,count:usize)->Result<(),String>{
    #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)]
    struct Clipboard {format:String,layer:Layer}
    if text.len()>crate::platform::clipboard::MAX_BYTES{return Err("Clipboard selection is too large".into());}
    let mut copy:Clipboard=serde_json::from_str(text).map_err(|_|"Copy a strip layer first")?;
    if copy.format!="pealayer.strip-layer" || program.layers.len()>=64{return Err("Paste requires a strip layer and fewer than 64 layers".into());}
    copy.layer.id=next_id("layer",program.layers.iter().map(|l|l.id.clone()));copy.layer.segment_id=segment.into();
    let mut next=program.clone();next.layers.push(copy.layer);next.validate(count)?;*program=next;Ok(())
}
