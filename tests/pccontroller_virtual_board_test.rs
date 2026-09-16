use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::thread;
use std::time::Duration;

use pealayer::four_d::protocol::{
    Command, decode_pccontroller_frame, encode_pccontroller_frame,
};

struct VirtualBoardProcess {
    child: Child,
    #[allow(dead_code)]
    pub port: u16,
}

impl VirtualBoardProcess {
    fn spawn(port: u16) -> Self {
        let bin_path = "scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board";
        let child = ProcessCommand::new(bin_path)
            .args(["--bind", "127.0.0.1", "--port", &port.to_string(), "--no-stdin", "--quiet"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn virtual_board executable. Has it been built?");

        // Give the virtual board a moment to start listening
        thread::sleep(Duration::from_millis(150));

        Self { child, port }
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

fn read_response(
    stream: &mut TcpStream,
    target_opcode: u8,
    target_seq: u8,
) -> (u8, u8, Vec<u8>) {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let frame_raw = read_one_frame(stream);
        let (opcode, seq, payload) =
            decode_pccontroller_frame(&frame_raw).expect("Failed to decode frame from VirtualBoard");
        if opcode == target_opcode && seq == target_seq {
            return (opcode, seq, payload);
        }
    }
    panic!("Timeout waiting for opcode {:#04x} with seq {}", target_opcode, target_seq);
}

#[test]
fn test_pccontroller_virtual_board_live_interaction() {
    let port = 8792;
    let _vb = VirtualBoardProcess::spawn(port);

    // Connect to VirtualBoard over TCP loopback
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .expect("Failed to connect to VirtualBoard TCP server");

    // 1. Send Hello handshake (Opcode 0x01)
    let hello_frame = encode_pccontroller_frame(0x01, 1, &[]);
    stream.write_all(&hello_frame).expect("Failed to send Hello frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x81, 1);
    assert_eq!(opcode, 0x81); // 0x81 = HelloResponse
    assert_eq!(seq, 1);
    assert!(!payload.is_empty()); // Returns build timestamp / version

    // 2. Send RelaySet (Relay 1 ON)
    let relay_cmd = Command::RelaySet { id: 1, state: true };
    let relay_frame = relay_cmd.to_pccontroller_frame(2);
    stream.write_all(&relay_frame).expect("Failed to send RelaySet frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x80, 2);
    assert_eq!(opcode, 0x80); // 0x80 = Ack
    assert_eq!(seq, 2);
    assert_eq!(payload[0], 0x31); // Confirms request opcode (RelaySet = 0x31)
    assert_eq!(payload[1], 0x00); // Error code 0 = NoError

    // 3. Send PwmSet (Channel 1, duty 200)
    let pwm_cmd = Command::PwmSet { channel: 1, value: 200 };
    let pwm_frame = pwm_cmd.to_pccontroller_frame(3);
    stream.write_all(&pwm_frame).expect("Failed to send PwmSet frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x80, 3);
    assert_eq!(opcode, 0x80); // Ack
    assert_eq!(seq, 3);
    assert_eq!(payload[0], 0x11); // PwmSet = 0x11
    assert_eq!(payload[1], 0x00); // NoError

    // 4. Send RelayAllOff (Clean emergency / pause shutdown)
    let all_off_cmd = Command::AllOff;
    let all_off_frame = all_off_cmd.to_pccontroller_frame(4);
    stream.write_all(&all_off_frame).expect("Failed to send AllOff frame");

    let (opcode, seq, payload) = read_response(&mut stream, 0x80, 4);
    assert_eq!(opcode, 0x80); // Ack
    assert_eq!(seq, 4);
    assert_eq!(payload[0], 0x33); // RelayAllOff = 0x33
    assert_eq!(payload[1], 0x00); // NoError
}
