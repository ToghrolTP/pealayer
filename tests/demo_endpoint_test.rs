use pealayer::four_d::controller::{is_controller_endpoint, is_demo_endpoint, available_endpoints, DEMO_ENDPOINT};

#[test]
fn test_demo_endpoint_recognition() {
    assert_eq!(DEMO_ENDPOINT, "demo://virtual-board");
    assert!(is_demo_endpoint(DEMO_ENDPOINT));
    assert!(is_controller_endpoint(DEMO_ENDPOINT));

    let endpoints = available_endpoints();
    assert!(endpoints.iter().any(|ep| ep == DEMO_ENDPOINT), "available_endpoints must include DEMO_ENDPOINT");
}
