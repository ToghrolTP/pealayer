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

/// Calculates CRC-8/ATM checksum using polynomial 0x07 (initial value 0x00).
/// Used by PCController wire contract.
pub fn crc8_atm(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
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
    let mut output = Vec::with_capacity(input.len());
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
            if input[idx] == 0 {
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

/// Strongly typed commands for 4D cinema hardware control.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Command {
    Ping,
    RelaySet { id: u8, state: bool },
    PwmSet { channel: u8, value: u8 },
    AllOff,
}

impl Command {
    /// Serializes command into raw payload bytes according to the wire specification:
    /// - Opcode 0x01: Ping (1 byte: `[0x01]`)
    /// - Opcode 0x02: RelaySet { id, state } (3 bytes: `[0x02, id, state: 0/1]`)
    /// - Opcode 0x03: PwmSet { channel, value } (3 bytes: `[0x03, channel, value: 0-255]`)
    /// - Opcode 0x04: AllOff (1 byte: `[0x04]`)
    pub fn to_payload(&self) -> Vec<u8> {
        match self {
            Self::Ping => vec![0x01],
            Self::RelaySet { id, state } => vec![0x02, *id, if *state { 1 } else { 0 }],
            Self::PwmSet { channel, value } => vec![0x03, *channel, *value],
            Self::AllOff => vec![0x04],
        }
    }

    /// Serializes command into framed bytes:
    /// 1. `payload` + `crc8(payload)`
    /// 2. `cobs_encode(payload_with_crc)`
    /// 3. Delimiter `0x00` appended at the end
    pub fn to_frame(&self) -> Vec<u8> {
        let mut data = self.to_payload();
        let crc = crc8(&data);
        data.push(crc);
        let mut framed = cobs_encode(&data);
        framed.push(0x00);
        framed
    }
}

/// Parses and validates a received frame into a typed `Command`:
/// 1. Strips trailing delimiter `0x00` if present.
/// 2. Decodes COBS framing.
/// 3. Validates length >= 2 bytes (payload + CRC-8).
/// 4. Verifies CRC-8 checksum against the payload.
/// 5. Parses opcode and deserializes command fields.
pub fn parse_frame(frame: &[u8]) -> Result<Command, ProtocolError> {
    let frame_data = if frame.last() == Some(&0x00) {
        &frame[..frame.len() - 1]
    } else {
        frame
    };

    let decoded = cobs_decode(frame_data)?;
    if decoded.len() < 2 {
        return Err(ProtocolError::PacketTooShort);
    }

    let payload = &decoded[..decoded.len() - 1];
    let expected_crc = decoded[decoded.len() - 1];

    if crc8(payload) != expected_crc {
        return Err(ProtocolError::CrcMismatch);
    }

    let opcode = payload[0];
    match opcode {
        0x01 => Ok(Command::Ping),
        0x02 => {
            if payload.len() < 3 {
                return Err(ProtocolError::PacketTooShort);
            }
            Ok(Command::RelaySet {
                id: payload[1],
                state: payload[2] != 0,
            })
        }
        0x03 => {
            if payload.len() < 3 {
                return Err(ProtocolError::PacketTooShort);
            }
            Ok(Command::PwmSet {
                channel: payload[1],
                value: payload[2],
            })
        }
        0x04 => Ok(Command::AllOff),
        _ => Err(ProtocolError::UnknownOpcode(opcode)),
    }
}

pub const PCCONTROLLER_MAGIC: u8 = 0xA5;
pub const PCCONTROLLER_REVISION: u8 = 0x01;

impl Command {
    /// Serializes command into PCController wire contract frame:
    /// `[0xA5, 0x01, opcode, sequence, payload_len, ...payload, crc8_atm]`
    /// followed by COBS encoding and delimiter `0x00`.
    pub fn to_pccontroller_frame(&self, sequence: u8) -> Vec<u8> {
        match self {
            Self::Ping => encode_pccontroller_frame(0x01, sequence, &[]),
            Self::RelaySet { id, state } => {
                encode_pccontroller_frame(0x31, sequence, &[*id, if *state { 1 } else { 0 }])
            }
            Self::PwmSet { channel, value } => {
                let pwm_12bit = ((*value as u32 * 4095) / 255) as u16;
                let val_bytes = pwm_12bit.to_le_bytes();
                encode_pccontroller_frame(0x11, sequence, &[*channel, val_bytes[0], val_bytes[1]])
            }
            Self::AllOff => encode_pccontroller_frame(0x33, sequence, &[]),
        }
    }
}

/// Encodes a PCController wire envelope:
/// `[0xA5, 0x01, opcode, sequence, payload_len, ...payload, crc8_atm]`
/// followed by COBS framing and trailing `0x00` delimiter.
pub fn encode_pccontroller_frame(opcode: u8, sequence: u8, payload: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(6 + payload.len());
    raw.push(PCCONTROLLER_MAGIC);
    raw.push(PCCONTROLLER_REVISION);
    raw.push(opcode);
    raw.push(sequence);
    raw.push(payload.len() as u8);
    raw.extend_from_slice(payload);
    let crc = crc8_atm(&raw);
    raw.push(crc);
    let mut framed = cobs_encode(&raw);
    framed.push(0x00);
    framed
}

/// Decodes and validates a PCController wire frame.
/// Returns `Ok((opcode, sequence, payload))` on success.
pub fn decode_pccontroller_frame(frame: &[u8]) -> Result<(u8, u8, Vec<u8>), ProtocolError> {
    let frame_data = if frame.last() == Some(&0x00) {
        &frame[..frame.len() - 1]
    } else {
        frame
    };
    let decoded = cobs_decode(frame_data)?;
    if decoded.len() < 6 {
        return Err(ProtocolError::PacketTooShort);
    }
    if decoded[0] != PCCONTROLLER_MAGIC {
        return Err(ProtocolError::InvalidCobs);
    }
    let payload_len = decoded[4] as usize;
    if decoded.len() != 6 + payload_len {
        return Err(ProtocolError::PacketTooShort);
    }
    let (body, crc_slice) = decoded.split_at(decoded.len() - 1);
    if crc8_atm(body) != crc_slice[0] {
        return Err(ProtocolError::CrcMismatch);
    }
    let opcode = decoded[2];
    let sequence = decoded[3];
    let payload = decoded[5..5 + payload_len].to_vec();
    Ok((opcode, sequence, payload))
}
