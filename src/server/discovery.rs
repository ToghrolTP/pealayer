//! Metadata-only LAN discovery. It never connects a player, acquires hardware
//! authority, forwards credentials, or infers a board from an advertisement.
use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

const PEALAYER_TYPE: &str = "urn:digitalogic-ir:device:Pealayer:1";
const PC_TYPE: &str = "urn:pccontroller-org:service:Controller:1";
const MOBILE_TYPE: &str = "urn:digitalogic-ir:device:ControllerMobile:1";
#[derive(Default)]
struct Registry {
    peers: BTreeMap<String, (Instant, Value)>,
    attempted: BTreeMap<String, Instant>,
    errors: BTreeMap<String, String>,
    active: BTreeSet<String>,
    own: BTreeSet<SocketAddr>,
}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(Default::default)
}
fn fault(protocol: &str, error: impl std::fmt::Display) {
    log::warn!("{protocol} discovery: {error}");
    if let Ok(mut r) = registry().lock() {
        r.active.remove(protocol);
        r.errors.insert(protocol.into(), error.to_string());
    }
}
pub fn snapshot() -> Value {
    let Ok(mut r) = registry().lock() else {
        return json!({"peers":[]});
    };
    r.peers
        .retain(|_, (at, _)| at.elapsed() < Duration::from_secs(120));
    json!({"peers":r.peers.values().map(|(_,v)|v).collect::<Vec<_>>(),"errors":r.errors,
        "configured_protocols":["dns-sd","mdns","ssdp","mndp"],"active_protocols":r.active,"automatic_authority":false})
}
pub(crate) fn rpc_payload(payload: &str) -> Option<String> {
    let value: Value = serde_json::from_str(payload).ok()?;
    if value["method"] != "pealayer.discovery.peers" {
        return None;
    }
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    if value["jsonrpc"] != "2.0"
        || (!id.is_null() && !id.is_number() && !id.is_string())
        || id.as_str().is_some_and(|s| s.len() > 128)
        || value
            .get("params")
            .is_some_and(|p| !p.is_null() && !p.as_object().is_some_and(|m| m.is_empty()))
    {
        return Some(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"Invalid discovery query"}}).to_string());
    }
    Some(json!({"jsonrpc":"2.0","id":id,"result":snapshot()}).to_string())
}
#[derive(Clone)]
struct Candidate {
    address: SocketAddr,
    name: String,
    product: String,
    protocol: &'static str,
}
fn local_source(address: SocketAddr) -> bool {
    matches!(address.ip(), IpAddr::V4(ip) if (ip.is_private() || ip.is_link_local()) && !ip.is_loopback() && !ip.is_broadcast() && address.port()!=0)
}
fn product_ok(value: &str) -> bool {
    matches!(value, "pccontroller" | "pealayer" | "controller-mobile")
}
fn offer(queue: &mpsc::SyncSender<Candidate>, candidate: Candidate) {
    if !local_source(candidate.address)
        || !product_ok(&candidate.product)
        || candidate.name.len() > 96
    {
        return;
    }
    let key = candidate.address.to_string();
    if let Ok(mut r) = registry().lock() {
        if r.own.contains(&candidate.address) {
            return;
        }
        r.attempted
            .retain(|_, at| at.elapsed() < Duration::from_secs(30));
        if r.attempted.contains_key(&key) || r.attempted.len() >= 128 {
            return;
        }
        r.attempted.insert(key, Instant::now());
    }
    let _ = queue.try_send(candidate);
}
fn verify(candidate: Candidate) {
    let Ok(client) = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(900))
        .build()
    else {
        return;
    };
    let Ok(mut response) = client
        .get(format!("http://{}/healthz", candidate.address))
        .send()
    else {
        return;
    };
    if !response.status().is_success() {
        return;
    }
    let mut body = Vec::new();
    if response.by_ref().take(4097).read_to_end(&mut body).is_err() || body.len() > 4096 {
        return;
    }
    let Ok(mut health) = serde_json::from_slice::<Value>(&body) else {
        return;
    };
    if candidate.product == "pccontroller" && health["ok"] == true && health["status"].is_null() {
        let Ok(mut response) = client
            .get(format!("http://{}/upnp/public.json", candidate.address))
            .send()
        else {
            return;
        };
        if !response.status().is_success() {
            return;
        }
        let mut body = Vec::new();
        if response
            .by_ref()
            .take(16385)
            .read_to_end(&mut body)
            .is_err()
            || body.len() > 16384
        {
            return;
        }
        let Ok(directory) = serde_json::from_slice::<Value>(&body) else {
            return;
        };
        if directory["product"] != "PCController"
            || directory["health"]["ok"] != true
            || !directory["hostname"].is_string()
        {
            return;
        }
        health = json!({"product":"pccontroller","status":"ok"});
    }
    if health["status"] != "ok"
        || health["product"].as_str().or(health["service"].as_str())
            != Some(candidate.product.as_str())
    {
        return;
    }
    if let Ok(mut r) = registry().lock() {
        r.peers
            .retain(|_, (at, _)| at.elapsed() < Duration::from_secs(120));
        let key = candidate.address.to_string();
        if r.peers.len() < 128 || r.peers.contains_key(&key) {
            let mut protocols = r
                .peers
                .get(&key)
                .and_then(|(_, v)| v["protocols"].as_array())
                .cloned()
                .unwrap_or_default();
            if !protocols.contains(&json!(candidate.protocol)) {
                protocols.push(json!(candidate.protocol));
            }
            r.peers.insert(key,(Instant::now(),json!({"name":candidate.name,"product":candidate.product,
                "endpoint":format!("http://{}",candidate.address),"verified":true,"protocols":protocols})));
        }
    }
}

