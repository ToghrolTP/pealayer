//! Typed authoring and an offline strip preview; drawing never publishes hardware.
use super::effects_library::designer_tr as tr;
use crate::app::ControllerEffectDraft;
use eframe::egui;
use serde_json::{Value, json};

pub(crate) fn valid(program: &Value) -> bool {
    if !program["dim_intensity"].is_null()
        && !program["dim_intensity"].as_u64().is_some_and(|v| v <= 255)
    {
        return false;
    }
    let Some(period) = program["period_ms"]
        .as_u64()
        .filter(|v| (50..=3_600_000).contains(v))
    else {
        return false;
    };
    for color in ["primary", "secondary"] {
        if !program[color].is_null()
            && ["red", "green", "blue"]
                .iter()
                .any(|key| !program[color][key].as_u64().is_some_and(|v| v <= 255))
        {
            return false;
        }
    }
    match program["primitive"].as_str() {
        Some("alternating-zones") => {
            program["step_ms"]
                .as_u64()
                .is_some_and(|v| v >= 10 && v <= period)
                && program["swap_after_steps"]
                    .as_u64()
                    .is_some_and(|v| (1..=1024).contains(&v))
        }
        Some("converging-points") => {
            program["tail_pixels"].is_null()
                || program["tail_pixels"].as_u64().is_some_and(|v| v <= 100)
        }
        Some("envelope") => {
            let Some(points) = program["envelope"]
                .as_array()
                .filter(|p| (2..=256).contains(&p.len()))
            else {
                return false;
            };
            let mut previous = None;
            points.iter().all(|p| {
                let Some(at) = p["at_ms"].as_u64() else {
                    return false;
                };
                let ok = at < period
                    && previous.is_none_or(|v| at > v)
                    && p["intensity"].as_u64().is_some_and(|v| v <= 255);
                previous = Some(at);
                ok
            })
        }
        _ => false,
    }
}

fn preset(index: usize) -> Value {
    let red = json!({"red":255,"green":0,"blue":0});
    match index {
        0 => {
            json!({"primitive":"alternating-zones","primary":red,"secondary":{"red":0,"green":0,"blue":255},"period_ms":800,"step_ms":100,"swap_after_steps":4,"dim_intensity":36})
        }
        1 => {
            json!({"primitive":"envelope","primary":{"red":255,"green":255,"blue":255},"period_ms":1600,"envelope":[{"at_ms":0,"intensity":255},{"at_ms":45,"intensity":24},{"at_ms":90,"intensity":220},{"at_ms":145,"intensity":52},{"at_ms":220,"intensity":150},{"at_ms":360,"intensity":0}]})
        }
        2 => {
            json!({"primitive":"converging-points","primary":red,"period_ms":2000,"tail_pixels":3})
        }
        _ => {
            json!({"primitive":"envelope","primary":{"red":0,"green":96,"blue":255},"period_ms":3000,"envelope":[{"at_ms":0,"intensity":0},{"at_ms":if index==3 {1400}else{500},"intensity":255},{"at_ms":if index==3 {2800}else{1000},"intensity":0}]})
        }
    }
}

fn color(program: &Value, name: &str, intensity: u64) -> egui::Color32 {
    let channel = |key: &str| {
        (program[name][key].as_u64().unwrap_or(0).min(255) * intensity.min(255) / 255) as u8
    };
    egui::Color32::from_rgb(channel("red"), channel("green"), channel("blue"))
}

