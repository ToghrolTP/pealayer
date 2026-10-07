use crate::config::{HardwareKeyBinding, KeyChord};
use eframe::egui;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareShortcutEvent {
    pub binding_id: String,
    pub pressed: bool,
}

/// Owns the native registration handles. `global-hotkey` maps to RegisterHotKey
/// on Windows, Carbon hotkeys on macOS, and X11 grabs on Linux. It also reports
/// key-up events, which lets a global hold binding release hardware reliably.
pub struct GlobalHardwareShortcutRuntime {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<HotKey>,
    binding_ids_by_hotkey: BTreeMap<u32, Vec<String>>,
    fingerprint: u64,
    pub errors: BTreeMap<String, String>,
}

impl Default for GlobalHardwareShortcutRuntime {
    fn default() -> Self {
        Self {
            manager: None,
            registered: Vec::new(),
            binding_ids_by_hotkey: BTreeMap::new(),
            fingerprint: u64::MAX,
            errors: BTreeMap::new(),
        }
    }
}

impl GlobalHardwareShortcutRuntime {
    pub fn sync(&mut self, bindings: &[HardwareKeyBinding]) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(bindings)
            .unwrap_or_default()
            .hash(&mut hasher);
        let fingerprint = hasher.finish();
        if fingerprint == self.fingerprint {
            return;
        }
        self.fingerprint = fingerprint;
        self.errors.clear();
        self.binding_ids_by_hotkey.clear();

        if let Some(manager) = self.manager.as_ref() {
            let _ = manager.unregister_all(&self.registered);
        }
        self.registered.clear();

        let globals = bindings
            .iter()
            .filter(|binding| binding.enabled && binding.global)
            .collect::<Vec<_>>();
        if globals.is_empty() {
            return;
        }
        if self.manager.is_none() {
            match GlobalHotKeyManager::new() {
                Ok(manager) => self.manager = Some(manager),
                Err(error) => {
                    let message = native_backend_error(error.to_string());
                    for binding in globals {
                        self.errors.insert(binding.id.clone(), message.clone());
                    }
                    return;
                }
            }
        }

        let mut unique = BTreeMap::<u32, HotKey>::new();
        for binding in globals {
            match binding.chord.native_hotkey_string().parse::<HotKey>() {
                Ok(hotkey) => {
                    unique.entry(hotkey.id()).or_insert(hotkey);
                    self.binding_ids_by_hotkey
                        .entry(hotkey.id())
                        .or_default()
                        .push(binding.id.clone());
                }
                Err(error) => {
                    self.errors.insert(binding.id.clone(), error.to_string());
                }
            }
        }

