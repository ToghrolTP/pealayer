use pealayer::four_d::controller::{ControllerClient, DEFAULT_ENDPOINT};
use pealayer::four_d::embedded_host::{EmbeddedHost, EmbeddedHostOptions};
use serde_json::json;
use serde_json::Value;

fn live_test_enabled() -> bool {
    std::env::var_os("PEALAYER_PCCONTROLLER_LIVE_TEST").is_some()
}

#[test]
fn pccontroller_json_rpc_and_board_cobs_roundtrip() {
    if !live_test_enabled() {
        eprintln!("set PEALAYER_PCCONTROLLER_LIVE_TEST=1 to exercise the installed coordinator and board");
        return;
    }

    let endpoint = std::env::var("PEALAYER_PCCONTROLLER_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
    let mut client = ControllerClient::connect(&endpoint)
        .unwrap_or_else(|error| panic!("connect to installed PCController: {error}"));

    let snapshot = client
        .call("controller.snapshot", json!({}))
        .unwrap_or_else(|error| panic!("PCController snapshot over JSON-RPC: {error}"));
    assert!(snapshot.is_object(), "snapshot must be a JSON object: {snapshot}");

    // controller.status crosses the high-level JSON-RPC boundary, is serialized by
    // PCController as its native COBS/CRC request, and completes only after the
    // board response is decoded and correlated back to this client.
    let status = client
        .call("controller.status", json!({}))
        .unwrap_or_else(|error| panic!("PCController/board status roundtrip: {error}"));
    assert!(status.is_object(), "board status must be a JSON object: {status}");
}

#[test]
fn pccontroller_embedded_host_lifecycle_roundtrip() {
    let Some(library_path) = std::env::var_os("PEALAYER_PCCONTROLLER_LIBRARY") else {
        eprintln!(
            "set PEALAYER_PCCONTROLLER_LIBRARY to exercise the real PCController DLL/SO Host lifecycle"
        );
        return;
    };
    let data_root = std::env::temp_dir().join(format!(
        "pealayer-pccontroller-host-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&data_root).expect("create embedded Host test data root");
    let options = EmbeddedHostOptions {
        data_root: data_root.clone(),
        app_id: format!("pealayer.integration-test.{}", std::process::id()),
        app_name: "Pealayer integration test".to_string(),
        disable_auto_connect: true,
        disable_native: true,
        enable_integrations: false,
        http_address: None,
    };
    let mut host = EmbeddedHost::load_and_start(std::path::Path::new(&library_path), &options)
        .unwrap_or_else(|error| panic!("load/start real PCController embedded Host: {error}"));
    let ping = host
        .call("controller.ping", json!({}))
        .unwrap_or_else(|error| panic!("canonical in-process ping: {error}"));
    assert_eq!(ping["ok"], true, "unexpected in-process ping: {ping}");
    assert!(
        !host.endpoints().is_null(),
        "host_endpoints must return the listeners that actually started"
    );
    host.shutdown()
        .unwrap_or_else(|error| panic!("host_stop then host_destroy: {error}"));
    drop(host);
    let _ = std::fs::remove_dir_all(data_root);
}

#[test]
fn pccontroller_exact_target_action_push_ack_roundtrip() {
    if !live_test_enabled() {
        eprintln!("set PEALAYER_PCCONTROLLER_LIVE_TEST=1 to exercise app-action delivery");
        return;
    }

    let (mut socket, _) = tungstenite::connect("ws://127.0.0.1:8787/ipc")
        .expect("connect to PCController WebSocket");
    if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
    }
    let instance_id = format!("pealayer:integration-test-{}", std::process::id());
    let rpc = |id, method: &str, params: Value| {
        tungstenite::Message::Text(
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
                .to_string()
                .into(),
        )
    };
    socket
        .send(rpc(
            1,
            "controller.subscribe",
            json!({"topics":["state","events"],"after_id":0}),
        ))
        .unwrap();
    socket
        .send(rpc(
            2,
            "controller.app.instance.report",
            json!({
                "id": instance_id,
                "surface": "pealayer",
                "page": "player",
                "state": "active",
                "lease_seconds": 45,
                "values": {"app_actions":"pealayer.play"},
            }),
        ))
        .unwrap();
    loop {
        let tungstenite::Message::Text(text) = socket.read().expect("confirm Pealayer instance report")
        else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).unwrap();
        if value["id"] == 2 {
            assert!(value.get("error").is_none(), "instance report failed: {value}");
            break;
        }
    }

    let mut controller = ControllerClient::connect(DEFAULT_ENDPOINT).unwrap();
    let operation_id = format!("pealayer-integration-{}", std::process::id());
    let queued = controller
        .call(
            "controller.app.action",
            json!({
                "kind": "pealayer.play",
                "target": instance_id,
                "operation_id": operation_id,
                "timeout_ms": 5000,
            }),
        )
        .expect("queue exact-target Pealayer action");
    assert_eq!(queued["operation"]["operation_id"], operation_id);

    let pushed = loop {
        let message = socket.read().expect("read PCController action push");
        let tungstenite::Message::Text(text) = message else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).unwrap();
        if value["method"] == "controller.state"
            && value["params"]["kind"] == "pealayer.play"
            && value["params"]["metadata"]["target_instance"] == instance_id
        {
            break value["params"].clone();
        }
    };
    let metadata = &pushed["metadata"];
    socket
        .send(rpc(
            3,
            "controller.app.action.ack",
            json!({
                "operation_id": operation_id,
                "delivery_id": metadata["operation_delivery_id"],
                "instance_id": instance_id,
                "state": "applied",
            }),
        ))
        .unwrap();

    loop {
        let tungstenite::Message::Text(text) = socket.read().expect("read action ACK response")
        else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).unwrap();
        if value["id"] == 3 {
            assert!(value.get("error").is_none(), "ACK failed: {value}");
            break;
        }
    }
    let outcome = controller
        .call(
            "controller.app.action.outcome",
            json!({"operation_id": operation_id}),
        )
        .expect("read exact-target Pealayer action outcome");
    assert!(
        outcome.to_string().contains("applied"),
        "action outcome must be applied: {outcome}"
    );
}
