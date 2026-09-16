use pealayer::four_d::protocol::{Command, ProtocolError, cobs_decode, cobs_encode, crc8, parse_frame};

#[test]
fn test_crc8_known_vector() {
    let data = b"123456789";
    let checksum = crc8(data);
    assert_eq!(checksum, 0xA1); // standard Dallas/Maxim polynomial 0x31
}

#[test]
fn test_cobs_roundtrip() {
    let original = vec![0x01, 0x00, 0x02, 0x03, 0x00, 0x04];
    let encoded = cobs_encode(&original);
    assert!(!encoded.contains(&0x00));
    let decoded = cobs_decode(&encoded).expect("COBS decode failed");
    assert_eq!(original, decoded);
}

#[test]
fn test_cobs_edge_cases() {
    // Empty buffer
    let empty: Vec<u8> = vec![];
    let encoded_empty = cobs_encode(&empty);
    assert!(!encoded_empty.contains(&0x00));
    assert_eq!(cobs_decode(&encoded_empty).unwrap(), empty);

    // All zeroes
    let zeroes = vec![0x00, 0x00, 0x00];
    let encoded_zeroes = cobs_encode(&zeroes);
    assert!(!encoded_zeroes.contains(&0x00));
    assert_eq!(cobs_decode(&encoded_zeroes).unwrap(), zeroes);

    // No zeroes
    let no_zeroes = vec![0x10, 0x20, 0x30, 0x40];
    let encoded_no_zeroes = cobs_encode(&no_zeroes);
    assert!(!encoded_no_zeroes.contains(&0x00));
    assert_eq!(cobs_decode(&encoded_no_zeroes).unwrap(), no_zeroes);

    // Long block (> 254 non-zero bytes)
    let long_data: Vec<u8> = (1..=255).map(|b| (b % 250 + 1) as u8).collect();
    let encoded_long = cobs_encode(&long_data);
    assert!(!encoded_long.contains(&0x00));
    assert_eq!(cobs_decode(&encoded_long).unwrap(), long_data);
}

#[test]
fn test_cobs_decode_invalid() {
    // Zero byte inside encoded payload is invalid for COBS
    let invalid_zero = vec![0x01, 0x00];
    assert_eq!(cobs_decode(&invalid_zero), Err(ProtocolError::InvalidCobs));

    // Code points beyond slice bounds
    let truncated = vec![0x05, 0x01, 0x02]; // expects 4 data bytes, only 2 present
    assert_eq!(cobs_decode(&truncated), Err(ProtocolError::InvalidCobs));
}

#[test]
fn test_command_roundtrip_pwm_set() {
    let cmd = Command::PwmSet {
        channel: 2,
        value: 180,
    };
    let frame = cmd.to_frame();
    let parsed = parse_frame(&frame).expect("Failed to parse valid frame");
    assert_eq!(cmd, parsed);
}

#[test]
fn test_command_roundtrip_all_variants() {
    let commands = vec![
        Command::Ping,
        Command::RelaySet { id: 1, state: true },
        Command::RelaySet { id: 2, state: false },
        Command::PwmSet { channel: 0, value: 0 },
        Command::PwmSet { channel: 255, value: 255 },
        Command::AllOff,
    ];
    for cmd in commands {
        let frame = cmd.to_frame();
        // Test with trailing 0x00 delimiter
        let parsed = parse_frame(&frame).expect("Failed to parse frame with trailing zero");
        assert_eq!(cmd, parsed);
        // Test without trailing 0x00 delimiter
        if frame.last() == Some(&0x00) {
            let without_delim = &frame[..frame.len() - 1];
            let parsed_no_delim = parse_frame(without_delim).expect("Failed to parse frame without trailing zero");
            assert_eq!(cmd, parsed_no_delim);
        }
    }
}

#[test]
fn test_corrupt_crc_rejected() {
    let cmd = Command::PwmSet {
        channel: 2,
        value: 180,
    };
    let payload = cmd.to_payload();
    let original_crc = crc8(&payload);

    for bit in 0..8 {
        let corrupt_crc = original_crc ^ (1 << bit);
        let mut corrupted_data = payload.clone();
        corrupted_data.push(corrupt_crc);
        let mut frame = cobs_encode(&corrupted_data);
        frame.push(0x00);

        let result = parse_frame(&frame);
        assert_eq!(result, Err(ProtocolError::CrcMismatch));
    }
}

#[test]
fn test_corrupt_cobs_rejected() {
    // Malformed block offset that exceeds slice length
    let invalid_offset = vec![0x05, 0x01, 0x02, 0x00];
    assert_eq!(parse_frame(&invalid_offset), Err(ProtocolError::InvalidCobs));

    // Zero byte inside COBS body
    let zero_inside = vec![0x03, 0x00, 0x01, 0x00];
    assert_eq!(parse_frame(&zero_inside), Err(ProtocolError::InvalidCobs));
}

#[test]
fn test_frame_too_short() {
    // Empty frame
    assert_eq!(parse_frame(&[]), Err(ProtocolError::PacketTooShort));
    // Only delimiter
    assert_eq!(parse_frame(&[0x00]), Err(ProtocolError::PacketTooShort));

    // Decoded payload has length 1 (needs at least opcode + CRC = 2 bytes)
    let single_byte_encoded = cobs_encode(&[0x01]);
    let mut frame = single_byte_encoded;
    frame.push(0x00);
    assert_eq!(parse_frame(&frame), Err(ProtocolError::PacketTooShort));

    // RelaySet with incomplete payload (opcode 0x02 + id 1 + CRC, missing state)
    let incomplete_payload = vec![0x02, 0x01];
    let crc = crc8(&incomplete_payload);
    let mut data = incomplete_payload;
    data.push(crc);
    let mut frame = cobs_encode(&data);
    frame.push(0x00);
    assert_eq!(parse_frame(&frame), Err(ProtocolError::PacketTooShort));
}

#[test]
fn test_unknown_opcode() {
    let bad_payload = vec![0x99, 0x01, 0x02];
    let crc = crc8(&bad_payload);
    let mut data = bad_payload;
    data.push(crc);
    let mut frame = cobs_encode(&data);
    frame.push(0x00);
    assert_eq!(parse_frame(&frame), Err(ProtocolError::UnknownOpcode(0x99)));
}
