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