pub(crate) fn preview(program: &Value, count: usize, at: u64) -> Vec<egui::Color32> {
    if !valid(program) || !(1..=100).contains(&count) {
        return Vec::new();
    }
    let period = program["period_ms"].as_u64().unwrap();
    let time = at % period;
    (0..count)
        .map(|pixel| match program["primitive"].as_str().unwrap() {
            "alternating-zones" => {
                let step = program["step_ms"].as_u64().unwrap();
                let phase = at / step % (period / step).max(2);
                let primary = (pixel < count.div_ceil(2))
                    == (phase < program["swap_after_steps"].as_u64().unwrap());
                color(
                    program,
                    if primary { "primary" } else { "secondary" },
                    if phase % 2 == 0 {
                        255
                    } else {
                        program["dim_intensity"].as_u64().unwrap_or(0)
                    },
                )
            }
            "envelope" => {
                let points = program["envelope"].as_array().unwrap();
                let mut intensity = 0;
                for (i, point) in points.iter().enumerate() {
                    let end = point["at_ms"].as_u64().unwrap();
                    if time < end {
                        if i > 0 {
                            let previous = &points[i - 1];
                            let start = previous["at_ms"].as_u64().unwrap();
                            let a = previous["intensity"].as_i64().unwrap();
                            let b = point["intensity"].as_i64().unwrap();
                            intensity =
                                (a + (b - a) * (time - start) as i64 / (end - start) as i64) as u64;
                        }
                        break;
                    }
                    intensity = point["intensity"].as_u64().unwrap();
                }
                color(program, "primary", intensity)
            }
            _ => {
                let head = (time * ((count - 1) / 2 + 1) as u64 / period) as usize;
                let tail = program["tail_pixels"].as_u64().unwrap_or(0) as usize;
                let tail = if tail == 0 { count / 12 } else { tail }.max(3);
                let intensity = |distance: usize| {
                    if distance <= tail {
                        255 * (tail + 1 - distance) / (tail + 1)
                    } else {
                        0
                    }
                };
                let left = if pixel <= head {
                    intensity(head - pixel)
                } else {
                    0
                };
                let right = if pixel >= count - 1 - head {
                    intensity(pixel - (count - 1 - head))
                } else {
                    0
                };
                color(program, "primary", left.max(right) as u64)
            }
        })
        .collect()
}

