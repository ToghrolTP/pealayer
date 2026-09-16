use pealayer::four_d::protocol::{ProtocolError, cobs_decode, cobs_encode, crc8};

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
