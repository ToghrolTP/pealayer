//! PakarController-style deliberate full-frame probes, adapted to a linear strip.
//! A preview is requested RGB, never optical feedback or board RGB readback.
use crate::{app::PealayerApp, four_d::controller::HardwareCapabilities};
use eframe::egui;

pub(crate) fn frame(count: u16, index: u16, rgb: [u8; 3]) -> Option<Vec<u8>> {
    if !(1..=100).contains(&count) || index >= count {
        return None;
    }
    let mut result = vec![0; usize::from(count) * 3];
    result[usize::from(index) * 3..usize::from(index) * 3 + 3].copy_from_slice(&rgb);
    Some(result)
}

#[derive(Clone, Default)]
struct ProbeState {
    index: u16,
    armed: Option<(String, u64, u16, std::time::Instant)>,
}

pub(crate) fn draw(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    count: u16,
) {
    let id = ui.id().with("pixel-diagnostic");
    let mut state = ui
        .data(|d| d.get_temp::<ProbeState>(id))
        .unwrap_or_default();
    state.index = state.index.min(count.saturating_sub(1));
    let ready =
        capabilities.board_connected && !app.estop_active && !app.hardware_programming_blocked();
    if !ready
        || state
            .armed
            .as_ref()
            .is_some_and(|(host, generation, pixels, at)| {
                host != &capabilities.host_instance_id
                    || *generation != capabilities.connection_generation
                    || *pixels != count
                    || at.elapsed().as_secs() >= 30
            })
    {
        state.armed = None;
    }
    let busy = app.hardware_effect_authoring.pending_operation.is_some();
    ui.label(app.tr("Pixel diagnostics"));
    ui.horizontal_wrapped(|ui| {
        for index in 0..count {
            if ui
                .add_sized(
                    [32., 32.],
                    egui::Button::new((index + 1).to_string()).selected(index == state.index),
                )
                .clicked()
            {
                state.index = index;
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(state.index > 0, egui::Button::new(app.tr("Previous")))
            .clicked()
        {
            state.index -= 1;
        }
        ui.label(format!(
            "{} {} / {}",
            app.tr("Pixel"),
            state.index + 1,
            count
        ));
        if ui
            .add_enabled(state.index + 1 < count, egui::Button::new(app.tr("Next")))
            .clicked()
        {
            state.index += 1;
        }
    });
    let mut arm = state.armed.is_some();
    if ui
        .add_enabled(
            ready && !busy,
            egui::Checkbox::new(&mut arm, app.tr("Arm one test")),
        )
        .changed()
    {
        state.armed = arm.then(|| {
            (
                capabilities.host_instance_id.clone(),
                capabilities.connection_generation,
                count,
                std::time::Instant::now(),
            )
        });
    }
    let mut send = None;
    ui.horizontal_wrapped(|ui| {
        for (label, rgb) in [
            ("Red", [255, 0, 0]),
            ("Green", [0, 255, 0]),
            ("Blue", [0, 0, 255]),
            ("White", [255, 255, 255]),
        ] {
            if ui
                .add_enabled(ready && !busy && arm, egui::Button::new(app.tr(label)))
                .on_hover_text(app.tr("Light the selected pixel; clear all others"))
                .clicked()
            {
                send = frame(count, state.index, rgb);
            }
        }
        if ui
            .add_enabled(ready && !busy, egui::Button::new(app.tr("All off")))
            .clicked()
        {
            send = Some(vec![0; usize::from(count) * 3]);
        }
    });
    if let Some(rgb) = send {
        state.armed = None;
        if let Err(error) = app.send_addressable_strip_frame(count, &rgb) {
            app.set_osd(error);
        }
    }
    if busy {
        ui.spinner();
    }
    ui.data_mut(|d| d.insert_temp(id, state));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probe_replaces_full_frame_without_stale_tail() {
        let f = frame(38, 37, [1, 2, 3]).unwrap();
        assert_eq!(f.len(), 114);
        assert!(f[..111].iter().all(|v| *v == 0));
        assert_eq!(&f[111..], &[1, 2, 3]);
    }
    #[test]
    fn invalid_probe_does_not_produce_a_mutation() {
        assert!(frame(3, 3, [255; 3]).is_none());
        assert!(frame(0, 0, [255; 3]).is_none());
        assert!(frame(101, 0, [255; 3]).is_none());
    }
}
