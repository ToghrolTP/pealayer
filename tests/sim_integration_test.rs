use std::io::{Cursor, Read};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use pealayer::four_d::protocol::{Command, parse_frame};

/// Stream decoder that buffers incoming raw byte chunks from a stream
/// (e.g. serial port, PTY, socket, Cursor, or channel) and extracts
/// framed 4D cinema commands delimited by `0x00`.
///
/// If noise, corrupt bytes, or malformed packets precede or interrupt
/// valid frames, the decoder discards invalid segments and resynchronizes
/// on the next valid frame delimiter.
#[derive(Debug, Default)]
pub struct StreamDecoder {
    buffer: Vec<u8>,
}

impl StreamDecoder {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Appends incoming raw bytes into the internal accumulation buffer.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Reads available bytes from any `std::io::Read` stream and feeds them into the decoder.
    pub fn read_from<R: Read>(&mut self, reader: &mut R) -> std::io::Result<usize> {
        let mut temp_buf = [0u8; 64];
        let bytes_read = reader.read(&mut temp_buf)?;
        if bytes_read > 0 {
            self.feed(&temp_buf[..bytes_read]);
        }
        Ok(bytes_read)
    }

    /// Attempts to extract the next valid `Command` from the accumulated buffer.
    ///
    /// Scans for delimiter `0x00`. When encountered:
    /// - Drains the byte slice up to and including the delimiter.
    /// - Discards empty delimiters (consecutive `0x00` bytes).
    /// - Tries to parse the frame via `parse_frame`.
    /// - If parsing fails (noise, invalid COBS, CRC mismatch, unknown opcode),
    ///   the corrupt frame is discarded and the decoder continues scanning
    ///   for the next delimited frame.
    /// - Returns `Some(Command)` as soon as a valid frame is parsed,
    ///   or `None` if no more valid complete frames are present.
    pub fn next_command(&mut self) -> Option<Command> {
        while let Some(pos) = self.buffer.iter().position(|&b| b == 0x00) {
            let frame: Vec<u8> = self.buffer.drain(..=pos).collect();
            // Discard empty delimiters
            if frame.len() <= 1 {
                continue;
            }
            if let Ok(cmd) = parse_frame(&frame) {
                return Some(cmd);
            }
            // Corrupt frame discarded; continue looking for next frame
        }
        None
    }

    /// Drains and decodes all currently available commands in the buffer.
    pub fn decode_all(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        while let Some(cmd) = self.next_command() {
            commands.push(cmd);
        }
        commands
    }
}

#[test]
fn test_continuous_stream_cursor() {
    // 1. Target sequence of 4 commands specified in requirement
    let expected_commands = vec![
        Command::Ping,
        Command::RelaySet { id: 1, state: true },
        Command::PwmSet {
            channel: 2,
            value: 200,
        },
        Command::AllOff,
    ];

    // 2. Serialize all commands into a continuous framed byte stream separated by 0x00
    let mut stream_bytes = Vec::new();
    for cmd in &expected_commands {
        stream_bytes.extend_from_slice(&cmd.to_frame());
    }

    // Verify stream ends with delimiter 0x00
    assert_eq!(stream_bytes.last(), Some(&0x00));

    // 3. Stream framed bytes through a buffer (std::io::Cursor)
    let mut cursor = Cursor::new(stream_bytes);
    let mut decoder = StreamDecoder::new();

    // Read through Cursor in small fragmented chunks (e.g. 3 bytes at a time)
    // to simulate real serial / PTY stream fragmentation
    let mut parsed_commands = Vec::new();
    let mut chunk = [0u8; 3];
    loop {
        let bytes_read = cursor.read(&mut chunk).expect("Cursor read failed");
        if bytes_read == 0 {
            break;
        }
        decoder.feed(&chunk[..bytes_read]);
        while let Some(cmd) = decoder.next_command() {
            parsed_commands.push(cmd);
        }
    }

    // 4. Verify all 4 commands were extracted one by one in exact sequence
    assert_eq!(parsed_commands.len(), 4);
    assert_eq!(parsed_commands, expected_commands);
}

#[test]
fn test_continuous_stream_channel() {
    let expected_commands = vec![
        Command::Ping,
        Command::RelaySet { id: 1, state: true },
        Command::PwmSet {
            channel: 2,
            value: 200,
        },
        Command::AllOff,
    ];

    let mut stream_bytes = Vec::new();
    for cmd in &expected_commands {
        stream_bytes.extend_from_slice(&cmd.to_frame());
    }

    // Stream framed bytes through a multi-producer single-consumer channel
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let producer = thread::spawn(move || {
        // Send bytes in irregular chunk sizes
        let chunk_sizes = [2, 5, 1, 4, 3, 2];
        let mut offset = 0;
        let mut idx = 0;
        while offset < stream_bytes.len() {
            let size = chunk_sizes[idx % chunk_sizes.len()];
            let end = (offset + size).min(stream_bytes.len());
            tx.send(stream_bytes[offset..end].to_vec()).unwrap();
            offset = end;
            idx += 1;
            thread::sleep(Duration::from_millis(1));
        }
    });

    let mut decoder = StreamDecoder::new();
    let mut parsed_commands = Vec::new();

    while let Ok(chunk) = rx.recv() {
        decoder.feed(&chunk);
        while let Some(cmd) = decoder.next_command() {
            parsed_commands.push(cmd);
        }
    }

    producer.join().unwrap();

    assert_eq!(parsed_commands.len(), 4);
    assert_eq!(parsed_commands, expected_commands);
}

