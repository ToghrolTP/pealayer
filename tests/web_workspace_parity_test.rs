//! Structural guards for the installable web workspace.
//!
//! These checks deliberately protect the cross-surface contract rather than a
//! particular screenshot. They catch the regressions that previously turned
//! the web client back into a separate, partial remote-control concept.

#[test]
fn web_workspace_exposes_the_same_primary_surfaces_as_the_native_app() {
    let app = include_str!("../web_ui/src/App.tsx");

    for route in [
        "key: 'player'",
        "key: 'timeline'",
        "key: 'effects'",
        "key: 'hardware'",
        "key: 'library'",
        "key: 'about'",
        "key: 'preferences'",
    ] {
        assert!(app.contains(route), "missing web workspace route {route}");
    }
    assert!(app.contains("<EffectsTab state={state}"));
    assert!(app.contains("<HardwareTab state={state}"));
    assert!(app.contains("surface=\"timeline\""));
    assert!(app.contains("apiEndpoint('/api/rpc')"));
}

#[test]
fn web_hardware_and_effects_consume_the_shared_live_contract() {
    let hardware = include_str!("../web_ui/src/components/HardwareTab.tsx");
    let effects = include_str!("../web_ui/src/components/EffectsTab.tsx");
    let timeline = include_str!("../web_ui/src/components/StudioTab.tsx");

    assert!(hardware.contains("state.hardware_details"));
    assert!(hardware.contains("hardware.action.invoke"));
    assert!(hardware.contains("hardware.pwm.set"));
    assert!(hardware.contains("hardware.strip.fill"));
    assert!(hardware.contains("hardware.front_panel.press"));
    assert!(!hardware.contains("sample"));

    assert!(effects.contains("New effect"));
    assert!(effects.contains("Sequence steps"));
    assert!(effects.contains("controller_effect.save"));
    assert!(timeline.contains("effect.lane"));
}

#[test]
fn effect_recording_is_offered_for_new_and_existing_sequences() {
    let native_hardware = include_str!("../src/ui/layout.rs");
    let native_effects = include_str!("../src/ui/effects_library.rs");
    let web_hardware = include_str!("../web_ui/src/components/HardwareTab.tsx");
    let web_effects = include_str!("../web_ui/src/components/EffectsTab.tsx");
    let web_studio = include_str!("../web_ui/src/components/StudioTab.tsx");

    assert!(!native_hardware.contains("draw_effect_recording_panel"));
    assert!(native_effects.contains("if app.effect_library_draft.kind == \"sequence\""));
    assert_eq!(
        native_effects
            .matches("draw_effect_capture_controls(app, ui)")
            .count(),
        1
    );
    assert!(!web_hardware.contains("EffectRecorder"));
    assert!(web_effects.contains("draft.kind === 'sequence' ? <>"));
    assert_eq!(web_effects.matches("<EffectRecorder").count(), 1);
    assert!(web_studio.contains("effectDraft.kind === 'sequence' && ("));
    assert_eq!(web_studio.matches("<EffectRecorder").count(), 1);
}

#[test]
fn pwa_never_caches_live_api_or_websocket_state() {
    let main = include_str!("../web_ui/src/main.tsx");
    let platform = include_str!("../web_ui/src/webPlatform.ts");
    let worker = include_str!("../web_ui/public/sw.js");
    let html = include_str!("../web_ui/index.html");

    assert!(main.contains("registerPealayerServiceWorker"));
    assert!(platform.contains("navigator.serviceWorker.register('/sw.js'"));
    assert!(html.contains("rel=\"manifest\""));
    assert!(worker.contains("(url.pathname.startsWith('/api/') && !precached)"));
    assert!(worker.contains("url.pathname === '/ws'"));
    assert!(worker.contains("request.mode === 'navigate'"));
}

#[test]
fn viewport_has_exactly_one_primary_scroll_owner() {
    let styles = include_str!("../web_ui/src/styles.css");

    assert!(styles.contains(
        "html, body, #root { width: 100%; height: 100%; min-height: 0; margin: 0; overflow: hidden; }"
    ));
    assert!(styles.contains(".app-shell { width: 100%; height: 100dvh;"));
    assert!(styles.contains(
        ".app-body { flex: 1 1 auto; min-width: 0; min-height: 0 !important; overflow: hidden;"
    ));
    assert!(styles.contains(".app-content { min-width: 0; min-height: 0;"));
    assert!(
        styles.contains("overflow: auto; overscroll-behavior: contain; scrollbar-gutter: stable;")
    );
}
