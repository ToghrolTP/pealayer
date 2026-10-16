use pealayer::four_d::controller::{DEMO_ENDPOINT, is_demo_endpoint};

#[test]
fn test_demo_endpoint_formatting_and_identity() {
    assert!(is_demo_endpoint(DEMO_ENDPOINT));
    assert!(!pealayer::four_d::controller::is_controller_endpoint(DEMO_ENDPOINT));
}

#[test]
fn test_demo_timeline_and_engine_playback_readiness() {
    let timeline = pealayer::four_d::demo_timeline::build_demo_showcase_timeline();
    assert!(!timeline.instances.is_empty(), "showcase timeline must have instances loaded");
    assert!(!timeline.templates.is_empty(), "showcase timeline must have templates loaded");

    let engine = pealayer::four_d::engine::spawn_engine();
    *engine.serial_port.lock().unwrap() = DEMO_ENDPOINT.to_string();
    // In demo mode, engine must not require coordinator prepared timeline arming
    assert!(!engine.request_prepared_play());
}
