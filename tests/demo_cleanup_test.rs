use std::fs;
use pealayer::four_d::demo_cleanup::{cleanup_stale_demo_resources, is_process_alive};

#[test]
fn test_is_process_alive() {
    let current_pid = std::process::id();
    assert!(is_process_alive(current_pid));
    // An absurdly high PID that is virtually guaranteed not to exist
    assert!(!is_process_alive(999_999_999));
}

#[test]
fn test_cleanup_stale_resources() {
    let temp = std::env::temp_dir();
    let dead_dir = temp.join("pealayer-demo-coordinator-999999999-123456789");
    let dead_file = temp.join("virtual-mcu-eeprom-999999999-123456789.bin");

    let _ = fs::create_dir_all(&dead_dir);
    let _ = fs::write(&dead_file, b"test-stale-eeprom");

    assert!(dead_dir.exists());
    assert!(dead_file.exists());

    cleanup_stale_demo_resources();

    assert!(!dead_dir.exists(), "stale coordinator temp dir should have been removed");
    assert!(!dead_file.exists(), "stale eeprom temp file should have been removed");
}
