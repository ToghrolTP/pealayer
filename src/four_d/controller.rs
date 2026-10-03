use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use crate::four_d::embedded_host::EmbeddedHost;
use crate::four_d::protocol::Command;

pub const DEFAULT_ENDPOINT: &str = "pccontroller://127.0.0.1:8787";
const CAPABILITY_INA219: u32 = 1 << 0;
const CAPABILITY_TEMPERATURES: u32 = 1 << 1;
const CAPABILITY_PWM: u32 = 1 << 2;
const CAPABILITY_RELAY_MOTION: u32 = 1 << 3;
const CAPABILITY_RF: u32 = 1 << 4;
const CAPABILITY_SEGMENTS: u32 = 1 << 5;
const CAPABILITY_LCD: u32 = 1 << 6;
const CAPABILITY_ADDRESSABLE_LED: u32 = 1 << 7;
const CAPABILITY_PERSISTENT_SETTINGS: u32 = 1 << 8;
const CAPABILITY_MOTION_BREAK: u32 = 1 << 21;
const CAPABILITY_STATUS_EFFECTS: u32 = 1 << 28;
const CAPABILITY_STATUS_LED_PUSH: u32 = 1 << 29;

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
    pub color: String,
    pub group: String,
    pub hidden: bool,
    pub locked: bool,
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
    pub expose_raw_relays: bool,
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
    pub supports_persistent_settings: bool,
    pub supports_measurements: bool,
    pub supports_temperature_sensors: bool,
    pub supports_motion_break_setting: bool,
    pub supports_status_led_settings: bool,
    pub status_led: Option<HardwareStatusLed>,
    pub status_led_revision: u64,
    pub settings: Option<HardwareBoardSettings>,
    pub front_panel: Option<HardwareFrontPanel>,
    pub telemetry: HardwareTelemetry,
    pub warnings: Vec<HardwareWarning>,
    pub strip_control: Option<HardwareStripControl>,
    pub strip_effects: Vec<HardwareStripEffect>,
    pub macros: Vec<HardwareMacro>,
}

