use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::four_d::models::Timeline;
use crate::four_d::protocol::Command;

enum HardwareTransport {
    Controller(crate::four_d::controller::ControllerClient),
    DirectSerial {
        port: Box<dyn serialport::SerialPort>,
        sequence: u8,
    },
}

impl HardwareTransport {
    fn send(&mut self, command: Command) -> Result<(), String> {
        match self {
            Self::Controller(client) => client.send_command(command),
            Self::DirectSerial { port, sequence } => {
                let request_sequence = *sequence;
                let frame = command.to_pccontroller_frame(request_sequence);
                *sequence = sequence.wrapping_add(1).max(1);
                port.write_all(&frame)
                    .and_then(|_| port.flush())
                    .map_err(|error| format!("direct COBS serial write failed: {error}"))?;

                let expected_opcode = if matches!(command, Command::Ping) {
                    0x81 // HELLO_RESP
                } else {
                    0x80 // ACK
                };
                let deadline = std::time::Instant::now() + Duration::from_millis(500);
                let mut encoded = Vec::new();
                let mut buffer = [0_u8; 64];
                while std::time::Instant::now() < deadline {
                    match port.read(&mut buffer) {
                        Ok(count) => {
                            for byte in &buffer[..count] {
                                if *byte != 0 {
                                    encoded.push(*byte);
                                    continue;
                                }
                                if encoded.is_empty() {
                                    continue;
                                }
                                encoded.push(0);
                                let decoded =
                                    crate::four_d::protocol::decode_pccontroller_frame(&encoded);
                                encoded.clear();
                                let Ok((opcode, response_sequence, payload)) = decoded else {
                                    continue;
                                };
                                if response_sequence != request_sequence {
                                    continue;
                                }
                                if opcode == 0x82 {
                                    return Err(format!(
                                        "direct board rejected sequence {request_sequence}: {}",
                                        payload
                                            .iter()
                                            .map(|byte| format!("{byte:02X}"))
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                    ));
                                }
                                if opcode == expected_opcode {
                                    return Ok(());
                                }
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                            ) => {}
                        Err(error) => {
                            return Err(format!("direct COBS serial read failed: {error}"));
                        }
                    }
                }
                Err(format!(
                    "direct board response timeout for sequence {request_sequence}"
                ))
            }
        }
    }

    fn needs_watchdog_ping(&self) -> bool {
        matches!(self, Self::DirectSerial { .. })
    }

    fn is_direct_serial(&self) -> bool {
        matches!(self, Self::DirectSerial { .. })
    }

    fn description(&self) -> String {
        match self {
            Self::Controller(client) => client.transport_description(),
            Self::DirectSerial { .. } => "direct:serial-diagnostic".to_string(),
        }
    }

    fn refresh_capabilities(
        &mut self,
    ) -> Result<Option<crate::four_d::controller::HardwareCapabilities>, String> {
        match self {
            Self::Controller(client) => client.hardware_capabilities().map(Some),
            Self::DirectSerial { .. } => Ok(None),
        }
    }

    fn call_controller(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self {
            Self::Controller(client) => client.call(method, params),
            Self::DirectSerial { .. } => Err(
                "this hardware action requires the PCController coordinator".to_string(),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledAction {
    pub time_ms: u64,
    pub relay_id: u8,
    pub state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledControllerMacro {
    pub time_ms: u64,
    pub id: u64,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledControllerStripEffect {
    pub time_ms: u64,
    pub id: String,
    pub start: bool,
}

#[derive(Debug, Clone)]
pub struct ControllerCallResult {
    pub operation: String,
    pub result: Result<serde_json::Value, String>,
}

pub enum EngineMessage {
    UpdateQueue(Vec<CompiledAction>),
    UpdateControllerMacros(Vec<CompiledControllerMacro>),
    UpdateControllerStripEffects(Vec<CompiledControllerStripEffect>),
    UpdateAnalogTracks(Vec<crate::four_d::curve::AnalogTrack>),
    LiveActuatorOverride { channel: u8, value: u8 },
    Seek(u64), // Emitted when user seeks, to clear current active queue and reset hardware
    SendCommand(Command), // Manual override or direct hardware command
    ControllerCall {
        method: String,
        params: serde_json::Value,
    },
    TrackedControllerCall {
        operation: String,
        method: String,
        params: serde_json::Value,
    },
    InvokeControllerAction {
        action_id: String,
    },
    UpdatePeripheralPresentation {
        key: String,
        name: Option<String>,
        icon: Option<String>,
        group: Option<String>,
        expected_revision: Option<String>,
        fallback_names: std::collections::BTreeMap<String, String>,
    },
}

pub struct EngineHandle {
    lifecycle: Arc<()>,
    pub playback_time_ms: Arc<AtomicU64>,
    pub is_playing: Arc<AtomicBool>,
    pub estop_active: Arc<AtomicBool>,
    pub connection_requested: Arc<AtomicBool>,
    pub is_connected: Arc<AtomicBool>,
    pub serial_port: Arc<Mutex<String>>,
    pub active_transport: Arc<Mutex<Option<String>>>,
    pub connection_error: Arc<Mutex<Option<String>>>,
    pub hardware_capabilities: Arc<Mutex<Option<crate::four_d::controller::HardwareCapabilities>>>,
    pub controller_call_results: Arc<Mutex<std::collections::VecDeque<ControllerCallResult>>>,
    catalog_refresh_requested: Arc<AtomicBool>,
    pub sender: mpsc::Sender<EngineMessage>,
}

/// Weak access to the live hardware view for PCController's push transport.
/// The weak references make the WebSocket worker self-cancelling when the app
/// and its engine are dropped, without keeping a hidden process-lifetime task.
#[derive(Clone)]
pub struct ControllerPushTarget {
    lifecycle: std::sync::Weak<()>,
    connection_requested: std::sync::Weak<AtomicBool>,
    serial_port: std::sync::Weak<Mutex<String>>,
    hardware_capabilities:
        std::sync::Weak<Mutex<Option<crate::four_d::controller::HardwareCapabilities>>>,
    catalog_refresh_requested: std::sync::Weak<AtomicBool>,
}

impl EngineHandle {
    pub fn controller_push_target(&self) -> ControllerPushTarget {
        ControllerPushTarget {
            lifecycle: Arc::downgrade(&self.lifecycle),
            connection_requested: Arc::downgrade(&self.connection_requested),
            serial_port: Arc::downgrade(&self.serial_port),
            hardware_capabilities: Arc::downgrade(&self.hardware_capabilities),
            catalog_refresh_requested: Arc::downgrade(&self.catalog_refresh_requested),
        }
    }

    pub fn request_controller_call(
        &self,
        operation: impl Into<String>,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<(), String> {
        self.sender
            .send(EngineMessage::TrackedControllerCall {
                operation: operation.into(),
                method: method.into(),
                params,
            })
            .map_err(|_| "hardware engine is unavailable".to_string())
    }

    pub fn request_catalog_refresh(&self) {
        self.catalog_refresh_requested.store(true, Ordering::Relaxed);
    }
}

impl ControllerPushTarget {
    pub(crate) fn is_alive(&self) -> bool {
        self.lifecycle.strong_count() > 0
    }

    /// Returns the selected WebSocket URL only while the user wants a
    /// PCController connection. Direct-serial diagnostics never start a second
    /// coordinator transport.
    pub(crate) fn websocket_endpoint(&self) -> Option<String> {
        if !self.connection_requested.upgrade()?.load(Ordering::Relaxed) {
            return None;
        }
        let endpoint = self.serial_port.upgrade()?.lock().ok()?.clone();
        if !crate::four_d::controller::is_controller_endpoint(&endpoint) {
            return None;
        }
        let address = endpoint
            .strip_prefix("pccontroller://")
            .or_else(|| endpoint.strip_prefix("tcp://"))
            .unwrap_or(&endpoint);
        Some(format!("ws://{address}/ipc"))
    }

    pub(crate) fn apply_notification(&self, method: &str, params: &serde_json::Value) -> bool {
        if matches!(method, "controller.state" | "controller.event")
            && params.get("kind").and_then(serde_json::Value::as_str)
                == Some("peripherals.changed")
        {
            if let Some(refresh) = self.catalog_refresh_requested.upgrade() {
                refresh.store(true, Ordering::Relaxed);
                return true;
            }
        }
        let Some(capabilities) = self.hardware_capabilities.upgrade() else {
            return false;
        };
        let Ok(mut capabilities) = capabilities.lock() else {
            return false;
        };
        let Some(capabilities) = capabilities.as_mut() else {
            return false;
        };
        match method {
            "controller.status" => capabilities.apply_status_notification(params),
            "controller.state" | "controller.event" => {
                capabilities.apply_state_notification(params)
            }
            "controller.error" => capabilities.mark_board_disconnected(),
            _ => false,
        }
    }

    /// Accepts the authoritative host identity from a fresh WebSocket
    /// subscription. A PCController process restart resets LED revisions, so
    /// the old process revision must not suppress the new process's first frame.
    pub(crate) fn observe_source_instance(&self, instance_id: &str) -> bool {
        let instance_id = instance_id.trim();
        if instance_id.is_empty() {
            return false;
        }
        let Some(capabilities) = self.hardware_capabilities.upgrade() else {
            return false;
        };
        let Ok(mut capabilities) = capabilities.lock() else {
            return false;
        };
        let Some(capabilities) = capabilities.as_mut() else {
            return false;
        };
        if capabilities.host_instance_id == instance_id {
            return false;
        }
        let source_changed = !capabilities.host_instance_id.is_empty();
        capabilities.host_instance_id = instance_id.to_string();
        if source_changed {
            capabilities.status_led = None;
            capabilities.status_led_revision = 0;
        }
        source_changed
    }
}

fn should_yield_direct_transport(
    is_direct: bool,
    diagnostic_override: bool,
    coordinator_reachable: bool,
) -> bool {
    is_direct && !diagnostic_override && coordinator_reachable
}

fn controller_method_is_unavailable(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("-32601") || error.contains("method not found") || error.contains("unknown method")
}

pub fn spawn_engine() -> EngineHandle {
    let lifecycle = Arc::new(());
    let playback_time_ms = Arc::new(AtomicU64::new(0));
    let is_playing = Arc::new(AtomicBool::new(false));
    let estop_active = Arc::new(AtomicBool::new(false));
    let connection_requested = Arc::new(AtomicBool::new(false));
    let is_connected = Arc::new(AtomicBool::new(false));
    let serial_port = Arc::new(Mutex::new(
        crate::four_d::controller::DEFAULT_ENDPOINT.to_string(),
    ));
    let active_transport_description = Arc::new(Mutex::new(None));
    let connection_error = Arc::new(Mutex::new(None));
    let hardware_capabilities = Arc::new(Mutex::new(None));
    let controller_call_results = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let catalog_refresh_requested = Arc::new(AtomicBool::new(false));

    let (tx, rx) = mpsc::channel();

    let engine_time = Arc::clone(&playback_time_ms);
    let engine_playing = Arc::clone(&is_playing);
    let engine_estop = Arc::clone(&estop_active);
    let engine_connection_requested = Arc::clone(&connection_requested);
    let engine_connected = Arc::clone(&is_connected);
    let engine_port = Arc::clone(&serial_port);
    let engine_transport_description = Arc::clone(&active_transport_description);
    let engine_conn_error = Arc::clone(&connection_error);
    let engine_capabilities = Arc::clone(&hardware_capabilities);
    let engine_controller_call_results = Arc::clone(&controller_call_results);
    let engine_catalog_refresh_requested = Arc::clone(&catalog_refresh_requested);

    thread::spawn(move || {
        let mut queue: Vec<CompiledAction> = Vec::new();
        let mut current_queue_index = 0;
        let mut controller_macros = Vec::<CompiledControllerMacro>::new();
        let mut current_controller_macro_index = 0;
        let mut controller_strip_effects = Vec::<CompiledControllerStripEffect>::new();
        let mut current_controller_strip_effect_index = 0;
        let mut active_strip_effect: Option<String> = None;
        let mut analog_tracks: Vec<crate::four_d::curve::AnalogTrack> = Vec::new();
        let mut last_pwm_values = [0u8; 16];
        let mut was_playing = false;
        let mut was_estop = false;
        let mut last_ping = std::time::Instant::now();
        let mut last_owner_check = std::time::Instant::now();
        let mut last_capability_refresh = std::time::Instant::now();
        let mut last_connect_attempt: Option<std::time::Instant> = None;

        let mut active_transport: Option<HardwareTransport> = None;

        loop {
            let estop_now = engine_estop.load(Ordering::Relaxed);
            let requested = engine_connection_requested.load(Ordering::Relaxed);
            let mut connected = active_transport.is_some();
            engine_connected.store(connected, Ordering::Relaxed);

            // Handle connection/disconnection transitions
            if requested
                && active_transport.is_none()
                && last_connect_attempt
                    .is_none_or(|attempt| attempt.elapsed() >= Duration::from_secs(1))
            {
                last_connect_attempt = Some(std::time::Instant::now());
                let endpoint = {
                    let guard = engine_port.lock().unwrap();
                    guard.clone()
                };
                let transport = if crate::four_d::controller::is_controller_endpoint(&endpoint) {
                    crate::four_d::controller::ControllerClient::connect_preferred(&endpoint)
                        .map(HardwareTransport::Controller)
                } else {
                    let direct_override =
                        std::env::var_os("PEALAYER_ALLOW_DIRECT_SERIAL").is_some();
                    if !direct_override
                        && crate::four_d::controller::ControllerClient::connect(
                            crate::four_d::controller::DEFAULT_ENDPOINT,
                        )
                        .is_ok()
                    {
                        Err("PCController is already reachable and owns the board. Use the pccontroller:// endpoint, or set PEALAYER_ALLOW_DIRECT_SERIAL=1 for an explicit diagnostic override.".to_string())
                    } else {
                        let port_name = crate::four_d::controller::direct_serial_name(&endpoint);
                        serialport::new(port_name, 115200)
                            .timeout(Duration::from_millis(15))
                            .open()
                            .map(|port| HardwareTransport::DirectSerial { port, sequence: 1 })
                            .map_err(|error| {
                                format!("open direct serial endpoint {port_name}: {error}")
                            })
                    }
                };
                match transport {
                    Ok(transport) => {
                        let description = transport.description();
                        println!(
                            "[Engine] Connected hardware transport: {}",
                            description
                        );
                        if let Ok(mut guard) = engine_transport_description.lock() {
                            *guard = Some(description);
                        }
                        active_transport = Some(transport);
                        if let Some(ref mut transport) = active_transport {
                            match transport.refresh_capabilities() {
                                Ok(capabilities) => {
                                    if let Ok(mut guard) = engine_capabilities.lock() {
                                        *guard = capabilities;
                                    }
                                }
                                Err(error) => {
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(format!(
                                            "connected, but capability discovery failed: {error}"
                                        ));
                                    }
                                }
                            }
                            // A coordinator-side renderer can outlive a dropped
                            // client socket. Reconnect starts from a known idle
                            // strip state; playback will issue the next cue.
                            let _ = transport.call_controller(
                                "controller.command.execute",
                                serde_json::json!({"command": "strip stop"}),
                            );
                            active_strip_effect = None;
                        }
                        engine_connected.store(true, Ordering::Relaxed);
                        connected = true;
                        let estop_reassert_error = engine_estop
                            .load(Ordering::Relaxed)
                            .then(|| {
                                active_transport
                                    .as_mut()
                                    .and_then(|transport| transport.send(Command::AllOff).err())
                            })
                            .flatten();
                        if let Some(error) = estop_reassert_error {
                            if let Ok(mut guard) = engine_conn_error.lock() {
                                *guard = Some(format!(
                                    "reassert emergency stop after reconnect: {error}"
                                ));
                            }
                            engine_connected.store(false, Ordering::Relaxed);
                            connected = false;
                            active_transport = None;
                            if let Ok(mut guard) = engine_transport_description.lock() {
                                *guard = None;
                            }
                            continue;
                        }
                        last_ping = std::time::Instant::now();
                        last_owner_check = std::time::Instant::now();
                        last_capability_refresh = std::time::Instant::now();
                    }
                    Err(error) => {
                        if let Ok(mut guard) = engine_conn_error.lock() {
                            *guard = Some(error);
                        }
                        engine_connected.store(false, Ordering::Relaxed);
                        if let Ok(mut guard) = engine_transport_description.lock() {
                            *guard = None;
                        }
                    }
                }
            } else if !requested && active_transport.is_some() {
                // Graceful disconnect: send AllOff
                if let Some(ref mut transport) = active_transport {
                    let _ = transport.call_controller(
                        "controller.command.execute",
                        serde_json::json!({"command": "macro cancel"}),
                    );
                    let _ = transport.call_controller(
                        "controller.command.execute",
                        serde_json::json!({"command": "strip stop"}),
                    );
                    let _ = transport.send(Command::AllOff);
                }
                active_strip_effect = None;
                active_transport = None;
                if let Ok(mut guard) = engine_transport_description.lock() {
                    *guard = None;
                }
                if let Ok(mut guard) = engine_capabilities.lock() {
                    *guard = None;
                }
                engine_connected.store(false, Ordering::Relaxed);
                connected = false;
                println!("[Engine] Disconnected hardware transport");
            }

            // Check for new messages (non-blocking)
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    EngineMessage::UpdateQueue(new_queue) => {
                        queue = new_queue;
                        let current_time = engine_time.load(Ordering::Relaxed);
                        current_queue_index = queue.partition_point(|x| x.time_ms < current_time);
                    }
                    EngineMessage::UpdateControllerMacros(new_queue) => {
                        controller_macros = new_queue;
                        let current_time = engine_time.load(Ordering::Relaxed);
                        current_controller_macro_index = controller_macros
                            .partition_point(|cue| cue.time_ms < current_time);
                    }
                    EngineMessage::UpdateControllerStripEffects(new_queue) => {
                        let current_time = engine_time.load(Ordering::Relaxed);
                        let playing = engine_playing.load(Ordering::Relaxed);
                        let desired = playing
                            .then(|| active_controller_strip_effect_at(&new_queue, current_time))
                            .flatten()
                            .map(str::to_owned);
                        if active_strip_effect != desired {
                            if let Some(ref mut transport) = active_transport {
                                if active_strip_effect.is_some() {
                                    let _ = transport.call_controller(
                                        "controller.command.execute",
                                        serde_json::json!({"command": "strip stop"}),
                                    );
                                }
                                active_strip_effect = None;
                                if let Some(id) = desired.as_deref() {
                                    if transport
                                        .call_controller(
                                            "controller.command.execute",
                                            serde_json::json!({"command": format!("strip effect play {id}")}),
                                        )
                                        .is_ok()
                                    {
                                        active_strip_effect = Some(id.to_string());
                                    }
                                }
                            }
                        }
                        controller_strip_effects = new_queue;
                        current_controller_strip_effect_index = if playing {
                            controller_strip_effects
                                .partition_point(|cue| cue.time_ms <= current_time)
                        } else {
                            controller_strip_effects
                                .partition_point(|cue| cue.time_ms < current_time)
                        };
                    }
                    EngineMessage::UpdateAnalogTracks(tracks) => {
                        analog_tracks = tracks;
                        last_pwm_values.fill(0);
                    }
                    EngineMessage::LiveActuatorOverride { channel, value } => {
                        let ch = channel as usize;
                        if ch < 16 && value != last_pwm_values[ch] {
                            last_pwm_values[ch] = value;
                            if connected {
                                if let Some(ref mut transport) = active_transport {
                                    let cmd = Command::PwmSet { channel, value };
                                    if let Err(e) = transport.send(cmd) {
                                        if let Ok(mut err_guard) = engine_conn_error.lock() {
                                            *err_guard = Some(e);
                                        }
                                        engine_connected
                                            .store(false, std::sync::atomic::Ordering::Relaxed);
                                        connected = false;
                                        active_transport = None;
                                        if let Ok(mut guard) = engine_transport_description.lock() {
                                            *guard = None;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    EngineMessage::Seek(time) => {
                        let resume_strip = engine_playing.load(Ordering::Relaxed)
                            && !engine_estop.load(Ordering::Relaxed);
                        if connected {
                            if let Some(ref mut transport) = active_transport {
                                let _ = transport.call_controller(
                                    "controller.command.execute",
                                    serde_json::json!({"command": "macro cancel"}),
                                );
                                let _ = transport.call_controller(
                                    "controller.command.execute",
                                    serde_json::json!({"command": "strip stop"}),
                                );
                                active_strip_effect = None;
                                if let Err(e) = transport.send(Command::AllOff) {
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(e);
                                    }
                                    engine_connected.store(false, Ordering::Relaxed);
                                } else if resume_strip {
                                    if let Some(id) = active_controller_strip_effect_at(
                                        &controller_strip_effects,
                                        time,
                                    ) {
                                        if transport
                                            .call_controller(
                                                "controller.command.execute",
                                                serde_json::json!({"command": format!("strip effect play {id}")}),
                                            )
                                            .is_ok()
                                        {
                                            active_strip_effect = Some(id.to_string());
                                        }
                                    }
                                }
                            }
                            let port_name = {
                                let guard = engine_port.lock().unwrap();
                                guard.clone()
                            };
                            println!("[{}] ALL_OFF (Seek to {}ms)", port_name, time);
                        }
                        current_queue_index = queue.partition_point(|x| x.time_ms < time);
                        current_controller_macro_index =
                            controller_macros.partition_point(|cue| cue.time_ms < time);
                        current_controller_strip_effect_index = if resume_strip {
                            controller_strip_effects.partition_point(|cue| cue.time_ms <= time)
                        } else {
                            controller_strip_effects.partition_point(|cue| cue.time_ms < time)
                        };
                        last_pwm_values.fill(0);
                    }
                    EngineMessage::SendCommand(cmd) => {
                        if connected {
                            if let Some(ref mut transport) = active_transport {
                                if let Err(e) = transport.send(cmd) {
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(e);
                                    }
                                    engine_connected.store(false, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    EngineMessage::ControllerCall { method, params } => {
                        if connected {
                            if let Some(ref mut transport) = active_transport {
                                if let Err(error) = transport.call_controller(&method, params) {
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(format!("{method}: {error}"));
                                    }
                                    engine_connected.store(false, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    EngineMessage::TrackedControllerCall {
                        operation,
                        method,
                        params,
                    } => {
                        let result = if connected {
                            active_transport
                                .as_mut()
                                .ok_or_else(|| "PCController transport is unavailable".to_string())
                                .and_then(|transport| transport.call_controller(&method, params))
                        } else {
                            Err("PCController is not connected".to_string())
                        };
                        if result.is_ok()
                            && matches!(operation.as_str(), "macro-save" | "macro-discard")
                        {
                            engine_catalog_refresh_requested.store(true, Ordering::Relaxed);
                        }
                        if let Ok(mut results) = engine_controller_call_results.lock() {
                            results.push_back(ControllerCallResult { operation, result });
                        }
                    }
                    EngineMessage::InvokeControllerAction { action_id } => {
                        if connected {
                            if let Some(ref mut transport) = active_transport {
                                match transport.call_controller(
                                    "controller.action.invoke",
                                    serde_json::json!({"action_id": action_id}),
                                ) {
                                    Ok(_) => {
                                        // Pull an authoritative snapshot immediately. Push
                                        // events remain the lowest-latency path, while this
                                        // closes the race for coordinators that do not emit
                                        // state changes for semantic actions yet.
                                        engine_catalog_refresh_requested
                                            .store(true, Ordering::Relaxed);
                                    }
                                    Err(error) => {
                                        if let Ok(mut guard) = engine_conn_error.lock() {
                                            *guard = Some(format!("invoke controller action: {error}"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    EngineMessage::UpdatePeripheralPresentation {
                        key,
                        name,
                        icon,
                        group,
                        expected_revision,
                        fallback_names,
                    } => {
                        if connected {
                            if let Some(ref mut transport) = active_transport {
                                let mut params = serde_json::Map::new();
                                params.insert("key".to_string(), serde_json::Value::String(key));
                                if let Some(name) = name {
                                    params.insert("name".to_string(), serde_json::Value::String(name));
                                }
                                if let Some(icon) = icon {
                                    params.insert("icon".to_string(), serde_json::Value::String(icon));
                                }
                                if let Some(group) = group {
                                    params.insert("group".to_string(), serde_json::Value::String(group));
                                }
                                if let Some(revision) = expected_revision {
                                    params.insert(
                                        "expected_revision".to_string(),
                                        serde_json::Value::String(revision),
                                    );
                                }
                                let update = transport.call_controller(
                                    "controller.peripheral.presentation.update",
                                    serde_json::Value::Object(params),
                                );
                                let update = match update {
                                    Err(error) if controller_method_is_unavailable(&error) => {
                                        transport.call_controller(
                                            "controller.peripherals.set",
                                            serde_json::json!({"peripheral_names": fallback_names}),
                                        )
                                    }
                                    result => result,
                                };
                                match update {
                                    Ok(_) => {
                                        engine_catalog_refresh_requested
                                            .store(true, Ordering::Relaxed);
                                    }
                                    Err(error) => {
                                        if let Ok(mut guard) = engine_conn_error.lock() {
                                            *guard = Some(format!(
                                                "update peripheral presentation: {error}"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !engine_connected.load(Ordering::Relaxed) && active_transport.is_some() {
                active_transport = None;
                if let Ok(mut guard) = engine_transport_description.lock() {
                    *guard = None;
                }
                if let Ok(mut guard) = engine_capabilities.lock() {
                    *guard = None;
                }
                connected = false;
            }

            // A non-forced direct diagnostic connection must yield as soon as
            // PCController comes online. This continuously enforces the single
            // UART-owner contract instead of checking only at open time.
            if connected
                && active_transport
                    .as_ref()
                    .is_some_and(HardwareTransport::is_direct_serial)
                && std::env::var_os("PEALAYER_ALLOW_DIRECT_SERIAL").is_none()
                && last_owner_check.elapsed() >= Duration::from_secs(1)
            {
                last_owner_check = std::time::Instant::now();
                let coordinator_reachable =
                    crate::four_d::controller::ControllerClient::is_reachable(
                        crate::four_d::controller::DEFAULT_ENDPOINT,
                        Duration::from_millis(200),
                    );
                if should_yield_direct_transport(true, false, coordinator_reachable) {
                    if let Some(ref mut transport) = active_transport {
                        let _ = transport.send(Command::AllOff);
                    }
                    active_transport = None;
                    if let Ok(mut guard) = engine_transport_description.lock() {
                        *guard = None;
                    }
                    connected = false;
                    engine_connected.store(false, Ordering::Relaxed);
                    engine_connection_requested.store(false, Ordering::Relaxed);
                    if let Ok(mut guard) = engine_capabilities.lock() {
                        *guard = None;
                    }
                    if let Ok(mut guard) = engine_conn_error.lock() {
                        *guard = Some("direct diagnostic transport released because PCController became reachable and owns the UART".to_string());
                    }
                }
            }

            let catalog_refresh_requested =
                engine_catalog_refresh_requested.swap(false, Ordering::Relaxed);
            if connected
                // Live relay, telemetry, and status-LED changes arrive on the
                // controller WebSocket. This slow refresh is only a recovery
                // baseline for catalog/config changes or a missed push epoch.
                && (catalog_refresh_requested
                    || last_capability_refresh.elapsed() >= Duration::from_secs(30))
                && active_transport
                    .as_ref()
                    .is_some_and(|transport| !transport.is_direct_serial())
            {
                last_capability_refresh = std::time::Instant::now();
                if let Some(ref mut transport) = active_transport {
                    match transport.refresh_capabilities() {
                        Ok(capabilities) => {
                            if let Ok(mut guard) = engine_capabilities.lock() {
                                let mut capabilities = capabilities;
                                if let (Some(current), Some(ref mut refreshed)) =
                                    (guard.as_ref(), capabilities.as_mut())
                                {
                                    refreshed.preserve_newer_live_led_from(current);
                                }
                                *guard = capabilities;
                            }
                        }
                        Err(error) => {
                            if let Ok(mut guard) = engine_conn_error.lock() {
                                *guard =
                                    Some(format!("refresh PCController capabilities: {error}"));
                            }
                            engine_connected.store(false, Ordering::Relaxed);
                        }
                    }
                }
            }

            if estop_now && !was_estop {
                last_pwm_values.fill(0);
                if connected {
                    if let Some(ref mut transport) = active_transport {
                        let _ = transport.send(Command::AllOff);
                    }
                    let port_name = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    println!("[{}] ALL_OFF (E-STOP)", port_name);
                }
            }
            was_estop = estop_now;

            let is_playing_now = engine_playing.load(Ordering::Relaxed) && !estop_now;

            // Handle pause state transition
            if was_playing && !is_playing_now {
                last_pwm_values.fill(0);
                if connected {
                    if let Some(ref mut transport) = active_transport {
                        let _ = transport.call_controller(
                            "controller.command.execute",
                            serde_json::json!({"command": "macro cancel"}),
                        );
                        let _ = transport.call_controller(
                            "controller.command.execute",
                            serde_json::json!({"command": "strip stop"}),
                        );
                        active_strip_effect = None;
                        let _ = transport.send(Command::AllOff);
                    }
                    let port_name = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    println!("[{}] ALL_OFF (Pause)", port_name);
                }
            }
            if !was_playing && is_playing_now {
                let current_time = engine_time.load(Ordering::Relaxed);
                let desired = active_controller_strip_effect_at(
                    &controller_strip_effects,
                    current_time,
                )
                .map(str::to_owned);
                if active_strip_effect != desired {
                    if let Some(ref mut transport) = active_transport {
                        if active_strip_effect.is_some() {
                            let _ = transport.call_controller(
                                "controller.command.execute",
                                serde_json::json!({"command": "strip stop"}),
                            );
                        }
                        active_strip_effect = None;
                        if let Some(id) = desired.as_deref() {
                            if transport
                                .call_controller(
                                    "controller.command.execute",
                                    serde_json::json!({"command": format!("strip effect play {id}")}),
                                )
                                .is_ok()
                            {
                                active_strip_effect = Some(id.to_string());
                            }
                        }
                    }
                }
                current_controller_strip_effect_index = controller_strip_effects
                    .partition_point(|cue| cue.time_ms <= current_time);
            }
            was_playing = is_playing_now;

            if is_playing_now {
                let current_time = engine_time.load(Ordering::Relaxed);

                while current_controller_macro_index < controller_macros.len() {
                    let cue = &controller_macros[current_controller_macro_index];
                    if cue.time_ms > current_time {
                        break;
                    }
                    if connected {
                        if let Some(ref mut transport) = active_transport {
                            let command = format!("macro play {} {}", cue.id, cue.mode);
                            if let Err(error) = transport.call_controller(
                                "controller.command.execute",
                                serde_json::json!({"command": command}),
                            ) {
                                if let Ok(mut guard) = engine_conn_error.lock() {
                                    *guard = Some(format!("start controller macro {}: {error}", cue.id));
                                }
                                engine_connected.store(false, Ordering::Relaxed);
                            }
                        }
                    }
                    current_controller_macro_index += 1;
                }

                while current_controller_strip_effect_index < controller_strip_effects.len() {
                    let cue = &controller_strip_effects[current_controller_strip_effect_index];
                    if cue.time_ms > current_time {
                        break;
                    }
                    if connected {
                        if let Some(ref mut transport) = active_transport {
                            if !cue.start
                                && active_strip_effect.as_deref() != Some(cue.id.as_str())
                            {
                                current_controller_strip_effect_index += 1;
                                continue;
                            }
                            let command = if cue.start {
                                format!("strip effect play {}", cue.id)
                            } else {
                                "strip stop".to_string()
                            };
                            if let Err(error) = transport.call_controller(
                                "controller.command.execute",
                                serde_json::json!({"command": command}),
                            ) {
                                if let Ok(mut guard) = engine_conn_error.lock() {
                                    *guard = Some(format!(
                                        "{} strip effect {}: {error}",
                                        if cue.start { "start" } else { "stop" },
                                        cue.id
                                    ));
                                }
                            } else if cue.start {
                                active_strip_effect = Some(cue.id.clone());
                            } else {
                                active_strip_effect = None;
                            }
                        }
                    }
                    current_controller_strip_effect_index += 1;
                }

                // Process all actions that are due
                while current_queue_index < queue.len() {
                    let action = &queue[current_queue_index];
                    if action.time_ms <= current_time {
                        if connected {
                            let port_name = {
                                let guard = engine_port.lock().unwrap();
                                guard.clone()
                            };
                            let state_str = if action.state { "ON" } else { "OFF" };
                            println!("[{}] {}:{}", port_name, action.relay_id, state_str);

                            if let Some(ref mut transport) = active_transport {
                                let cmd = Command::RelaySet {
                                    id: action.relay_id,
                                    state: action.state,
                                };
                                if let Err(e) = transport.send(cmd) {
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(e);
                                    }
                                    engine_connected.store(false, Ordering::Relaxed);
                                }
                            }
                        }
                        current_queue_index += 1;
                    } else {
                        break;
                    }
                }

                // Evaluate continuous analog curves
                for track in &analog_tracks {
                    if track.enabled && !track.muted && (track.channel as usize) < 16 {
                        let ch = track.channel as usize;
                        let val = track.evaluate_u8(current_time);
                        if val != last_pwm_values[ch] {
                            last_pwm_values[ch] = val;
                            if connected {
                                if let Some(ref mut transport) = active_transport {
                                    let cmd = Command::PwmSet {
                                        channel: track.channel,
                                        value: val,
                                    };
                                    if let Err(e) = transport.send(cmd) {
                                        if let Ok(mut guard) = engine_conn_error.lock() {
                                            *guard = Some(e);
                                        }
                                        engine_connected.store(false, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Periodic watchdog heartbeat (every 50ms when connected)
            if connected
                && active_transport
                    .as_ref()
                    .is_some_and(HardwareTransport::needs_watchdog_ping)
                && last_ping.elapsed() >= Duration::from_millis(50)
            {
                if let Some(ref mut transport) = active_transport {
                    let _ = transport.send(Command::Ping);
                }
                last_ping = std::time::Instant::now();
            }

            thread::sleep(Duration::from_millis(5));
        }
    });

    EngineHandle {
        lifecycle,
        playback_time_ms,
        is_playing,
        estop_active,
        connection_requested,
        is_connected,
        serial_port,
        active_transport: active_transport_description,
        connection_error,
        hardware_capabilities,
        controller_call_results,
        catalog_refresh_requested,
        sender: tx,
    }
}

pub fn compile_timeline(
    timeline: &Timeline,
    muted: &std::collections::BTreeSet<u8>,
    soloed: &std::collections::BTreeSet<u8>,
) -> Vec<CompiledAction> {
    let mut compiled = Vec::new();

    let mut interesting_times: Vec<u64> = Vec::new();

    for instance in &timeline.instances {
        if let Some(effect) = timeline
            .templates
            .iter()
            .find(|t| t.id == instance.effect_id)
        {
            interesting_times.push(instance.start_time_ms);
            interesting_times.push(instance.start_time_ms + effect.duration_ms);

            for action in &effect.actions {
                interesting_times.push(instance.start_time_ms + action.offset_ms);
            }
        }
    }

    interesting_times.sort_unstable();
    interesting_times.dedup();

    let relay_ids = timeline
        .templates
        .iter()
        .flat_map(|effect| effect.actions.iter().map(|action| action.relay_id))
        .filter(|relay_id| *relay_id != 0)
        .collect::<std::collections::BTreeSet<_>>();
    let mut current_relay_states = std::collections::BTreeMap::<u8, bool>::new();

    let has_solo = !soloed.is_empty();

    for &t in &interesting_times {
        // Evaluate desired state based on Z-Index
        for relay_id in relay_ids.iter().copied() {
            let mut desired_state = false;

            let is_ignored = muted.contains(&relay_id) || (has_solo && !soloed.contains(&relay_id));

            if !is_ignored {
                // Reverse order = highest Z-index first
                for instance in timeline.instances.iter().rev() {
                    if let Some(effect) = timeline
                        .templates
                        .iter()
                        .find(|tmpl| tmpl.id == instance.effect_id)
                    {
                        let end_time = instance.start_time_ms + effect.duration_ms;

                        if t >= instance.start_time_ms && t < end_time {
                            let offset_t = t - instance.start_time_ms;

                            let mut latest_action_state = None;
                            let mut max_offset = 0;

                            for action in &effect.actions {
                                if action.relay_id == relay_id && action.offset_ms <= offset_t {
                                    // Find the action closest to the current time within this effect
                                    if latest_action_state.is_none()
                                        || action.offset_ms >= max_offset
                                    {
                                        max_offset = action.offset_ms;
                                        latest_action_state = Some(action.state);
                                    }
                                }
                            }

                            if let Some(state) = latest_action_state {
                                desired_state = state;
                                break; // Stop looking at lower layers
                            }
                        }
                    }
                }
            }

            if desired_state
                != current_relay_states
                    .get(&relay_id)
                    .copied()
                    .unwrap_or(false)
            {
                compiled.push(CompiledAction {
                    time_ms: t,
                    relay_id,
                    state: desired_state,
                });
                current_relay_states.insert(relay_id, desired_state);
            }
        }
    }

    compiled
}

pub fn compile_controller_macros(timeline: &Timeline) -> Vec<CompiledControllerMacro> {
    let mut compiled = timeline
        .instances
        .iter()
        .filter_map(|instance| {
            let effect = timeline
                .templates
                .iter()
                .find(|template| template.id == instance.effect_id)?;
            let controller_macro = effect.controller_macro.as_ref()?;
            Some(CompiledControllerMacro {
                time_ms: instance.start_time_ms,
                id: controller_macro.id,
                mode: controller_macro.mode.clone(),
            })
        })
        .collect::<Vec<_>>();
    compiled.sort_by_key(|cue| cue.time_ms);
    compiled
}

pub fn compile_controller_strip_effects(
    timeline: &Timeline,
) -> Vec<CompiledControllerStripEffect> {
    let mut compiled = Vec::new();
    for instance in &timeline.instances {
        let Some(effect) = timeline
            .templates
            .iter()
            .find(|template| template.id == instance.effect_id)
        else {
            continue;
        };
        let Some(strip) = effect.controller_strip_effect.as_ref() else {
            continue;
        };
        if !crate::four_d::controller::valid_strip_effect_id(&strip.id) {
            continue;
        }
        compiled.push(CompiledControllerStripEffect {
            time_ms: instance.start_time_ms,
            id: strip.id.clone(),
            start: true,
        });
        compiled.push(CompiledControllerStripEffect {
            time_ms: instance.start_time_ms.saturating_add(effect.duration_ms.max(1)),
            id: strip.id.clone(),
            start: false,
        });
    }
    compiled.sort_by(|left, right| {
        left.time_ms
            .cmp(&right.time_ms)
            // Stop a previous cue before starting another cue at the same frame.
            .then_with(|| left.start.cmp(&right.start))
    });
    compiled
}

fn active_controller_strip_effect_at(
    cues: &[CompiledControllerStripEffect],
    time_ms: u64,
) -> Option<&str> {
    let mut active = None;
    for cue in cues.iter().take_while(|cue| cue.time_ms <= time_ms) {
        if cue.start {
            active = Some(cue.id.as_str());
        } else if active == Some(cue.id.as_str()) {
            active = None;
        }
    }
    active
}

pub fn evaluate_relay_state(
    timeline: &Timeline,
    relay_id: u8,
    t_ms: u64,
    muted: &std::collections::BTreeSet<u8>,
    soloed: &std::collections::BTreeSet<u8>,
) -> bool {
    let has_solo = !soloed.is_empty();
    if muted.contains(&relay_id) || (has_solo && !soloed.contains(&relay_id)) {
        return false;
    }

    let mut desired_state = false;

    // Reverse order = highest Z-index first
    for instance in timeline.instances.iter().rev() {
        if let Some(effect) = timeline
            .templates
            .iter()
            .find(|tmpl| tmpl.id == instance.effect_id)
        {
            let end_time = instance.start_time_ms + effect.duration_ms;

            if t_ms >= instance.start_time_ms && t_ms < end_time {
                let offset_t = t_ms - instance.start_time_ms;

                let mut latest_action_state = None;
                let mut max_offset = 0;

                for action in &effect.actions {
                    if action.relay_id == relay_id && action.offset_ms <= offset_t {
                        if latest_action_state.is_none() || action.offset_ms >= max_offset {
                            max_offset = action.offset_ms;
                            latest_action_state = Some(action.state);
                        }
                    }
                }

                if let Some(state) = latest_action_state {
                    desired_state = state;
                    break; // Stop looking at lower layers
                }
            }
        }
    }

    desired_state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::four_d::models::{AtomicAction, Effect, EffectInstance, Timeline};

    #[test]
    fn compiles_controller_macro_as_one_video_synchronized_cue() {
        let mut timeline = Timeline::new();
        let effect = Effect::controller_macro(
            "Seat sweep".to_string(),
            String::new(),
            1_500,
            9,
            "mcu".to_string(),
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(effect_id, 2_250));

        assert_eq!(
            compile_controller_macros(&timeline),
            vec![CompiledControllerMacro {
                time_ms: 2_250,
                id: 9,
                mode: "mcu".to_string(),
            }]
        );
        assert!(compile_timeline(&timeline, &Default::default(), &Default::default()).is_empty());
    }

    #[test]
    fn compiles_advertised_strip_effect_into_bounded_start_and_stop_cues() {
        let mut timeline = Timeline::new();
        let effect = Effect::controller_strip_effect(
            "Police".to_string(),
            5_000,
            "police".to_string(),
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(effect_id, 2_250));

        assert_eq!(
            compile_controller_strip_effects(&timeline),
            vec![
                CompiledControllerStripEffect {
                    time_ms: 2_250,
                    id: "police".to_string(),
                    start: true,
                },
                CompiledControllerStripEffect {
                    time_ms: 7_250,
                    id: "police".to_string(),
                    start: false,
                },
            ]
        );
        let compiled = compile_controller_strip_effects(&timeline);
        assert_eq!(active_controller_strip_effect_at(&compiled, 2_249), None);
        assert_eq!(active_controller_strip_effect_at(&compiled, 2_250), Some("police"));
        assert_eq!(active_controller_strip_effect_at(&compiled, 7_250), None);

        let overlap = vec![
            CompiledControllerStripEffect { time_ms: 0, id: "police".to_string(), start: true },
            CompiledControllerStripEffect { time_ms: 50, id: "white-thunder".to_string(), start: true },
            CompiledControllerStripEffect { time_ms: 100, id: "police".to_string(), start: false },
            CompiledControllerStripEffect { time_ms: 150, id: "white-thunder".to_string(), start: false },
        ];
        assert_eq!(
            active_controller_strip_effect_at(&overlap, 100),
            Some("white-thunder")
        );
        assert_eq!(active_controller_strip_effect_at(&overlap, 150), None);

        let unsafe_effect = Effect::controller_strip_effect(
            "Malformed".to_string(),
            5_000,
            "police 100 30".to_string(),
        );
        let unsafe_effect_id = unsafe_effect.id;
        timeline.templates.push(unsafe_effect);
        timeline
            .instances
            .push(EffectInstance::new(unsafe_effect_id, 10_000));
        let compiled = compile_controller_strip_effects(&timeline);
        assert_eq!(compiled.len(), 2);
        assert!(compiled.iter().all(|cue| cue.id == "police"));
    }

    #[test]
    fn test_compile_timeline_basic() {
        let mut timeline = Timeline::new();
        let effect = Effect::new(
            "Test Constant".to_string(),
            "🧪".to_string(),
            1000,
            vec![
                AtomicAction {
                    relay_id: 1,
                    state: true,
                    offset_ms: 0,
                },
                AtomicAction {
                    relay_id: 1,
                    state: false,
                    offset_ms: 1000,
                },
            ],
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);

        let instance = EffectInstance::new(effect_id, 500);
        timeline.instances.push(instance);

        let muted = std::collections::BTreeSet::new();
        let soloed = std::collections::BTreeSet::new();
        let compiled = compile_timeline(&timeline, &muted, &soloed);

        assert_eq!(compiled.len(), 2);
        assert_eq!(compiled[0].time_ms, 500);
        assert_eq!(compiled[0].relay_id, 1);
        assert_eq!(compiled[0].state, true);

        assert_eq!(compiled[1].time_ms, 1500);
        assert_eq!(compiled[1].relay_id, 1);
        assert_eq!(compiled[1].state, false);
    }

    #[test]
    fn test_compile_timeline_overlap() {
        let mut timeline = Timeline::new();

        // Effect A: relay 1 ON at 0, OFF at 1000
        let effect_a = Effect::new(
            "Effect A".to_string(),
            "A".to_string(),
            1000,
            vec![
                AtomicAction {
                    relay_id: 1,
                    state: true,
                    offset_ms: 0,
                },
                AtomicAction {
                    relay_id: 1,
                    state: false,
                    offset_ms: 1000,
                },
            ],
        );
        let id_a = effect_a.id;
        timeline.templates.push(effect_a);

        // Effect B: relay 1 OFF at 0, ON at 500, OFF at 1000 (effectively starts OFF then turns ON)
        let effect_b = Effect::new(
            "Effect B".to_string(),
            "B".to_string(),
            1000,
            vec![
                AtomicAction {
                    relay_id: 1,
                    state: false,
                    offset_ms: 0,
                },
                AtomicAction {
                    relay_id: 1,
                    state: true,
                    offset_ms: 500,
                },
                AtomicAction {
                    relay_id: 1,
                    state: false,
                    offset_ms: 1000,
                },
            ],
        );
        let id_b = effect_b.id;
        timeline.templates.push(effect_b);

        // Instance A placed at 0ms.
        timeline.instances.push(EffectInstance::new(id_a, 0));
        // Instance B placed at 200ms. Since it is pushed later, it has higher Z-index.
        timeline.instances.push(EffectInstance::new(id_b, 200));

        let muted = std::collections::BTreeSet::new();
        let soloed = std::collections::BTreeSet::new();

        // Let's verify state at 300ms.
        // For Instance A (offset 300): it should be ON.
        // For Instance B (offset 100): it should be OFF.
        // Since Instance B has higher Z-index, the state at 300ms should be OFF.
        let state_300 = evaluate_relay_state(&timeline, 1, 300, &muted, &soloed);
        assert_eq!(state_300, false);

        // At 800ms:
        // Instance A (offset 800): ON
        // Instance B (offset 600): ON
        let state_800 = evaluate_relay_state(&timeline, 1, 800, &muted, &soloed);
        assert_eq!(state_800, true);
    }

    #[test]
    fn test_compile_timeline_muted_soloed() {
        let mut timeline = Timeline::new();
        let effect = Effect::new(
            "Test Constant".to_string(),
            "🧪".to_string(),
            1000,
            vec![
                AtomicAction {
                    relay_id: 1,
                    state: true,
                    offset_ms: 0,
                },
                AtomicAction {
                    relay_id: 2,
                    state: true,
                    offset_ms: 0,
                },
            ],
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(effect_id, 500));

        // Mute Relay 1
        let muted = std::collections::BTreeSet::from([1]);
        let soloed = std::collections::BTreeSet::new();

        let compiled = compile_timeline(&timeline, &muted, &soloed);
        // Only Relay 2 should produce compiled actions
        assert!(compiled.iter().all(|act| act.relay_id != 1));
        assert!(compiled.iter().any(|act| act.relay_id == 2));

        // Solo Relay 1
        let muted = std::collections::BTreeSet::new();
        let soloed = std::collections::BTreeSet::from([1]);

        let compiled = compile_timeline(&timeline, &muted, &soloed);
        // Only Relay 1 should produce compiled actions since it is soloed
        assert!(compiled.iter().any(|act| act.relay_id == 1));
        assert!(compiled.iter().all(|act| act.relay_id != 2));
    }

    #[test]
    fn test_engine_message_send_command() {
        let handle = spawn_engine();
        assert!(!handle.connection_requested.load(Ordering::Relaxed));
        assert!(!handle.is_connected.load(Ordering::Relaxed));
        let res = handle
            .sender
            .send(EngineMessage::SendCommand(Command::PwmSet {
                channel: 1,
                value: 200,
            }));
        assert!(res.is_ok());
        let res_all_off = handle
            .sender
            .send(EngineMessage::SendCommand(Command::AllOff));
        assert!(res_all_off.is_ok());
    }

    #[test]
    fn test_engine_message_update_analog_tracks() {
        let handle = spawn_engine();
        let mut track = crate::four_d::curve::AnalogTrack::new("Wind Fan", 0);
        track.add_keyframe(crate::four_d::curve::Keyframe::new(
            0,
            0.5,
            crate::four_d::curve::Interpolation::Linear,
        ));
        let res = handle
            .sender
            .send(EngineMessage::UpdateAnalogTracks(vec![track]));
        assert!(res.is_ok());
    }

    #[test]
    fn direct_transport_yields_to_coordinator_unless_explicitly_forced() {
        assert!(should_yield_direct_transport(true, false, true));
        assert!(!should_yield_direct_transport(true, true, true));
        assert!(!should_yield_direct_transport(true, false, false));
        assert!(!should_yield_direct_transport(false, false, true));
    }

    #[test]
    fn presentation_change_notification_requests_authoritative_catalog_refresh() {
        let handle = spawn_engine();
        let target = handle.controller_push_target();
        assert!(!handle.catalog_refresh_requested.load(Ordering::Relaxed));
        assert!(target.apply_notification(
            "controller.state",
            &serde_json::json!({"kind": "peripherals.changed", "action": "refresh"}),
        ));
        assert!(handle.catalog_refresh_requested.load(Ordering::Relaxed));
    }

    #[test]
    fn presentation_fallback_only_accepts_unknown_method_errors() {
        assert!(controller_method_is_unavailable(
            "PCController JSON-RPC error: {\"code\":-32601,\"message\":\"method not found\"}"
        ));
        assert!(!controller_method_is_unavailable(
            "PCController JSON-RPC error: revision conflict"
        ));
    }

    #[test]
    fn controller_push_target_tracks_selected_coordinator_and_lifetime() {
        let handle = spawn_engine();
        let target = handle.controller_push_target();
        assert!(target.is_alive());
        assert_eq!(target.websocket_endpoint(), None);

        *handle.serial_port.lock().unwrap() = "pccontroller://controller.test:9000".to_string();
        handle.connection_requested.store(true, Ordering::Relaxed);
        assert_eq!(
            target.websocket_endpoint().as_deref(),
            Some("ws://controller.test:9000/ipc")
        );

        *handle.serial_port.lock().unwrap() = "direct:COM9".to_string();
        assert_eq!(target.websocket_endpoint(), None);
        drop(handle);
        assert!(!target.is_alive());
    }

    #[test]
    fn controller_error_immediately_marks_the_board_disconnected() {
        let handle = spawn_engine();
        *handle.hardware_capabilities.lock().unwrap() =
            Some(crate::four_d::controller::HardwareCapabilities {
                board_connected: true,
                status_led: Some(crate::four_d::controller::HardwareStatusLed {
                    red: 10,
                    green: 20,
                    blue: 30,
                    brightness: 255,
                    effect: 0,
                    condition: 0,
                }),
                ..Default::default()
            });
        let target = handle.controller_push_target();
        assert!(target.apply_notification(
            "controller.error",
            &serde_json::json!({"message": "board disconnected"}),
        ));
        let capabilities = handle.hardware_capabilities.lock().unwrap();
        let capabilities = capabilities.as_ref().unwrap();
        assert!(!capabilities.board_connected);
        assert!(capabilities.status_led.is_none());
    }

    #[test]
    fn restarted_controller_accepts_low_led_revisions_from_new_instance() {
        let handle = spawn_engine();
        *handle.hardware_capabilities.lock().unwrap() =
            Some(crate::four_d::controller::HardwareCapabilities {
                board_connected: true,
                host_instance_id: "old-host".to_string(),
                status_led_revision: 100,
                status_led: Some(crate::four_d::controller::HardwareStatusLed::default()),
                ..Default::default()
            });
        let target = handle.controller_push_target();
        assert!(target.observe_source_instance("new-host"));
        assert!(target.apply_notification(
            "controller.state",
            &serde_json::json!({
                "kind": "status_led.changed",
                "metadata": {
                    "red": "1", "green": "2", "blue": "3",
                    "brightness": "255", "effect": "0", "condition": "0",
                    "revision": "1"
                }
            }),
        ));
        let capabilities = handle.hardware_capabilities.lock().unwrap();
        let capabilities = capabilities.as_ref().unwrap();
        assert_eq!(capabilities.host_instance_id, "new-host");
        assert_eq!(capabilities.status_led_revision, 1);
        assert_eq!(capabilities.status_led.as_ref().unwrap().red, 1);
    }
}
