//! PCController's declarative segmented-strip contract. No persistence or I/O.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub(crate) fn valid(program: &Value) -> bool {
    if program["primitive"] == "layers" {
        return serde_json::from_value::<crate::four_d::strip_program::Program>(program.clone())
            .is_ok_and(|p| p.validate(100).is_ok());
    }
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

pub(crate) fn validate_definition(
    program: &Value,
    pixels: u16,
    fps: u8,
    duration: u64,
) -> Result<(), String> {
    if !(1..=100).contains(&pixels)
        || !(1..=120).contains(&fps)
        || !(100..=3_600_000).contains(&duration)
    {
        return Err(
            "Strip requires 1–100 pixels, 1–120 FPS and a duration of 100 ms–1 hour".into(),
        );
    }
    if program["primitive"] == "layers" {
        return serde_json::from_value::<Program>(program.clone())
            .map_err(|e| e.to_string())?
            .validate(pixels as usize);
    }
    if valid(program) {
        Ok(())
    } else {
        Err("Invalid strip program".into())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub id: String,
    pub name: String,
    pub pixels: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub segment_id: String,
    pub enabled: bool,
    pub primitive: String,
    pub primary: Color,
    pub secondary: Color,
    pub period_ms: u64,
    pub phase_ms: u64,
    pub duty_percent: u8,
    pub easing: String,
    pub reverse: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub primitive: String,
    pub period_ms: u64,
    pub pixels_per_meter: u16,
    pub segments: Vec<Segment>,
    pub layers: Vec<Layer>,
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}
fn name(s: &str) -> bool {
    !s.trim().is_empty() && s.chars().count() <= 64 && !s.chars().any(char::is_control)
}
impl Layer {
    pub fn new(id: String, segment_id: String) -> Self {
        Self {
            name: "Layer".into(),
            id,
            segment_id,
            enabled: true,
            primitive: "solid".into(),
            primary: Color {
                red: 0,
                green: 96,
                blue: 255,
            },
            secondary: Color::default(),
            period_ms: 3000,
            phase_ms: 0,
            duty_percent: 50,
            easing: "linear".into(),
            reverse: false,
        }
    }
}
impl Program {
    pub fn new(pixels: usize) -> Self {
        Self {
            primitive: "layers".into(),
            period_ms: 3000,
            pixels_per_meter: 60,
            segments: vec![Segment {
                id: "all".into(),
                name: "All pixels".into(),
                pixels: (0..pixels).collect(),
            }],
            layers: vec![Layer::new("layer-1".into(), "all".into())],
        }
    }
    pub fn validate(&self, count: usize) -> Result<(), String> {
        let fail = |s: &str| Err(s.to_owned());
        if self.primitive != "layers"
            || !(50..=3_600_000).contains(&self.period_ms)
            || !(1..=100).contains(&count)
            || !(1..=1000).contains(&self.pixels_per_meter)
        {
            return fail("Invalid strip cycle, pixel count or density");
        }
        if !(1..=64).contains(&self.segments.len()) || !(1..=64).contains(&self.layers.len()) {
            return fail("Use 1–64 segments and layers");
        }
        let mut ids = HashSet::new();
        for segment in &self.segments {
            if !token(&segment.id) || !name(&segment.name) || !ids.insert(&segment.id) {
                return fail("Segments require unique IDs and names");
            }
            let mut seen = HashSet::new();
            if segment.pixels.is_empty()
                || segment.pixels.len() > count
                || segment
                    .pixels
                    .iter()
                    .any(|p| *p >= count || !seen.insert(p))
            {
                return fail("Segment pixels must be unique and inside the strip");
            }
        }
        let mut layers = HashSet::new();
        for layer in &self.layers {
            if !token(&layer.id)
                || !name(&layer.name)
                || !layers.insert(&layer.id)
                || !ids.contains(&layer.segment_id)
            {
                return fail("Layer requires a unique ID, name and existing segment");
            }
            if !(50..=3_600_000).contains(&layer.period_ms)
                || layer.phase_ms >= layer.period_ms
                || layer.duty_percent > 100
            {
                return fail("Invalid layer cycle, phase or duty");
            }
            if !matches!(
                layer.primitive.as_str(),
                "solid" | "gradient" | "ramp" | "blink" | "breathe"
            ) || !matches!(
                layer.easing.as_str(),
                "linear" | "ease-in" | "ease-out" | "ease-in-out"
            ) {
                return fail("Unsupported layer pattern or easing");
            }
        }
        Ok(())
    }
    pub fn frame(&self, count: usize, at: u64) -> Vec<Color> {
        if self.validate(count).is_err() {
            return Vec::new();
        }
        let mut frame = vec![Color::default(); count];
        for layer in self.layers.iter().filter(|l| l.enabled) {
            let segment = self
                .segments
                .iter()
                .find(|s| s.id == layer.segment_id)
                .unwrap();
            let phase = (at % layer.period_ms + layer.phase_ms) % layer.period_ms;
            let t = phase as f64 / (layer.period_ms - 1) as f64;
            for (index, pixel) in segment.pixels.iter().enumerate() {
                let mut mix = match layer.primitive.as_str() {
                    "gradient" => {
                        if segment.pixels.len() > 1 {
                            index as f64 / (segment.pixels.len() - 1) as f64
                        } else {
                            0.
                        }
                    }
                    "ramp" => t,
                    "blink" => {
                        if phase * 100 >= layer.period_ms * u64::from(layer.duty_percent) {
                            1.
                        } else {
                            0.
                        }
                    }
                    "breathe" => (1. - (t * std::f64::consts::TAU).cos()) / 2.,
                    _ => 0.,
                };
                if layer.reverse {
                    mix = 1. - mix;
                }
                mix = match layer.easing.as_str() {
                    "ease-in" => mix * mix,
                    "ease-out" => 1. - (1. - mix) * (1. - mix),
                    "ease-in-out" => mix * mix * (3. - 2. * mix),
                    _ => mix,
                };
                let channel = |a: u8, b: u8| {
                    (f64::from(a) + (f64::from(b) - f64::from(a)) * mix).round() as u8
                };
                frame[*pixel] = Color {
                    red: channel(layer.primary.red, layer.secondary.red),
                    green: channel(layer.primary.green, layer.secondary.green),
                    blue: channel(layer.primary.blue, layer.secondary.blue),
                };
            }
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn segments_are_explicit_ordered_and_bounded() {
        let mut p = Program::new(5);
        p.segments[0].pixels = vec![4, 0, 2];
        p.layers[0].primitive = "gradient".into();
        p.layers[0].primary = Color {
            red: 255,
            ..Default::default()
        };
        p.layers[0].secondary = Color {
            blue: 255,
            ..Default::default()
        };
        let frame = p.frame(5, 0);
        assert_eq!(frame[4].red, 255);
        assert_eq!(frame[2].blue, 255);
        assert_eq!(frame[1], Color::default());
        assert_eq!(frame[0].red, 128);
        assert!(p.validate(3).is_err());
        p.segments[0].pixels = vec![0, 0];
        assert!(p.validate(5).is_err());
    }
    #[test]
    fn disabled_layers_never_replace_pixels() {
        let mut p = Program::new(2);
        let mut l = p.layers[0].clone();
        l.id = "second".into();
        l.enabled = false;
        l.primary.red = 255;
        p.layers.push(l);
        assert_eq!(p.frame(2, 0)[0].red, 0);
        p.layers[1].enabled = true;
        assert_eq!(p.frame(2, 0)[0].red, 255);
    }
    #[test]
    fn cycle_and_phase_are_deterministic() {
        let mut p = Program::new(1);
        p.layers[0].primitive = "blink".into();
        assert_eq!(p.frame(1, 1500)[0], Color::default());
        assert_eq!(p.frame(1, 0), p.frame(1, 3000));
        p.layers[0].phase_ms = 1500;
        assert_eq!(p.frame(1, 0)[0], Color::default());
    }
    #[test]
    fn all_patterns_and_curves_render() {
        for kind in ["solid", "gradient", "ramp", "blink", "breathe"] {
            for easing in ["linear", "ease-in", "ease-out", "ease-in-out"] {
                let mut p = Program::new(100);
                p.layers[0].primitive = kind.into();
                p.layers[0].easing = easing.into();
                assert!(p.validate(100).is_ok());
                assert_eq!(p.frame(100, 1234).len(), 100);
            }
        }
    }
    #[test]
    fn current_contract_requires_enabled_and_known_easing() {
        let p = Program::new(2);
        let mut v = serde_json::to_value(p).unwrap();
        v["layers"][0].as_object_mut().unwrap().remove("enabled");
        assert!(serde_json::from_value::<Program>(v).is_err());
    }
}
