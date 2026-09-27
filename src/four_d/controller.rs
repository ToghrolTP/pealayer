use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use crate::four_d::protocol::Command;

pub const DEFAULT_ENDPOINT: &str = "pccontroller://127.0.0.1:8787";
const CAPABILITY_PWM: u32 = 1 << 2;
const CAPABILITY_RELAY_MOTION: u32 = 1 << 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareOutput {
    pub id: u8,
    pub key: String,
    pub name: String,
    pub role: String,
    pub control: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareCapabilities {
    pub board_connected: bool,
    pub board_name: String,
    pub capability_bits: u32,
    pub active_relays: u8,
    pub relays: Vec<HardwareOutput>,
    pub pwm_channels: Vec<HardwareOutput>,
    pub macros: Vec<HardwareMacro>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareMacroStep {
    pub at_us: u64,
    pub kind: String,
    pub target: Option<u8>,
    pub value: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareMacro {
    pub name: String,
    pub category: String,
    pub steps: Vec<HardwareMacroStep>,
}

/// Returns the coordinator plus serial ports reported by the current operating
/// system. No example or cross-platform placeholder device is synthesized.
pub fn available_endpoints() -> Vec<String> {
    let mut endpoints = vec![DEFAULT_ENDPOINT.to_string()];
    if let Ok(ports) = serialport::available_ports() {
        let mut direct = ports
            .into_iter()
            .map(|port| format!("direct:{}", port.port_name))
            .collect::<Vec<_>>();
        direct.sort_by_key(|name| name.to_ascii_lowercase());
        direct.dedup();
        endpoints.extend(direct);
    }
    endpoints
}

pub struct ControllerClient {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    next_id: u64,
}

impl ControllerClient {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        Self::connect_with_timeouts(endpoint, Duration::from_secs(2), Duration::from_secs(3))
    }

    fn connect_with_timeouts(
        endpoint: &str,
        connect_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Self, String> {
        let address = normalize_endpoint(endpoint)?;
        let socket = address
            .to_socket_addrs()
            .map_err(|error| format!("resolve PCController endpoint {address}: {error}"))?
            .next()
            .ok_or_else(|| format!("PCController endpoint {address} resolved to no address"))?;
        let writer = TcpStream::connect_timeout(&socket, connect_timeout)
            .map_err(|error| format!("connect to PCController at {address}: {error}"))?;
        writer
            .set_read_timeout(Some(io_timeout))
            .map_err(|error| format!("configure PCController read timeout: {error}"))?;
        writer
            .set_write_timeout(Some(io_timeout))
            .map_err(|error| format!("configure PCController write timeout: {error}"))?;
        writer
            .set_nodelay(true)
            .map_err(|error| format!("configure PCController TCP stream: {error}"))?;
        let reader = BufReader::new(
            writer
                .try_clone()
                .map_err(|error| format!("clone PCController TCP stream: {error}"))?,
        );
        let mut client = Self {
            writer,
            reader,
            next_id: 1,
        };
        client.call("controller.ping", json!({}))?;
        Ok(client)
    }

    pub fn is_reachable(endpoint: &str, timeout: Duration) -> bool {
        Self::connect_with_timeouts(endpoint, timeout, timeout).is_ok()
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        serde_json::to_writer(&mut self.writer, &request)
            .map_err(|error| format!("encode PCController JSON-RPC request: {error}"))?;
        self.writer
            .write_all(b"\n")
            .and_then(|_| self.writer.flush())
            .map_err(|error| format!("write PCController JSON-RPC request: {error}"))?;

        loop {
            let mut line = String::new();
            let read = self
                .reader
                .read_line(&mut line)
                .map_err(|error| format!("read PCController JSON-RPC response: {error}"))?;
            if read == 0 {
                return Err("PCController closed the JSON-RPC connection".to_string());
            }
            let response: Value = serde_json::from_str(line.trim())
                .map_err(|error| format!("decode PCController JSON-RPC response: {error}"))?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error").filter(|value| !value.is_null()) {
                return Err(format!("PCController JSON-RPC error: {error}"));
            }
            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    pub fn send_command(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::Ping => {
                self.call("controller.ping", json!({}))?;
            }
            Command::RelaySet { id, state } => {
                self.call(
                    "controller.command.execute",
                    json!({"command": format!("relay {id} {}", if state { "on" } else { "off" })}),
                )?;
            }
            Command::PwmSet { channel, value } => {
                let value = (u32::from(value) * 4095 / 255) as u16;
                self.call(
                    "controller.pwm.set",
                    json!({"channel": channel, "value": value}),
                )?;
            }
            Command::AllOff => {
                self.call(
                    "controller.command.execute",
                    json!({"command": "relay off"}),
                )?;
                self.call("controller.pwm.off", json!({}))?;
            }
        }
        Ok(())
    }

    pub fn hardware_capabilities(&mut self) -> Result<HardwareCapabilities, String> {
        let snapshot = self.call("controller.snapshot", json!({}))?;
        let peripherals = self.call("controller.peripherals.get", json!({}))?;
        Ok(parse_hardware_capabilities(&snapshot, &peripherals))
    }
}

fn parse_hardware_capabilities(snapshot: &Value, catalog: &Value) -> HardwareCapabilities {
    let board_connected = snapshot
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let capability_bits = snapshot
        .pointer("/hello/capabilities")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let board_name = snapshot
        .pointer("/hello/name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let active_relays = snapshot
        .pointer("/status/active_relays")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8;
    let custom_names = catalog
        .get("peripheral_names")
        .and_then(Value::as_object);

    let mut outputs = catalog
        .get("peripherals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let key = entry.get("key")?.as_str()?.to_string();
            let id = entry.get("index")?.as_u64().and_then(|id| u8::try_from(id).ok())?;
            let name = custom_names
                .and_then(|names| names.get(&key))
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .or_else(|| entry.get("default_name").and_then(Value::as_str))?
                .to_string();
            Some((
                entry.get("kind")?.as_str()?.to_string(),
                HardwareOutput {
                    id,
                    key,
                    name,
                    role: entry
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    control: entry
                        .get("control")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                },
            ))
        })
        .collect::<Vec<_>>();
    outputs.sort_by_key(|(_, output)| output.id);

    let relays = if board_connected && capability_bits & CAPABILITY_RELAY_MOTION != 0 {
        outputs
            .iter()
            .filter(|(kind, output)| {
                kind == "relay" && output.control == "relay" && (1..=8).contains(&output.id)
            })
            .map(|(_, output)| output.clone())
            .collect()
    } else {
        Vec::new()
    };
    let pwm_channels = if board_connected && capability_bits & CAPABILITY_PWM != 0 {
        outputs
            .iter()
            .filter(|(kind, output)| {
                kind == "pwm" && output.control == "pwm-user" && output.id < 16
            })
            .map(|(_, output)| output.clone())
            .collect()
    } else {
        Vec::new()
    };
    let macros = snapshot
        .pointer("/macros/library")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let name = entry.get("name")?.as_str()?.to_string();
            let category = entry
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("PCController")
                .to_string();
            let steps = entry
                .get("steps")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|step| {
                    Some(HardwareMacroStep {
                        at_us: step.get("at_us").and_then(Value::as_u64).unwrap_or(0),
                        kind: step.get("kind")?.as_str()?.to_string(),
                        target: step
                            .get("target")
                            .and_then(Value::as_u64)
                            .and_then(|target| u8::try_from(target).ok()),
                        value: step
                            .get("value")
                            .and_then(Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok()),
                    })
                })
                .collect();
            Some(HardwareMacro {
                name,
                category,
                steps,
            })
        })
        .collect();

    HardwareCapabilities {
        board_connected,
        board_name,
        capability_bits,
        active_relays,
        relays,
        pwm_channels,
        macros,
    }
}

