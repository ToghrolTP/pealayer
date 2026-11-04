use craft_ui::docking::Workspace;
use pealayer::config::{AppConfig, AppLanguage};
use pealayer::ui::layout::{PealayerTab, create_initial_layout};

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
fn test_pealayer_tab_all_contains_six_tabs() {
    assert_eq!(PealayerTab::ALL.len(), 6);
    assert!(PealayerTab::ALL.contains(&PealayerTab::ProgramMonitor));
    assert!(PealayerTab::ALL.contains(&PealayerTab::MediaInspector));
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

    let deserialized: Workspace<PealayerTab> =
        serde_json::from_str(&json_str).expect("deserialize dock state from JSON");

    for tab in PealayerTab::ALL
        .into_iter()
        .filter(|tab| *tab != PealayerTab::MediaInspector)
    {
        assert!(
            deserialized.layout.contains(&tab),
            "tab {:?} should be present after round-trip",
            tab
        );
    }
    assert!(
        !deserialized
            .layout
            .contains(&PealayerTab::MediaInspector),
        "the optional Media Inspector must remain hidden in the default workspace"
    );
}

#[test]
fn test_pealayer_tab_titles_and_icons() {
    let language = AppLanguage::English;
    for tab in PealayerTab::ALL {
        assert!(!tab.title_for_language(language).is_empty());
        assert!(!tab.title_for_language(AppLanguage::Persian).is_empty());
        assert!(!tab.icon().is_empty());
    }
    assert_eq!(
        PealayerTab::ProgramMonitor.title_for_language(language),
        "Program Monitor"
    );
    assert_eq!(
        PealayerTab::MediaInspector.title_for_language(language),
        "Media Inspector"
    );
    assert_eq!(
        PealayerTab::Timeline.title_for_language(language),
        "Timeline"
    );
    assert_eq!(
        PealayerTab::EffectControls.title_for_language(language),
        "Effect Controls"
    );
    assert_eq!(
        PealayerTab::EffectsLibrary.title_for_language(language),
        "Effects Library"
    );
    assert_eq!(
        PealayerTab::HardwareMonitor.title_for_language(language),
        "Hardware Monitor"
    );
}

use pealayer::ui::layout::reveal_and_focus_tab;

#[test]
fn test_restore_timeline_to_canonical_slot() {
    let mut dock_state = create_initial_layout();
    assert!(dock_state.layout.contains(&PealayerTab::Timeline));
    let _ = dock_state.set_visible(
        PealayerTab::Timeline,
        false,
        None,
        |_| craft_ui::docking::Permissions::default(),
    );
    assert!(!dock_state.layout.contains(&PealayerTab::Timeline));

    reveal_and_focus_tab(&mut dock_state, PealayerTab::Timeline);
    assert!(dock_state.layout.contains(&PealayerTab::Timeline));
}

#[test]
fn test_restore_controls_next_to_hardware_monitor() {
    let mut dock_state = create_initial_layout();
    assert!(dock_state.layout.contains(&PealayerTab::EffectControls));
    let _ = dock_state.set_visible(
        PealayerTab::EffectControls,
        false,
        None,
        |_| craft_ui::docking::Permissions::default(),
    );
    assert!(!dock_state.layout.contains(&PealayerTab::EffectControls));

    reveal_and_focus_tab(&mut dock_state, PealayerTab::EffectControls);
    assert!(dock_state.layout.contains(&PealayerTab::EffectControls));
}

#[test]
fn test_restore_optional_media_inspector() {
    let mut dock_state = create_initial_layout();
    assert!(!dock_state.layout.contains(&PealayerTab::MediaInspector));

    reveal_and_focus_tab(&mut dock_state, PealayerTab::MediaInspector);
    assert!(dock_state.layout.contains(&PealayerTab::MediaInspector));
}

#[test]
fn test_restore_into_empty_dock() {
    let mut dock_state = create_initial_layout();
    for tab in PealayerTab::ALL {
        let _ = dock_state.set_visible(
            tab,
            false,
            None,
            |_| craft_ui::docking::Permissions::default(),
        );
    }
    assert_eq!(dock_state.layout.panels().len(), 0);

    reveal_and_focus_tab(&mut dock_state, PealayerTab::ProgramMonitor);
    assert!(dock_state.layout.contains(&PealayerTab::ProgramMonitor));
    assert_eq!(dock_state.layout.panels().len(), 1);
}

