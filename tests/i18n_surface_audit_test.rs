#[test]
fn native_ui_routes_known_application_copy_through_localization() {
    let surfaces = [
        include_str!("../src/ui/controls.rs"),
        include_str!("../src/ui/four_d.rs"),
        include_str!("../src/ui/layout.rs"),
        include_str!("../src/ui/video.rs"),
    ]
    .join("\n");

    for forbidden in [
        "Window::new(\"Controls\")",
        "on_hover_text(\"Set to current playback position\")",
        "on_hover_text(\"Seek video to this event\")",
        "RichText::new(\"🗑 Delete Cue\")",
        "RichText::new(\"🗑 Delete All Selected\")",
        "ui.button(\"Linear\")",
        "ui.button(\"Smooth (Hermite)\")",
        "ui.button(\"Step\")",
        "ui.button(\"Delete Keyframe\")",
        "ui.label(\"No recent media\")",
        "ui.button(\"Clear Recent\")",
    ] {
        assert!(
            !surfaces.contains(forbidden),
            "application copy bypasses the native localization boundary: {forbidden}"
        );
    }
}

#[test]
fn web_ui_routes_known_application_copy_through_localization() {
    let surfaces = [
        include_str!("../web_ui/src/components/RemoteControlTab.tsx"),
        include_str!("../web_ui/src/components/MediaLibraryTab.tsx"),
        include_str!("../web_ui/src/components/PlayerInfoTab.tsx"),
    ]
    .join("\n");

    for forbidden in [
        ">Time</Text>",
        ">Status</Text>",
        ">Volume</Text>",
        "message.info('Volume muted')",
        "message.error('Failed loading directory')",
        "placeholder=\"Search media files...\"",
        "label=\"Playback Status\"",
        "label=\"Hardware Acceleration\"",
    ] {
        assert!(
            !surfaces.contains(forbidden),
            "application copy bypasses the web localization boundary: {forbidden}"
        );
    }

    let dictionary = include_str!("../web_ui/src/i18n.ts");
    for required in [
        "'Remote Control'",
        "'No Media Active'",
        "'Search media files...'",
        "'System & Media Metadata'",
        "'Playback Status'",
    ] {
        assert!(dictionary.contains(required), "missing Persian key {required}");
    }
}
