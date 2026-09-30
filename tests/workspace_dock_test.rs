use pealayer::config::AppConfig;

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