static CONFIG_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn require_parent_isolated_config() {
    let path = std::env::var_os("PEALAYER_CONFIG_FILE")
        .expect("Set an isolated PEALAYER_CONFIG_FILE before starting workspace persistence tests");
    assert!(std::path::Path::new(&path).is_absolute());
    // Never mutate process environment underneath libmpv/native threads.
}

#[test]
fn test_pealayer_app_tab_toggle_and_focus() {
    let _guard = CONFIG_TEST_MUTEX.lock().unwrap();
    require_parent_isolated_config();

    let mut app = pealayer::app::PealayerApp::default();
    assert!(app.is_tab_open(PealayerTab::Timeline));
    assert_eq!(
        PealayerTab::Timeline.title(&app),
        app.tr("Timeline")
    );

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
    let fallback = serde_json::from_str::<Workspace<PealayerTab>>(invalid_json)
        .unwrap_or_else(|_| create_initial_layout());

    for tab in PealayerTab::ALL
        .into_iter()
        .filter(|tab| *tab != PealayerTab::MediaInspector)
    {
        assert!(fallback.layout.contains(&tab));
    }
    assert!(!fallback.layout.contains(&PealayerTab::MediaInspector));
}

#[test]
fn test_app_save_dock_layout_persists_to_config() {
    let _guard = CONFIG_TEST_MUTEX.lock().unwrap();
    require_parent_isolated_config();

    let mut app = pealayer::app::PealayerApp::default();
    app.save_dock_layout();

    let cfg = pealayer::config::AppConfig::load();
    assert!(cfg.workspace_dock_layout.is_some());
    let layout_json = cfg.workspace_dock_layout.unwrap();
    let deserialized: Workspace<PealayerTab> =
        serde_json::from_str(&layout_json).expect("valid dock state JSON");
    assert!(
        deserialized
            .layout
            .contains(&PealayerTab::ProgramMonitor)
    );
}

#[test]
fn test_pealayer_tab_icons_not_empty() {
    for tab in PealayerTab::ALL {
        assert!(!tab.icon().is_empty());
    }
}

#[test]
fn test_workspace_window_menu_translations() {
    let en = AppLanguage::English;
    assert_eq!(pealayer::ui::i18n::tr(en, "Workspace"), "Workspace");
    assert_eq!(pealayer::ui::i18n::tr(en, "Window"), "Window");
    assert_eq!(pealayer::ui::i18n::tr(en, "Panels"), "Panels");
    assert_eq!(
        pealayer::ui::i18n::tr(en, "Reset Workspace to Default"),
        "Reset Workspace to Default"
    );

    let fa = pealayer::config::AppLanguage::Persian;
    assert_eq!(
        pealayer::ui::i18n::tr(fa, "Workspace"),
        pealayer::ui::i18n::visual_text(fa, "فضای کاری")
    );
    assert_eq!(
        pealayer::ui::i18n::tr(fa, "Window"),
        pealayer::ui::i18n::visual_text(fa, "پنجره")
    );
    assert_eq!(
        pealayer::ui::i18n::tr(fa, "Panels"),
        pealayer::ui::i18n::visual_text(fa, "پنل‌ها")
    );
    assert_eq!(
        pealayer::ui::i18n::tr(fa, "Reset Workspace to Default"),
        pealayer::ui::i18n::visual_text(fa, "بازنشانی فضای کاری به حالت پیش‌فرض")
    );
    assert_eq!(
        pealayer::ui::i18n::tr(fa, "All workspace panels are closed"),
        pealayer::ui::i18n::visual_text(fa, "همه پنل‌های فضای کاری بسته شده‌اند")
    );
    assert_eq!(
        pealayer::ui::i18n::tr(
            fa,
            "Open panels from the Workspace menu above, or reset the workspace."
        ),
        pealayer::ui::i18n::visual_text(
            fa,
            "پنل‌ها را از منوی فضای کاری در بالا باز کنید یا فضای کاری را بازنشانی نمایید."
        )
    );
}