/// Advertise only sockets that really bound to LAN addresses. Loopback-only
/// development/fixtures never open multicast sockets or discovery daemons.
pub fn start(listeners: &[std::net::TcpListener], identity: &str) {
    let bound: Vec<_> = listeners
        .iter()
        .filter_map(|s| s.local_addr().ok())
        .filter(|a| a.is_ipv4() && !a.ip().is_loopback())
        .collect();
    if bound.is_empty() {
        return;
    }
    let addresses: Vec<Ipv4Addr> = crate::network::discover_bind_targets()
        .into_iter()
        .filter_map(|a| a.value.parse::<Ipv4Addr>().ok())
        .filter(|ip| {
            !ip.is_unspecified()
                && !ip.is_loopback()
                && bound
                    .iter()
                    .any(|b| b.ip().is_unspecified() || b.ip() == IpAddr::V4(*ip))
        })
        .collect();
    if addresses.is_empty() {
        return;
    }
    let port = bound[0].port();
    if let Ok(mut r) = registry().lock() {
        r.own
            .extend(addresses.iter().map(|ip| SocketAddr::from((*ip, port))));
    }
    let mut name = String::new();
    for c in identity.chars().filter(|c| !c.is_control()) {
        if name.len() + c.len_utf8() > 60 {
            break;
        }
        name.push(c);
    }
    if name.is_empty() {
        name = "Pealayer".into();
    }
    // A distinct application instance, not a physical board or shared brand ID.
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::sync_channel::<Candidate>(64);
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..4 {
        let rx = rx.clone();
        thread::spawn(move || {
            loop {
                let value = {
                    let Ok(rx) = rx.lock() else {
                        return;
                    };
                    rx.recv()
                };
                match value {
                    Ok(candidate) => verify(candidate),
                    Err(_) => return,
                }
            }
        });
    }
    start_mdns(&addresses, port, &name, &id, tx.clone());
    start_udp(
        addresses.clone(),
        port,
        name.clone(),
        id.clone(),
        false,
        tx.clone(),
    );
    start_udp(addresses, port, name, id, true, tx);
}
fn start_mdns(
    addresses: &[Ipv4Addr],
    port: u16,
    name: &str,
    id: &str,
    queue: mpsc::SyncSender<Candidate>,
) {
    let result = (|| -> Result<(), String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        // Explicit addresses: never advertise a loopback/non-listening NIC.
        let service = ServiceInfo::new(
            "_pealayer._tcp.local.",
            name,
            &format!("pealayer-{}.local.", &id[..12]),
            addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            port,
            [
                ("product", "pealayer"),
                ("path", "/api/ipc"),
                ("health", "/healthz"),
                ("instance", id),
            ]
            .as_slice(),
        )
        .map_err(|e| e.to_string())?;
        daemon.register(service).map_err(|e| e.to_string())?;
        let monitor = daemon.monitor().map_err(|e| e.to_string())?;
        thread::spawn(move || {
            while let Ok(event) = monitor.recv() {
                if let DaemonEvent::Error(error) = event {
                    fault("mdns", error);
                } else if matches!(event, DaemonEvent::Announce(..)) {
                    if let Ok(mut r) = registry().lock() {
                        r.active.insert("mdns".into());
                        r.active.insert("dns-sd".into());
                    }
                }
            }
        });
        for (kind, product) in [
            ("_pccontroller._tcp.local.", "pccontroller"),
            ("_pealayer._tcp.local.", "pealayer"),
            ("_digitalogic._tcp.local.", "controller-mobile"),
        ] {
            let receiver = daemon.browse(kind).map_err(|e| e.to_string())?;
            let queue = queue.clone();
            thread::spawn(move || {
                while let Ok(event) = receiver.recv() {
                    if let ServiceEvent::ServiceResolved(service) = event {
                        for address in service.get_addresses_v4() {
                            offer(
                                &queue,
                                Candidate {
                                    address: SocketAddr::from((address, service.get_port())),
                                    name: service
                                        .get_fullname()
                                        .split("._")
                                        .next()
                                        .unwrap_or("Peer")
                                        .chars()
                                        .take(96)
                                        .collect(),
                                    product: product.into(),
                                    protocol: "dns-sd",
                                },
                            );
                        }
                    }
                }
            });
        }
        // Daemon handle must remain alive for process lifetime, not per-frame.
        thread::spawn(move || {
            let _daemon = daemon;
            thread::park();
        });
        Ok(())
    })();
    if let Err(error) = result {
        fault("mdns", error);
    }
}
fn udp_socket(port: u16, group: Ipv4Addr, addresses: &[Ipv4Addr]) -> std::io::Result<UdpSocket> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    socket.set_reuse_address(true)?;
    socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    let socket: UdpSocket = socket.into();
    for ip in addresses {
        socket.join_multicast_v4(&group, ip)?;
    }
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;
    Ok(socket)
}
fn start_udp(
    addresses: Vec<Ipv4Addr>,
    port: u16,
    name: String,
    id: String,
    mndp: bool,
    queue: mpsc::SyncSender<Candidate>,
) {
    let group = if mndp {
        Ipv4Addr::new(224, 0, 0, 88)
    } else {
        Ipv4Addr::new(239, 255, 255, 250)
    };
    let udp_port = if mndp { 5678 } else { 1900 };
    let protocol = if mndp { "mndp" } else { "ssdp" };
    let socket = match udp_socket(udp_port, group, &addresses) {
        Ok(s) => s,
        Err(e) => {
            fault(protocol, e);
            return;
        }
    };
    if let Ok(mut r) = registry().lock() {
        r.active.insert(protocol.into());
    }
    thread::spawn(move || {
        let mut bytes = [0u8; 4096];
        let mut last = Instant::now() - Duration::from_secs(31);
        let mut sequence = 0u16;
        let mut replies: BTreeMap<IpAddr, Instant> = BTreeMap::new();
        loop {
            if last.elapsed() >= Duration::from_secs(30) {
                let probe = if mndp {
                    vec![0, 0, 0, 0]
                } else {
                    b"M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 1\r\nST: ssdp:all\r\n\r\n".to_vec()
                };
                let _ = socket.send_to(&probe, (group, udp_port));
                if mndp {
                    let _ = socket.send_to(&probe, (Ipv4Addr::BROADCAST, udp_port));
                    let packet = encode_mndp(&name, &id, port, sequence);
                    sequence = sequence.wrapping_add(1);
                    let _ = socket.send_to(&packet, (Ipv4Addr::BROADCAST, udp_port));
                }
                last = Instant::now();
            }
            let (length, source) = match socket.recv_from(&mut bytes) {
                Ok(v) => v,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(e) => {
                    fault(protocol, e);
                    return;
                }
            };
            if !local_source(source) {
                continue;
            }
            let request = &bytes[..length];
            let discovery = if mndp {
                request == [0, 0, 0, 0]
            } else {
                std::str::from_utf8(request).is_ok_and(|s| {
                    s.starts_with("M-SEARCH * HTTP/1.1\r\n")
                        && s.to_ascii_lowercase().contains("man: \"ssdp:discover\"")
                        && (s.to_ascii_lowercase().contains("st: ssdp:all")
                            || s.contains(PEALAYER_TYPE))
                })
            };
            if discovery {
                replies.retain(|_, at| at.elapsed() < Duration::from_secs(1));
                if replies.contains_key(&source.ip()) || replies.len() >= 128 {
                    continue;
                }
                replies.insert(source.ip(), Instant::now());
                let response = if mndp {
                    encode_mndp(&name, &id, port, sequence)
                } else {
                    let ip = addresses
                        .iter()
                        .find(|ip| {
                            ip.octets()[..3]
                                == match source.ip() {
                                    IpAddr::V4(v) => v.octets()[..3].to_vec(),
                                    _ => vec![],
                                }
                        })
                        .unwrap_or(&addresses[0]);
                    format!("HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=60\r\nEXT:\r\nST: {PEALAYER_TYPE}\r\nUSN: uuid:{id}::{PEALAYER_TYPE}\r\nLOCATION: http://{ip}:{port}/api/discovery\r\nX-Digitalogic-Product: pealayer\r\nX-Digitalogic-Name: {name}\r\n\r\n").into_bytes()
                };
                let _ = socket.send_to(&response, source);
            } else if let Some(candidate) = if mndp {
                parse_mndp(request, source)
            } else {
                parse_ssdp(request, source)
            } {
                offer(&queue, candidate);
            }
        }
    });
}
fn encode_mndp(name: &str, id: &str, port: u16, sequence: u16) -> Vec<u8> {
    let mut packet = vec![0, 0];
    packet.extend(sequence.to_be_bytes());
    let digest = Sha256::digest(format!("pealayer:{id}"));
    let mut mac = digest[..6].to_vec();
    mac[0] = (mac[0] | 2) & !1;
    // Application-scoped virtual MAC, not an invented physical board/NIC MAC.
    for (kind, data) in [
        (1u16, mac),
        (5, name.as_bytes().to_vec()),
        (8, b"Digitalogic".to_vec()),
        (11, id.as_bytes().to_vec()),
        (12, b"Application endpoint".to_vec()),
        (16, b"application".to_vec()),
        (0xff01, b"pealayer".to_vec()),
        (0xff02, port.to_be_bytes().to_vec()),
    ] {
        packet.extend(kind.to_be_bytes());
        packet.extend((data.len() as u16).to_be_bytes());
        packet.extend(data);
    }
    packet
}
fn parse_mndp(packet: &[u8], source: SocketAddr) -> Option<Candidate> {
    if !(14..=2048).contains(&packet.len()) || packet[..2] != [0, 0] {
        return None;
    }
    let mut fields = BTreeMap::new();
    let mut at = 4;
    while at < packet.len() {
        if at + 4 > packet.len() {
            return None;
        }
        let kind = u16::from_be_bytes(packet[at..at + 2].try_into().ok()?);
        let len = u16::from_be_bytes(packet[at + 2..at + 4].try_into().ok()?) as usize;
        at += 4;
        if at + len > packet.len() || fields.insert(kind, &packet[at..at + len]).is_some() {
            return None;
        }
        at += len;
    }
    if fields.get(&1)?.len() != 6 {
        return None;
    }
    let name = std::str::from_utf8(fields.get(&5)?).ok()?.to_string();
    if name.is_empty() || name.len() > 96 || name.chars().any(char::is_control) {
        return None;
    }
    let product = std::str::from_utf8(fields.get(&0xff01)?).ok()?.to_string();
    let port = u16::from_be_bytes((*fields.get(&0xff02)?).try_into().ok()?);
    Some(Candidate {
        address: SocketAddr::new(source.ip(), port),
        name,
        product,
        protocol: "mndp",
    })
}
fn parse_ssdp(packet: &[u8], source: SocketAddr) -> Option<Candidate> {
    let text = std::str::from_utf8(packet).ok()?;
    let mut lines = text.lines();
    if !lines.next()?.starts_with("HTTP/1.1 200 ") {
        return None;
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            if headers
                .insert(key.trim().to_ascii_lowercase(), value.trim())
                .is_some()
            {
                return None;
            }
        }
    }
    let product = match *headers.get("st")? {
        PC_TYPE => "pccontroller",
        PEALAYER_TYPE => "pealayer",
        MOBILE_TYPE => "controller-mobile",
        _ => return None,
    };
    let url = url::Url::parse(headers.get("location")?).ok()?;
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str()?.parse::<IpAddr>().ok()? != source.ip()
    {
        return None;
    }
    Some(Candidate {
        address: SocketAddr::new(source.ip(), url.port_or_known_default()?),
        name: headers
            .get("x-digitalogic-name")
            .or(headers.get("x-pccontroller-name"))
            .copied()
            .unwrap_or(url.host_str()?)
            .chars()
            .take(96)
            .collect(),
        product: product.into(),
        protocol: "ssdp",
    })
}
