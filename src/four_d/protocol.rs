#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ProtocolError {
    InvalidCobs,
    CrcMismatch,
    PacketTooShort,
    UnknownOpcode(u8),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCobs => write!(f, "Invalid COBS framing"),
            Self::CrcMismatch => write!(f, "CRC-8 mismatch"),
            Self::PacketTooShort => write!(f, "Packet payload too short"),
            Self::UnknownOpcode(code) => write!(f, "Unknown opcode: {:#04x}", code),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Calculates CRC-8 checksum using the standard Dallas/Maxim 1-Wire polynomial
/// (0x31 reflected as 0x8C, initial value 0x00).
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if (crc & 0x01) != 0 {
                crc = (crc >> 1) ^ 0x8C;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/// Encodes a byte buffer using Consistent Overhead Byte Stuffing (COBS).
/// The resulting slice is guaranteed to contain no `0x00` bytes.
pub fn cobs_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() + input.len() / 254 + 2);
    let mut code_idx = 0;
    output.push(1);
    let mut code: u8 = 1;

    for &b in input {
        if b == 0 {
            output[code_idx] = code;
            code_idx = output.len();
            output.push(1);
            code = 1;
        } else {
            output.push(b);
            code += 1;
            if code == 0xFF {
                output[code_idx] = code;
                code_idx = output.len();
                output.push(1);
                code = 1;
            }
        }
    }
    output[code_idx] = code;
    output
}

/// Decodes a COBS-encoded byte buffer.
/// Returns `Err(ProtocolError::InvalidCobs)` if input contains `0x00` or malformed block offsets.
pub fn cobs_decode(input: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let mut output = Vec::new();
    let mut idx = 0;

    while idx < input.len() {
        let code = input[idx] as usize;
        if code == 0 {
            return Err(ProtocolError::InvalidCobs);
        }
        idx += 1;
        for _ in 1..code {
            if idx >= input.len() {
                return Err(ProtocolError::InvalidCobs);
            }
            output.push(input[idx]);
            idx += 1;
        }
        if code < 0xFF && idx < input.len() {
            output.push(0x00);
        }
    }
    Ok(output)
}
