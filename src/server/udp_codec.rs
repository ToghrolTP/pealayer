//! Current Digitalogic binary host values; firmware's COBS/CRC-8/ATM framing.
use serde_json::{Map, Number, Value};
pub const MAX_MESSAGE: usize = 256 * 1024;
pub const CHUNK: usize = 1024;
pub const DISCOVERY: &str = "239.255.87.87:8790";
pub const DISCOVER: u8 = 1;
pub const ANNOUNCE: u8 = 2;
pub const REQUEST: u8 = 3;
pub const RESPONSE: u8 = 4;
pub const SUBSCRIBE: u8 = 5;
pub const EVENT: u8 = 6;
pub const UNSUBSCRIBE: u8 = 7;

#[derive(Clone)]
pub struct Packet {
    pub kind: u8,
    pub source: [u8; 16],
    pub id: u64,
    pub index: u16,
    pub count: u16,
    pub data: Vec<u8>,
}
pub fn encode_packet(p: &Packet) -> Vec<u8> {
    let mut b = b"DGLU".to_vec();
    b.push(p.kind);
    b.extend(p.source);
    b.extend(p.id.to_le_bytes());
    b.extend(p.index.to_le_bytes());
    b.extend(p.count.to_le_bytes());
    b.extend(&p.data);
    b.push(crate::four_d::protocol::crc8_atm(&b));
    let mut encoded = crate::four_d::protocol::cobs_encode(&b);
    encoded.push(0);
    encoded
}
pub fn decode_packet(b: &[u8]) -> Result<Packet, String> {
    if b.len() < 2 || b.len() > 1100 || b.last() != Some(&0) {
        return Err("Invalid UDP size/delimiter".into());
    }
    let b = crate::four_d::protocol::cobs_decode(&b[..b.len() - 1]).map_err(|e| e.to_string())?;
    if b.len() < 34
        || &b[..4] != b"DGLU"
        || crate::four_d::protocol::crc8_atm(&b[..b.len() - 1]) != b[b.len() - 1]
    {
        return Err("Invalid UDP framing/CRC".into());
    }
    let p = Packet {
        kind: b[4],
        source: b[5..21].try_into().unwrap(),
        id: u64::from_le_bytes(b[21..29].try_into().unwrap()),
        index: u16::from_le_bytes(b[29..31].try_into().unwrap()),
        count: u16::from_le_bytes(b[31..33].try_into().unwrap()),
        data: b[33..b.len() - 1].into(),
    };
    if !(DISCOVER..=UNSUBSCRIBE).contains(&p.kind)
        || p.id == 0
        || p.count == 0
        || p.count > 256
        || p.index >= p.count
        || p.data.len() > CHUNK
        || (p.index + 1 < p.count && p.data.len() != CHUNK)
    {
        return Err("Invalid UDP fragment".into());
    }
    Ok(p)
}
pub fn encode(v: &Value) -> Result<Vec<u8>, String> {
    let mut b = Vec::new();
    write(v, &mut b, 0)?;
    Ok(b)
}
fn write(v: &Value, b: &mut Vec<u8>, depth: usize) -> Result<(), String> {
    if depth > 32 {
        return Err("Binary nesting exceeds 32".into());
    }
    let length = |b: &mut Vec<u8>, n: usize| b.extend((n as u32).to_le_bytes());
    match v {
        Value::Null => b.push(0),
        Value::Bool(false) => b.push(1),
        Value::Bool(true) => b.push(2),
        Value::Number(n) => {
            if let Some(n) = n.as_i64() {
                b.push(3);
                b.extend(n.to_le_bytes());
            } else if let Some(n) = n.as_u64() {
                b.push(4);
                b.extend(n.to_le_bytes());
            } else {
                b.push(5);
                b.extend(n.as_f64().ok_or("Invalid number")?.to_le_bytes());
            }
        }
        Value::String(s) => {
            b.push(6);
            length(b, s.len());
            b.extend(s.as_bytes());
        }
        Value::Array(a) => {
            b.push(7);
            length(b, a.len());
            for v in a {
                write(v, b, depth + 1)?;
            }
        }
        Value::Object(m) => {
            b.push(8);
            length(b, m.len());
            let mut keys: Vec<_> = m.keys().collect();
            keys.sort();
            for k in keys {
                write(&Value::String(k.clone()), b, depth + 1)?;
                write(&m[k], b, depth + 1)?;
            }
        }
    }
    if b.len() > MAX_MESSAGE {
        return Err("Binary message exceeds 256 KiB".into());
    }
    Ok(())
}
fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
    if b.len() < n {
        return Err("Truncated binary value".into());
    }
    let (v, rest) = b.split_at(n);
    *b = rest;
    Ok(v)
}
pub fn decode(mut b: &[u8]) -> Result<Value, String> {
    if b.len() > MAX_MESSAGE {
        return Err("Oversize binary message".into());
    }
    let v = read(&mut b, 0)?;
    if !b.is_empty() {
        return Err("Trailing binary data".into());
    }
    Ok(v)
}
fn read(b: &mut &[u8], depth: usize) -> Result<Value, String> {
    if depth > 32 {
        return Err("Binary nesting exceeds 32".into());
    }
    let tag = take(b, 1)?[0];
    match tag {
        0 => return Ok(Value::Null),
        1 => return Ok(Value::Bool(false)),
        2 => return Ok(Value::Bool(true)),
        3..=5 => {
            let n: u64 = u64::from_le_bytes(take(b, 8)?.try_into().unwrap());
            return Ok(Value::Number(match tag {
                3 => Number::from(n as i64),
                4 => Number::from(n),
                _ => Number::from_f64(f64::from_bits(n)).ok_or("Invalid float")?,
            }));
        }
        _ => {}
    }
    let n = u32::from_le_bytes(take(b, 4)?.try_into().unwrap()) as usize;
    if n > b.len() {
        return Err("Invalid binary length".into());
    }
    match tag {
        6 => Ok(Value::String(
            std::str::from_utf8(take(b, n)?)
                .map_err(|_| "Invalid UTF-8")?
                .into(),
        )),
        7 => {
            let mut a = Vec::with_capacity(n);
            for _ in 0..n {
                a.push(read(b, depth + 1)?);
            }
            Ok(Value::Array(a))
        }
        8 => {
            let mut m = Map::new();
            for _ in 0..n {
                let Value::String(k) = read(b, depth + 1)? else {
                    return Err("Object key is not text".into());
                };
                if m.contains_key(&k) {
                    return Err("Duplicate object key".into());
                }
                m.insert(k, read(b, depth + 1)?);
            }
            Ok(Value::Object(m))
        }
        _ => Err("Unknown binary tag".into()),
    }
}
