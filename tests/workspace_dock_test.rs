use egui_dock::DockState;
use pealayer::config::AppConfig;
use pealayer::ui::layout::{create_initial_layout, PealayerTab};

#[test]
fn test_config_workspace_dock_layout_field() {
    let mut config = AppConfig::default();
    assert_eq!(config.workspace_dock_layout, None);

    config.workspace_dock_layout = Some(r#"{"dummy": true}"#.to_string());
    let json_str = serde_json::to_string(&config).expect("serialize config to JSON");
    let loaded: AppConfig = serde_json::from_str(&json_str).expect("deserialize config from JSON");
    assert_eq!(
        loaded.workspace_dock_layout,
        Some(r#"{"dummy": true}"#.to_string())
    );
}

#[test]
fn test_pealayer_tab_all_contains_five_tabs() {
    assert_eq!(PealayerTab::ALL.len(), 5);
    assert!(PealayerTab::ALL.contains(&PealayerTab::ProgramMonitor));
    assert!(PealayerTab::ALL.contains(&PealayerTab::Timeline));
    assert!(PealayerTab::ALL.contains(&PealayerTab::EffectControls));
    assert!(PealayerTab::ALL.contains(&PealayerTab::EffectsLibrary));
    assert!(PealayerTab::ALL.contains(&PealayerTab::HardwareMonitor));
}

#[test]
fn test_dock_state_serde_roundtrip() {
    let dock_state = create_initial_layout();
    let json_str = serde_json::to_string(&dock_state).expect("serialize dock state to JSON");
    assert!(!json_str.is_empty());

    let deserialized: DockState<PealayerTab> =
        serde_json::from_str(&json_str).expect("deserialize dock state from JSON");

    for tab in PealayerTab::ALL {
        assert!(
            deserialized.find_tab(&tab).is_some(),
            "tab {:?} should be present after round-trip",
            tab
        );
    }
}

#[test]
fn test_pealayer_tab_titles_and_icons() {
    let app = pealayer::app::PealayerApp::default();
    for tab in PealayerTab::ALL {
        assert!(!tab.title(&app).is_empty());
        assert!(!tab.icon().is_empty());
    }
    assert_eq!(PealayerTab::ProgramMonitor.title(&app), "Program Monitor");
    assert_eq!(PealayerTab::Timeline.title(&app), "Timeline");
    assert_eq!(PealayerTab::EffectControls.title(&app), "Effect Controls");
    assert_eq!(PealayerTab::EffectsLibrary.title(&app), "Effects Library");
    assert_eq!(PealayerTab::HardwareMonitor.title(&app), "Hardware Monitor");
}

use pealayer::ui::layout::restore_tab_to_canonical_slot;

#[test]
fn test_restore_timeline_to_canonical_slot() {
    let mut dock_state = create_initial_layout();
    let path = dock_state.find_tab(&PealayerTab::Timeline).expect("find timeline");
    dock_state.remove_tab(path);
    assert!(dock_state.find_tab(&PealayerTab::Timeline).is_none());

    restore_tab_to_canonical_slot(&mut dock_state, PealayerTab::Timeline);
    assert!(dock_state.find_tab(&PealayerTab::Timeline).is_some());
}

#[test]
fn test_restore_controls_next_to_hardware_monitor() {
    let mut dock_state = create_initial_layout();
    let path = dock_state.find_tab(&PealayerTab::EffectControls).expect("find effect controls");
    dock_state.remove_tab(path);
    assert!(dock_state.find_tab(&PealayerTab::EffectControls).is_none());

    restore_tab_to_canonical_slot(&mut dock_state, PealayerTab::EffectControls);
    assert!(dock_state.find_tab(&PealayerTab::EffectControls).is_some());
}

#[test]
fn test_restore_into_empty_dock() {
    let mut dock_state = create_initial_layout();
    for tab in PealayerTab::ALL {
        if let Some(path) = dock_state.find_tab(&tab) {
            dock_state.remove_tab(path);
        }
    }
    assert_eq!(dock_state.iter_all_tabs().count(), 0);

    restore_tab_to_canonical_slot(&mut dock_state, PealayerTab::ProgramMonitor);
    assert!(dock_state.find_tab(&PealayerTab::ProgramMonitor).is_some());
    assert_eq!(dock_state.iter_all_tabs().count(), 1);
}

#[test]
fn test_pealayer_app_tab_toggle_and_focus() {
    let mut app = pealayer::app::PealayerApp::default();
    assert!(app.is_tab_open(PealayerTab::Timeline));

    // Toggle Timeline off
    app.toggle_tab(PealayerTab::Timeline);
    assert!(!app.is_tab_open(PealayerTab::Timeline));

    // Toggle Timeline on
    app.toggle_tab(PealayerTab::Timeline);
    assert!(app.is_tab_open(PealayerTab::Timeline));

    // Calling open_or_focus_tab on already open tab keeps it open
    app.open_or_focus_tab(PealayerTab::Timeline);
    assert!(app.is_tab_open(PealayerTab::Timeline));
}

#[test]
fn test_corrupt_dock_json_fallback() {
    let invalid_json = "{ corrupted invalid json ...";
    let fallback = serde_json::from_str::<DockState<PealayerTab>>(invalid_json)
        .unwrap_or_else(|_| create_initial_layout());

    for tab in PealayerTab::ALL {
        assert!(fallback.find_tab(&tab).is_some());
    }
}

#[test]
fn test_app_save_dock_layout_persists_to_config() {
    let mut app = pealayer::app::PealayerApp::default();
    app.save_dock_layout();

    let cfg = pealayer::config::AppConfig::load();
    assert!(cfg.workspace_dock_layout.is_some());
    let layout_json = cfg.workspace_dock_layout.unwrap();
    let deserialized: DockState<PealayerTab> = serde_json::from_str(&layout_json).expect("valid dock state JSON");
    assert!(deserialized.find_tab(&PealayerTab::ProgramMonitor).is_some());
}
