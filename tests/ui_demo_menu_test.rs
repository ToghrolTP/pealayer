use pealayer::four_d::controller::{DEMO_ENDPOINT, is_demo_endpoint};

#[test]
fn test_demo_endpoint_formatting_and_identity() {
    assert!(is_demo_endpoint(DEMO_ENDPOINT));
}
