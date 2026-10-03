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
    assert!(app.contains("target.pathname = '/api/player/command'"));
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
fn pwa_never_caches_live_api_or_websocket_state() {
    let main = include_str!("../web_ui/src/main.tsx");
    let worker = include_str!("../web_ui/public/sw.js");
    let html = include_str!("../web_ui/index.html");

    assert!(main.contains("navigator.serviceWorker.register('/sw.js'"));
    assert!(html.contains("rel=\"manifest\""));
    assert!(worker.contains("url.pathname.startsWith('/api/')"));
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