pub fn normalize_endpoint(endpoint: &str) -> Result<&str, String> {
    let address = endpoint
        .strip_prefix("pccontroller://")
        .or_else(|| endpoint.strip_prefix("tcp://"))
        .unwrap_or(endpoint)
        .trim();
    if address.is_empty() || !address.contains(':') {
        return Err(format!("invalid PCController endpoint: {endpoint}"));
    }
    Ok(address)
}

pub fn is_controller_endpoint(endpoint: &str) -> bool {
    endpoint.starts_with("pccontroller://") || endpoint.starts_with("tcp://")
}

pub fn direct_serial_name(endpoint: &str) -> &str {
    endpoint.strip_prefix("direct:").unwrap_or(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn normalizes_controller_endpoints() {
        assert_eq!(
            normalize_endpoint(DEFAULT_ENDPOINT).unwrap(),
            "127.0.0.1:8787"
        );
        assert_eq!(normalize_endpoint("tcp://host:9000").unwrap(), "host:9000");
        assert!(normalize_endpoint("missing-port").is_err());
    }

    #[test]
    fn discovered_endpoints_never_invent_serial_devices() {
        let endpoints = available_endpoints();
        assert_eq!(endpoints.first().map(String::as_str), Some(DEFAULT_ENDPOINT));
        assert!(
            endpoints
                .iter()
                .skip(1)
                .all(|endpoint| endpoint.starts_with("direct:") && endpoint.len() > 7)
        );
    }

    #[test]
    fn exchanges_correlated_ndjson_rpc() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for _ in 0..2 {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(line.trim()).unwrap();
                let response = json!({"jsonrpc":"2.0","id":request["id"],"result":{"ok":true}});
                writeln!(stream, "{response}").unwrap();
            }
        });

        let mut client = ControllerClient::connect(&format!("pccontroller://{address}")).unwrap();
        let result = client.call("controller.snapshot", json!({})).unwrap();
        assert_eq!(result["ok"], true);
        server.join().unwrap();
    }

    #[test]
    fn capability_catalog_uses_advertised_outputs_and_custom_names() {
        let snapshot = json!({
            "connected": true,
            "hello": {"name": "Cinema", "capabilities": CAPABILITY_PWM | CAPABILITY_RELAY_MOTION},
            "status": {"active_relays": 16},
            "macros": {"library": [{"name": "Thunder"}]}
        });
        let catalog = json!({
            "peripheral_names": {"relay.5": "Left Air"},
            "peripherals": [
                {"key":"relay.5","kind":"relay","role":"user-output","index":5,"default_name":"User Relay 5","control":"relay"},
                {"key":"pwm.0","kind":"pwm","role":"user-output","index":0,"default_name":"MOSFET 1","control":"pwm-user"},
                {"key":"pwm.15","kind":"pwm","role":"status-blue","index":15,"default_name":"Status blue","control":"role-specific"}
            ]
        });
        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert!(parsed.board_connected);
        assert_eq!(parsed.board_name, "Cinema");
        assert_eq!(parsed.relays[0].name, "Left Air");
        assert_eq!(parsed.pwm_channels.len(), 1);
        assert_eq!(parsed.macros[0].name, "Thunder");
    }

    #[test]
    fn disconnected_snapshot_exposes_no_live_controls() {
        let snapshot = json!({"connected": false, "hello": {"capabilities": u32::MAX}});
        let catalog = json!({"peripherals": [{"key":"relay.5","kind":"relay","role":"user-output","index":5,"default_name":"Relay","control":"relay"}]});
        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert!(parsed.relays.is_empty());
        assert!(parsed.pwm_channels.is_empty());
    }
}
