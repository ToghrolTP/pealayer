use super::{ControlState, udp_codec::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub listen: String,
    pub discovery: bool,
    pub broadcast_events: bool,
    pub peers: std::collections::BTreeMap<String, String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            listen: "127.0.0.1:8080".into(),
            discovery: false,
            broadcast_events: false,
            peers: Default::default(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        let a: SocketAddr = self
            .listen
            .parse()
            .map_err(|_| "UDP listen needs a literal IPv4 address and port")?;
        if !a.is_ipv4() || a.port() == 0 {
            return Err("UDP listen needs IPv4 and nonzero port".into());
        }
        if self.enabled && self.discovery && a.ip().is_loopback() {
            return Err("UDP discovery requires a LAN or wildcard listening address".into());
        }
        if self.broadcast_events && !self.discovery {
            return Err("UDP event broadcast requires discovery".into());
        }
        if self.peers.len() > 64 {
            return Err("Too many UDP peers".into());
        }
        for (name, addr) in &self.peers {
            let a: SocketAddr = addr
                .parse()
                .map_err(|_| "UDP peer needs literal IPv4 address and port")?;
            if name.is_empty()
                || name.len() > 128
                || !a.is_ipv4()
                || a.port() == 0
                || a.ip().is_unspecified()
                || a.ip().is_multicast()
                || a.ip() == Ipv4Addr::BROADCAST
            {
                return Err("UDP peers require names and unicast IPv4 endpoints".into());
            }
        }
        Ok(())
    }
}
struct Assembly {
    parts: Vec<Option<Vec<u8>>>,
    at: Instant,
}
struct Peer {
    value: Value,
    at: Instant,
}
struct Pending {
    target: SocketAddr,
    tx: mpsc::SyncSender<Value>,
}
struct Data {
    peers: HashMap<String, Peer>,
    events: VecDeque<Value>,
    subs: HashMap<SocketAddr, Instant>,
    pending: HashMap<u64, Pending>,
    assemblies: HashMap<(SocketAddr, [u8; 16], u64, u8), Assembly>,
    receipts: HashMap<(SocketAddr, [u8; 16]), (u64, u64)>,
    last_events: HashMap<[u8; 16], u64>,
}
pub(super) struct Endpoint {
    socket: UdpSocket,
    config: Config,
    source: [u8; 16],
    sequence: AtomicU64,
    event_sequence: AtomicU64,
    stop: AtomicBool,
    workers: AtomicUsize,
    data: Mutex<Data>,
    state: ControlState,
}
struct Runtime {
    endpoint: Option<Arc<Endpoint>>,
    error: Option<String>,
    state: Option<ControlState>,
}
static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
fn runtime() -> &'static Mutex<Runtime> {
    RUNTIME.get_or_init(|| {
        Mutex::new(Runtime {
            endpoint: None,
            error: None,
            state: None,
        })
    })
}
pub(super) fn start(state: ControlState) {
    {
        let mut runtime = runtime().lock().unwrap();
        if runtime.state.is_some() {
            return;
        }
        runtime.state = Some(state.clone());
    }
    thread::spawn(move || {
        let mut applied = None;
        let mut revision = u64::MAX;
        let mut config = Config::default();
        let mut retry_at = Instant::now();
        let mut retry_delay = Duration::from_secs(2);
        loop {
            let next_revision = crate::platform::interop::live_config_revision();
            if revision != next_revision {
                config = crate::platform::interop::get_live_config().binary_udp;
                revision = next_revision;
            }
            let retry = config.enabled
                && runtime().lock().unwrap().endpoint.is_none()
                && Instant::now() >= retry_at;
            if applied.as_ref() != Some(&config) || retry {
                let previous = {
                    let mut r = runtime().lock().unwrap();
                    r.endpoint.take()
                };
                if let Some(p) = previous {
                    p.stop.store(true, Ordering::Release);
                }
                // Readers use a bounded deadline; finish before rebinding the same port.
                if applied.is_some() {
                    thread::sleep(Duration::from_millis(300));
                }
                applied = Some(config.clone());
                if config.enabled {
                    match Endpoint::open(config.clone(), state.clone()) {
                        Ok(ep) => {
                            let mut r = runtime().lock().unwrap();
                            r.endpoint = Some(ep);
                            r.error = None;
                            retry_delay = Duration::from_secs(2);
                        }
                        Err(error) => {
                            let mut r = runtime().lock().unwrap();
                            if r.error.as_ref() != Some(&error) {
                                log::error!("Binary UDP: {error}");
                            }
                            r.error = Some(error);
                            retry_at = Instant::now() + retry_delay;
                            retry_delay = (retry_delay * 2).min(Duration::from_secs(30));
                        }
                    }
                } else {
                    runtime().lock().unwrap().error = None;
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
impl Endpoint {
    fn open(config: Config, state: ControlState) -> Result<Arc<Self>, String> {
        config.validate()?;
        let socket = UdpSocket::bind(&config.listen).map_err(|e| e.to_string())?;
        socket
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| e.to_string())?;
        socket.set_broadcast(true).map_err(|e| e.to_string())?;
        let discovery = if config.discovery {
            let interface = match socket.local_addr().map_err(|e| e.to_string())?.ip() {
                std::net::IpAddr::V4(ip) => ip,
                _ => return Err("IPv4 required".into()),
            };
            socket2::SockRef::from(&socket)
                .set_multicast_if_v4(&interface)
                .map_err(|e| e.to_string())?;
            let s = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )
            .map_err(|e| e.to_string())?;
            s.set_reuse_address(true).map_err(|e| e.to_string())?;
            s.bind(&"0.0.0.0:8790".parse::<SocketAddr>().unwrap().into())
                .map_err(|e| e.to_string())?;
            let s: UdpSocket = s.into();
            s.join_multicast_v4(&Ipv4Addr::new(239, 255, 87, 87), &interface)
                .map_err(|e| e.to_string())?;
            s.set_read_timeout(Some(Duration::from_millis(200)))
                .map_err(|e| e.to_string())?;
            Some(s)
        } else {
            None
        };
        let ep = Arc::new(Self {
            socket,
            config,
            source: *uuid::Uuid::new_v4().as_bytes(),
            sequence: AtomicU64::new(0),
            event_sequence: AtomicU64::new(0),
            stop: AtomicBool::new(false),
            workers: AtomicUsize::new(0),
            state,
            data: Mutex::new(Data {
                peers: HashMap::new(),
                events: VecDeque::new(),
                subs: HashMap::new(),
                pending: HashMap::new(),
                assemblies: HashMap::new(),
                receipts: HashMap::new(),
                last_events: HashMap::new(),
            }),
        });
        let receiver = ep.socket.try_clone().map_err(|e| e.to_string())?;
        let p = ep.clone();
        thread::spawn(move || p.read(receiver, false));
        if let Some(s) = discovery {
            let p = ep.clone();
            thread::spawn(move || p.read(s, true));
        }
        let p = ep.clone();
        thread::spawn(move || {
            let mut last = Instant::now() - Duration::from_secs(16);
            while !p.stop.load(Ordering::Acquire) {
                if last.elapsed() >= Duration::from_secs(15) {
                    last = Instant::now();
                    if p.config.discovery {
                        for target in [DISCOVERY, "255.255.255.255:8790"] {
                            let _ = p.send(
                                ANNOUNCE,
                                p.next(),
                                &p.announcement(),
                                target.parse().unwrap(),
                            );
                        }
                    }
                }
                thread::sleep(Duration::from_millis(200));
            }
        });
        Ok(ep)
    }
    fn next(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed) + 1
    }
    fn id(&self) -> String {
        self.source.iter().map(|v| format!("{v:02x}")).collect()
    }
    fn announcement(&self) -> Value {
        json!({"id":self.id(),"identity":{"app":"pealayer","instance":self.state.application_identity.as_ref()},"port":self.socket.local_addr().unwrap().port(),"transport":"digitalogic-binary-udp","commands":"unicast","max_message":MAX_MESSAGE})
    }
    fn send(&self, kind: u8, id: u64, value: &Value, target: SocketAddr) -> Result<(), String> {
        let b = encode(value)?;
        let count = b.len().div_ceil(CHUNK).max(1);
        for (i, data) in b.chunks(CHUNK).enumerate() {
            self.socket
                .send_to(
                    &encode_packet(&Packet {
                        kind,
                        source: self.source,
                        id,
                        index: i as u16,
                        count: count as u16,
                        data: data.into(),
                    }),
                    target,
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn assemble(&self, p: &Packet, from: SocketAddr) -> Option<Value> {
        let mut d = self.data.lock().ok()?;
        d.assemblies
            .retain(|_, a| a.at.elapsed() < Duration::from_secs(3));
        let key = (from, p.source, p.id, p.kind);
        if !d.assemblies.contains_key(&key) {
            if d.assemblies.len() >= 16 {
                return None;
            }
            d.assemblies.insert(
                key,
                Assembly {
                    parts: vec![None; p.count as usize],
                    at: Instant::now(),
                },
            );
        }
        let a = d.assemblies.get_mut(&key)?;
        if a.parts.len() != p.count as usize {
            d.assemblies.remove(&key);
            return None;
        }
        if let Some(old) = &a.parts[p.index as usize] {
            if old != &p.data {
                d.assemblies.remove(&key);
            }
            return None;
        }
        a.parts[p.index as usize] = Some(p.data.clone());
        if a.parts.iter().any(Option::is_none) {
            return None;
        }
        let b: Vec<u8> = a
            .parts
            .iter()
            .flat_map(|v| v.as_ref().unwrap().iter().copied())
            .collect();
        d.assemblies.remove(&key);
        decode(&b).ok()
    }
    fn claim(&self, p: &Packet, from: SocketAddr) -> bool {
        let mut d = self.data.lock().unwrap();
        let key = (from, p.source);
        if !d.receipts.contains_key(&key) && d.receipts.len() >= 256 {
            return false;
        }
        let (high, bits) = d.receipts.entry(key).or_default();
        if p.id > *high {
            let delta = p.id - *high;
            *bits = if delta >= 64 { 0 } else { *bits << delta };
            *high = p.id;
            *bits |= 1;
            true
        } else {
            let delta = *high - p.id;
            if delta >= 64 || *bits & (1 << delta) != 0 {
                false
            } else {
                *bits |= 1 << delta;
                true
            }
        }
    }
    fn read(self: Arc<Self>, socket: UdpSocket, discovery: bool) {
        let mut buffer = [0u8; 1101];
        while !self.stop.load(Ordering::Acquire) {
            let (n, from) = match socket.recv_from(&mut buffer) {
                Ok(v) => v,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                Err(_) => break,
            };
            let Ok(p) = decode_packet(&buffer[..n]) else {
                continue;
            };
            if p.source == self.source
                || (discovery && !matches!(p.kind, DISCOVER | ANNOUNCE | EVENT))
            {
                continue;
            }
            let Some(value) = self.assemble(&p, from) else {
                continue;
            };
            match p.kind {
                DISCOVER => {
                    let _ = self.send(ANNOUNCE, p.id, &self.announcement(), from);
                }
                ANNOUNCE => {
                    let Some(port) = value["port"].as_u64().filter(|p| *p > 0 && *p <= 65535)
                    else {
                        continue;
                    };
                    let id: String = p.source.iter().map(|v| format!("{v:02x}")).collect();
                    if value["id"] != id {
                        continue;
                    }
                    let mut d = self.data.lock().unwrap();
                    if let Some(c) = d.pending.get(&p.id).filter(|c| c.target == from) {
                        let _ = c.tx.try_send(value.clone());
                    }
                    d.peers
                        .retain(|_, p| p.at.elapsed() < Duration::from_secs(45));
                    if d.peers.len() < 64 || d.peers.contains_key(&id) {
                        d.peers.insert(id.clone(),Peer{at:Instant::now(),value:json!({"id":id,"address":SocketAddr::new(from.ip(),port as u16).to_string(),"identity":value["identity"]})});
                    }
                }
                RESPONSE => {
                    let d = self.data.lock().unwrap();
                    if let Some(c) = d.pending.get(&p.id).filter(|c| c.target == from) {
                        let _ = c.tx.try_send(value);
                    }
                }
                EVENT => {
                    let Some(seq) = value["sequence"].as_u64().filter(|s| *s > 0) else {
                        continue;
                    };
                    let mut d = self.data.lock().unwrap();
                    let previous = d.last_events.get(&p.source).copied().unwrap_or(0);
                    if seq <= previous || (previous == 0 && d.last_events.len() >= 256) {
                        continue;
                    }
                    d.last_events.insert(p.source, seq);
                    if d.events.len() >= 64 {
                        d.events.pop_front();
                    }
                    let source: String = p.source.iter().map(|v| format!("{v:02x}")).collect();
                    d.events.push_back(json!({"source":source,"sequence":seq,"value":value["event"],"refresh_required":previous==0||seq!=previous+1}));
                }
                REQUEST | SUBSCRIBE | UNSUBSCRIBE => {
                    if value["target_id"] != self.id() {
                        continue;
                    }
                    let value = value["body"].clone();
                    if !self.claim(&p, from) {
                        continue;
                    }
                    if self.workers.fetch_add(1, Ordering::AcqRel) >= 16 {
                        self.workers.fetch_sub(1, Ordering::AcqRel);
                        let _=self.send(RESPONSE,p.id,&json!({"error":{"code":-32000,"message":"UDP dispatcher busy; request not executed"}}),from);
                        continue;
                    }
                    let ep = self.clone();
                    thread::spawn(move || {
                        let result = if p.kind == SUBSCRIBE || p.kind == UNSUBSCRIBE {
                            if !crate::platform::interop::get_live_config().web_sync_state {
                                json!({"error":{"code":-32003,"message":"State sharing disabled"}})
                            } else {
                                let mut d = ep.data.lock().unwrap();
                                if p.kind == SUBSCRIBE {
                                    if d.subs.len() >= 64 && !d.subs.contains_key(&from) {
                                        json!({"error":"UDP subscriptions full"})
                                    } else {
                                        d.subs
                                            .insert(from, Instant::now() + Duration::from_secs(30));
                                        json!({"subscribed":true,"lease_ms":30000})
                                    }
                                } else {
                                    d.subs.remove(&from);
                                    json!({"subscribed":false})
                                }
                            }
                        } else {
                            dispatch(&ep.state, &value, from)
                        };
                        if let Err(error) = ep.send(RESPONSE, p.id, &result, from) {
                            let _=ep.send(RESPONSE,p.id,&json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32010,"message":error}}),from);
                        }
                        ep.workers.fetch_sub(1, Ordering::AcqRel);
                    });
                }
                _ => {}
            }
        }
    }
    fn call(
        &self,
        peer: &str,
        kind: u8,
        value: &Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let target: SocketAddr = self
            .config
            .peers
            .get(peer)
            .ok_or("UDP target is not provisioned")?
            .parse()
            .map_err(|_| "Invalid UDP endpoint")?;
        let start = Instant::now();
        let wrapped;
        if kind != DISCOVER {
            let announcement = self.call(peer, DISCOVER, &json!({}), timeout)?;
            wrapped = json!({"target_id":announcement["id"],"body":value});
        } else {
            wrapped = value.clone();
        }
        let timeout = timeout.saturating_sub(start.elapsed());
        let id = self.next();
        let (tx, rx) = mpsc::sync_channel(1);
        {
            let mut d = self.data.lock().unwrap();
            if d.pending.len() >= 64 {
                return Err("UDP pending calls full".into());
            }
            d.pending.insert(id, Pending { target, tx });
        }
        let result = self.send(kind, id, &wrapped, target).and_then(|_| {
            rx.recv_timeout(timeout)
                .map_err(|_| "UDP outcome unknown: request is not retried".to_string())
        });
        self.data.lock().unwrap().pending.remove(&id);
        result
    }
    fn publish(&self, value: &Value) {
        if !crate::platform::interop::get_live_config().web_sync_state {
            return;
        }
        let targets = {
            let mut d = self.data.lock().unwrap();
            d.subs.retain(|_, until| *until > Instant::now());
            d.subs.keys().copied().collect::<Vec<_>>()
        };
        let id = self.next();
        let value =
            json!({"sequence":self.event_sequence.fetch_add(1,Ordering::Relaxed)+1,"event":value});
        for target in targets {
            let _ = self.send(EVENT, id, &value, target);
        }
        if self.config.broadcast_events && self.config.discovery {
            let _ = self.send(EVENT, id, &value, DISCOVERY.parse().unwrap());
        }
    }
    fn status(&self) -> Value {
        let d = self.data.lock().unwrap();
        json!({"running":!self.stop.load(Ordering::Acquire),"listen":self.socket.local_addr().ok().map(|v|v.to_string()),"id":self.id(),"peers":d.peers.values().filter(|p|p.at.elapsed()<Duration::from_secs(45)).map(|p|p.value.clone()).collect::<Vec<_>>(),"received_events":d.events.len()})
    }
}
pub(super) fn publish(value: &Value) {
    let ep = runtime().lock().ok().and_then(|r| r.endpoint.clone());
    if let Some(ep) = ep {
        ep.publish(value);
    }
}
// Presentation-only discovery. The autoconnect selector must not treat an
// unprovisioned announcement as permission to choose a controller.
pub(crate) fn discovered_controllers() -> Vec<String> {
    let Some(endpoint) = runtime().lock().ok().and_then(|r| r.endpoint.clone()) else {
        return vec![];
    };
    let Ok(data) = endpoint.data.lock() else {
        return vec![];
    };
    let mut endpoints = data
        .peers
        .values()
        .filter(|p| {
            p.at.elapsed() < Duration::from_secs(45)
                && p.value.pointer("/identity/app").and_then(Value::as_str) == Some("pccontroller")
        })
        .filter_map(|p| p.value["address"].as_str().map(|a| format!("udp://{a}")))
        .collect::<Vec<_>>();
    endpoints.sort();
    endpoints.dedup();
    endpoints
}
fn dispatch(state: &ControlState, value: &Value, from: SocketAddr) -> Value {
    if serde_json::from_value::<crate::platform::interop::JsonRpcRequest>(value.clone()).is_err() {
        return json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"A binary RPC request with a method is required"}});
    }
    if value["jsonrpc"] != "2.0" {
        return json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32600,"message":"Binary host RPC requires jsonrpc 2.0"}});
    }
    if value["method"]
        .as_str()
        .is_some_and(|m| m.starts_with("pealayer.udp."))
    {
        return rpc(value, from.ip().is_loopback())
            .unwrap_or_else(|| json!({"error":"Unknown UDP method"}));
    }
    if let Ok(request) =
        serde_json::from_value::<crate::platform::interop::JsonRpcRequest>(value.clone())
    {
        if matches!(
            request.method.as_str(),
            "config.update" | "pealayer.config.update" | "config.reload" | "pealayer.config.reload"
        ) && !crate::platform::interop::get_live_config().web_allow_configuration
        {
            return json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32003,"message":"Configuration permission disabled"}});
        }
        if matches!(
            request.method.as_str(),
            "config.update" | "pealayer.config.update"
        ) && request.params.get("binary_udp").is_some()
            && !from.ip().is_loopback()
        {
            return json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32003,"message":"UDP provisioning requires local IPC"}});
        }
        if let Ok(Some(command)) = crate::platform::interop::command_from_json_rpc(&request) {
            if let Some(error) = super::remote_command_permission(&command) {
                return json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32003,"message":error}});
            }
            if !crate::platform::interop::get_live_config().web_allow_control {
                return json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32003,"message":"Control permission disabled"}});
            }
        }
    }
    let text = value.to_string();
    let response = super::handle_websocket_text(state, &text).unwrap_or_else(|| {
        json!({"jsonrpc":"2.0","id":value["id"],"result":{"accepted":true}}).to_string()
    });
    serde_json::from_str(&response).unwrap_or_else(
        |_| json!({"error":{"code":-32000,"message":"Invalid application response"}}),
    )
}
pub(crate) fn rpc(value: &Value, local: bool) -> Option<Value> {
    let method = value["method"].as_str()?;
    if !method.starts_with("pealayer.udp.") {
        return None;
    }
    let id = &value["id"];
    let result = (|| -> Result<Value, String> {
        if method == "pealayer.udp.config.get" {
            if !crate::platform::interop::get_live_config().web_allow_configuration {
                return Err("Configuration permission disabled".into());
            }
            return serde_json::to_value(crate::platform::interop::get_live_config().binary_udp)
                .map_err(|e| e.to_string());
        }
        let ep = runtime().lock().unwrap().endpoint.clone();
        if method == "pealayer.udp.status" {
            return Ok(ep.map(|e| e.status()).unwrap_or_else(
                || json!({"running":false,"error":runtime().lock().unwrap().error}),
            ));
        }
        if method == "pealayer.udp.events.get" {
            let ep = ep.ok_or("UDP transport is not running")?;
            let data = ep.data.lock().unwrap();
            let mut events = VecDeque::new();
            for event in data.events.iter().rev() {
                events.push_front(event.clone());
                let bytes = encode(&json!(events))?;
                if bytes.len() > 128 * 1024 {
                    events.pop_front();
                    break;
                }
            }
            return Ok(json!(events));
        }
        if !local {
            return Err(
                "UDP provisioning/routing requires local IPC; remote pivots are not allowed".into(),
            );
        }
        if method == "pealayer.udp.config.set" {
            if !crate::platform::interop::get_live_config().web_allow_configuration {
                return Err("Configuration permission disabled".into());
            }
            let config: Config =
                serde_json::from_value(value["params"].clone()).map_err(|e| e.to_string())?;
            config.validate()?;
            let values = json!({"binary_udp":config});
            crate::platform::interop::get_live_config().apply_patch(&values)?;
            let state = runtime()
                .lock()
                .unwrap()
                .state
                .clone()
                .ok_or("Configuration dispatcher unavailable")?;
            state
                .command_tx
                .send(crate::platform::interop::InteropCommand::UpdateConfig { values })
                .map_err(|e| e.to_string())?;
            state.egui_ctx.request_repaint();
            return Ok(json!({"accepted":true}));
        }
        if !crate::platform::interop::get_live_config().web_allow_control {
            return Err("Control permission disabled".into());
        }
        let ep = ep.ok_or("UDP transport is not running")?;
        let params = &value["params"];
        let peer = params["peer"]
            .as_str()
            .ok_or("A configured peer is required")?;
        let timeout = params["timeout_ms"].as_u64().unwrap_or(5000);
        if !(100..=30000).contains(&timeout) {
            return Err("timeout_ms must be 100..30000".into());
        }
        let kind = match method {
            "pealayer.udp.call" => REQUEST,
            "pealayer.udp.subscribe" => SUBSCRIBE,
            "pealayer.udp.unsubscribe" => UNSUBSCRIBE,
            _ => return Err("Unknown UDP method".into()),
        };
        ep.call(
            peer,
            kind,
            &params["request"],
            Duration::from_millis(timeout),
        )
    })();
    Some(match result {
        Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
        Err(error) => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":error}}),
    })
}