#[test]
fn test_noisy_stream_recovery_garbage_prefix() {
    // 1. Random garbage bytes before a valid framed Command::Ping
    let mut noisy_stream = Vec::new();

    // Insert random noise bytes terminated by a delimiter (simulating noise burst on line)
    let garbage_frame_1 = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x00];
    let garbage_frame_2 = vec![0xFF, 0xAA, 0x55, 0x12, 0x34, 0x00];
    noisy_stream.extend_from_slice(&garbage_frame_1);
    noisy_stream.extend_from_slice(&garbage_frame_2);

    // Followed by a valid framed Command::Ping
    let ping_cmd = Command::Ping;
    noisy_stream.extend_from_slice(&ping_cmd.to_frame());

    // 2. Stream through Cursor and verify decoder cleanly recovers and parses Ping
    let mut cursor = Cursor::new(noisy_stream);
    let mut decoder = StreamDecoder::new();

    while decoder.read_from(&mut cursor).unwrap() > 0 {}

    let parsed = decoder.decode_all();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0], Command::Ping);
}

#[test]
fn test_noisy_stream_interleaved_corruption() {
    let mut stream = Vec::new();

    // 1. Valid Ping
    stream.extend_from_slice(&Command::Ping.to_frame());

    // 2. Corrupted CRC frame (valid COBS structure, invalid CRC)
    let bad_crc_frame = vec![0x03, 0x02, 0x01, 0xFF, 0x00];
    stream.extend_from_slice(&bad_crc_frame);

    // 3. Valid RelaySet
    let relay_cmd = Command::RelaySet { id: 1, state: true };
    stream.extend_from_slice(&relay_cmd.to_frame());

    // 4. Invalid COBS frame (offset points out of bounds)
    let bad_cobs_frame = vec![0x09, 0x01, 0x02, 0x00];
    stream.extend_from_slice(&bad_cobs_frame);

    // 5. Valid PwmSet
    let pwm_cmd = Command::PwmSet {
        channel: 2,
        value: 200,
    };
    stream.extend_from_slice(&pwm_cmd.to_frame());

    // 6. Unknown opcode frame
    let unknown_opcode_frame = vec![0x04, 0x99, 0x01, 0x02, 0x00];
    stream.extend_from_slice(&unknown_opcode_frame);

    // 7. Valid AllOff
    stream.extend_from_slice(&Command::AllOff.to_frame());

    // Process stream through decoder
    let mut cursor = Cursor::new(stream);
    let mut decoder = StreamDecoder::new();

    while decoder.read_from(&mut cursor).unwrap() > 0 {}

    let parsed = decoder.decode_all();
    assert_eq!(
        parsed,
        vec![
            Command::Ping,
            Command::RelaySet { id: 1, state: true },
            Command::PwmSet {
                channel: 2,
                value: 200,
            },
            Command::AllOff,
        ]
    );
}

#[test]
fn test_empty_and_consecutive_delimiters_ignored() {
    let mut stream = Vec::new();

    // Consecutive 0x00 delimiters (e.g. idle line / zero padding)
    stream.extend_from_slice(&[0x00, 0x00, 0x00]);

    // Followed by valid command
    stream.extend_from_slice(&Command::Ping.to_frame());

    // More consecutive delimiters
    stream.extend_from_slice(&[0x00, 0x00]);

    // Followed by another valid command
    stream.extend_from_slice(&Command::AllOff.to_frame());

    // Trailing delimiters
    stream.push(0x00);

    let mut cursor = Cursor::new(stream);
    let mut decoder = StreamDecoder::new();

    while decoder.read_from(&mut cursor).unwrap() > 0 {}

    let parsed = decoder.decode_all();
    assert_eq!(parsed, vec![Command::Ping, Command::AllOff]);
}

#[test]
fn test_fragmented_byte_by_byte_stream() {
    let cmd = Command::RelaySet { id: 4, state: true };
    let frame = cmd.to_frame();

    let mut decoder = StreamDecoder::new();

    // Feed all bytes except the last delimiter byte
    for &byte in &frame[..frame.len() - 1] {
        decoder.feed(&[byte]);
        assert_eq!(decoder.next_command(), None, "Should not emit command before delimiter");
    }

    // Feed the trailing 0x00 delimiter
    decoder.feed(&[*frame.last().unwrap()]);
    assert_eq!(
        decoder.next_command(),
        Some(cmd),
        "Should emit command immediately once delimiter arrives"
    );
    assert_eq!(decoder.next_command(), None);
}
