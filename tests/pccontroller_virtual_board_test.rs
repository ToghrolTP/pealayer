use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::thread;
use std::time::Duration;

use pealayer::four_d::protocol::{Command, decode_pccontroller_frame, encode_pccontroller_frame};
use pealayer::four_d::controller::ControllerClient;
use pealayer::four_d::embedded_host::{EmbeddedHost, EmbeddedHostOptions};
use serde_json::json;

struct VirtualBoardProcess {
    child: Child,
    #[allow(dead_code)]
    pub port: u16,
}

impl VirtualBoardProcess {
    fn try_spawn() -> Option<Self> {
        let bin_path = std::env::var_os("PEALAYER_VIRTUAL_BOARD_BINARY")?;
        assert!(std::path::Path::new(&bin_path).is_file(), "configured shared-firmware VirtualBoard binary does not exist");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve test port");
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let child = ProcessCommand::new(bin_path)
            .args([
                "--bind",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--no-stdin",
                "--quiet",
                "--memory",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start explicitly configured shared firmware");

        Some(Self { child, port })
    }

    fn connect(&self) -> TcpStream {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(stream) = TcpStream::connect(("127.0.0.1", self.port)) { return stream; }
            assert!(std::time::Instant::now() < deadline, "configured VirtualBoard never became ready");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for VirtualBoardProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_one_frame(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Failed to set read timeout");
    let mut frame = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        match stream.read_exact(&mut buf) {
            Ok(_) => {
                if buf[0] == 0x00 {
                    if !frame.is_empty() {
                        frame.push(0x00);
                        return frame;
                    }
                } else {
                    frame.push(buf[0]);
                }
            }
            Err(e) => panic!("Timed out or failed reading from VirtualBoard: {}", e),
        }
    }
}

fn read_response(stream: &mut TcpStream, target_opcode: u8, target_seq: u8) -> (u8, u8, Vec<u8>) {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let frame_raw = read_one_frame(stream);
        let (opcode, seq, payload) = decode_pccontroller_frame(&frame_raw)
            .expect("Failed to decode frame from VirtualBoard");
        if opcode == target_opcode && seq == target_seq {
            return (opcode, seq, payload);
        }
    }
    panic!(
        "Timeout waiting for opcode {:#04x} with seq {}",
        target_opcode, target_seq
    );
}

#[test]
fn test_pccontroller_virtual_board_live_interaction() {
    let vb = match VirtualBoardProcess::try_spawn() {
        Some(vb) => vb,
        None => {
            eprintln!("set PEALAYER_VIRTUAL_BOARD_BINARY to test the actual shared production firmware");
            return;
        }
    };

    // Connect to VirtualBoard over TCP loopback
    let mut stream = vb.connect();

    // 1. Send Hello handshake (Opcode 0x01)
    let hello_frame = encode_pccontroller_frame(0x01, 1, &[]);
    stream
        .write_all(&hello_frame)
        .expect("Failed to send Hello frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x81, 1);
    assert_eq!(opcode, 0x81); // 0x81 = HelloResponse
    assert_eq!(seq, 1);
    assert_eq!(payload.len(), 16);
    assert_ne!(payload[15] & 128, 0, "refuse physical/non-native actuation");

    // 2. Send RelaySet (Relay 1 ON)
    let relay_cmd = Command::RelaySet { id: 1, state: true };
    let relay_frame = relay_cmd.to_pccontroller_frame(2);
    stream
        .write_all(&relay_frame)
        .expect("Failed to send RelaySet frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x80, 2);
    assert_eq!(opcode, 0x80); // 0x80 = Ack
    assert_eq!(seq, 2);
    assert_eq!(payload[0], 0x31); // Confirms request opcode (RelaySet = 0x31)
    assert_eq!(payload[1], 0x00); // Error code 0 = NoError

    // 3. Send PwmSet (Channel 1, duty 200)
    let pwm_cmd = Command::PwmSet {
        channel: 1,
        value: 200,
    };
    let pwm_frame = pwm_cmd.to_pccontroller_frame(3);
    stream
        .write_all(&pwm_frame)
        .expect("Failed to send PwmSet frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x80, 3);
    assert_eq!(opcode, 0x80); // Ack
    assert_eq!(seq, 3);
    assert_eq!(payload[0], 0x11); // PwmSet = 0x11
    assert_eq!(payload[1], 0x00); // NoError

    // 4. Send RelayAllOff (Clean emergency / pause shutdown)
    let all_off_cmd = Command::AllOff;
    let all_off_frame = all_off_cmd.to_pccontroller_frame(4);
    stream
        .write_all(&all_off_frame)
        .expect("Failed to send AllOff frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x80, 4);
    assert_eq!(opcode, 0x80); // Ack
    assert_eq!(seq, 4);
    assert_eq!(payload[0], 0x33); // RelayAllOff = 0x33
    assert_eq!(payload[1], 0x00); // NoError
}

#[test]
fn pealayer_coordinator_catalog_and_outputs_without_hardware() {
    let Some(library) = std::env::var_os("PEALAYER_PCCONTROLLER_LIBRARY") else {
        eprintln!("set PEALAYER_PCCONTROLLER_LIBRARY and PEALAYER_VIRTUAL_BOARD_BINARY for the full no-hardware stack");
        return;
    };
    let Some(vb) = VirtualBoardProcess::try_spawn() else {
        panic!("PEALAYER_VIRTUAL_BOARD_BINARY is required when opting into this integration");
    };
    // Readiness against our child only; no system discovery or physical endpoint.
    drop(vb.connect());
    let run_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("pealayer-virtual-development-{}-{run_id}", std::process::id()));
    assert!(!root.exists(), "refuse to overwrite previous test evidence");
    let options = EmbeddedHostOptions {
        data_root: root.clone(), app_id: "pealayer.virtual-test".into(),
        app_name: "Pealayer VirtualBoard test".into(), disable_auto_connect: true,
        disable_native: true, enable_integrations: false,
        http_address: Some("127.0.0.1:0".into()),
    };
    let mut host = EmbeddedHost::load_and_start(std::path::Path::new(&library), &options).expect("isolated packaged coordinator");
    let address = host.endpoints().as_array().unwrap()[0]["address"].as_str().unwrap().to_owned();
    let mut client = ControllerClient::connect(&address).expect("Pealayer current TCP coordinator client");
    let connected = client.call("controller.connect", json!({"port": format!("tcp://127.0.0.1:{}", vb.port)})).expect("attach own native child");
    assert_eq!(connected["connected"], true);
    assert_ne!(connected["hello"]["build_features"].as_u64().unwrap() & 128, 0, "refuse non-native actuation");
    let profile = client.call("controller.board_profile.get", json!({})).unwrap();
    client.call("controller.board_profile.update", json!({"key":"development", "mode":"cinema-seat-motion", "expected_revision": profile["revision"]})).expect("explicit cinema authoring profile");
    let hardware = client.hardware_capabilities().expect("real Pealayer catalog reconciliation");
    assert!(hardware.board_connected);
    assert!(hardware.supports_segment_display && hardware.supports_addressable_led);
    assert!(hardware.settings.is_some() && hardware.front_panel.is_some(), "native caches must bootstrap without a controller GUI");
    assert!(hardware.controls.iter().any(|control| control.key == "seat.a"));
    assert!(hardware.controls.iter().any(|control| control.key == "seat.b"));
    assert!(hardware.telemetry.bus_mv.is_none(), "missing power must not become synthetic voltage");
    assert_eq!(hardware.peripheral_names.get("sensor.temperature-led").map(String::as_str), Some("Native temperature 1"));
    client.call("controller.command.execute", json!({"command":"relay 5 on"})).expect("native relay dispatch");
    let status = client.call("controller.status", json!({})).unwrap();
    assert_ne!(status["active_relays"].as_u64().unwrap() & 16, 0);
    let pwm = client.call("controller.pwm.set", json!({"channel":0,"raw_value":1024})).expect("native PWM dispatch");
    assert!(pwm.to_string().contains("1024"), "PWM must be observed, not just queued");
    client.call("controller.display.send", json!({"target":"segments","text":"1234","duration_ms":5000})).expect("real native segment presentation");
    // Production boot/door notices keep their priority on the native target;
    // observe scheduled presentation rather than assuming an ACK painted it.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let panel = loop {
        let panel = client.call("controller.front_panel", json!({})).unwrap();
        if panel["raw_segments"] == json!([6,91,79,102]) { break panel; }
        assert!(std::time::Instant::now() < deadline, "native display never presented requested cells: {panel}");
        thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(panel["raw_segments"], json!([6,91,79,102]));
    assert_eq!(panel["tm1637_detected"], true);
    client.call("controller.command.execute", json!({"command":"relay off"})).unwrap();
    client.call("controller.pwm.off", json!({})).unwrap();
    let stopped = client.call("controller.status", json!({})).unwrap();
    assert_eq!(stopped["active_relays"], 0);
    assert!(client.hardware_capabilities().unwrap().board_connected);
    drop(client);
    host.shutdown().expect("stop own coordinator");
    drop(host);
    std::fs::remove_dir_all(root).expect("remove only own isolated test state");
}
