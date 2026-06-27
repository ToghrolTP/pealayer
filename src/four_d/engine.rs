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
    pub sender: mpsc::Sender<EngineMessage>,
}

pub fn spawn_engine() -> EngineHandle {
    let playback_time_ms = Arc::new(AtomicU64::new(0));
    let is_playing = Arc::new(AtomicBool::new(false));
    let estop_active = Arc::new(AtomicBool::new(false));
    let is_connected = Arc::new(AtomicBool::new(false));
    let serial_port = Arc::new(Mutex::new("COM3".to_string()));
    
    let (tx, rx) = mpsc::channel();
    
    let engine_time = Arc::clone(&playback_time_ms);
    let engine_playing = Arc::clone(&is_playing);
    let engine_estop = Arc::clone(&estop_active);
    let engine_connected = Arc::clone(&is_connected);
    let engine_port = Arc::clone(&serial_port);
    
    thread::spawn(move || {
        let mut queue: Vec<CompiledAction> = Vec::new();
        let mut current_queue_index = 0;
        let mut was_playing = false;
        let mut was_estop = false;
        
        loop {
            let estop_now = engine_estop.load(Ordering::Relaxed);
            let connected = engine_connected.load(Ordering::Relaxed);
            
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
                            let port = {
                                let guard = engine_port.lock().unwrap();
                                guard.clone()
                            };
                            for i in 1..=8 {
                                println!("[{}] {}:OFF", port, i);
                            }
                        }
                        current_queue_index = queue.partition_point(|x| x.time_ms < time);
                    }
                }
            }
            
            if estop_now && !was_estop {
                if connected {
                    let port = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    for i in 1..=8 {
                        println!("[{}] {}:OFF (E-STOP)", port, i);
                    }
                }
            }
            was_estop = estop_now;
            
            let is_playing_now = engine_playing.load(Ordering::Relaxed) && !estop_now;
            
            // Handle pause state transition
            if was_playing && !is_playing_now {
                if connected {
                    let port = {
                        let guard = engine_port.lock().unwrap();
                        guard.clone()
                    };
                    for i in 1..=8 {
                        println!("[{}] {}:OFF", port, i);
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
                            let port = {
                                let guard = engine_port.lock().unwrap();
                                guard.clone()
                            };
                            let state_str = if action.state { "ON" } else { "OFF" };
                            println!("[{}] {}:{}", port, action.relay_id, state_str);
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

