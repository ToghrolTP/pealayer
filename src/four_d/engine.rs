use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::four_d::models::Timeline;

#[derive(Debug, Clone)]
pub struct CompiledAction {
    pub time_ms: u64,
    pub relay_id: u8,
    pub state: bool,
}

pub enum EngineMessage {
    UpdateQueue(Vec<CompiledAction>),
    Seek(u64), // Emitted when user seeks, to clear current active queue and reset hardware
}

pub struct EngineHandle {
    pub playback_time_ms: Arc<AtomicU64>,
    pub is_playing: Arc<AtomicBool>,
    pub estop_active: Arc<AtomicBool>,
    pub is_connected: Arc<AtomicBool>,
    pub serial_port: Arc<Mutex<String>>,
    pub connection_error: Arc<Mutex<Option<String>>>,
    pub sender: mpsc::Sender<EngineMessage>,
}

pub fn spawn_engine() -> EngineHandle {
    let playback_time_ms = Arc::new(AtomicU64::new(0));
    let is_playing = Arc::new(AtomicBool::new(false));
    let estop_active = Arc::new(AtomicBool::new(false));
    let is_connected = Arc::new(AtomicBool::new(false));
    let serial_port = Arc::new(Mutex::new("COM3".to_string()));
    let connection_error = Arc::new(Mutex::new(None));
    
    let (tx, rx) = mpsc::channel();
    
    let engine_time = Arc::clone(&playback_time_ms);
    let engine_playing = Arc::clone(&is_playing);
    let engine_estop = Arc::clone(&estop_active);
    let engine_connected = Arc::clone(&is_connected);
    let engine_port = Arc::clone(&serial_port);
    let engine_conn_error = Arc::clone(&connection_error);
    
    thread::spawn(move || {
        let mut queue: Vec<CompiledAction> = Vec::new();
        let mut current_queue_index = 0;
        let mut was_playing = false;
        let mut was_estop = false;
        
        let mut active_port: Option<Box<dyn serialport::SerialPort>> = None;
        
        loop {
            let estop_now = engine_estop.load(Ordering::Relaxed);
            let connected = engine_connected.load(Ordering::Relaxed);
            
            // Handle connection/disconnection transitions
            if connected && active_port.is_none() {
                let port_name = {
                    let guard = engine_port.lock().unwrap();
                    guard.clone()
                };
                match serialport::new(&port_name, 9600)
                    .timeout(Duration::from_millis(15))
                    .open()
                {
                    Ok(p) => {
                        active_port = Some(p);
                        println!("[Engine] Connected to serial port: {}", port_name);
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to open port {}: {}", port_name, e);
                        if let Ok(mut guard) = engine_conn_error.lock() {
                            *guard = Some(err_msg);
                        }
                        engine_connected.store(false, Ordering::Relaxed);
                    }
                }
            } else if !connected && active_port.is_some() {
                // Graceful disconnect: turn off relays
                if let Some(ref mut port) = active_port {
                    for i in 1..=8 {
                        let _ = port.write_all(format!("R{}:0\n", i).as_bytes());
                    }
                }
                active_port = None;
                println!("[Engine] Disconnected from serial port");
            }
            
            // Check for new messages (non-blocking)
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    EngineMessage::UpdateQueue(new_queue) => {
                        queue = new_queue;
                        let current_time = engine_time.load(Ordering::Relaxed);
                        current_queue_index = queue.partition_point(|x| x.time_ms < current_time);
                    }
                    EngineMessage::Seek(time) => {
                        if connected {
                            if let Some(ref mut port) = active_port {
                                for i in 1..=8 {
                                    let _ = port.write_all(format!("R{}:0\n", i).as_bytes());
                                }
                            }
                            let port_name = {
                                let guard = engine_port.lock().unwrap();
                                guard.clone()
                            };
                            for i in 1..=8 {
                                println!("[{}] {}:OFF", port_name, i);
                            }
                        }
                        current_queue_index = queue.partition_point(|x| x.time_ms < time);
                    }
                }
            }
            
            if estop_now && !was_estop {
                if connected {
                    if let Some(ref mut port) = active_port {
                        for i in 1..=8 {
                            let _ = port.write_all(format!("R{}:0\n", i).as_bytes());
                        }
                    }
                    let port_name = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    for i in 1..=8 {
                        println!("[{}] {}:OFF (E-STOP)", port_name, i);
                    }
                }
            }
            was_estop = estop_now;
            
            let is_playing_now = engine_playing.load(Ordering::Relaxed) && !estop_now;
            
            // Handle pause state transition
            if was_playing && !is_playing_now {
                if connected {
                    if let Some(ref mut port) = active_port {
                        for i in 1..=8 {
                            let _ = port.write_all(format!("R{}:0\n", i).as_bytes());
                        }
                    }
                    let port_name = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    for i in 1..=8 {
                        println!("[{}] {}:OFF", port_name, i);
                    }
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
                            
                            if let Some(ref mut port) = active_port {
                                let cmd = format!("R{}:{}\n", action.relay_id, if action.state { 1 } else { 0 });
                                if let Err(e) = port.write_all(cmd.as_bytes()) {
                                    let err_msg = format!("Serial write error: {}", e);
                                    if let Ok(mut guard) = engine_conn_error.lock() {
                                        *guard = Some(err_msg);
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
            }
            
            thread::sleep(Duration::from_millis(5));
        }
    });

    EngineHandle {
        playback_time_ms,
        is_playing,
        estop_active,
        is_connected,
        serial_port,
        connection_error,
        sender: tx,
    }
}

pub fn compile_timeline(timeline: &Timeline, muted: &[bool; 9], soloed: &[bool; 9]) -> Vec<CompiledAction> {
    let mut compiled = Vec::new();
    
    let mut interesting_times: Vec<u64> = Vec::new();
    
    for instance in &timeline.instances {
        if let Some(effect) = timeline.templates.iter().find(|t| t.id == instance.effect_id) {
            interesting_times.push(instance.start_time_ms);
            interesting_times.push(instance.start_time_ms + effect.duration_ms);
            
            for action in &effect.actions {
                interesting_times.push(instance.start_time_ms + action.offset_ms);
            }
        }
    }
    
    interesting_times.sort_unstable();
    interesting_times.dedup();
    
    // Track the currently emitted state of each relay (1-8)
    let mut current_relay_states = [false; 9]; // index 0 is unused
    
    let has_solo = soloed.iter().any(|&s| s);
    
    for &t in &interesting_times {
        // Evaluate desired state based on Z-Index
        for relay_id in 1..=8 {
            let mut desired_state = false;
            
            let is_ignored = muted[relay_id as usize] || (has_solo && !soloed[relay_id as usize]);
            
            if !is_ignored {
                // Reverse order = highest Z-index first
                for instance in timeline.instances.iter().rev() {
                    if let Some(effect) = timeline.templates.iter().find(|tmpl| tmpl.id == instance.effect_id) {
                        let end_time = instance.start_time_ms + effect.duration_ms;
                        
                        if t >= instance.start_time_ms && t < end_time {
                            let offset_t = t - instance.start_time_ms;
                            
                            let mut latest_action_state = None;
                            let mut max_offset = 0;
                            
                            for action in &effect.actions {
                                if action.relay_id == relay_id && action.offset_ms <= offset_t {
                                    // Find the action closest to the current time within this effect
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
            }
            
            if desired_state != current_relay_states[relay_id as usize] {
                compiled.push(CompiledAction {
                    time_ms: t,
                    relay_id,
                    state: desired_state,
                });
                current_relay_states[relay_id as usize] = desired_state;
            }
        }
    }
    
    compiled
}

pub fn evaluate_relay_state(timeline: &Timeline, relay_id: u8, t_ms: u64, muted: &[bool; 9], soloed: &[bool; 9]) -> bool {
    let has_solo = soloed.iter().any(|&s| s);
    if muted[relay_id as usize] || (has_solo && !soloed[relay_id as usize]) {
        return false;
    }
    
    let mut desired_state = false;
    
    // Reverse order = highest Z-index first
    for instance in timeline.instances.iter().rev() {
        if let Some(effect) = timeline.templates.iter().find(|tmpl| tmpl.id == instance.effect_id) {
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
    use crate::four_d::models::{Timeline, Effect, EffectInstance, AtomicAction};

    #[test]
    fn test_compile_timeline_basic() {
        let mut timeline = Timeline::new();
        let effect = Effect::new(
            "Test Constant".to_string(),
            "🧪".to_string(),
            1000,
            vec![
                AtomicAction { relay_id: 1, state: true, offset_ms: 0 },
                AtomicAction { relay_id: 1, state: false, offset_ms: 1000 },
            ],
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);
        
        let instance = EffectInstance::new(effect_id, 500);
        timeline.instances.push(instance);
        
        let muted = [false; 9];
        let soloed = [false; 9];
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
                AtomicAction { relay_id: 1, state: true, offset_ms: 0 },
                AtomicAction { relay_id: 1, state: false, offset_ms: 1000 },
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
                AtomicAction { relay_id: 1, state: false, offset_ms: 0 },
                AtomicAction { relay_id: 1, state: true, offset_ms: 500 },
                AtomicAction { relay_id: 1, state: false, offset_ms: 1000 },
            ],
        );
        let id_b = effect_b.id;
        timeline.templates.push(effect_b);
        
        // Instance A placed at 0ms.
        timeline.instances.push(EffectInstance::new(id_a, 0));
        // Instance B placed at 200ms. Since it is pushed later, it has higher Z-index.
        timeline.instances.push(EffectInstance::new(id_b, 200));
        
        let muted = [false; 9];
        let soloed = [false; 9];
        
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
                AtomicAction { relay_id: 1, state: true, offset_ms: 0 },
                AtomicAction { relay_id: 2, state: true, offset_ms: 0 },
            ],
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(effect_id, 500));
        
        // Mute Relay 1
        let mut muted = [false; 9];
        muted[1] = true;
        let soloed = [false; 9];
        
        let compiled = compile_timeline(&timeline, &muted, &soloed);
        // Only Relay 2 should produce compiled actions
        assert!(compiled.iter().all(|act| act.relay_id != 1));
        assert!(compiled.iter().any(|act| act.relay_id == 2));
        
        // Solo Relay 1
        let muted = [false; 9];
        let mut soloed = [false; 9];
        soloed[1] = true;
        
        let compiled = compile_timeline(&timeline, &muted, &soloed);
        // Only Relay 1 should produce compiled actions since it is soloed
        assert!(compiled.iter().any(|act| act.relay_id == 1));
        assert!(compiled.iter().all(|act| act.relay_id != 2));
    }
}