pub(crate) fn draw(ui: &mut egui::Ui, draft: &mut ControllerEffectDraft) {
    let parsed = serde_json::from_str::<Value>(&draft.program_json);
    let mut program = parsed.unwrap_or(Value::Null);
    let writable = valid(&program) || draft.program_json.trim().is_empty();
    let mut changed = false;
    ui.add_enabled_ui(writable, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (index, name) in [
                "Police red / blue",
                "White thunder",
                "Converging red dots",
                "Fade",
                "Pulse",
            ]
            .iter()
            .enumerate()
            {
                if ui.button(tr(ui, name)).clicked() {
                    program = preset(index);
                    changed = true;
                }
            }
        });
    });
    if !valid(&program) {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            tr(ui, "Select a preset or correct the lighting program"),
        );
        return;
    }
    egui::Grid::new("strip-program-fields")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(tr(ui, "Cycle"));
            let mut period = program["period_ms"].as_u64().unwrap();
            let minimum = program["envelope"]
                .as_array()
                .and_then(|p| p.last())
                .and_then(|p| p["at_ms"].as_u64())
                .map_or(50, |v| v + 1)
                .max(program["step_ms"].as_u64().unwrap_or(50))
                .max(50);
            if ui
                .add(
                    egui::Slider::new(&mut period, minimum..=3_600_000)
                        .logarithmic(true)
                        .suffix(" ms")
                        .clamping(egui::SliderClamping::Edits),
                )
                .changed()
            {
                program["period_ms"] = json!(period);
                changed = true;
            }
            ui.end_row();
            for (key, label) in [("primary", "Color"), ("secondary", "Secondary color")] {
                if key == "secondary" && program["primitive"] != "alternating-zones" {
                    continue;
                }
                ui.label(tr(ui, label));
                let c = color(&program, key, 255);
                let mut rgb = [c.r(), c.g(), c.b()];
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    program[key] = json!({"red":rgb[0],"green":rgb[1],"blue":rgb[2]});
                    changed = true;
                }
                ui.end_row();
            }
            if program["primitive"] == "converging-points" {
                ui.label(tr(ui, "Tail pixels"));
                let mut tail = program["tail_pixels"].as_u64().unwrap_or(0);
                if ui.add(egui::Slider::new(&mut tail, 0..=100)).changed() {
                    program["tail_pixels"] = json!(tail);
                    changed = true;
                }
                ui.end_row();
            }
            if program["primitive"] == "alternating-zones" {
                for (key, label, minimum, maximum) in [
                    ("step_ms", "Flash interval", 10, period),
                    ("swap_after_steps", "Swap after flashes", 1, 1024),
                    ("dim_intensity", "Dim brightness", 0, 255),
                ] {
                    ui.label(tr(ui, label));
                    let mut value = program[key].as_u64().unwrap_or(minimum);
                    if ui
                        .add(egui::Slider::new(&mut value, minimum..=maximum))
                        .changed()
                    {
                        program[key] = json!(value);
                        changed = true;
                    }
                    ui.end_row();
                }
            }
        });
    if program["primitive"] == "envelope" {
        ui.label(tr(ui, "Brightness envelope"));
        let period = program["period_ms"].as_u64().unwrap();
        if let Some(points) = program["envelope"].as_array_mut() {
            let bounds: Vec<_> = (0..points.len())
                .map(|i| {
                    (
                        if i == 0 {
                            0
                        } else {
                            points[i - 1]["at_ms"].as_u64().unwrap() + 1
                        },
                        if i + 1 == points.len() {
                            period - 1
                        } else {
                            points[i + 1]["at_ms"].as_u64().unwrap() - 1
                        },
                    )
                })
                .collect();
            for (i, p) in points.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.horizontal(|ui| {
                        let mut at = p["at_ms"].as_u64().unwrap();
                        let mut level = p["intensity"].as_u64().unwrap();
                        if ui
                            .add(
                                egui::Slider::new(&mut at, bounds[i].0..=bounds[i].1).suffix(" ms"),
                            )
                            .changed()
                        {
                            p["at_ms"] = json!(at);
                            changed = true;
                        }
                        if ui.add(egui::Slider::new(&mut level, 0..=255)).changed() {
                            p["intensity"] = json!(level);
                            changed = true;
                        }
                    });
                });
            }
        }
    }
    if changed {
        draft.program_json = serde_json::to_string_pretty(&program).unwrap();
    }
    if !valid(&program) {
        ui.colored_label(
            ui.visuals().error_fg_color,
            tr(
                ui,
                "Envelope times must increase and remain inside the cycle",
            ),
        );
        return;
    }
    ui.separator();
    ui.label(tr(ui, "Preview"));
    let id = ui.id().with("strip-preview-phase");
    let mut phase = ui.data(|d| d.get_temp::<u64>(id)).unwrap_or(0);
    ui.add(egui::Slider::new(&mut phase, 0..=999).show_value(false));
    ui.data_mut(|d| d.insert_temp(id, phase));
    let colors = preview(
        &program,
        usize::from(draft.default_pixels.clamp(1, 100)),
        phase * program["period_ms"].as_u64().unwrap() / 1000,
    );
    ui.horizontal_wrapped(|ui| {
        for (index, c) in colors.iter().enumerate() {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(20., 20.), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4., *c);
            response.on_hover_text(format!(
                "{} · #{:02X}{:02X}{:02X}",
                index + 1,
                c.r(),
                c.g(),
                c.b()
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_render_small_strips() {
        for i in 0..5 {
            let p = preset(i);
            assert!(valid(&p));
            assert_eq!(preview(&p, 3, 1000).len(), 3);
        }
    }
    #[test]
    fn flash_preview_matches_host_when_cycle_is_not_a_multiple_of_step() {
        let mut p = preset(0);
        p["period_ms"] = json!(850);
        assert_eq!(preview(&p, 3, 900)[0], egui::Color32::from_rgb(36, 0, 0));
    }
    #[test]
    fn rejects_bad_envelope_without_normalizing() {
        let mut p = preset(3);
        p["envelope"][1]["at_ms"] = json!(0);
        assert!(!valid(&p));
        assert!(preview(&p, 3, 0).is_empty());
    }
    #[test]
    fn pulse_and_fade_have_distinct_timing() {
        assert_ne!(preview(&preset(3), 3, 500), preview(&preset(4), 3, 500));
    }
}
