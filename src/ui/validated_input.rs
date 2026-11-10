//! Input filtering happens before TextEdit sees an event, never after an
//! illegal character has been painted or committed to a hardware command.
use eframe::egui;

#[derive(Clone, Copy)]
pub enum InputAlphabet { Decimal, Time }

impl InputAlphabet {
    fn accepts(self, text: &str) -> bool {
        text.chars().all(|c| c.is_ascii_digit() || c == '.' || c.is_ascii_whitespace()
            || match self {
                Self::Decimal => c == '%',
                Self::Time => c == ':' || "usmhecinodtrlµμ".contains(c.to_ascii_lowercase()),
            })
    }
}

pub struct ValidatedDrag<'a> {
    pub inner: egui::DragValue<'a>,
    pub alphabet: InputAlphabet,
}

impl egui::Widget for ValidatedDrag<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let id = ui.next_auto_id(); // DragValue deliberately uses this same ID in both modes.
        let focused = ui.memory(|memory| memory.has_focus(id));
        let rejected = focused && ui.input_mut(|input| {
            let mut rejected = false;
            input.events.retain(|event| {
                let text = match event {
                    egui::Event::Text(text) | egui::Event::Paste(text)
                    | egui::Event::Ime(egui::ImeEvent::Commit(text)) => Some(text),
                    _ => None,
                };
                let keep = text.is_none_or(|text| self.alphabet.accepts(text));
                rejected |= !keep;
                keep
            });
            rejected
        });
        let red = invalid_feedback(ui, id, rejected);
        let old_visuals = ui.visuals().clone();
        if red {
            let fill = egui::Color32::from_rgb(125, 35, 40);
            ui.visuals_mut().extreme_bg_color = fill;
            ui.visuals_mut().widgets.active.bg_fill = fill;
        }
        let response = self.inner.ui(ui);
        *ui.visuals_mut() = old_visuals;
        response
    }
}

pub fn invalid_feedback(ui: &egui::Ui, id: egui::Id, rejected: bool) -> bool {
    let feedback_id = id.with("invalid-input-feedback");
    let now = ui.input(|input| input.time);
    let last = ui.data_mut(|data| data.get_temp::<f64>(feedback_id));
    if rejected && last.is_none_or(|last| now - last >= 1.5) {
        ui.data_mut(|data| data.insert_temp(feedback_id, now));
        #[cfg(not(test))]
        input_ding();
    }
    let last = ui.data_mut(|data| data.get_temp::<f64>(feedback_id));
    let elapsed = last.map(|last| now - last);
    // Three restrained pulses at 2 Hz, bounded even for a held invalid key.
    let flashing = elapsed.is_some_and(|elapsed| elapsed < 1.5);
    if flashing { ui.ctx().request_repaint_after(std::time::Duration::from_millis(125)); }
    flashing && elapsed.is_some_and(|elapsed| (elapsed * 4.0) as u32 % 2 == 0)
}

#[cfg(not(test))]
fn input_ding() {
    #[cfg(target_os = "windows")]
    unsafe {
        let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(
            windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING);
    }
    // Native terminal bell is the platform fallback, not a hardware-board tone.
    #[cfg(not(target_os = "windows"))]
    { use std::io::Write; let _ = std::io::stderr().write_all(b"\x07"); }
}

/// Decimal editing and view share one value: .5 -> 0.5, 1..2 -> 1.2,
/// trailing dots disappear on commit. Never interpret invalid text as zero.
pub fn parse_percentage(text: &str) -> Option<f64> {
    let text = text.trim().strip_suffix('%').unwrap_or(text.trim()).trim();
    if !text.chars().all(|c| c.is_ascii_digit() || c == '.') { return None; }
    let mut dot = false;
    let normalized: String = text.chars().filter(|c| {
        if *c != '.' { true } else if dot { false } else { dot = true; true }
    }).collect();
    let normalized = normalized.trim_end_matches('.');
    let value: f64 = normalized.parse().ok()?;
    value.is_finite().then_some(value.clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percentage_editing_normalizes_without_losing_fraction() {
        for (text, value) in [(".5", 0.5), ("0.5%", 0.5), ("1..2..", 1.2), ("5.", 5.0)] {
            assert_eq!(parse_percentage(text), Some(value));
        }
        for text in [".", "", "NaN", "1e2", "5foo"] { assert_eq!(parse_percentage(text), None); }
    }
    #[test]
    fn timing_rejects_illegal_characters_before_editing() {
        for text in ["01:23.456", "1m 20s", "250ms", "5µs"] { assert!(InputAlphabet::Time.accepts(text)); }
        for text in ["1!", "1e+9", "<script>", "five", "1\u{200b}"] { assert!(!InputAlphabet::Time.accepts(text)); }
    }

    #[test]
    fn numeric_widget_preserves_leading_fraction_and_rejects_invalid_events() {
        let ctx = egui::Context::default();
        let mut value = 20.0;
        let mut id = egui::Id::NULL;
        let mut frame = |events: Vec<egui::Event>, focus: bool| {
            let mut output = ctx.run_ui(egui::RawInput {events, ..Default::default()}, |ui| {
                id = ui.next_auto_id();
                if focus { ui.memory_mut(|memory| memory.request_focus(id)); }
                ui.add(ValidatedDrag {alphabet:InputAlphabet::Decimal,
                    inner:egui::DragValue::new(&mut value).range(0.0..=100.0)
                        .fixed_decimals(1).suffix("%").update_while_editing(false)
                        .custom_parser(parse_percentage)});
            });
            output.textures_delta.clear();
            value
        };
        frame(vec![], false);
        frame(vec![], true);
        assert_eq!(frame(vec![egui::Event::Text(".5".into())], false), 20.0);
        assert_eq!(frame(vec![egui::Event::Text("invalid".into())], false), 20.0);
        let enter = egui::Event::Key {key:egui::Key::Enter, physical_key:Some(egui::Key::Enter),
            pressed:true, repeat:false, modifiers:egui::Modifiers::NONE};
        assert_eq!(frame(vec![enter], false), 0.5);
    }
}