impl HardwareCapabilities {
    /// Applies PCController's authoritative response to a presentation update
    /// immediately. A background catalog refresh still follows as an
    /// eventual-consistency check, but the UI and the next edit must use the
    /// returned revision instead of racing a stale cached catalog.
    pub(crate) fn apply_presentation_update(&mut self, result: &Value) -> Result<String, String> {
        let peripheral = result
            .get("peripheral")
            .and_then(Value::as_object)
            .ok_or_else(|| "PCController omitted the updated peripheral".to_string())?;
        let key = peripheral
            .get("key")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| "PCController returned an updated peripheral without a key".to_string())?
            .to_string();
        let presentation = result
            .get("control")
            .and_then(Value::as_object)
            .unwrap_or(peripheral);
        let string_field = |field: &str| {
            presentation
                .get(field)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let default_name = peripheral
            .get("default_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let name = presentation
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .or_else(|| peripheral.get("name").and_then(Value::as_str))
            .filter(|name| !name.trim().is_empty())
            .or_else(|| (!default_name.is_empty()).then_some(default_name.as_str()))
            .ok_or_else(|| "PCController omitted the updated channel name".to_string())?
            .to_string();
        let control = self
            .controls
            .iter_mut()
            .find(|control| control.key == key)
            .ok_or_else(|| format!("PCController updated unknown channel {key:?}"))?;
        control.name.clone_from(&name);
        if !default_name.is_empty() {
            control.default_name.clone_from(&default_name);
        }
        control.icon = string_field("icon");
        control.color = string_field("color");
        control.group = string_field("group");
        control.hidden = presentation
            .get("hidden")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        control.locked = presentation
            .get("locked")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if name == control.default_name {
            self.peripheral_names.remove(&key);
        } else {
            self.peripheral_names.insert(key.clone(), name.clone());
        }
        for output in self
            .peripherals
            .iter_mut()
            .chain(self.relays.iter_mut())
            .chain(self.pwm_channels.iter_mut())
            .filter(|output| output.key == key)
        {
            output.name.clone_from(&name);
        }
        if let Some(controls) = result.get("controls").and_then(Value::as_array) {
            for entry in controls {
                let Some(entry_key) = entry.get("key").and_then(Value::as_str) else {
                    continue;
                };
                let Some(order) = entry
                    .get("order")
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                else {
                    continue;
                };
                if let Some(control) = self.controls.iter_mut().find(|item| item.key == entry_key) {
                    control.order = order;
                }
            }
            self.controls.sort_by(|left, right| {
                left.order
                    .cmp(&right.order)
                    .then_with(|| left.key.cmp(&right.key))
            });
        }
        if let Some(revision) = result
            .pointer("/board_profile/revision")
            .and_then(Value::as_str)
        {
            if let Some(profile) = self.board_profile.as_mut() {
                profile.revision = revision.to_string();
            }
        }
        Ok(key)
    }

    /// Applies a local optimistic channel move while PCController persists the
    /// same kind-local rank. The authoritative response replaces these ranks;
    /// a rejected request schedules a catalog refresh and therefore rolls back.
    pub(crate) fn apply_control_reorder(
        &mut self,
        key: &str,
        requested_order: u16,
    ) -> Result<(), String> {
        let kind = self
            .controls
            .iter()
            .find(|control| control.key == key)
            .map(|control| control.kind.clone())
            .ok_or_else(|| format!("unknown channel {key:?}"))?;
        let mut peers = self
            .controls
            .iter()
            .filter(|control| control.kind == kind)
            .map(|control| control.key.clone())
            .collect::<Vec<_>>();
        if usize::from(requested_order) >= peers.len() {
            return Err(format!(
                "channel order {} is outside 0..{}",
                requested_order,
                peers.len().saturating_sub(1)
            ));
        }
        let current = peers
            .iter()
            .position(|item| item == key)
            .ok_or_else(|| format!("channel {key:?} disappeared during reorder"))?;
        let moved = peers.remove(current);
        peers.insert(usize::from(requested_order), moved);
        for (rank, peer_key) in peers.iter().enumerate() {
            if let Some(control) = self.controls.iter_mut().find(|item| item.key == *peer_key) {
                control.order = u16::try_from(rank).unwrap_or(u16::MAX);
            }
        }
        self.controls.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.key.cmp(&right.key))
        });
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareBoardIdentity {
    pub product_name: String,
    pub stored_name: String,
    pub stored_name_available: bool,
    pub stored_name_persisted: bool,
    pub board_kind: u64,
    pub identity_schema: u64,
    pub build_hash: Option<u64>,
    pub build_timestamp: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwarePort {
    pub name: String,
    pub display_name: String,
    pub friendly_name: String,
    pub product: String,
    pub manufacturer: String,
    pub vid: String,
    pub pid: String,
    pub serial_number: String,
    pub instance_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareBoardSettings {
    pub flags: u8,
    pub silent: bool,
    pub programming_latch: bool,
    pub swap_temperature_roles: bool,
    pub motion_door_policy: u8,
    pub door_audio_enabled: bool,
    pub relay_audio_enabled: bool,
    pub light_mode: u8,
    pub on_brightness: u8,
    pub off_brightness: u8,
    pub display_brightness: u8,
    pub display_closed_brightness: u8,
    pub status_brightness: u8,
    pub output_persistence: u8,
    pub stream_period_ms: u64,
    pub default_page: u8,
    pub extended_flags: u8,
    pub save_last_page: bool,
    pub status_color: u8,
    pub voltage_decimals: u8,
    pub current_decimals: u8,
    pub motion_exit_hold_seconds: u8,
    pub motion_break_ms: u64,
    pub relay_restore_mask: u8,
    pub persisted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareFrontPanel {
    pub raw_segments: Vec<u8>,
    pub brightness: u8,
    pub blink: bool,
    pub segments_active: bool,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HardwareMacroStep {
    #[serde(default)]
    pub at_us: u64,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_hz: Option<u16>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub destination: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bits: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pulse_us: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opcode: Option<u8>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub payload_hex: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareMacro {
    pub id: u64,
    pub name: String,
    pub category: String,
    pub icon: String,
    pub group_icon: String,
    pub mode: String,
    pub duration_ms: u64,
    pub steps: Vec<HardwareMacroStep>,
    pub color: String,
    pub label: String,
    pub lcd_message: String,
    pub timing_tolerance_us: u32,
    pub keep_outputs_on_cancel: bool,
    pub board_profile_key: String,
    pub board_profile_mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareStripEffect {
    pub id: String,
    pub name: String,
    pub category: String,
    pub icon: String,
    pub group_icon: String,
    pub description: String,
    pub program: Value,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardwareStripControl {
    pub minimum_pixels: u16,
    pub maximum_pixels: u16,
    pub default_pixels: u16,
    pub minimum_fps: u8,
    pub maximum_fps: u8,
    pub default_fps: u8,
    pub modes: Vec<String>,
    pub running: bool,
    pub active_name: String,
}

impl HardwareStripControl {
    pub fn supports(&self, mode: &str) -> bool {
        self.modes.iter().any(|item| item == mode)
    }
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
        // controller.snapshot already carries the latest authoritative
        // front-panel state. A synchronous controller.front_panel call waits
        // for another board transaction and can exceed the RPC timeout on a
        // busy serial link, poisoning an otherwise healthy persistent stream.
        // Live updates continue over the controller WebSocket.
        Ok(parse_hardware_capabilities_with_front_panel(
            &snapshot,
            &peripherals,
            None,
        ))
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
    let entries = value.as_array();
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
                icon: entry
                    .get("icon")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                group_icon: entry
                    .get("group_icon")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                description: entry
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                program: entry.get("program").cloned().unwrap_or(Value::Null),
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
                default_duration_ms: entry
                    .get("default_duration_ms")
                    .or_else(|| entry.get("duration_ms"))
                    .and_then(Value::as_u64),
                default_pixels: word(entry, "default_pixels"),
                minimum_fps: byte(entry, "minimum_fps").or_else(|| byte(entry, "min_fps")),
                maximum_fps: byte(entry, "maximum_fps").or_else(|| byte(entry, "max_fps")),
                minimum_pixels: word(entry, "minimum_pixels").or_else(|| word(entry, "min_pixels")),
                maximum_pixels: word(entry, "maximum_pixels").or_else(|| word(entry, "max_pixels")),
            })
        })
        .collect()
}

fn parse_strip_control(snapshot: &Value, catalog: &Value) -> Option<HardwareStripControl> {
    let descriptor = catalog.get("strip")?.as_object()?;
    let word = |key: &str| {
        descriptor
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
    };
    let byte = |key: &str| {
        descriptor
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
    };
    let minimum_pixels = word("minimum_pixels")?;
    let maximum_pixels = word("maximum_pixels")?;
    let default_pixels = word("default_pixels")?;
    let minimum_fps = byte("minimum_fps")?;
    let maximum_fps = byte("maximum_fps")?;
    let default_fps = byte("default_fps")?;
    if minimum_pixels == 0
        || minimum_pixels > default_pixels
        || default_pixels > maximum_pixels
        || minimum_fps == 0
        || minimum_fps > default_fps
        || default_fps > maximum_fps
    {
        return None;
    }
    let modes = descriptor
        .get("modes")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if modes.is_empty() {
        return None;
    }
    let active_name = snapshot
        .pointer("/outputs/strip_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let running = snapshot
        .pointer("/outputs/strip_id")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        != 0
        || !active_name.is_empty();
    Some(HardwareStripControl {
        minimum_pixels,
        maximum_pixels,
        default_pixels,
        minimum_fps,
        maximum_fps,
        default_fps,
        modes,
        running,
        active_name,
    })
}

#[cfg(test)]
fn parse_hardware_capabilities(snapshot: &Value, catalog: &Value) -> HardwareCapabilities {
    parse_hardware_capabilities_with_front_panel(snapshot, catalog, None)
}

fn parse_hardware_capabilities_with_front_panel(
    snapshot: &Value,
    catalog: &Value,
    exact_front_panel: Option<&Value>,
) -> HardwareCapabilities {
    let board_connected = snapshot
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let capability_bits = snapshot
        .pointer("/hello/capabilities")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let product_name = snapshot
        .pointer("/hello/name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let stored_name_available = snapshot
        .get("have_board_name")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stored_name = snapshot
        .pointer("/board_name/name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let stored_name_persisted = snapshot
        .pointer("/board_name/persisted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let board_name = if stored_name_available && !stored_name.trim().is_empty() {
        stored_name.clone()
    } else {
        product_name.clone()
    };
    let board_identity = HardwareBoardIdentity {
        product_name,
        stored_name,
        stored_name_available,
        stored_name_persisted,
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
        display_name: snapshot
            .pointer("/port/display_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        friendly_name: snapshot
            .pointer("/port/friendly_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        product: snapshot
            .pointer("/port/product")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        manufacturer: snapshot
            .pointer("/port/manufacturer")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        vid: snapshot
            .pointer("/port/vid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        serial_number: snapshot
            .pointer("/port/serial_number")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        instance_id: snapshot
            .pointer("/port/instance_id")
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
            expose_raw_relays: profile
                .get("expose_raw_relays")
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
                color: entry
                    .get("color")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                group: entry
                    .get("group")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                hidden: entry
                    .get("hidden")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                locked: entry
                    .get("locked")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
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

    // An alpha/prototyping board may advertise the authoritative raw relay
    // direction/enable wiring before a named board profile has been saved.
    // Keep the higher-level seat controls available in that state while also
    // exposing the raw relays separately. The commands still run through
    // PCController's motion interlock and door-policy implementation.
    if board_connected
        && !controls.iter().any(|control| {
            control.kind.eq_ignore_ascii_case("motion")
                || control.control.eq_ignore_ascii_case("seat")
                || control.control.eq_ignore_ascii_case("motion")
        })
    {
        let has_raw_pair = |direction: u8, enable: u8| {
            outputs.iter().any(|(kind, output)| {
                kind == "relay"
                    && output.id == direction
                    && output.role.eq_ignore_ascii_case("motion-direction")
            }) && outputs.iter().any(|(kind, output)| {
                kind == "relay"
                    && output.id == enable
                    && output.role.eq_ignore_ascii_case("motion-enable")
            })
        };
        for (side, direction, enable, order, name) in [
            ("a", 1_u8, 2_u8, 1_u16, "Seat A"),
            ("b", 3_u8, 4_u8, 2_u16, "Seat B"),
        ] {
            if has_raw_pair(direction, enable) {
                controls.push(HardwareControl {
                    key: format!("seat.{side}"),
                    kind: "motion".to_string(),
                    order,
                    name: name.to_string(),
                    default_name: name.to_string(),
                    control: "raw-motion".to_string(),
                    icon: "seat".to_string(),
                    color: String::new(),
                    group: "Motion / seat".to_string(),
                    hidden: false,
                    locked: false,
                    actions: ["up", "down", "stop"]
                        .into_iter()
                        .map(|verb| HardwareAction {
                            id: format!("seat.{side}.{verb}"),
                            verb: verb.to_string(),
                            name: match verb {
                                "up" => "Up",
                                "down" => "Down",
                                _ => "Stop",
                            }
                            .to_string(),
                            icon: verb.to_string(),
                        })
                        .collect(),
                });
            }
        }
        controls.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.key.cmp(&right.key))
        });
    }

    let relays = if board_connected && capability_bits & CAPABILITY_RELAY_MOTION != 0 {
        outputs
            .iter()
            .filter(|(kind, output)| kind == "relay" && output.id != 0)
            .map(|(_, output)| output.clone())
            .collect()
    } else {
        Vec::new()
    };
    let pwm_channels = if board_connected && capability_bits & CAPABILITY_PWM != 0 {
        outputs
            .iter()
            .filter(|(kind, _)| kind == "pwm")
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
    let settings_flags = snapshot
        .pointer("/settings/flags")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8;
    let extended_flags = snapshot
        .pointer("/settings/extended_flags")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8;
    let decimal_setting = |shift: u8| {
        let encoded = (extended_flags >> shift) & 0x03;
        if encoded == 0 { 2 } else { encoded - 1 }
    };
    let settings = (board_connected
        && snapshot
            .get("have_settings")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    .then(|| HardwareBoardSettings {
        flags: settings_flags,
        silent: settings_flags & 0x01 != 0,
        programming_latch: settings_flags & 0x02 != 0,
        swap_temperature_roles: settings_flags & 0x04 != 0,
        motion_door_policy: (settings_flags >> 3) & 0x03,
        door_audio_enabled: settings_flags & 0x20 == 0,
        relay_audio_enabled: settings_flags & 0x40 == 0,
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
        display_closed_brightness: snapshot
            .pointer("/settings/display_closed_brightness")
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
        extended_flags,
        save_last_page: extended_flags & 0x01 != 0,
        status_color: (extended_flags >> 1) & 0x07,
        voltage_decimals: decimal_setting(4),
        current_decimals: decimal_setting(6),
        motion_exit_hold_seconds: snapshot
            .pointer("/settings/motion_exit_hold_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(1) as u8,
        motion_break_ms: snapshot
            .pointer("/settings/motion_break_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        relay_restore_mask: snapshot
            .pointer("/settings/relay_restore_mask")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        persisted: snapshot
            .pointer("/settings/persisted")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    });
    let snapshot_has_exact_front_panel = snapshot
        .get("have_front_panel")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let front_panel_source = exact_front_panel
        .filter(|panel| {
            panel
                .get("raw_segments")
                .and_then(Value::as_array)
                .is_some_and(|segments| segments.len() == 4)
        })
        .or_else(|| {
            snapshot_has_exact_front_panel
                .then(|| snapshot.get("front_panel"))
                .flatten()
        });
    let front_panel = (board_connected)
        .then_some(front_panel_source)
        .flatten()
        .map(|panel| HardwareFrontPanel {
            raw_segments: panel
                .get("raw_segments")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_u64().and_then(|value| u8::try_from(value).ok()))
                .collect(),
            brightness: panel.get("brightness").and_then(Value::as_u64).unwrap_or(0) as u8,
            blink: panel.get("blink").and_then(Value::as_bool).unwrap_or(false),
            segments_active: panel
                .get("segments_active")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            pressed_keys: panel
                .get("pressed_keys")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            menu_page: panel.get("menu_page").and_then(Value::as_u64).unwrap_or(0) as u8,
            program_mode: panel
                .get("program_mode")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            lcd_available: panel
                .get("lcd_available")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            lcd_address: panel
                .get("lcd_address")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            lcd_line_1: panel
                .get("lcd_line_1")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            lcd_line_2: panel
                .get("lcd_line_2")
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
    // PCController's living effect contract is the single authoritative
    // catalog. Split macro/strip fields remain a fallback only for a host that
    // has not yet refreshed its snapshot after startup.
    let unified_effects = snapshot.get("effects").and_then(Value::as_array);
    let strip_effects = unified_effects
        .map(|effects| parse_strip_effects(&Value::Array(effects.clone())))
        .or_else(|| snapshot.get("strip_effects").map(parse_strip_effects))
        .unwrap_or_default();
    let strip_control = board_connected
        .then(|| parse_strip_control(snapshot, catalog))
        .flatten();
    let sequence_entries = unified_effects.or_else(|| {
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
                .filter_map(|step| serde_json::from_value(step.clone()).ok())
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
                        .map(|step: &HardwareMacroStep| {
                            step.at_us.div_ceil(1_000)
                                + u64::from(step.duration_ms.unwrap_or_default())
                        })
                        .max()
                        .unwrap_or(1)
                })
                .max(1);
            Some(HardwareMacro {
                id,
                name,
                category,
                icon: entry
                    .get("icon")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                group_icon: entry
                    .get("group_icon")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                mode,
                duration_ms,
                steps,
                color: entry
                    .pointer("/properties/color")
                    .and_then(Value::as_str)
                    .unwrap_or("green")
                    .to_string(),
                label: entry
                    .pointer("/properties/label")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                lcd_message: entry
                    .pointer("/properties/lcd_message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                timing_tolerance_us: entry
                    .pointer("/properties/timing_tolerance_us")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .unwrap_or(0),
                keep_outputs_on_cancel: entry
                    .pointer("/properties/keep_outputs_on_cancel")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                board_profile_key: entry
                    .pointer("/properties/board_profile_key")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                board_profile_mode: entry
                    .pointer("/properties/board_profile_mode")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
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
        supports_persistent_settings: board_connected
            && capability_bits & CAPABILITY_PERSISTENT_SETTINGS != 0,
        supports_measurements: board_connected && capability_bits & CAPABILITY_INA219 != 0,
        supports_temperature_sensors: board_connected
            && capability_bits & CAPABILITY_TEMPERATURES != 0,
        supports_motion_break_setting: board_connected
            && capability_bits & CAPABILITY_MOTION_BREAK != 0,
        supports_status_led_settings: board_connected
            && capability_bits & (CAPABILITY_STATUS_EFFECTS | CAPABILITY_STATUS_LED_PUSH) != 0,
        status_led,
        status_led_revision,
        settings,
        front_panel,
        telemetry,
        warnings,
        strip_control,
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
    fn presentation_update_immediately_replaces_name_and_profile_revision() {
        let output = HardwareOutput {
            id: 5,
            key: "relay.5".to_string(),
            name: "User Relay 5".to_string(),
            role: "user-output".to_string(),
            control: "relay".to_string(),
        };
        let mut capabilities = HardwareCapabilities {
            board_profile: Some(HardwareBoardProfile {
                revision: "old-revision".to_string(),
                ..Default::default()
            }),
            controls: vec![HardwareControl {
                key: "relay.5".to_string(),
                kind: "relay".to_string(),
                name: "User Relay 5".to_string(),
                default_name: "User Relay 5".to_string(),
                ..Default::default()
            }],
            peripherals: vec![output.clone()],
            relays: vec![output],
            ..Default::default()
        };

        let renamed = serde_json::json!({
            "board_profile": {"revision": "renamed-revision"},
            "peripheral": {
                "key": "relay.5",
                "name": "Aisle lamp",
                "default_name": "User Relay 5"
            },
            "control": {
                "key": "relay.5",
                "name": "Aisle lamp",
                "icon": "lightbulb",
                "color": "#38D27A",
                "group": "Auditorium",
                "hidden": false,
                "locked": true
            }
        });
        assert_eq!(
            capabilities.apply_presentation_update(&renamed).unwrap(),
            "relay.5"
        );
        assert_eq!(capabilities.controls[0].name, "Aisle lamp");
        assert_eq!(capabilities.controls[0].icon, "lightbulb");
        assert!(capabilities.controls[0].locked);
        assert_eq!(capabilities.relays[0].name, "Aisle lamp");
        assert_eq!(capabilities.peripherals[0].name, "Aisle lamp");
        assert_eq!(capabilities.peripheral_names["relay.5"], "Aisle lamp");
        assert_eq!(
            capabilities.board_profile.as_ref().unwrap().revision,
            "renamed-revision"
        );

        let restored = serde_json::json!({
            "board_profile": {"revision": "restored-revision"},
            "peripheral": {
                "key": "relay.5",
                "name": "User Relay 5",
                "default_name": "User Relay 5"
            },
            "control": {"key": "relay.5", "name": "User Relay 5"}
        });
        capabilities.apply_presentation_update(&restored).unwrap();
        assert_eq!(capabilities.controls[0].name, "User Relay 5");
        assert!(!capabilities.peripheral_names.contains_key("relay.5"));
        assert_eq!(
            capabilities.board_profile.as_ref().unwrap().revision,
            "restored-revision"
        );
    }

    #[test]
    fn presentation_update_applies_authoritative_channel_order() {
        let mut capabilities = HardwareCapabilities {
            controls: vec![
                HardwareControl {
                    key: "relay.5".to_string(),
                    kind: "relay".to_string(),
                    order: 0,
                    name: "Five".to_string(),
                    default_name: "Five".to_string(),
                    ..Default::default()
                },
                HardwareControl {
                    key: "relay.6".to_string(),
                    kind: "relay".to_string(),
                    order: 1,
                    name: "Six".to_string(),
                    default_name: "Six".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let update = serde_json::json!({
            "peripheral": {"key": "relay.6", "name": "Six", "default_name": "Six"},
            "control": {"key": "relay.6", "name": "Six", "order": 0},
            "controls": [
                {"key": "relay.6", "order": 0},
                {"key": "relay.5", "order": 1}
            ]
        });
        capabilities.apply_presentation_update(&update).unwrap();
        assert_eq!(capabilities.controls[0].key, "relay.6");
        assert_eq!(capabilities.controls[0].order, 0);
        assert_eq!(capabilities.controls[1].key, "relay.5");
        assert_eq!(capabilities.controls[1].order, 1);
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
                "name": "Left pair", "control": "motion", "icon": "seat", "color": "#A142F4",
                "group": "auditorium-a", "hidden": true, "locked": true,
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
        assert_eq!(control.color, "#A142F4");
        assert_eq!(control.group, "auditorium-a");
        assert!(control.hidden);
        assert!(control.locked);
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
        for id in [
            "lighting-primary",
            "effect-blue",
            "effect_red.v2",
            "effect9",
        ] {
            assert!(valid_strip_effect_id(id), "expected {id:?} to be valid");
        }
        for id in [
            "",
            "Lighting",
            "effect 100 30",
            "effect/100",
            "effect\nstop",
        ] {
            assert!(!valid_strip_effect_id(id), "expected {id:?} to be rejected");
        }
        assert!(!valid_strip_effect_id(&"a".repeat(65)));
    }

    #[test]
    fn lighting_effects_are_consumed_from_the_live_snapshot_catalog() {
        let snapshot = json!({
            "connected": true,
            "hello": {"capabilities": CAPABILITY_ADDRESSABLE_LED},
            "effects": [
            {
                "id": "lighting-primary",
                "name": "Primary lighting",
                "kind": "strip-stream",
                "icon": "lightbulb",
                "group_icon": "lamp",
                "description": "Red and blue sweep",
                "program": {"primitive":"alternating-zones","primary":{"red":255,"green":0,"blue":0},"secondary":{"red":0,"green":0,"blue":255},"period_ms":800},
                "default_fps": 20,
                "min_fps": 1,
                "max_fps": 30,
                "min_pixels": 1,
                "max_pixels": 100
            },
            {
                "id": "effect 100 30",
                "name": "Injected arguments"
            }
        ]});
        let catalog = json!({
            "strip": {
                "minimum_pixels": 1,
                "maximum_pixels": 100,
                "default_pixels": 100,
                "minimum_fps": 1,
                "maximum_fps": 30,
                "default_fps": 20,
                "modes": ["solid", "pixel", "frame", "rainbow", "effect"]
            },
            "strip_effects": [{"id":"must-not-be-read","name":"Old split catalog"}]
        });

        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert_eq!(parsed.strip_effects.len(), 1);
        assert_eq!(parsed.strip_effects[0].id, "lighting-primary");
        assert_eq!(parsed.strip_effects[0].maximum_pixels, Some(100));
        assert_eq!(parsed.strip_effects[0].icon, "lightbulb");
        assert_eq!(parsed.strip_effects[0].group_icon, "lamp");
        let strip = parsed
            .strip_control
            .as_ref()
            .expect("connected addressable strip control");
        assert_eq!(strip.minimum_pixels, 1);
        assert_eq!(strip.maximum_pixels, 100);
        assert_eq!(strip.default_pixels, 100);
        assert_eq!(strip.minimum_fps, 1);
        assert_eq!(strip.maximum_fps, 30);
        assert_eq!(strip.default_fps, 20);
        assert!(strip.supports("frame"));

        let mut disconnected_snapshot = snapshot;
        disconnected_snapshot["connected"] = json!(false);
        let disconnected = parse_hardware_capabilities(&disconnected_snapshot, &catalog);
        assert_eq!(disconnected.strip_effects.len(), 1);
        assert!(disconnected.strip_control.is_none());
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
    fn capability_snapshot_avoids_redundant_front_panel_and_split_catalog_calls() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for call in 0..3 {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(line.trim()).unwrap();
                let response = match call {
                    0 => json!({"jsonrpc":"2.0","id":request["id"],"result":{"ok":true}}),
                    1 => json!({"jsonrpc":"2.0","id":request["id"],"result":{
                        "connected": true,
                        "hello": {"capabilities": CAPABILITY_ADDRESSABLE_LED | CAPABILITY_SEGMENTS},
                        "have_front_panel": true,
                        "front_panel": {
                            "raw_segments": [63, 6, 91, 79],
                            "segments_active": true
                        }
                    }}),
                    2 => json!({"jsonrpc":"2.0","id":request["id"],"result":{"peripherals":[]}}),
                    _ => unreachable!(),
                };
                writeln!(stream, "{response}").unwrap();
            }
        });

        let mut client = ControllerClient::connect(&format!("pccontroller://{address}")).unwrap();
        let capabilities = client.hardware_capabilities().unwrap();
        assert!(capabilities.strip_effects.is_empty());
        assert_eq!(
            capabilities.front_panel.unwrap().raw_segments,
            vec![63, 6, 91, 79]
        );
        server.join().unwrap();
    }

    #[test]
    fn capability_catalog_uses_advertised_outputs_and_custom_names() {
        let snapshot = json!({
            "connected": true,
            "hello": {"name": "Cinema", "capabilities": CAPABILITY_PWM | CAPABILITY_RELAY_MOTION},
            "board_name": {"name": "CAFE-01", "persisted": true},
            "have_board_name": true,
            "port": {
                "name": "COM4",
                "display_name": "USB-SERIAL CH340",
                "friendly_name": "USB-SERIAL CH340",
                "product": "USB Serial",
                "manufacturer": "QinHeng",
                "vid": "1A86",
                "pid": "7523",
                "serial_number": "BOARD-1",
                "instance_id": "USB\\VID_1A86&PID_7523\\BOARD-1"
            },
            "status": {"active_relays": 16},
            "macros": {"library": [{
                "id": 3,
                "name": "Thunder",
                "mode": "mcu",
                "steps": [{
                    "at_us": 250000,
                    "kind": "display",
                    "text": "GO",
                    "destination": "segments",
                    "duration_ms": 200,
                    "action_ids": ["seat.a.up"]
                }],
                "properties": {
                    "color": "amber",
                    "label": "Seat rise",
                    "lcd_message": "Motion",
                    "timing_tolerance_us": 25000,
                    "keep_outputs_on_cancel": true,
                    "board_profile_key": "cafe-cinema",
                    "board_profile_mode": "motion"
                }
            }]}
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
        assert_eq!(parsed.board_name, "CAFE-01");
        assert_eq!(parsed.board_identity.product_name, "Cinema");
        assert_eq!(parsed.board_identity.stored_name, "CAFE-01");
        assert!(parsed.board_identity.stored_name_persisted);
        assert_eq!(parsed.port.name, "COM4");
        assert_eq!(parsed.port.display_name, "USB-SERIAL CH340");
        assert_eq!(parsed.port.manufacturer, "QinHeng");
        assert_eq!(parsed.port.serial_number, "BOARD-1");
        assert_eq!(parsed.relays[0].name, "Left Air");
        assert!(parsed.active_relays.contains(&5));
        assert_eq!(parsed.pwm_channels.len(), 2);
        assert!(parsed.pwm_channels.iter().any(|channel| channel.id == 15));
        assert_eq!(parsed.macros[0].name, "Thunder");
        assert_eq!(parsed.macros[0].id, 3);
        assert_eq!(parsed.macros[0].duration_ms, 450);
        assert_eq!(parsed.macros[0].steps[0].text, "GO");
        assert_eq!(parsed.macros[0].steps[0].destination, "segments");
        assert_eq!(parsed.macros[0].steps[0].duration_ms, Some(200));
        assert_eq!(parsed.macros[0].steps[0].action_ids, ["seat.a.up"]);
        assert_eq!(parsed.macros[0].color, "amber");
        assert_eq!(parsed.macros[0].label, "Seat rise");
        assert_eq!(parsed.macros[0].lcd_message, "Motion");
        assert_eq!(parsed.macros[0].timing_tolerance_us, 25_000);
        assert!(parsed.macros[0].keep_outputs_on_cancel);
        assert_eq!(parsed.macros[0].board_profile_key, "cafe-cinema");
        assert_eq!(parsed.macros[0].board_profile_mode, "motion");
    }

    #[test]
    fn semantic_profiles_do_not_hide_advertised_raw_relay_outputs() {
        let snapshot = json!({
            "connected": true,
            "hello": {"capabilities": CAPABILITY_RELAY_MOTION}
        });
        let catalog = json!({
            "peripherals": [
                {"key":"relay.1","kind":"relay","role":"motion-left-up","index":1,"default_name":"Raw relay 1","control":"relay"},
                {"key":"relay.2","kind":"relay","role":"motion-left-down","index":2,"default_name":"Raw relay 2","control":"relay"},
                {"key":"relay.3","kind":"relay","role":"motion-right-up","index":3,"default_name":"Raw relay 3","control":"relay"},
                {"key":"relay.4","kind":"relay","role":"motion-right-down","index":4,"default_name":"Raw relay 4","control":"relay"}
            ],
            "controls": [
                {"key":"seat.a","kind":"motion","default_name":"Seat A","control":"seat","actions":[]}
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
    fn unconfigured_motion_wiring_exposes_seat_controls_and_raw_relays() {
        let snapshot = json!({
            "connected": true,
            "hello": {"capabilities": CAPABILITY_RELAY_MOTION}
        });
        let catalog = json!({
            "board_profile": {
                "mode": "unconfigured",
                "configured": false,
                "attached": true,
                "expose_raw_relays": false
            },
            "peripherals": [
                {"key":"relay.1","kind":"relay","role":"motion-direction","index":1,"default_name":"Side A Direction","control":"unavailable"},
                {"key":"relay.2","kind":"relay","role":"motion-enable","index":2,"default_name":"Side A Output","control":"unavailable"},
                {"key":"relay.3","kind":"relay","role":"motion-direction","index":3,"default_name":"Side B Direction","control":"unavailable"},
                {"key":"relay.4","kind":"relay","role":"motion-enable","index":4,"default_name":"Side B Output","control":"unavailable"},
                {"key":"relay.5","kind":"relay","role":"user-output","index":5,"default_name":"User Relay 5","control":"relay"}
            ],
            "controls": []
        });

        let parsed = parse_hardware_capabilities(&snapshot, &catalog);
        assert_eq!(
            parsed
                .relays
                .iter()
                .map(|relay| relay.id)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        let seats = parsed
            .controls
            .iter()
            .filter(|control| control.control == "raw-motion")
            .collect::<Vec<_>>();
        assert_eq!(seats.len(), 2);
        assert_eq!(seats[0].key, "seat.a");
        assert_eq!(seats[1].key, "seat.b");
        assert_eq!(seats[0].name, "Seat A");
        assert_eq!(seats[1].name, "Seat B");
        assert_eq!(
            seats[0]
                .actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>(),
            ["seat.a.up", "seat.a.down", "seat.a.stop"]
        );
        assert_eq!(
            seats[1]
                .actions
                .iter()
                .map(|action| action.id.as_str())
                .collect::<Vec<_>>(),
            ["seat.b.up", "seat.b.down", "seat.b.stop"]
        );
        assert!(!parsed.board_profile.as_ref().unwrap().expose_raw_relays);
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
                "name": "PCController",
                "board_kind": 7,
                "identity_schema": 2,
                "build_hash": 0xA97EC116_u64,
                "build_timestamp": "260929223718",
                "capabilities": CAPABILITY_SEGMENTS
                    | CAPABILITY_PERSISTENT_SETTINGS
                    | CAPABILITY_INA219
                    | CAPABILITY_TEMPERATURES
                    | CAPABILITY_MOTION_BREAK
                    | CAPABILITY_STATUS_EFFECTS
            },
            "board_name": {"name":"CAFE-01", "persisted":true},
            "have_board_name": true,
            "port": {"name":"COM3", "display_name":"USB-SERIAL CH340", "friendly_name":"USB-SERIAL CH340", "product":"USB-SERIAL CH340", "manufacturer":"QinHeng", "vid":"1A86", "pid":"7523", "serial_number":"BOARD-3", "instance_id":"USB\\VID_1A86&PID_7523\\BOARD-3"},
            "have_settings": true,
            "settings": {
                "flags": 0x55,
                "light_mode": 2,
                "on_brightness": 210,
                "off_brightness": 12,
                "display_brightness": 5,
                "display_closed_brightness": 2,
                "status_brightness": 128,
                "output_persistence": 3,
                "stream_period_ms": 25,
                "default_page": 4,
                "extended_flags": 0x69,
                "motion_exit_hold_seconds": 7,
                "motion_break_ms": 180,
                "relay_restore_mask": 0xA5,
                "persisted": true
            },
            "have_front_panel": false,
            "front_panel": {
                "raw_segments": [63, 6, 91, 79],
                "brightness": 5,
                "blink": true,
                "segments_active": true,
                "pressed_keys": 3,
                "menu_page": 4,
                "program_mode": 2,
                "lcd_available": true,
                "lcd_address": 39,
                "lcd_line_1": "Cinema",
                "lcd_line_2": "Ready"
            }
        });
        let exact_front_panel = snapshot["front_panel"].clone();
        let parsed = parse_hardware_capabilities_with_front_panel(
            &snapshot,
            &json!({}),
            Some(&exact_front_panel),
        );
        assert_eq!(parsed.board_name, "CAFE-01");
        assert_eq!(parsed.board_identity.product_name, "PCController");
        assert_eq!(parsed.board_identity.stored_name, "CAFE-01");
        assert_eq!(parsed.board_identity.build_hash, Some(0xA97EC116));
        assert_eq!(parsed.port.name, "COM3");
        assert_eq!(parsed.port.display_name, "USB-SERIAL CH340");
        assert_eq!(parsed.port.serial_number, "BOARD-3");
        let settings = parsed.settings.expect("settings must be advertised");
        assert!(settings.silent);
        assert!(!settings.programming_latch);
        assert!(settings.swap_temperature_roles);
        assert_eq!(settings.motion_door_policy, 2);
        assert!(settings.door_audio_enabled);
        assert!(!settings.relay_audio_enabled);
        assert_eq!(settings.stream_period_ms, 25);
        assert_eq!(settings.display_closed_brightness, 2);
        assert!(settings.save_last_page);
        assert_eq!(settings.status_color, 4);
        assert_eq!(settings.voltage_decimals, 1);
        assert_eq!(settings.current_decimals, 0);
        assert_eq!(settings.motion_exit_hold_seconds, 7);
        assert_eq!(settings.relay_restore_mask, 0xA5);
        assert!(parsed.supports_persistent_settings);
        assert!(parsed.supports_measurements);
        assert!(parsed.supports_temperature_sensors);
        assert!(parsed.supports_motion_break_setting);
        assert!(parsed.supports_status_led_settings);
        let front_panel = parsed.front_panel.expect("front panel must be advertised");
        assert_eq!(front_panel.raw_segments, [63, 6, 91, 79]);
        assert!(front_panel.segments_active);
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
