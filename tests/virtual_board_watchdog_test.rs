use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use pealayer::four_d::protocol::{decode_pccontroller_frame, encode_pccontroller_frame};
use pealayer::four_d::virtual_board::VirtualBoardSession;

fn read_one_frame(stream: &mut TcpStream) -> (u8, u8, Vec<u8>) {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).expect("read byte from virtual board");
        buf.push(byte[0]);
        if byte[0] == 0x00 {
            break;
        }
    }
    decode_pccontroller_frame(&buf).expect("decode wire frame")
}

fn read_response(stream: &mut TcpStream, target_opcode: u8, target_seq: u8) -> (u8, u8, Vec<u8>) {
    loop {
        let (op, seq, payload) = read_one_frame(stream);
        if op == target_opcode && seq == target_seq {
            return (op, seq, payload);
        }
    }
}

#[test]
fn test_virtual_board_autonomous_watchdog_relay_cutoff() {
    let bin = VirtualBoardSession::resolve_binary_path();
    eprintln!("RESOLVED BINARY: {:?}", bin);
    if bin.is_none() {
        eprintln!("Skipping test: virtual_board binary not found");
        return;
    }

    let session = VirtualBoardSession::spawn().expect("spawn virtual board");
    let mut stream = TcpStream::connect(("127.0.0.1", session.port)).expect("connect to virtual board");
    stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();

    // Drain initial connected frames (HelloResponse 0x81 and initial StatusResponse 0x90)
    let (hello_op, _seq, _payload) = read_response(&mut stream, 0x81, 0);
    assert_eq!(hello_op, 0x81, "expected initial HelloResponse (0x81)");

    // 1. Send RelaySet frame: Turn on relay 0
    let relay_on = encode_pccontroller_frame(0x31, 1, &[0, 1]);
    stream.write_all(&relay_on).expect("send relay on");

    // Read ACK response (0x80, seq 1)
    let (opcode, _seq, _payload) = read_response(&mut stream, 0x80, 1);
    assert_eq!(opcode, 0x80, "expected ACK opcode (0x80) for RelaySet");

    // 2. Query status immediately; relay 0 must be active
    let get_status = encode_pccontroller_frame(0x02, 2, &[]);
    stream.write_all(&get_status).expect("send get status");
    let (opcode, _seq, payload) = read_response(&mut stream, 0x90, 2);
    assert_eq!(opcode, 0x90, "expected StatusResponse opcode (0x90)");
    assert!(payload.len() >= 29, "status payload must contain relay mask at index 28");
    let relay_mask = payload[28];
    assert_ne!(relay_mask & 1, 0, "relay 0 must be active");

    // 3. Go silent for >5.0 seconds (watchdog timeout is 5.0s)
    thread::sleep(Duration::from_millis(5200));

    // 4. Query status again; the watchdog must have asserted Safe Torque Off
    let get_status = encode_pccontroller_frame(0x02, 3, &[]);
    stream.write_all(&get_status).expect("send get status");
    let (opcode, _seq, payload) = read_response(&mut stream, 0x90, 3);
    assert_eq!(opcode, 0x90, "expected StatusResponse opcode (0x90)");
    let relay_mask = payload[28];
    let flags = (payload[24] as u16) | ((payload[25] as u16) << 8);
    eprintln!("WATCHDOG STATUS PAYLOAD len={}: relays={relay_mask}, flags={flags:#06x}", payload.len());
    assert_eq!(relay_mask, 0, "watchdog must have de-energized all relays after 5.2s inactivity");

    drop(stream);
    drop(session);
}
