//! Reusable binary UDP session. Never retransmit an outcome-uncertain mutation.
use super::udp_codec::*;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

pub(crate) struct Client {
    socket: UdpSocket,
    target: SocketAddr,
    source: [u8; 16],
    remote: [u8; 16],
    target_id: String,
    next: u64,
    timeout: Duration,
    parts: HashMap<(u64, u8), (Instant, Vec<Option<Vec<u8>>>)>,
    events: VecDeque<Value>,
    last_event: u64,
}
impl Client {
    pub fn connect(address: &str, timeout: Duration) -> Result<Self, String> {
        let target: SocketAddr = address
            .parse()
            .map_err(|_| "Binary UDP requires a literal IPv4 endpoint")?;
        if !target.is_ipv4()
            || target.port() == 0
            || target.ip().is_multicast()
            || target.ip().is_unspecified()
            || target.ip() == std::net::Ipv4Addr::BROADCAST
        {
            return Err("Binary UDP needs a unicast IPv4 endpoint".into());
        }
        let socket = UdpSocket::bind(if target.ip().is_loopback() {
            "127.0.0.1:0"
        } else {
            "0.0.0.0:0"
        })
        .map_err(|e| e.to_string())?;
        socket.connect(target).map_err(|e| e.to_string())?;
        let mut c = Self {
            socket,
            target,
            source: *uuid::Uuid::new_v4().as_bytes(),
            remote: [0; 16],
            target_id: String::new(),
            next: 0,
            timeout,
            parts: HashMap::new(),
            events: VecDeque::new(),
            last_event: 0,
        };
        let identity = c.exchange(DISCOVER, &json!({}), timeout)?;
        c.target_id = identity["id"]
            .as_str()
            .ok_or("Missing UDP identity")?
            .into();
        Ok(c)
    }
    fn send(&mut self, kind: u8, value: &Value) -> Result<u64, String> {
        self.next = self.next.checked_add(1).ok_or("UDP sequence exhausted")?;
        let wrapped = if kind == DISCOVER {
            value.clone()
        } else {
            json!({"target_id":self.target_id,"body":value})
        };
        let b = encode(&wrapped)?;
        let count = b.len().div_ceil(CHUNK);
        for (i, data) in b.chunks(CHUNK).enumerate() {
            self.socket
                .send(&encode_packet(&Packet {
                    kind,
                    source: self.source,
                    id: self.next,
                    index: i as u16,
                    count: count as u16,
                    data: data.into(),
                }))
                .map_err(|e| e.to_string())?;
        }
        Ok(self.next)
    }
    fn receive(&mut self, timeout: Duration) -> Result<Option<(u8, u64, Value)>, String> {
        self.socket
            .set_read_timeout(Some(timeout.max(Duration::from_millis(1))))
            .map_err(|e| e.to_string())?;
        let mut buffer = [0; 1101];
        let (n, from) = match self.socket.recv_from(&mut buffer) {
            Ok(v) => v,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Ok(None);
            }
            Err(e) => return Err(e.to_string()),
        };
        if from != self.target {
            return Ok(None);
        }
        let Ok(p) = decode_packet(&buffer[..n]) else {
            return Ok(None);
        };
        if !matches!(p.kind, ANNOUNCE | RESPONSE | EVENT) {
            return Ok(None);
        }
        if self.remote != [0; 16] && p.source != self.remote {
            return Err("UDP peer restarted; authoritative reconnect required".into());
        }
        self.parts
            .retain(|_, (at, _)| at.elapsed() < Duration::from_secs(3));
        let key = (p.id, p.kind);
        if !self.parts.contains_key(&key) && self.parts.len() >= 16 {
            return Ok(None);
        }
        let (_, parts) = self
            .parts
            .entry(key)
            .or_insert_with(|| (Instant::now(), vec![None; p.count as usize]));
        if parts.len() != p.count as usize {
            self.parts.remove(&key);
            return Ok(None);
        }
        if let Some(previous) = &parts[p.index as usize] {
            if previous != &p.data {
                self.parts.remove(&key);
            }
            return Ok(None);
        }
        parts[p.index as usize] = Some(p.data);
        if parts.iter().any(Option::is_none) {
            return Ok(None);
        }
        let b: Vec<u8> = parts
            .iter()
            .flat_map(|v| v.as_ref().unwrap().iter().copied())
            .collect();
        self.parts.remove(&key);
        let value = decode(&b)?;
        if self.remote == [0; 16] && p.kind == ANNOUNCE {
            let id: String = p.source.iter().map(|v| format!("{v:02x}")).collect();
            if value["id"] != id {
                return Err("UDP identity mismatch".into());
            }
            self.remote = p.source;
        }
        Ok(Some((p.kind, p.id, value)))
    }
    fn exchange(&mut self, kind: u8, value: &Value, timeout: Duration) -> Result<Value, String> {
        let id = self.send(kind, value)?;
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some((received, corr, value)) =
                self.receive(deadline.saturating_duration_since(Instant::now()))?
            {
                if received == EVENT {
                    if self.events.len() >= 64 {
                        return Err("UDP event queue overflow; refresh required".into());
                    }
                    self.events.push_back(value);
                    continue;
                }
                if corr == id && received == if kind == DISCOVER { ANNOUNCE } else { RESPONSE } {
                    return Ok(value);
                }
            }
        }
        Err("UDP outcome unknown: request was not retried".into())
    }
    pub fn call(&mut self, value: &Value, timeout: Option<Duration>) -> Result<Value, String> {
        self.exchange(REQUEST, value, timeout.unwrap_or(self.timeout))
    }
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    pub fn subscribe(&mut self) -> Result<(), String> {
        let response = self.exchange(SUBSCRIBE, &json!({}), self.timeout)?;
        if response["subscribed"] != true {
            return Err(format!("UDP subscription rejected: {response}"));
        }
        Ok(())
    }
    pub fn event(&mut self, timeout: Duration) -> Result<Option<Value>, String> {
        let deadline = Instant::now() + timeout;
        loop {
            let value = if let Some(v) = self.events.pop_front() {
                Some(v)
            } else {
                self.receive(deadline.saturating_duration_since(Instant::now()))?
                    .and_then(|(kind, _, v)| (kind == EVENT).then_some(v))
            };
            if let Some(value) = value {
                let seq = value["sequence"]
                    .as_u64()
                    .ok_or("Missing UDP event sequence")?;
                if seq <= self.last_event {
                    continue;
                }
                if self.last_event != 0 && seq != self.last_event + 1 {
                    return Err("UDP events lost/reordered; authoritative refresh required".into());
                }
                self.last_event = seq;
                return Ok(Some(value["event"].clone()));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
        }
    }
}