        let Some(manager) = self.manager.as_ref() else {
            return;
        };
        for (id, hotkey) in unique {
            match manager.register(hotkey) {
                Ok(()) => self.registered.push(hotkey),
                Err(error) => {
                    let message = native_backend_error(error.to_string());
                    if let Some(binding_ids) = self.binding_ids_by_hotkey.remove(&id) {
                        for binding_id in binding_ids {
                            self.errors.insert(binding_id, message.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn drain_events(&self) -> Vec<HardwareShortcutEvent> {
        let mut events = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            let Some(binding_ids) = self.binding_ids_by_hotkey.get(&event.id) else {
                continue;
            };
            let pressed = event.state == HotKeyState::Pressed;
            events.extend(
                binding_ids
                    .iter()
                    .cloned()
                    .map(|binding_id| HardwareShortcutEvent {
                        binding_id,
                        pressed,
                    }),
            );
        }
        events
    }
}

fn native_backend_error(error: String) -> String {
    #[cfg(target_os = "linux")]
    {
        format!("{error} (global shortcuts require an X11 session)")
    }
    #[cfg(not(target_os = "linux"))]
    {
        error
    }
}

pub fn chord_from_egui_event(event: &egui::Event) -> Option<KeyChord> {
    let egui::Event::Key {
        key,
        physical_key,
        pressed: true,
        repeat: false,
        modifiers,
    } = event
    else {
        return None;
    };
    let key = physical_key.unwrap_or(*key);
    Some(KeyChord {
        key: egui_key_code(key)?.to_string(),
        control: modifiers.ctrl,
        alt: modifiers.alt,
        shift: modifiers.shift,
        super_key: modifiers.mac_cmd,
    })
}

pub fn event_matches_chord(event: &egui::Event, chord: &KeyChord) -> Option<bool> {
    let egui::Event::Key {
        key,
        physical_key,
        pressed,
        repeat,
        modifiers,
    } = event
    else {
        return None;
    };
    if *repeat && *pressed {
        return None;
    }
    let key = physical_key.unwrap_or(*key);
    let key_matches = egui_key_code(key)? == chord.key;
    let modifiers_match = modifiers.ctrl == chord.control
        && modifiers.alt == chord.alt
        && modifiers.shift == chord.shift
        && modifiers.mac_cmd == chord.super_key;
    // Users can release a modifier before the primary key. A hold binding must
    // still receive that primary-key release or the hardware action could stay
    // active. The caller only dispatches release to bindings it marked active.
    (key_matches && (!*pressed || modifiers_match)).then_some(*pressed)
}

pub(crate) fn egui_key_code(key: egui::Key) -> Option<&'static str> {
    use egui::Key;
    Some(match key {
        Key::A => "KeyA",
        Key::B => "KeyB",
        Key::C => "KeyC",
        Key::D => "KeyD",
        Key::E => "KeyE",
        Key::F => "KeyF",
        Key::G => "KeyG",
        Key::H => "KeyH",
        Key::I => "KeyI",
        Key::J => "KeyJ",
        Key::K => "KeyK",
        Key::L => "KeyL",
        Key::M => "KeyM",
        Key::N => "KeyN",
        Key::O => "KeyO",
        Key::P => "KeyP",
        Key::Q => "KeyQ",
        Key::R => "KeyR",
        Key::S => "KeyS",
        Key::T => "KeyT",
        Key::U => "KeyU",
        Key::V => "KeyV",
        Key::W => "KeyW",
        Key::X => "KeyX",
        Key::Y => "KeyY",
        Key::Z => "KeyZ",
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::ArrowUp => "ArrowUp",
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::Escape => "Escape",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Comma => "Comma",
        Key::Slash | Key::Questionmark => "Slash",
        Key::Backslash | Key::Pipe => "Backslash",
        Key::OpenBracket | Key::OpenCurlyBracket => "BracketLeft",
        Key::CloseBracket | Key::CloseCurlyBracket => "BracketRight",
        Key::Backtick => "Backquote",
        Key::Minus => "Minus",
        Key::Period => "Period",
        Key::Plus | Key::Equals => "Equal",
        Key::Semicolon | Key::Colon => "Semicolon",
        Key::Quote => "Quote",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_and_match_use_one_portable_physical_chord() {
        let event = egui::Event::Key {
            key: egui::Key::R,
            physical_key: Some(egui::Key::R),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        };
        let chord = chord_from_egui_event(&event).expect("captured chord");
        assert_eq!(chord.native_hotkey_string(), "shift+control+KeyR");
        assert_eq!(event_matches_chord(&event, &chord), Some(true));
    }

    #[test]
    fn repeated_keydown_never_retriggers_a_binding() {
        let event = egui::Event::Key {
            key: egui::Key::F8,
            physical_key: Some(egui::Key::F8),
            pressed: true,
            repeat: true,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(chord_from_egui_event(&event).is_none());
        assert!(event_matches_chord(&event, &KeyChord::default()).is_none());
    }

    #[test]
    fn release_matches_after_modifier_was_released_first() {
        let chord = KeyChord {
            key: "KeyU".to_string(),
            control: true,
            ..Default::default()
        };
        let event = egui::Event::Key {
            key: egui::Key::U,
            physical_key: Some(egui::Key::U),
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert_eq!(event_matches_chord(&event, &chord), Some(false));
    }
}
