use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::thread;
use std::time::Duration;

use pealayer::four_d::curve::{AnalogTrack, Interpolation};
use pealayer::four_d::curve_record::RecordingSession;
use pealayer::four_d::protocol::{Command, decode_pccontroller_frame};

struct TestVirtualBoard {
    child: Child,
    pub port: u16,
}

impl TestVirtualBoard {
    fn spawn(port: u16) -> Self {
        let bin = "scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board";
        let child = ProcessCommand::new(bin)
            .args(["--bind", "127.0.0.1", "--port", &port.to_string(), "--no-stdin", "--quiet"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("VirtualBoard executable not found");
        thread::sleep(Duration::from_millis(150));
        Self { child, port }
    }
}

impl Drop for TestVirtualBoard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn test_live_recording_to_virtual_board_stream() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let vb = TestVirtualBoard::spawn(port);

    let mut stream = TcpStream::connect(("127.0.0.1", vb.port))
        .expect("Failed to connect to VirtualBoard");

    // 1. Simulate live motion capture session at 50Hz for 500ms (10 samples)
    let mut track = AnalogTrack::new("Wind Turbine", 0);
    track.armed = true;
    let track_id = track.id;

    let mut session = RecordingSession::new();
    let mut seq = 10u8;

    for i in 0..10 {
        let t = i * 50;
        let val = (i as f32) / 10.0;
        session.record_sample(track_id, t, val);

        // Hardware pass-through frame
        let byte_val = (val * 255.0).round() as u8;
        let cmd = Command::PwmSet { channel: track.channel, value: byte_val };
        let frame = cmd.to_pccontroller_frame(seq);
        seq = seq.wrapping_add(1);
        stream.write_all(&frame).expect("Write failed");
    }

    // 2. Punch-out & commit with RDP decimation
    session.commit_to_track(&mut track, 0.02, Interpolation::Smooth);

    // Linear ramp from 0 to 450ms should decimate to start & end keyframes
    assert!(track.keyframes.len() <= 3, "RDP should decimate linear ramp: got {}", track.keyframes.len());
    assert_eq!(track.keyframes.first().unwrap().time_ms, 0);
    assert_eq!(track.keyframes.last().unwrap().time_ms, 450);

    // 3. Read back last ACK from VirtualBoard
    let mut buf = [0u8; 1024];
    stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let n = stream.read(&mut buf).expect("Failed to read ACK from VirtualBoard");
    assert!(n > 0);

    // Verify frames from VirtualBoard and confirm receipt of PwmSet ACK
    let mut found_ack = false;
    let mut chunk = &buf[..n];
    while let Some(pos) = chunk.iter().position(|&b| b == 0) {
        let frame_slice = &chunk[..pos];
        if let Ok((opcode, _seq, payload)) = decode_pccontroller_frame(frame_slice) {
            if opcode == 0x80 {
                assert_eq!(payload[0], 0x11); // Confirms PwmSet opcode (0x11)
                assert_eq!(payload[1], 0x00); // Error code 0 = NoError
                found_ack = true;
                break;
            }
        }
        chunk = &chunk[pos + 1..];
    }
    // If not in first read chunk, read more until ACK received
    if !found_ack {
        while let Ok(len) = stream.read(&mut buf) {
            if len == 0 {
                break;
            }
            let mut subchunk = &buf[..len];
            while let Some(pos) = subchunk.iter().position(|&b| b == 0) {
                let frame_slice = &subchunk[..pos];
                if let Ok((opcode, _seq, payload)) = decode_pccontroller_frame(frame_slice) {
                    if opcode == 0x80 {
                        assert_eq!(payload[0], 0x11);
                        assert_eq!(payload[1], 0x00);
                        found_ack = true;
                        break;
                    }
                }
                subchunk = &subchunk[pos + 1..];
            }
            if found_ack {
                break;
            }
        }
    }
    assert!(found_ack, "Expected to decode PwmSet ACK from VirtualBoard");
}
