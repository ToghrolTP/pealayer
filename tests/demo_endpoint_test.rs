use pealayer::four_d::controller::{is_controller_endpoint, is_demo_endpoint, DEMO_ENDPOINT};

#[test]
fn test_demo_endpoint_recognition() {
    assert_eq!(DEMO_ENDPOINT, "demo://virtual-board");
    assert!(is_demo_endpoint(DEMO_ENDPOINT));
    assert!(!is_controller_endpoint(DEMO_ENDPOINT));

    assert!(!is_demo_endpoint("pccontroller://127.0.0.1:8787"));
    assert!(!is_demo_endpoint("direct:/dev/ttyACM0"));
}
