use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use crate::four_d::embedded_host::EmbeddedHost;
use crate::four_d::protocol::Command;

pub const DEFAULT_ENDPOINT: &str = "pccontroller://127.0.0.1:8787";
const CAPABILITY_PWM: u32 = 1 << 2;
const CAPABILITY_RELAY_MOTION: u32 = 1 << 3;
const CAPABILITY_RF: u32 = 1 << 4;
const CAPABILITY_SEGMENTS: u32 = 1 << 5;
const CAPABILITY_LCD: u32 = 1 << 6;
const CAPABILITY_ADDRESSABLE_LED: u32 = 1 << 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareOutput {
    pub id: u8,
    pub key: String,
    pub name: String,
    pub role: String,
    pub control: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareAction {
    pub id: String,
    pub verb: String,
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareControl {
    pub key: String,
    pub kind: String,
    pub order: u16,
    pub name: String,
    pub default_name: String,
    pub control: String,
    pub icon: String,
    pub group: String,
    pub actions: Vec<HardwareAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareBoardProfile {
    pub key: String,
    pub board_identity: String,
    pub identity_source: String,
    pub identity_stable: bool,
    pub mode: String,
    pub configured: bool,
    pub attached: bool,
    pub revision: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareCapabilities {
    pub board_connected: bool,
    pub board_name: String,
    pub board_identity: HardwareBoardIdentity,
    pub port: HardwarePort,
    pub host_instance_id: String,
    pub capability_bits: u32,
    pub active_relays: std::collections::BTreeSet<u8>,
    pub board_profile: Option<HardwareBoardProfile>,
    pub controls: Vec<HardwareControl>,
    pub peripheral_names: std::collections::BTreeMap<String, String>,
    pub relays: Vec<HardwareOutput>,
    pub pwm_channels: Vec<HardwareOutput>,
    pub peripherals: Vec<HardwareOutput>,
    pub supports_rf_transmit: bool,
    pub supports_segment_display: bool,
    pub supports_lcd_display: bool,
    pub supports_addressable_led: bool,
    pub status_led: Option<HardwareStatusLed>,
    pub status_led_revision: u64,
    pub settings: Option<HardwareBoardSettings>,
    pub front_panel: Option<HardwareFrontPanel>,
    pub telemetry: HardwareTelemetry,
    pub warnings: Vec<HardwareWarning>,
    pub strip_effects: Vec<HardwareStripEffect>,
    pub macros: Vec<HardwareMacro>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareBoardIdentity {
    pub board_kind: u64,
    pub identity_schema: u64,
    pub build_hash: Option<u64>,
    pub build_timestamp: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwarePort {
    pub name: String,
    pub product: String,
    pub vid: String,
    pub pid: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareBoardSettings {
    pub silent: bool,
    pub light_mode: u8,
    pub on_brightness: u8,
    pub off_brightness: u8,
    pub display_brightness: u8,
    pub status_brightness: u8,
    pub output_persistence: u8,
    pub stream_period_ms: u64,
    pub default_page: u8,
    pub motion_break_ms: u64,
    pub persisted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareFrontPanel {
    pub raw_segments: Vec<u8>,
    pub brightness: u8,
    pub blink: bool,
    pub pressed_keys: u8,
    pub menu_page: u8,
    pub program_mode: u8,
    pub lcd_available: bool,
    pub lcd_address: u8,
    pub lcd_line_1: String,
    pub lcd_line_2: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ControllerEndpointHealth {
    pub reachable: bool,
    pub board_connected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareStatusLed {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub brightness: u8,
    pub effect: u8,
    pub condition: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareTelemetry {
    pub supply_mv: Option<i32>,
    pub bus_mv: Option<i32>,
    pub current_ma: Option<i32>,
    pub power_mw: Option<i32>,
    pub led_temperature_centi_c: Option<i32>,
    pub audio_temperature_centi_c: Option<i32>,
    pub pwm_channel: Option<u8>,
    pub pwm_value: Option<u16>,
    pub door_open: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareWarning {
    pub code: String,
    pub severity: String,
    pub message: String,
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
    pub id: u64,
    pub name: String,
    pub category: String,
    pub mode: String,
    pub duration_ms: u64,
    pub steps: Vec<HardwareMacroStep>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareStripEffect {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub pattern: String,
    pub engine: String,
    pub editable: bool,
    pub default_fps: Option<u8>,
    pub default_duration_ms: Option<u64>,
    pub default_pixels: Option<u16>,
    pub minimum_fps: Option<u8>,
    pub maximum_fps: Option<u8>,
    pub minimum_pixels: Option<u16>,
    pub maximum_pixels: Option<u16>,
}

impl HardwareCapabilities {
    /// Applies the typed status payload pushed by `controller.status`.
    /// Static capability/catalog data stays intact; only live board state is
    /// replaced. Returning `true` lets callers repaint only for real changes.
    pub(crate) fn apply_status_notification(&mut self, params: &Value) -> bool {
        let Some(status) = params.get("status").filter(|value| value.is_object()) else {
            return false;
        };
        let was_board_connected = self.board_connected;
        let before_relays = self.active_relays.clone();
        let before_telemetry = self.telemetry.clone();

        self.board_connected = true;

        if let Some(mask) = status.get("active_relays").and_then(Value::as_u64) {
            self.active_relays = active_relays_from_mask(&self.relays, mask);
        }
        self.telemetry = telemetry_from_status(status, self.board_connected);

        !was_board_connected
            || self.active_relays != before_relays
            || self.telemetry != before_telemetry
    }

    /// Applies changed-only state events that are not part of telemetry
    /// polling. PCController publishes the physical LED result after its MCU
    /// compositor has applied priority, brightness, and procedural effects.
    pub(crate) fn apply_state_notification(&mut self, event: &Value) -> bool {
        match event.get("kind").and_then(Value::as_str) {
            Some("status_led.changed") => {
                let Some(metadata) = event.get("metadata").filter(|value| value.is_object()) else {
                    return false;
                };
                let byte = |name| {
                    metadata
                        .get(name)
                        .and_then(value_as_u64)
                        .and_then(|value| u8::try_from(value).ok())
                };
                let (
                    Some(red),
                    Some(green),
                    Some(blue),
                    Some(brightness),
                    Some(effect),
                    Some(condition),
                ) = (
                    byte("red"),
                    byte("green"),
                    byte("blue"),
                    byte("brightness"),
                    byte("effect"),
                    byte("condition"),
                )
                else {
                    return false;
                };
                let revision = metadata.get("revision").and_then(value_as_u64).unwrap_or(0);
                if revision > 0
                    && self.status_led_revision > 0
                    && revision <= self.status_led_revision
                {
                    return false;
                }
                let next = HardwareStatusLed {
                    red,
                    green,
                    blue,
                    brightness,
                    effect,
                    condition,
                };
                if self.status_led.as_ref() == Some(&next)
                    && (revision == 0 || revision == self.status_led_revision)
                {
                    false
                } else {
                    self.status_led = Some(next);
                    if revision > 0 {
                        self.status_led_revision = revision;
                    }
                    true
                }
            }
            Some("relay") => {
                let Some(device) = event.get("device").filter(|value| value.is_object()) else {
                    return false;
                };
                // `relay_mask` is omitted by Go's JSON encoder when all relays
                // are off, so a relay event with no field authoritatively means 0.
                let mask = device
                    .get("relay_mask")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let next = active_relays_from_mask(&self.relays, mask);
                if next == self.active_relays {
                    false
                } else {
                    self.active_relays = next;
                    true
                }
            }
            _ => false,
        }
    }

    pub(crate) fn preserve_newer_live_led_from(&mut self, current: &Self) {
        let same_host = self.host_instance_id.is_empty()
            || current.host_instance_id.is_empty()
            || self.host_instance_id == current.host_instance_id;
        if self.board_connected
            && same_host
            && current.status_led.is_some()
            && current.status_led_revision > 0
            && current.status_led_revision >= self.status_led_revision
        {
            self.status_led = current.status_led.clone();
            self.status_led_revision = current.status_led_revision;
        }
    }

    pub(crate) fn mark_board_disconnected(&mut self) -> bool {
        let changed = self.board_connected
            || !self.active_relays.is_empty()
            || self.status_led.is_some()
            || self.telemetry != HardwareTelemetry::default();
        self.board_connected = false;
        self.active_relays.clear();
        self.status_led = None;
        self.telemetry = HardwareTelemetry::default();
        changed
    }
}

fn value_as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
}

fn active_relays_from_mask(relays: &[HardwareOutput], mask: u64) -> std::collections::BTreeSet<u8> {
    relays
        .iter()
        .filter(|relay| {
            relay.id > 0
                && relay.id <= u64::BITS as u8
                && mask & (1_u64 << u32::from(relay.id - 1)) != 0
        })
        .map(|relay| relay.id)
        .collect()
}

fn telemetry_from_status(status: &Value, board_connected: bool) -> HardwareTelemetry {
    HardwareTelemetry {
        supply_mv: status
            .get("ina219_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| status.get("supply_mv").and_then(Value::as_i64).unwrap_or(0) as i32),
        bus_mv: status
            .get("ina219_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| status.get("bus_mv").and_then(Value::as_i64).unwrap_or(0) as i32),
        current_ma: status
            .get("ina219_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| {
                status
                    .get("current_ma")
                    .and_then(Value::as_i64)
                    .unwrap_or(0) as i32
            }),
        power_mw: status
            .get("ina219_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| status.get("power_mw").and_then(Value::as_i64).unwrap_or(0) as i32),
        led_temperature_centi_c: status
            .get("temperature_led_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| {
                status
                    .get("temperature_led_centi_c")
                    .and_then(Value::as_i64)
                    .unwrap_or(0) as i32
            }),
        audio_temperature_centi_c: status
            .get("temperature_bt_audio_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| {
                status
                    .get("temperature_bt_audio_centi_c")
                    .and_then(Value::as_i64)
                    .unwrap_or(0) as i32
            }),
        pwm_channel: status
            .get("pwm_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| {
                status
                    .get("pwm_channel")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u8
            }),
        pwm_value: status
            .get("pwm_available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| status.get("pwm_value").and_then(Value::as_u64).unwrap_or(0) as u16),
        door_open: board_connected.then(|| {
            status
                .get("door_open")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        }),
    }
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

enum ControllerBackend {
    Embedded(EmbeddedHost),
    Tcp {
        writer: TcpStream,
        reader: BufReader<TcpStream>,
        next_id: u64,
    },
}

pub struct ControllerClient {
    backend: ControllerBackend,
}

impl ControllerClient {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        Self::connect_with_timeouts(endpoint, Duration::from_secs(2), Duration::from_secs(3))
    }

    /// Prefer the packaged in-process Host for the canonical local endpoint.
    /// If the library is absent or another coordinator already owns the Host,
    /// fall back to the requested external coordinator without opening UART.
    pub fn connect_preferred(endpoint: &str) -> Result<Self, String> {
        if is_default_controller_endpoint(endpoint) {
            match EmbeddedHost::discover_and_start() {
                Ok(host) => {
                    return Ok(Self {
                        backend: ControllerBackend::Embedded(host),
                    });
                }
                Err(embedded_error) => {
                    return Self::connect(endpoint).map_err(|external_error| {
                        format!(
                            "embedded PCController unavailable ({embedded_error}); external coordinator unavailable ({external_error})"
                        )
                    });
                }
            }
        }
        Self::connect(endpoint)
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
            backend: ControllerBackend::Tcp {
                writer,
                reader,
                next_id: 1,
            },
        };
        client.call("controller.ping", json!({}))?;
        Ok(client)
    }

    pub fn is_reachable(endpoint: &str, timeout: Duration) -> bool {
        Self::connect_with_timeouts(endpoint, timeout, timeout).is_ok()
    }

    pub fn endpoint_health(endpoint: &str, timeout: Duration) -> ControllerEndpointHealth {
        let Ok(mut client) = Self::connect_with_timeouts(endpoint, timeout, timeout) else {
            return ControllerEndpointHealth::default();
        };
        let snapshot = client.call("controller.snapshot", json!({})).ok();
        let catalog = client.call("controller.peripherals.get", json!({})).ok();
        let transport_has_board = snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.get("connected").and_then(Value::as_bool))
            .unwrap_or(false);
        let profile = catalog
            .as_ref()
            .and_then(|catalog| catalog.get("board_profile"));
        let board_connected = transport_has_board
            && profile
                .and_then(|profile| profile.get("attached"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            && profile
                .and_then(|profile| profile.get("configured"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
        ControllerEndpointHealth {
            reachable: true,
            board_connected,
        }
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let (writer, reader, next_id) = match &mut self.backend {
            ControllerBackend::Embedded(host) => return host.call(method, params),
            ControllerBackend::Tcp {
                writer,
                reader,
                next_id,
            } => (writer, reader, next_id),
        };

        let id = *next_id;
        *next_id = next_id.wrapping_add(1).max(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        serde_json::to_writer(&mut *writer, &request)
            .map_err(|error| format!("encode PCController JSON-RPC request: {error}"))?;
        writer
            .write_all(b"\n")
            .and_then(|_| writer.flush())
            .map_err(|error| format!("write PCController JSON-RPC request: {error}"))?;

        loop {
            let mut line = String::new();
            let read = reader
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

    pub fn transport_description(&self) -> String {
        match &self.backend {
            ControllerBackend::Embedded(_) => "embedded".to_string(),
            ControllerBackend::Tcp { .. } => "external:tcp".to_string(),
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
        let mut capabilities = parse_hardware_capabilities(&snapshot, &peripherals);
        if capabilities.supports_addressable_led && capabilities.strip_effects.is_empty() {
            let catalog = self
                .call(
                    "controller.command.execute",
                    json!({"command": "strip effect list"}),
                )
                .ok()
                .and_then(|result| {
                    result
                        .get("output")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .and_then(|output| serde_json::from_str::<Value>(&output).ok());
            if let Some(catalog) = catalog {
                capabilities.strip_effects = parse_strip_effects(&catalog);
            }
        }
        Ok(capabilities)
    }
}

/// Selects the startup endpoint without inventing hardware. A configured
/// coordinator with a live board wins, followed by the canonical local
/// coordinator when it owns a live board. A healthy coordinator without a
/// board remains useful for discovery and reconnect. If none is running yet,
/// the canonical local endpoint is returned so the engine can start the
/// bundled host or keep retrying the external service.
pub fn select_autoconnect_endpoint(configured_endpoint: &str, timeout: Duration) -> Option<String> {
    select_autoconnect_endpoint_with(configured_endpoint, &available_endpoints(), |endpoint| {
        ControllerClient::endpoint_health(endpoint, timeout)
    })
}

fn select_autoconnect_endpoint_with<F>(
    configured_endpoint: &str,
    available: &[String],
    mut health: F,
) -> Option<String>
where
    F: FnMut(&str) -> ControllerEndpointHealth,
{
    let configured_endpoint = configured_endpoint.trim();
    let configured_is_controller = is_controller_endpoint(configured_endpoint);
    let configured = configured_is_controller.then(|| configured_endpoint.to_string());
    let default = DEFAULT_ENDPOINT.to_string();

    let configured_health = configured.as_deref().map(&mut health).unwrap_or_default();
    let default_health = if configured.as_deref() == Some(DEFAULT_ENDPOINT) {
        configured_health
    } else {
        health(DEFAULT_ENDPOINT)
    };

    if configured_health.board_connected {
        return configured;
    }
    if default_health.board_connected {
        return Some(default);
    }
    if configured_health.reachable {
        return configured;
    }
    if default_health.reachable {
        return Some(default);
    }

    if !configured_is_controller
        && !configured_endpoint.is_empty()
        && available
            .iter()
            .any(|candidate| candidate == configured_endpoint)
    {
        return Some(configured_endpoint.to_string());
    }

    // This is intentionally selected even before it is reachable: the engine
    // owns bundled-host startup and retry behavior for the canonical endpoint.
    Some(DEFAULT_ENDPOINT.to_string())
}

pub(crate) fn valid_strip_effect_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn parse_strip_effects(value: &Value) -> Vec<HardwareStripEffect> {
    let entries = value
        .as_array()
        .or_else(|| value.get("strip_effects").and_then(Value::as_array))
        .or_else(|| value.get("effects").and_then(Value::as_array));
    let Some(entries) = entries else {
        return Vec::new();
    };
    let byte = |entry: &Value, key: &str| {
        entry
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
    };
    let word = |entry: &Value, key: &str| {
        entry
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
    };
    entries
        .iter()
        .filter_map(|entry| {
            if entry
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind != "strip-stream")
            {
                return None;
            }
            let id = entry.get("id")?.as_str()?.trim();
            if !valid_strip_effect_id(id) {
                return None;
            }
            Some(HardwareStripEffect {
                id: id.to_string(),
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or(id)
                    .to_string(),
                category: entry
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("Lighting")
                    .to_string(),
                description: entry
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                pattern: entry
                    .get("pattern")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .to_string(),
                engine: entry
                    .get("engine")
                    .and_then(Value::as_str)
                    .unwrap_or("host-stream")
                    .to_string(),
                editable: entry
                    .get("editable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                default_fps: byte(entry, "default_fps"),
                default_duration_ms: entry.get("default_duration_ms").and_then(Value::as_u64),
                default_pixels: word(entry, "default_pixels"),
                minimum_fps: byte(entry, "minimum_fps").or_else(|| byte(entry, "min_fps")),
                maximum_fps: byte(entry, "maximum_fps").or_else(|| byte(entry, "max_fps")),
                minimum_pixels: word(entry, "minimum_pixels").or_else(|| word(entry, "min_pixels")),
                maximum_pixels: word(entry, "maximum_pixels").or_else(|| word(entry, "max_pixels")),
            })
        })
        .collect()
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
    let board_identity = HardwareBoardIdentity {
        board_kind: snapshot
            .pointer("/hello/board_kind")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        identity_schema: snapshot
            .pointer("/hello/identity_schema")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        build_hash: snapshot
            .pointer("/hello/build_hash")
            .and_then(Value::as_u64),
        build_timestamp: snapshot
            .pointer("/hello/build_timestamp")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string),
    };
    let port = HardwarePort {
        name: snapshot
            .pointer("/port/name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        product: snapshot
            .pointer("/port/product")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        vid: snapshot
            .pointer("/port/vid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        pid: snapshot
            .pointer("/port/pid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    };
    let host_instance_id = snapshot
        .get("host_instance_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let active_relay_bits = snapshot
        .pointer("/status/active_relays")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let custom_names = catalog.get("peripheral_names").and_then(Value::as_object);
    let peripheral_names = custom_names
        .into_iter()
        .flat_map(|names| names.iter())
        .filter_map(|(key, name)| {
            name.as_str()
                .filter(|name| !name.trim().is_empty())
                .map(|name| (key.clone(), name.to_string()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut outputs = catalog
        .get("peripherals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let key = entry.get("key")?.as_str()?.to_string();
            let id = entry
                .get("index")?
                .as_u64()
                .and_then(|id| u8::try_from(id).ok())?;
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

    let board_profile = catalog
        .get("board_profile")
        .filter(|profile| profile.is_object())
        .map(|profile| HardwareBoardProfile {
            key: profile
                .get("key")
                .or_else(|| profile.get("profile_key"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            board_identity: profile
                .get("board_identity")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            identity_source: profile
                .get("identity_source")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            identity_stable: profile
                .get("identity_stable")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            mode: profile
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            configured: profile
                .get("configured")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            attached: profile
                .get("attached")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            revision: profile
                .get("revision")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    let default_names = catalog
        .get("peripherals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some((
                entry.get("key")?.as_str()?.to_string(),
                entry
                    .get("default_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            ))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut controls = catalog
        .get("controls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let key = entry.get("key")?.as_str()?.to_string();
            let actions = entry
                .get("actions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|action| {
                    if let Some(id) = action.as_str() {
                        return Some(HardwareAction {
                            id: id.to_string(),
                            verb: id.rsplit('.').next().unwrap_or(id).to_string(),
                            name: id.rsplit('.').next().unwrap_or(id).to_string(),
                            icon: String::new(),
                        });
                    }
                    let id = action
                        .get("id")
                        .or_else(|| action.get("action_id"))
                        .and_then(Value::as_str)?
                        .to_string();
                    Some(HardwareAction {
                        verb: action
                            .get("verb")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| id.rsplit('.').next().unwrap_or(&id))
                            .to_string(),
                        name: action
                            .get("name")
                            .or_else(|| action.get("label"))
                            .or_else(|| action.get("verb"))
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| id.rsplit('.').next().unwrap_or(&id))
                            .to_string(),
                        icon: action
                            .get("icon")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        id,
                    })
                })
                .collect();
            Some(HardwareControl {
                kind: entry
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                order: entry
                    .get("order")
                    .and_then(Value::as_u64)
                    .and_then(|order| u16::try_from(order).ok())
                    .unwrap_or(0),
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| peripheral_names.get(&key).map(String::as_str))
                    .or_else(|| default_names.get(&key).map(String::as_str))
                    .unwrap_or(&key)
                    .to_string(),
                default_name: entry
                    .get("default_name")
                    .and_then(Value::as_str)
                    .or_else(|| default_names.get(&key).map(String::as_str))
                    .unwrap_or_default()
                    .to_string(),
                control: entry
                    .get("control")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                icon: entry
                    .get("icon")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                group: entry
                    .get("group")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                actions,
                key,
            })
        })
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.key.cmp(&right.key))
    });

    let relays = if board_connected && capability_bits & CAPABILITY_RELAY_MOTION != 0 {
        outputs
            .iter()
            .filter(|(kind, output)| kind == "relay" && output.control == "relay" && output.id != 0)
            .map(|(_, output)| output.clone())
            .collect()
    } else {
        Vec::new()
    };
    let pwm_channels = if board_connected && capability_bits & CAPABILITY_PWM != 0 {
        outputs
            .iter()
            .filter(|(kind, output)| kind == "pwm" && output.control == "pwm-user")
            .map(|(_, output)| output.clone())
            .collect()
    } else {
        Vec::new()
    };
    let peripherals = if board_connected {
        outputs.iter().map(|(_, output)| output.clone()).collect()
    } else {
        Vec::new()
    };
    let status_led = (board_connected
        && snapshot
            .get("have_status_led")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    .then(|| HardwareStatusLed {
        red: snapshot
            .pointer("/status_led/red")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        green: snapshot
            .pointer("/status_led/green")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        blue: snapshot
            .pointer("/status_led/blue")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        brightness: snapshot
            .pointer("/status_led/brightness")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        effect: snapshot
            .pointer("/status_led/effect")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        condition: snapshot
            .pointer("/status_led/condition")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
    });
    let status_led_revision = snapshot
        .get("status_led_revision")
        .and_then(value_as_u64)
        .unwrap_or(0);
    let settings = (board_connected
        && snapshot
            .get("have_settings")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    .then(|| HardwareBoardSettings {
        silent: snapshot
            .pointer("/settings/flags")
            .and_then(Value::as_u64)
            .is_some_and(|flags| flags & 1 != 0),
        light_mode: snapshot
            .pointer("/settings/light_mode")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        on_brightness: snapshot
            .pointer("/settings/on_brightness")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        off_brightness: snapshot
            .pointer("/settings/off_brightness")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        display_brightness: snapshot
            .pointer("/settings/display_brightness")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        status_brightness: snapshot
            .pointer("/settings/status_brightness")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        output_persistence: snapshot
            .pointer("/settings/output_persistence")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        stream_period_ms: snapshot
            .pointer("/settings/stream_period_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        default_page: snapshot
            .pointer("/settings/default_page")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        motion_break_ms: snapshot
            .pointer("/settings/motion_break_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        persisted: snapshot
            .pointer("/settings/persisted")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    });
    let front_panel = (board_connected
        && snapshot
            .get("have_front_panel")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    .then(|| HardwareFrontPanel {
        raw_segments: snapshot
            .pointer("/front_panel/raw_segments")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_u64().and_then(|value| u8::try_from(value).ok()))
            .collect(),
        brightness: snapshot
            .pointer("/front_panel/brightness")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        blink: snapshot
            .pointer("/front_panel/blink")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        pressed_keys: snapshot
            .pointer("/front_panel/pressed_keys")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        menu_page: snapshot
            .pointer("/front_panel/menu_page")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        program_mode: snapshot
            .pointer("/front_panel/program_mode")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        lcd_available: snapshot
            .pointer("/front_panel/lcd_available")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        lcd_address: snapshot
            .pointer("/front_panel/lcd_address")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        lcd_line_1: snapshot
            .pointer("/front_panel/lcd_line_1")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        lcd_line_2: snapshot
            .pointer("/front_panel/lcd_line_2")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    });
    let empty_status = Value::Null;
    let status = snapshot.get("status").unwrap_or(&empty_status);
    let telemetry = telemetry_from_status(status, board_connected);
    let warnings = snapshot
        .get("hardware_problems")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|problem| HardwareWarning {
            code: problem
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("hardware_problem")
                .to_string(),
            severity: problem
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("warning")
                .to_string(),
            message: problem
                .get("description")
                .or_else(|| problem.get("impact"))
                .and_then(Value::as_str)
                .unwrap_or("Hardware requires attention")
                .to_string(),
        })
        .collect();
    let strip_effects = if snapshot.get("effects").is_some() {
        snapshot
            .get("effects")
            .map(parse_strip_effects)
            .unwrap_or_default()
    } else if board_connected && capability_bits & CAPABILITY_ADDRESSABLE_LED != 0 {
        let parsed = parse_strip_effects(catalog);
        if parsed.is_empty() {
            parse_strip_effects(snapshot)
        } else {
            parsed
        }
    } else {
        Vec::new()
    };
    let sequence_entries = snapshot
        .get("effects")
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
        .or_else(|| {
            snapshot
                .pointer("/macros/library")
                .and_then(Value::as_array)
        });
    let macros = sequence_entries
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry
                .get("kind")
                .and_then(Value::as_str)
                .is_none_or(|kind| kind == "sequence")
        })
        .filter_map(|entry| {
            let id = entry
                .get("id")
                .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))?;
            let name = entry.get("name")?.as_str()?.to_string();
            let category = entry
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("PCController")
                .to_string();
            let steps: Vec<HardwareMacroStep> = entry
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
            let mode = entry
                .get("engine")
                .or_else(|| entry.get("mode"))
                .and_then(Value::as_str)
                .unwrap_or("mcu")
                .to_string();
            let duration_ms = entry
                .get("duration_ms")
                .and_then(Value::as_u64)
                .or_else(|| {
                    entry
                        .get("duration_us")
                        .and_then(Value::as_u64)
                        .map(|duration| duration.div_ceil(1_000))
                })
                .unwrap_or_else(|| {
                    steps
                        .iter()
                        .map(|step: &HardwareMacroStep| step.at_us.div_ceil(1_000))
                        .max()
                        .unwrap_or(1)
                })
                .max(1);
            Some(HardwareMacro {
                id,
                name,
                category,
                mode,
                duration_ms,
                steps,
            })
        })
        .collect();

    let active_relays = active_relays_from_mask(&relays, active_relay_bits);

    HardwareCapabilities {
        board_connected,
        board_name,
        board_identity,
        port,
        host_instance_id,
        capability_bits,
        active_relays,
        board_profile,
        controls,
        peripheral_names,
        relays,
        pwm_channels,
        peripherals,
        supports_rf_transmit: board_connected && capability_bits & CAPABILITY_RF != 0,
        supports_segment_display: board_connected && capability_bits & CAPABILITY_SEGMENTS != 0,
        supports_lcd_display: board_connected && capability_bits & CAPABILITY_LCD != 0,
        supports_addressable_led: board_connected
            && capability_bits & CAPABILITY_ADDRESSABLE_LED != 0,
        status_led,
        status_led_revision,
        settings,
        front_panel,
        telemetry,
        warnings,
        strip_effects,
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
    let endpoint = endpoint.trim();
    endpoint.starts_with("pccontroller://")
        || endpoint.starts_with("tcp://")
        // Accept the natural host:port form entered into the custom endpoint
        // field while keeping explicit direct paths and OS filesystem paths on
        // the direct-diagnostic transport.
        || (!endpoint.starts_with("direct:")
            && !endpoint.contains('/')
            && !endpoint.contains('\\')
            && normalize_endpoint(endpoint).is_ok())
}

fn is_default_controller_endpoint(endpoint: &str) -> bool {
    normalize_endpoint(endpoint).ok() == normalize_endpoint(DEFAULT_ENDPOINT).ok()
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
    fn recognizes_custom_controller_addresses_without_misclassifying_device_paths() {
        assert!(is_controller_endpoint("pccontroller://cafe-pc.local:8787"));
        assert!(is_controller_endpoint("tcp://10.0.0.12:8787"));
        assert!(is_controller_endpoint("127.0.0.1:8787"));
        assert!(!is_controller_endpoint("direct:COM4"));
        assert!(!is_controller_endpoint(r"C:\\devices\\controller"));
        assert!(!is_controller_endpoint("/dev/ttyACM0"));
    }

    #[test]
    fn discovered_endpoints_never_invent_serial_devices() {
        let endpoints = available_endpoints();
        assert_eq!(
            endpoints.first().map(String::as_str),
            Some(DEFAULT_ENDPOINT)
        );
        assert!(
            endpoints
                .iter()
                .skip(1)
                .all(|endpoint| endpoint.starts_with("direct:") && endpoint.len() > 7)
        );
    }

    #[test]
    fn autoconnect_prefers_an_attached_board_then_the_canonical_fallback() {
        let selected = select_autoconnect_endpoint_with(
            "pccontroller://cafe-pc.local:8787",
            &[DEFAULT_ENDPOINT.to_string()],
            |endpoint| match endpoint {
                "pccontroller://cafe-pc.local:8787" => ControllerEndpointHealth {
                    reachable: true,
                    board_connected: false,
                },
                DEFAULT_ENDPOINT => ControllerEndpointHealth {
                    reachable: true,
                    board_connected: true,
                },
                _ => ControllerEndpointHealth::default(),
            },
        );
        assert_eq!(selected.as_deref(), Some(DEFAULT_ENDPOINT));

        let selected =
            select_autoconnect_endpoint_with("pccontroller://cafe-pc.local:8787", &[], |_| {
                ControllerEndpointHealth::default()
            });
        assert_eq!(selected.as_deref(), Some(DEFAULT_ENDPOINT));
    }

    #[test]
    fn parses_stable_profile_controls_actions_and_mutable_presentation() {
        let snapshot = json!({
            "connected": true,
            "hello": {"name": "Cinema controller", "capabilities": CAPABILITY_RELAY_MOTION}
        });
        let catalog = json!({
            "board_profile": {
                "key": "cinema-seat-v1",
                "board_identity": "board-42",
                "identity_source": "hardware",
                "identity_stable": true,
                "mode": "cinema-seat-motion",
                "configured": true,
                "attached": true,
                "revision": "profile-7"
            },
            "peripheral_names": {"seat.a": "Left pair"},
            "peripherals": [{
                "key": "seat.a", "kind": "motion", "role": "motion-side",
                "index": 1, "default_name": "Side A motion", "control": "motion"
            }],
            "controls": [{
                "key": "seat.a", "kind": "side", "order": 1,
                "name": "Left pair", "control": "motion", "icon": "seat",
                "group": "auditorium-a",
                "actions": [
                    {"id": "seat.a.up", "verb": "up", "name": "Up", "icon": "arrow-up"},
                    "seat.a.stop"
                ]
            }]
        });

        let capabilities = parse_hardware_capabilities(&snapshot, &catalog);
        assert_eq!(
            capabilities
                .board_profile
                .as_ref()
                .map(|profile| profile.key.as_str()),
            Some("cinema-seat-v1")
        );
        assert_eq!(capabilities.peripheral_names["seat.a"], "Left pair");
        assert_eq!(capabilities.controls.len(), 1);
        let control = &capabilities.controls[0];
        assert_eq!(control.key, "seat.a");
        assert_eq!(control.default_name, "Side A motion");
        assert_eq!(control.icon, "seat");
        assert_eq!(control.group, "auditorium-a");
        assert_eq!(control.actions[0].verb, "up");
        assert_eq!(
            control
                .actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>(),
            vec!["seat.a.up", "seat.a.stop"]
        );
    }

    #[test]
    fn strip_effect_ids_are_bounded_command_tokens() {
        for id in ["police", "white-thunder", "converging_red.v2", "effect9"] {
            assert!(valid_strip_effect_id(id), "expected {id:?} to be valid");
        }
        for id in ["", "Police", "police 100 30", "police/100", "police\nstop"] {
            assert!(!valid_strip_effect_id(id), "expected {id:?} to be rejected");
        }
        assert!(!valid_strip_effect_id(&"a".repeat(65)));
    }

    #[test]
    fn strip_effects_exist_only_when_the_attached_board_advertises_them() {
        let snapshot = json!({
            "connected": true,
            "hello": {"capabilities": CAPABILITY_ADDRESSABLE_LED}
        });
        let catalog = json!({"strip_effects": [
            {
                "id": "police",
                "name": "Police",
                "description": "Red and blue sweep",
                "default_fps": 20,
                "min_fps": 1,
                "max_fps": 30,
                "min_pixels": 1,
                "max_pixels": 100
            },
            {
                "id": "police 100 30",
                "name": "Injected arguments"
            }
        ]});

        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert_eq!(parsed.strip_effects.len(), 1);
        assert_eq!(parsed.strip_effects[0].id, "police");
        assert_eq!(parsed.strip_effects[0].maximum_pixels, Some(100));

        let disconnected = parse_hardware_capabilities(
            &json!({"connected": false, "hello": {"capabilities": CAPABILITY_ADDRESSABLE_LED}}),
            &catalog,
        );
        assert!(disconnected.strip_effects.is_empty());
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
    fn legacy_strip_catalog_fallback_is_gated_by_advertised_capability() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for call in 0..4 {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(line.trim()).unwrap();
                let response = match call {
                    0 => json!({"jsonrpc":"2.0","id":request["id"],"result":{"ok":true}}),
                    1 => json!({"jsonrpc":"2.0","id":request["id"],"result":{
                        "connected": true,
                        "hello": {"capabilities": CAPABILITY_ADDRESSABLE_LED}
                    }}),
                    2 => json!({"jsonrpc":"2.0","id":request["id"],"result":{"peripherals":[]}}),
                    _ => json!({"jsonrpc":"2.0","id":request["id"],"result":{"output":
                        "[{\"id\":\"white-thunder\",\"name\":\"White thunder\",\"default_fps\":30}]"
                    }}),
                };
                writeln!(stream, "{response}").unwrap();
            }
        });

        let mut client = ControllerClient::connect(&format!("pccontroller://{address}")).unwrap();
        let capabilities = client.hardware_capabilities().unwrap();
        assert_eq!(capabilities.strip_effects.len(), 1);
        assert_eq!(capabilities.strip_effects[0].id, "white-thunder");
        server.join().unwrap();
    }

    #[test]
    fn capability_catalog_uses_advertised_outputs_and_custom_names() {
        let snapshot = json!({
            "connected": true,
            "hello": {"name": "Cinema", "capabilities": CAPABILITY_PWM | CAPABILITY_RELAY_MOTION},
            "status": {"active_relays": 16},
            "macros": {"library": [{"id": 3, "name": "Thunder", "mode": "mcu", "steps": [{"at_us": 250000, "kind": "relay-mask"}]}]}
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
        assert!(parsed.active_relays.contains(&5));
        assert_eq!(parsed.pwm_channels.len(), 1);
        assert_eq!(parsed.macros[0].name, "Thunder");
        assert_eq!(parsed.macros[0].id, 3);
        assert_eq!(parsed.macros[0].duration_ms, 250);
    }

    #[test]
    fn semantic_profiles_do_not_hide_advertised_raw_relay_outputs() {
        let snapshot = json!({
            "connected": true,
            "hello": {"capabilities": CAPABILITY_RELAY_MOTION}
        });
        let catalog = json!({
            "peripherals": [
                {"key":"relay.1","kind":"relay","role":"motion-left-up","index":1,"default_name":"R1","control":"relay"},
                {"key":"relay.2","kind":"relay","role":"motion-left-down","index":2,"default_name":"R2","control":"relay"},
                {"key":"relay.3","kind":"relay","role":"motion-right-up","index":3,"default_name":"R3","control":"relay"},
                {"key":"relay.4","kind":"relay","role":"motion-right-down","index":4,"default_name":"R4","control":"relay"}
            ],
            "controls": [
                {"key":"seat.left","kind":"motion","default_name":"Left seat","control":"seat","actions":[]}
            ]
        });
        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert_eq!(
            parsed
                .relays
                .iter()
                .map(|relay| relay.id)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn disconnected_snapshot_exposes_no_live_controls() {
        let snapshot = json!({"connected": false, "hello": {"capabilities": u32::MAX}});
        let catalog = json!({"peripherals": [{"key":"relay.5","kind":"relay","role":"user-output","index":5,"default_name":"Relay","control":"relay"}]});
        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert!(parsed.relays.is_empty());
        assert!(parsed.pwm_channels.is_empty());
    }

    #[test]
    fn parses_live_board_identity_settings_port_and_front_panel_without_defaults() {
        let snapshot = json!({
            "connected": true,
            "hello": {
                "name": "CAFE-01",
                "board_kind": 7,
                "identity_schema": 2,
                "build_hash": 0xA97EC116_u64,
                "build_timestamp": "260929223718",
                "capabilities": 0
            },
            "port": {"name":"COM3", "product":"USB-SERIAL CH340", "vid":"1A86", "pid":"7523"},
            "have_settings": true,
            "settings": {
                "flags": 1,
                "light_mode": 2,
                "on_brightness": 210,
                "off_brightness": 12,
                "display_brightness": 5,
                "status_brightness": 128,
                "output_persistence": 3,
                "stream_period_ms": 25,
                "default_page": 4,
                "motion_break_ms": 180,
                "persisted": true
            },
            "have_front_panel": true,
            "front_panel": {
                "raw_segments": [63, 6, 91, 79],
                "brightness": 5,
                "blink": true,
                "pressed_keys": 3,
                "menu_page": 4,
                "program_mode": 2,
                "lcd_available": true,
                "lcd_address": 39,
                "lcd_line_1": "Cinema",
                "lcd_line_2": "Ready"
            }
        });
        let parsed = parse_hardware_capabilities(&snapshot, &json!({}));
        assert_eq!(parsed.board_name, "CAFE-01");
        assert_eq!(parsed.board_identity.build_hash, Some(0xA97EC116));
        assert_eq!(parsed.port.name, "COM3");
        let settings = parsed.settings.expect("settings must be advertised");
        assert!(settings.silent);
        assert_eq!(settings.stream_period_ms, 25);
        let front_panel = parsed.front_panel.expect("front panel must be advertised");
        assert_eq!(front_panel.raw_segments, [63, 6, 91, 79]);
        assert_eq!(front_panel.lcd_line_2, "Ready");
    }

    fn live_capabilities() -> HardwareCapabilities {
        let snapshot = json!({
            "connected": true,
            "hello": {"name": "Cinema", "capabilities": CAPABILITY_RELAY_MOTION},
            "status": {"active_relays": 0},
            "have_status_led": true,
            "status_led": {"red": 0, "green": 0, "blue": 0, "brightness": 0, "effect": 0, "condition": 0}
        });
        let catalog = json!({"peripherals": [
            {"key":"relay.5","kind":"relay","role":"user-output","index":5,"default_name":"Relay 5","control":"relay"},
            {"key":"relay.6","kind":"relay","role":"user-output","index":6,"default_name":"Relay 6","control":"relay"}
        ]});
        parse_hardware_capabilities(&snapshot, &catalog)
    }

    #[test]
    fn pushed_status_replaces_live_relay_and_telemetry_state() {
        let mut capabilities = live_capabilities();
        let changed = capabilities.apply_status_notification(&json!({"status": {
            "active_relays": 48,
            "ina219_available": true,
            "supply_mv": 12100,
            "bus_mv": 12000,
            "current_ma": 750,
            "power_mw": 9000,
            "temperature_led_available": false,
            "temperature_bt_audio_available": false,
            "pwm_available": true,
            "pwm_channel": 3,
            "pwm_value": 2048,
            "door_open": true
        }}));

        assert!(changed);
        assert_eq!(capabilities.active_relays, [5, 6].into_iter().collect());
        assert_eq!(capabilities.telemetry.supply_mv, Some(12100));
        assert_eq!(capabilities.telemetry.pwm_channel, Some(3));
        assert_eq!(capabilities.telemetry.door_open, Some(true));
        assert!(!capabilities.apply_status_notification(&json!({"status": {
            "active_relays": 48,
            "ina219_available": true,
            "supply_mv": 12100,
            "bus_mv": 12000,
            "current_ma": 750,
            "power_mw": 9000,
            "temperature_led_available": false,
            "temperature_bt_audio_available": false,
            "pwm_available": true,
            "pwm_channel": 3,
            "pwm_value": 2048,
            "door_open": true
        }})));
    }

    #[test]
    fn pushed_state_updates_physical_led_and_all_relays_off() {
        let mut capabilities = live_capabilities();
        capabilities.active_relays = [5, 6].into_iter().collect();
        assert!(capabilities.apply_state_notification(&json!({
            "kind": "status_led.changed",
            "metadata": {
                "red": "18",
                "green": "52",
                "blue": "86",
                "brightness": "120",
                "effect": "4",
                "condition": "5",
                "revision": "42"
            }
        })));
        assert_eq!(
            capabilities.status_led,
            Some(HardwareStatusLed {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                brightness: 0x78,
                effect: 4,
                condition: 5,
            })
        );
        assert_eq!(capabilities.status_led_revision, 42);
        assert!(!capabilities.apply_state_notification(&json!({
            "kind": "status_led.changed",
            "metadata": {
                "red": "255", "green": "0", "blue": "0",
                "brightness": "255", "effect": "0", "condition": "0",
                "revision": "41"
            }
        })));
        assert!(capabilities.apply_state_notification(&json!({
            "kind": "relay",
            "device": {"type": 11}
        })));
        assert!(capabilities.active_relays.is_empty());
    }

    #[test]
    fn malformed_or_unrelated_push_does_not_replace_live_state() {
        let mut capabilities = live_capabilities();
        let original = capabilities.clone();
        assert!(!capabilities.apply_status_notification(&json!({"status": null})));
        assert!(!capabilities.apply_state_notification(&json!({
            "kind": "status_led.changed",
            "metadata": {"red": "not-a-number"}
        })));
        assert!(!capabilities.apply_state_notification(&json!({"kind": "door"})));
        assert_eq!(capabilities, original);
    }

    #[test]
    fn controller_error_clears_live_board_state() {
        let mut capabilities = live_capabilities();
        capabilities.active_relays = [5].into_iter().collect();
        capabilities.telemetry.supply_mv = Some(12_000);
        assert!(capabilities.mark_board_disconnected());
        assert!(!capabilities.board_connected);
        assert!(capabilities.active_relays.is_empty());
        assert!(capabilities.status_led.is_none());
        assert_eq!(capabilities.telemetry, HardwareTelemetry::default());
        assert!(!capabilities.mark_board_disconnected());
    }

    #[test]
    fn successful_status_push_recovers_board_after_controller_error() {
        let mut capabilities = live_capabilities();
        assert!(capabilities.mark_board_disconnected());
        assert!(capabilities.apply_status_notification(&json!({
            "status": {"active_relays": 0, "pwm_available": false}
        })));
        assert!(capabilities.board_connected);
    }

    #[test]
    fn stale_snapshot_does_not_replace_newer_live_led() {
        let mut current = live_capabilities();
        current.host_instance_id = "host-a".to_string();
        current.status_led_revision = 12;
        current.status_led = Some(HardwareStatusLed {
            red: 12,
            green: 34,
            blue: 56,
            brightness: 255,
            effect: 1,
            condition: 2,
        });
        let mut stale = live_capabilities();
        stale.host_instance_id = "host-a".to_string();
        stale.status_led_revision = 11;
        stale.status_led = Some(HardwareStatusLed::default());
        stale.preserve_newer_live_led_from(&current);
        assert_eq!(stale.status_led_revision, 12);
        assert_eq!(stale.status_led, current.status_led);

        let mut restarted = live_capabilities();
        restarted.host_instance_id = "host-b".to_string();
        restarted.status_led_revision = 1;
        restarted.status_led = Some(HardwareStatusLed::default());
        restarted.preserve_newer_live_led_from(&current);
        assert_eq!(restarted.status_led_revision, 1);
    }
}
