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

    fn refresh_capabilities(
        &mut self,
    ) -> Result<Option<crate::four_d::controller::HardwareCapabilities>, String> {
        match self {
            Self::Controller(client) => client.hardware_capabilities().map(Some),
            Self::DirectSerial { .. } => Ok(None),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledAction {
    pub time_ms: u64,
    pub relay_id: u8,
    pub state: bool,
}

pub enum EngineMessage {
    UpdateQueue(Vec<CompiledAction>),
    UpdateAnalogTracks(Vec<crate::four_d::curve::AnalogTrack>),
    LiveActuatorOverride { channel: u8, value: u8 },
    Seek(u64), // Emitted when user seeks, to clear current active queue and reset hardware
    SendCommand(Command), // Manual override or direct hardware command
}

pub struct EngineHandle {
    pub playback_time_ms: Arc<AtomicU64>,
    pub is_playing: Arc<AtomicBool>,
    pub estop_active: Arc<AtomicBool>,
    pub connection_requested: Arc<AtomicBool>,
    pub is_connected: Arc<AtomicBool>,
    pub serial_port: Arc<Mutex<String>>,
    pub connection_error: Arc<Mutex<Option<String>>>,
    pub hardware_capabilities: Arc<Mutex<Option<crate::four_d::controller::HardwareCapabilities>>>,
    pub sender: mpsc::Sender<EngineMessage>,
}

fn should_yield_direct_transport(
    is_direct: bool,
    diagnostic_override: bool,
    coordinator_reachable: bool,
) -> bool {
    is_direct && !diagnostic_override && coordinator_reachable
}

pub fn spawn_engine() -> EngineHandle {
    let playback_time_ms = Arc::new(AtomicU64::new(0));
    let is_playing = Arc::new(AtomicBool::new(false));
    let estop_active = Arc::new(AtomicBool::new(false));
    let connection_requested = Arc::new(AtomicBool::new(false));
    let is_connected = Arc::new(AtomicBool::new(false));
    let serial_port = Arc::new(Mutex::new(
        crate::four_d::controller::DEFAULT_ENDPOINT.to_string(),
    ));
    let connection_error = Arc::new(Mutex::new(None));
    let hardware_capabilities = Arc::new(Mutex::new(None));

    let (tx, rx) = mpsc::channel();

    let engine_time = Arc::clone(&playback_time_ms);
    let engine_playing = Arc::clone(&is_playing);
    let engine_estop = Arc::clone(&estop_active);
    let engine_connection_requested = Arc::clone(&connection_requested);
    let engine_connected = Arc::clone(&is_connected);
    let engine_port = Arc::clone(&serial_port);
    let engine_conn_error = Arc::clone(&connection_error);
    let engine_capabilities = Arc::clone(&hardware_capabilities);

    thread::spawn(move || {
        let mut queue: Vec<CompiledAction> = Vec::new();
        let mut current_queue_index = 0;
        let mut analog_tracks: Vec<crate::four_d::curve::AnalogTrack> = Vec::new();
        let mut last_pwm_values = [0u8; 16];
        let mut was_playing = false;
        let mut was_estop = false;
        let mut last_ping = std::time::Instant::now();
        let mut last_owner_check = std::time::Instant::now();
        let mut last_capability_refresh = std::time::Instant::now();

        let mut active_transport: Option<HardwareTransport> = None;

        loop {
            let estop_now = engine_estop.load(Ordering::Relaxed);
            let requested = engine_connection_requested.load(Ordering::Relaxed);
            let mut connected = active_transport.is_some();
            engine_connected.store(connected, Ordering::Relaxed);

            // Handle connection/disconnection transitions
            if requested && active_transport.is_none() {
                let endpoint = {
                    let guard = engine_port.lock().unwrap();
                    guard.clone()
                };
                let transport = if crate::four_d::controller::is_controller_endpoint(&endpoint) {
                    crate::four_d::controller::ControllerClient::connect(&endpoint)
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
                        println!("[Engine] Connected hardware transport: {endpoint}");
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
                        }
                        engine_connected.store(true, Ordering::Relaxed);
                        connected = true;
                        last_ping = std::time::Instant::now();
                        last_owner_check = std::time::Instant::now();
                        last_capability_refresh = std::time::Instant::now();
                    }
                    Err(error) => {
                        if let Ok(mut guard) = engine_conn_error.lock() {
                            *guard = Some(error);
                        }
                        engine_connection_requested.store(false, Ordering::Relaxed);
                        engine_connected.store(false, Ordering::Relaxed);
                    }
                }
            } else if !requested && active_transport.is_some() {
                // Graceful disconnect: send AllOff
                if let Some(ref mut transport) = active_transport {
                    let _ = transport.send(Command::AllOff);
                }
                active_transport = None;
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
                                    }
                                }
                            }
                        }
                    }
                    EngineMessage::Seek(time) => {
                        if connected {
                            if let Some(ref mut transport) = active_transport {
                                if let Err(e) = transport.send(Command::AllOff) {
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(e);
                                    }
                                    engine_connected.store(false, Ordering::Relaxed);
                                }
                            }
                            let port_name = {
                                let guard = engine_port.lock().unwrap();
                                guard.clone()
                            };
                            println!("[{}] ALL_OFF (Seek to {}ms)", port_name, time);
                        }
                        current_queue_index = queue.partition_point(|x| x.time_ms < time);
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
                }
            }

            if !engine_connected.load(Ordering::Relaxed) && active_transport.is_some() {
                active_transport = None;
                if let Ok(mut guard) = engine_capabilities.lock() {
                    *guard = None;
                }
                engine_connection_requested.store(false, Ordering::Relaxed);
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

            if connected
                && last_capability_refresh.elapsed() >= Duration::from_secs(1)
                && active_transport
                    .as_ref()
                    .is_some_and(|transport| !transport.is_direct_serial())
            {
                last_capability_refresh = std::time::Instant::now();
                if let Some(ref mut transport) = active_transport {
                    match transport.refresh_capabilities() {
                        Ok(capabilities) => {
                            if let Ok(mut guard) = engine_capabilities.lock() {
                                *guard = capabilities;
                            }
                        }
                        Err(error) => {
                            if let Ok(mut guard) = engine_conn_error.lock() {
                                *guard =
                                    Some(format!("refresh PCController capabilities: {error}"));
                            }
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
                        let _ = transport.send(Command::AllOff);
                    }
                    let port_name = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    println!("[{}] ALL_OFF (Pause)", port_name);
                }
            }
            was_playing = is_playing_now;

            if is_playing_now {
                let current_time = engine_time.load(Ordering::Relaxed);

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
        playback_time_ms,
        is_playing,
        estop_active,
        connection_requested,
        is_connected,
        serial_port,
        connection_error,
        hardware_capabilities,
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
}
