use eframe::egui;
use egui_dock::TabViewer;
use pealayer::app::PealayerApp;
use pealayer::four_d::controller::{HardwareAction, HardwareCapabilities, HardwareControl};
use pealayer::four_d::curve::{AnalogTrack, Interpolation, Keyframe};
use pealayer::four_d::models::{AtomicAction, Effect, EffectInstance, TimelineKeyframe};
use pealayer::ui::layout::{PealayerTab, PealayerTabViewer};

#[test]
fn native_timeline_mouse_buttons_do_not_crash() {
    replay_timeline_pointer_buttons(true);
}

#[test]
fn empty_timeline_all_mouse_buttons_do_not_crash() {
    // Station-1 report: no media, no cue/keyframe, any mouse button in empty tracks.
    replay_timeline_pointer_buttons(false);
}

fn replay_timeline_pointer_buttons(with_content: bool) {
    let mut app = PealayerApp::default();
    app.duration = if with_content { 60.0 } else { 0.0 };
    app.update_hardware_capabilities(Some(HardwareCapabilities {
        board_connected: true,
        board_name: "Pointer fixture".into(),
        controls: [
            ("seat.a", "seat", "Seat A", vec!["up", "down", "stop"]),
            ("seat.b", "seat", "Seat B", vec!["up", "down", "stop"]),
            ("relay.5", "relay", "R5", vec!["on", "off", "toggle"]),
            ("pwm.0", "mosfet", "P1", vec!["set"]),
        ]
        .into_iter()
        .map(|(key, kind, name, verbs)| HardwareControl {
            key: key.into(),
            kind: kind.into(),
            name: name.into(),
            default_name: name.into(),
            actions: verbs
                .into_iter()
                .map(|verb| HardwareAction {
                    verb: verb.into(),
                    name: verb.into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        })
        .collect(),
        ..Default::default()
    }));
    if with_content {
        let mut track = AnalogTrack::new("Pointer regression fixture", 0);
        track.add_keyframe(Keyframe::new(1000, 0.5, Interpolation::Linear));
        app.timeline.analog_tracks.push(track);
        let effect = Effect::new(
            "Pointer cue".into(),
            String::new(),
            1000,
            vec![AtomicAction {
                relay_id: 5,
                state: true,
                offset_ms: 0,
            }],
        );
        app.timeline
            .instances
            .push(EffectInstance::new(effect.id, 1000));
        app.timeline.templates.push(effect);
        app.timeline.keyframes.push(TimelineKeyframe::new(1000));
    }
    let context = egui::Context::default();
    // Match Windows' activated accessibility adapter, not just egui paint/input.
    context.enable_accesskit();
    let mut time = 0.0;
    let mut frame = |events| {
        time += 0.02;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 420.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                PealayerTabViewer { app: &mut app }.ui(ui, &mut PealayerTab::Timeline);
            },
        );
        if let Some(update) = &output.platform_output.accesskit_update {
            assert!(
                update.nodes.iter().any(|(id, _)| *id == update.focus),
                "focused accessibility node {:?} must be present in the frame's node list",
                update.focus,
            );
        }
        output.textures_delta.clear();
    };
    frame(vec![]);
    frame(vec![]);
    let mut opened_menus = 0;
    for button in [
        egui::PointerButton::Primary,
        egui::PointerButton::Secondary,
        egui::PointerButton::Middle,
    ] {
        for pos in [70.0, 110.0, 155.0, 200.0, 255.0, 350.0]
            .into_iter()
            .flat_map(|y| {
                [
                    egui::pos2(150.0, y),
                    egui::pos2(450.0, y),
                    egui::pos2(650.0, y),
                ]
            })
        {
            eprintln!("timeline pointer {button:?} at {pos:?}");
            frame(vec![egui::Event::PointerMoved(pos)]);
            frame(vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }]);
            frame(vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            if button == egui::PointerButton::Secondary && egui::Popup::is_any_open(&context) {
                opened_menus += 1;
            }
            frame(vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            frame(vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            frame(vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }]);
            let destination = pos + egui::vec2(40.0, 20.0);
            frame(vec![egui::Event::PointerMoved(destination)]);
            frame(vec![egui::Event::PointerButton {
                pos: destination,
                button,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            frame(vec![]);
        }
    }
    assert!(
        opened_menus > 0,
        "right clicks must actually open context menus"
    );
}

#[test]
fn panic_hook_records_real_failure_in_a_separate_process() {
    const CHILD_ENV: &str = "PEALAYER_DIAGNOSTICS_TEST_CHILD";
    if std::env::var_os(CHILD_ENV).is_some() {
        pealayer::diagnostics::install_panic_reporter();
        panic!("isolated diagnostic fixture panic");
    }
    let directory =
        std::env::temp_dir().join(format!("pealayer-panic-hook-{}", uuid::Uuid::new_v4()));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "panic_hook_records_real_failure_in_a_separate_process",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .env("PEALAYER_CONFIG_FILE", directory.join("config.json"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let path = directory.join("diagnostics/latest-panic.log");
    let report = std::fs::read_to_string(&path).unwrap();
    assert!(report.contains("isolated diagnostic fixture panic"));
    assert!(report.contains(env!("PEALAYER_GIT_COMMIT")));
    assert!(report.contains("thread="));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory.join("diagnostics")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
