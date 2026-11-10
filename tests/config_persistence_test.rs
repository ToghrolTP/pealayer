use pealayer::config::{AppConfig, StorageMode};
use std::path::PathBuf;

#[test]
fn test_app_config_persistence_cycle() {
    let temp_dir = std::env::temp_dir().join(format!("pealayer_e2e_cfg_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Session 1: User modifies settings
    let mut session1_config = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
    session1_config.volume = 125.0;
    session1_config.is_muted = true;
    session1_config.pin_controls = true;
    session1_config.show_remaining_time = true;
    session1_config
        .recent_media
        .push(PathBuf::from("/test/movie1.mkv"));
    session1_config
        .recent_media
        .push(PathBuf::from("/test/movie2.mp4"));

    session1_config
        .save_with_mode(StorageMode::Portable, &temp_dir)
        .unwrap();

    // Session 2: Fresh launch restores all settings
    let session2_config = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
    assert_eq!(session2_config.volume, 125.0);
    assert!(session2_config.is_muted);
    assert!(session2_config.pin_controls);
    assert!(session2_config.show_remaining_time);
    assert_eq!(session2_config.recent_media.len(), 2);
    assert_eq!(
        session2_config.recent_media[0],
        PathBuf::from("/test/movie1.mkv")
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
