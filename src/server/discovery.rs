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
const SSDP_GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;
const SSDP_SEARCH: &[u8] = b"M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 1\r\nST: ssdp:all\r\n\r\n";
#[derive(Default)]
struct Registry {
    peers: BTreeMap<String, (Instant, Value)>,
    attempted: BTreeMap<(SocketAddr, &'static str), Instant>,
    errors: BTreeMap<String, String>,
    active: BTreeSet<String>,
    advertising: BTreeSet<String>,
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
        if protocol == "mdns" {
            r.active.remove("dns-sd");
            r.advertising.remove("dns-sd");
        }
    }
}
fn active(protocol: &str) {
    if let Ok(mut r) = registry().lock() {
        r.errors.remove(protocol);
        r.active.insert(protocol.into());
    }
}
pub fn snapshot() -> Value {
    let Ok(mut r) = registry().lock() else {
        return json!({"peers":[]});
    };
    r.peers
        .retain(|_, (at, _)| at.elapsed() < Duration::from_secs(120));
    json!({"peers":r.peers.values().map(|(_,v)|v).collect::<Vec<_>>(),"errors":r.errors,
        "configured_protocols":["dns-sd","mdns","ssdp","mndp"],"active_protocols":r.active,
        "advertising_protocols":r.advertising,"automatic_authority":false})
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
impl Registry {
    fn admit(&mut self, candidate: &Candidate, now: Instant) -> bool {
        let key = (candidate.address, candidate.protocol);
        self.attempted
            .retain(|_, at| now.duration_since(*at) < Duration::from_secs(30));
        if self.own.contains(&candidate.address)
            || self.attempted.contains_key(&key)
            || self.attempted.len() >= 128
        {
            return false;
        }
        self.attempted.insert(key, now);
        true
    }
}
fn offer(queue: &mpsc::SyncSender<Candidate>, candidate: Candidate) {
    if !local_source(candidate.address)
        || !product_ok(&candidate.product)
        || candidate.name.len() > 96
    {
        return;
    }
    if let Ok(mut r) = registry().lock() {
        if r.admit(&candidate, Instant::now()) {
            let key = (candidate.address, candidate.protocol);
            if queue.try_send(candidate).is_err() {
                r.attempted.remove(&key);
            }
        }
    }
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
                .filter(|(_, v)| v["product"] == candidate.product)
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

/// Browse even with a loopback-only Web listener. Advertising a Pealayer
/// endpoint still requires an actual LAN listener on that address.
pub fn start(listeners: &[std::net::TcpListener], identity: &str) {
    if listeners.is_empty() {
        return;
    }
    let bound: Vec<_> = listeners
        .iter()
        .filter_map(|s| s.local_addr().ok())
        .filter(|a| a.is_ipv4() && !a.ip().is_loopback())
        .collect();
    let addresses: Vec<Ipv4Addr> = crate::network::discover_bind_targets()
        .into_iter()
        .filter_map(|a| a.value.parse::<Ipv4Addr>().ok())
        .filter(|ip| ip.is_private() || ip.is_link_local())
        .collect();
    if addresses.is_empty() {
        return;
    }
    let advertised = bound
        .first()
        .map(|listener| {
            let port = listener.port();
            let advertised: Vec<_> = addresses
                .iter()
                .copied()
                .filter(|ip| {
                    bound.iter().any(|socket| {
                        socket.port() == port
                            && (socket.ip().is_unspecified() || socket.ip() == IpAddr::V4(*ip))
                    })
                })
                .collect();
            (port, advertised)
        })
        .filter(|(_, addresses)| !addresses.is_empty());
    if let (Some((port, advertised)), Ok(mut r)) = (&advertised, registry().lock()) {
        r.own
            .extend(advertised.iter().map(|ip| SocketAddr::from((*ip, *port))));
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
    start_mdns(advertised.as_ref(), &name, &id, tx.clone());
    start_ssdp_search(&addresses, tx.clone());
    if let Some((port, advertised)) = advertised {
        start_ssdp_responder(&advertised, port, name.clone(), id.clone());
        start_mndp(advertised, port, name, id, tx);
    }
}
fn start_mdns(
    advertised: Option<&(u16, Vec<Ipv4Addr>)>,
    name: &str,
    id: &str,
    queue: mpsc::SyncSender<Candidate>,
) {
    let result = (|| -> Result<(), String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        let monitor = daemon.monitor().map_err(|e| e.to_string())?;
        if let Some((port, addresses)) = advertised {
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
                *port,
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
        }
        thread::spawn(move || {
            while let Ok(event) = monitor.recv() {
                if let DaemonEvent::Error(error) = event {
                    fault("mdns", error);
                } else if matches!(event, DaemonEvent::Announce(..)) {
                    if let Ok(mut r) = registry().lock() {
                        r.advertising.insert("dns-sd".into());
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
                    if matches!(event, ServiceEvent::SearchStarted(_)) {
                        active("mdns");
                        active("dns-sd");
                    }
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
    let mut joined = false;
    let mut last_error = None;
    for ip in addresses {
        match socket.join_multicast_v4(&group, ip) {
            Ok(()) => joined = true,
            Err(error) => {
                log::warn!("Discovery multicast join on {ip}: {error}");
                last_error = Some(error);
            }
        }
    }
    if !joined && let Some(error) = last_error {
        return Err(error);
    }
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;
    Ok(socket)
}

fn idle_udp_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::ConnectionReset
    )
}

fn ssdp_search_socket(ip: Ipv4Addr) -> std::io::Result<UdpSocket> {
    let socket = UdpSocket::bind((ip, 0))?;
    socket2::SockRef::from(&socket).set_multicast_if_v4(&ip)?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    Ok(socket)
}

fn start_ssdp_search(addresses: &[Ipv4Addr], queue: mpsc::SyncSender<Candidate>) {
    let mut last_error = None;
    let mut started = false;
    for &ip in addresses {
        let socket = match ssdp_search_socket(ip) {
            Ok(socket) => socket,
            Err(error) => {
                log::warn!("SSDP search socket on {ip}: {error}");
                last_error = Some(error);
                continue;
            }
        };
        started = true;
        let queue = queue.clone();
        thread::spawn(move || {
            let mut last = Instant::now() - Duration::from_secs(30);
            let mut bytes = [0u8; 4096];
            loop {
                if last.elapsed() >= Duration::from_secs(30) {
                    match socket.send_to(SSDP_SEARCH, (SSDP_GROUP, SSDP_PORT)) {
                        Ok(_) => active("ssdp"),
                        Err(error) => log::warn!("SSDP search on {ip}: {error}"),
                    }
                    last = Instant::now();
                }
                match socket.recv_from(&mut bytes) {
                    Ok((len, source)) if last.elapsed() <= Duration::from_secs(3) => {
                        if let Some(candidate) = parse_ssdp(&bytes[..len], source) {
                            offer(&queue, candidate);
                        }
                    }
                    Ok(_) => {}
                    Err(error) if idle_udp_error(&error) => {}
                    Err(error) => {
                        log::warn!("SSDP search receive on {ip}: {error}");
                        return;
                    }
                }
            }
        });
    }
    if !started && let Some(error) = last_error {
        fault("ssdp", error);
    }
}

fn is_ssdp_search(request: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(request) else {
        return false;
    };
    let mut lines = text.lines();
    if lines.next() != Some("M-SEARCH * HTTP/1.1") {
        return false;
    }
    let (mut discover, mut target) = (false, false);
    for (name, value) in lines.map_while(|line| line.split_once(':')) {
        if name.eq_ignore_ascii_case("man") {
            discover = value.trim().eq_ignore_ascii_case("\"ssdp:discover\"");
        } else if name.eq_ignore_ascii_case("st") {
            let value = value.trim();
            target = value.eq_ignore_ascii_case("ssdp:all") || value == PEALAYER_TYPE;
        }
    }
    discover && target
}

fn ssdp_reply_socket(source: SocketAddr, advertised: &[Ipv4Addr]) -> Option<(UdpSocket, Ipv4Addr)> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(source).ok()?;
    let IpAddr::V4(ip) = socket.local_addr().ok()?.ip() else {
        return None;
    };
    advertised.contains(&ip).then_some((socket, ip))
}

fn start_ssdp_responder(advertised: &[Ipv4Addr], port: u16, name: String, id: String) {
    let socket = match udp_socket(SSDP_PORT, SSDP_GROUP, advertised) {
        Ok(socket) => socket,
        Err(error) => {
            fault("ssdp-advertising", error);
            return;
        }
    };
    if let Ok(mut r) = registry().lock() {
        r.advertising.insert("ssdp".into());
    }
    let advertised = advertised.to_vec();
    thread::spawn(move || {
        let mut replies = BTreeMap::new();
        let mut bytes = [0u8; 4096];
        loop {
            let (len, source) = match socket.recv_from(&mut bytes) {
                Ok(packet) => packet,
                Err(error) if idle_udp_error(&error) => continue,
                Err(error) => {
                    fault("ssdp-advertising", error);
                    if let Ok(mut r) = registry().lock() {
                        r.advertising.remove("ssdp");
                    }
                    return;
                }
            };
            if !local_source(source) || !is_ssdp_search(&bytes[..len]) {
                continue;
            }
            replies.retain(|_, at: &mut Instant| at.elapsed() < Duration::from_secs(1));
            if replies.len() >= 128
                || replies.contains_key(&source)
                || replies
                    .keys()
                    .filter(|key: &&SocketAddr| key.ip() == source.ip())
                    .count()
                    >= 4
            {
                continue;
            }
            if let Some((reply, ip)) = ssdp_reply_socket(source, &advertised) {
                replies.insert(source, Instant::now());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=60\r\nEXT:\r\nST: {PEALAYER_TYPE}\r\nUSN: uuid:{id}::{PEALAYER_TYPE}\r\nLOCATION: http://{ip}:{port}/api/discovery\r\nX-Digitalogic-Product: pealayer\r\nX-Digitalogic-Name: {name}\r\n\r\n"
                );
                let _ = reply.send(response.as_bytes());
            }
        }
    });
}

fn start_mndp(
    addresses: Vec<Ipv4Addr>,
    port: u16,
    name: String,
    id: String,
    queue: mpsc::SyncSender<Candidate>,
) {
    let group = Ipv4Addr::new(224, 0, 0, 88);
    let udp_port = 5678;
    let protocol = "mndp";
    let socket = match udp_socket(udp_port, group, &addresses) {
        Ok(s) => s,
        Err(e) => {
            fault(protocol, e);
            return;
        }
    };
    if let Ok(mut r) = registry().lock() {
        r.active.insert(protocol.into());
        r.advertising.insert(protocol.into());
    }
    thread::spawn(move || {
        let mut bytes = [0u8; 4096];
        let mut last = Instant::now() - Duration::from_secs(31);
        let mut sequence = 0u16;
        let mut replies: BTreeMap<IpAddr, Instant> = BTreeMap::new();
        loop {
            if last.elapsed() >= Duration::from_secs(30) {
                let probe = [0; 4];
                let _ = socket.send_to(&probe, (group, udp_port));
                let _ = socket.send_to(&probe, (Ipv4Addr::BROADCAST, udp_port));
                let packet = encode_mndp(&name, &id, port, sequence);
                sequence = sequence.wrapping_add(1);
                let _ = socket.send_to(&packet, (Ipv4Addr::BROADCAST, udp_port));
                last = Instant::now();
            }
            let (length, source) = match socket.recv_from(&mut bytes) {
                Ok(v) => v,
                Err(e) if idle_udp_error(&e) => continue,
                Err(e) => {
                    fault(protocol, e);
                    return;
                }
            };
            if !local_source(source) {
                continue;
            }
            let request = &bytes[..length];
            if request == [0; 4] {
                replies.retain(|_, at| at.elapsed() < Duration::from_secs(1));
                if replies.contains_key(&source.ip()) || replies.len() >= 128 {
                    continue;
                }
                replies.insert(source.ip(), Instant::now());
                let response = encode_mndp(&name, &id, port, sequence);
                let _ = socket.send_to(&response, source);
            } else if let Some(candidate) = parse_mndp(request, source) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_can_be_verified_over_both_protocols_in_either_order() {
        for protocols in [["ssdp", "dns-sd"], ["dns-sd", "ssdp"]] {
            let mut registry = Registry::default();
            let now = Instant::now();
            let mut candidate = Candidate {
                address: ([192, 168, 1, 20], 8080).into(),
                name: "Pealayer".into(),
                product: "pealayer".into(),
                protocol: protocols[0],
            };
            assert!(registry.admit(&candidate, now));
            assert!(!registry.admit(&candidate, now));
            candidate.protocol = protocols[1];
            assert!(registry.admit(&candidate, now));
            registry.own.insert(candidate.address);
            assert!(!registry.admit(&candidate, now + Duration::from_secs(31)));
        }
    }

    #[test]
    fn ssdp_search_validates_exact_target_and_discovery_headers() {
        assert!(is_ssdp_search(SSDP_SEARCH));
        let targeted =
            format!("M-SEARCH * HTTP/1.1\r\nMan: \"ssdp:discover\"\r\nSt: {PEALAYER_TYPE}\r\n\r\n");
        assert!(is_ssdp_search(targeted.as_bytes()));
        assert!(!is_ssdp_search(targeted.replace("St:", "X-ST:").as_bytes()));
        assert!(!is_ssdp_search(
            targeted
                .replace("\"ssdp:discover\"", "\"anything\"")
                .as_bytes()
        ));
    }

    #[test]
    fn ssdp_responses_use_an_advertised_route_address() {
        let receiver = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let destination = receiver.local_addr().unwrap();
        assert!(ssdp_reply_socket(destination, &[Ipv4Addr::new(192, 168, 1, 2)]).is_none());
        let (reply, ip) = ssdp_reply_socket(destination, &[Ipv4Addr::LOCALHOST]).unwrap();
        let message = format!(
            "HTTP/1.1 200 OK\r\nST: {PEALAYER_TYPE}\r\nLOCATION: http://{ip}:8080/api/discovery\r\n\r\n"
        );
        reply.send(message.as_bytes()).unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut packet = [0; 256];
        let (len, source) = receiver.recv_from(&mut packet).unwrap();
        assert_eq!(
            parse_ssdp(&packet[..len], source).unwrap().address,
            ([127, 0, 0, 1], 8080).into()
        );
        assert!(parse_ssdp(&packet[..len], ([192, 168, 1, 2], source.port()).into()).is_none());
    }
}
