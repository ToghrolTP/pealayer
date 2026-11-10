//! Memory-only, capability-driven Windows tray board actions.
use crate::four_d::controller::{HardwareBoardIdentity, HardwareCapabilities, HardwarePort};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BoardTrayAction {
    Monitor = 2201,
    EngageEmergencyStop,
    ReleaseEmergencyStop,
    StopBuzzer,
    StopEffects,
    StopStrip,
    ClearStrip,
    AutomaticStatusLed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppLanguage;
    use crate::four_d::controller::{HardwareMacro, HardwareMelody, HardwareStripControl};

    fn hardware() -> HardwareCapabilities {
        HardwareCapabilities {
            board_connected: true,
            host_instance_id: "fixture-host".into(),
            board_name: "Renamed board".into(),
            melodies: vec![HardwareMelody::default()],
            macros: vec![HardwareMacro::default()],
            supports_addressable_led: true,
            strip_control: Some(HardwareStripControl {
                running: true,
                ..Default::default()
            }),
            supports_status_led_settings: true,
            ..Default::default()
        }
    }

    #[test]
    fn disconnected_missing_or_unknown_coordinator_has_no_board_submenu() {
        assert!(
            BoardTrayMenu::build(None, true, false, false, false, AppLanguage::English).is_none()
        );
        let mut hardware = hardware();
        assert!(
            BoardTrayMenu::build(
                Some(&hardware),
                false,
                false,
                false,
                false,
                AppLanguage::English
            )
            .is_none()
        );
        hardware.board_connected = false;
        assert!(
            BoardTrayMenu::build(
                Some(&hardware),
                true,
                false,
                false,
                false,
                AppLanguage::English
            )
            .is_none()
        );
        hardware.board_connected = true;
        hardware.host_instance_id.clear();
        assert!(
            BoardTrayMenu::build(
                Some(&hardware),
                true,
                false,
                false,
                false,
                AppLanguage::English
            )
            .is_none()
        );
    }

    #[test]
    fn menu_omits_unadvertised_actions_and_uses_semantic_command_ids() {
        let mut hardware = hardware();
        let menu = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        assert_eq!(menu.title, "Hardware · Renamed board");
        let ids: std::collections::BTreeSet<_> =
            menu.items.iter().map(|item| item.action as u32).collect();
        assert_eq!(ids.len(), menu.items.len());
        for item in &menu.items {
            assert_eq!(
                BoardTrayAction::from_command(item.action as u32),
                Some(item.action)
            );
        }
        assert!(BoardTrayAction::from_command(super::super::windows::TRAY_CMD_PLAYPAUSE).is_none());
        hardware.melodies.clear();
        hardware.macros.clear();
        hardware.strip_control = None;
        hardware.supports_status_led_settings = false;
        let menu = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        assert_eq!(
            menu.items
                .iter()
                .map(|item| item.action)
                .collect::<Vec<_>>(),
            vec![
                BoardTrayAction::Monitor,
                BoardTrayAction::EngageEmergencyStop
            ]
        );
        hardware.capability_bits = 1 << 27; // Buzzer push, even with no catalog.
        hardware.buzzer.melody_id = 9;
        let menu = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        assert!(menu.allows(BoardTrayAction::StopBuzzer, &menu.target));
    }

    #[test]
    fn buzzer_stop_tracks_idle_running_and_completed_output_even_under_estop() {
        let mut hardware = hardware();
        for (id, allowed) in [(0, false), (9, true), (0, false)] {
            hardware.buzzer.melody_id = id;
            let menu = BoardTrayMenu::build(Some(&hardware), true, true, false, false, AppLanguage::English).unwrap();
            assert_eq!(menu.allows(BoardTrayAction::StopBuzzer, &menu.target), allowed);
        }
    }

    #[test]
    fn busy_and_stopped_animation_disable_only_contextual_actions() {
        let hardware = hardware();
        let menu = BoardTrayMenu::build(
            Some(&hardware),
            true,
            true,
            true,
            true,
            AppLanguage::English,
        )
        .unwrap();
        assert!(!menu.allows(BoardTrayAction::EngageEmergencyStop, &menu.target));
        assert!(menu.allows(BoardTrayAction::ReleaseEmergencyStop, &menu.target));
        for action in [
            BoardTrayAction::StopBuzzer,
            BoardTrayAction::StopEffects,
            BoardTrayAction::StopStrip,
            BoardTrayAction::ClearStrip,
            BoardTrayAction::AutomaticStatusLed,
        ] {
            assert!(!menu.allows(action, &menu.target));
        }
        let menu = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            true,
            true,
            AppLanguage::English,
        )
        .unwrap();
        assert!(menu.allows(BoardTrayAction::EngageEmergencyStop, &menu.target));
        let mut hardware = hardware;
        hardware.strip_control.as_mut().unwrap().running = false;
        let menu = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        assert!(!menu.allows(BoardTrayAction::StopStrip, &menu.target));
        assert!(menu.allows(BoardTrayAction::ClearStrip, &menu.target));
    }

    #[test]
    fn selection_target_does_not_follow_a_coordinator_or_port_switch() {
        let mut hardware = hardware();
        let before = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        hardware.host_instance_id = "another-host".into();
        let after = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        assert!(!after.allows(BoardTrayAction::ClearStrip, &before.target));
        hardware.host_instance_id = "fixture-host".into();
        hardware.port.name = "another-port".into();
        let after = BoardTrayMenu::build(
            Some(&hardware),
            true,
            false,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        assert!(!after.allows(BoardTrayAction::ReleaseEmergencyStop, &before.target));
    }

    #[test]
    fn effects_and_led_release_follow_live_state_and_reject_stale_enabled_selections() {
        let mut hardware = hardware();
        hardware.strip_control.as_mut().unwrap().running = false;
        let build = |hardware: &HardwareCapabilities| BoardTrayMenu::build(
            Some(hardware), true, true, false, false, AppLanguage::English).unwrap();
        for running in [None, Some(false), Some(true), Some(false)] {
            hardware.macro_running = running;
            let menu = build(&hardware);
            assert_eq!(menu.allows(BoardTrayAction::StopEffects, &menu.target), running == Some(true));
        }
        hardware.macro_running = Some(true);
        hardware.status_led_override = Some(true);
        let active = build(&hardware);
        assert!(active.allows(BoardTrayAction::AutomaticStatusLed, &active.target));
        hardware.macro_running = Some(false);
        hardware.status_led_override = Some(false);
        let idle = build(&hardware);
        assert!(!idle.allows(BoardTrayAction::StopEffects, &active.target));
        assert!(!idle.allows(BoardTrayAction::AutomaticStatusLed, &active.target));
        hardware.status_led_override = None;
        assert!(!build(&hardware).allows(BoardTrayAction::AutomaticStatusLed, &active.target));
        // Animation can retain pixels after ending: clear is still useful.
        assert!(idle.allows(BoardTrayAction::ClearStrip, &idle.target));
    }

    #[test]
    fn remote_operations_block_ordinary_actions_but_not_emergency_stop() {
        let mut hardware = hardware();
        hardware.buzzer.melody_id = 1;
        hardware.macro_running = Some(true);
        hardware.status_led_override = Some(true);
        hardware.operations.push(crate::four_d::controller::HardwareOperation {
            id: "remote-operation".into(), kind: "firmware".into(),
            state: "programming".into(), ..Default::default()
        });
        let menu = BoardTrayMenu::build(Some(&hardware), true, false, false, false, AppLanguage::English).unwrap();
        assert!(menu.allows(BoardTrayAction::EngageEmergencyStop, &menu.target));
        assert!(menu.items.iter().filter(|item| !matches!(item.action,
            BoardTrayAction::Monitor | BoardTrayAction::EngageEmergencyStop)).all(|item| !item.enabled));
        hardware.operations[0].state = "completed".into();
        let menu = BoardTrayMenu::build(Some(&hardware), true, false, false, false, AppLanguage::English).unwrap();
        assert!(menu.allows(BoardTrayAction::StopEffects, &menu.target));
        assert!(menu.allows(BoardTrayAction::AutomaticStatusLed, &menu.target));
    }

    #[test]
    fn default_board_title_is_hardware() {
        let mut hardware = hardware();
        for name in ["", "PCController"] {
            hardware.board_name = name.into();
            assert_eq!(BoardTrayMenu::build(Some(&hardware), true, false, false, false,
                AppLanguage::English).unwrap().title, "Hardware");
        }
    }

    #[test]
    fn board_actions_are_translated_without_translating_board_names() {
        let hardware = hardware();
        let english = BoardTrayMenu::build(
            Some(&hardware),
            true,
            true,
            false,
            false,
            AppLanguage::English,
        )
        .unwrap();
        let persian = BoardTrayMenu::build(
            Some(&hardware),
            true,
            true,
            false,
            false,
            AppLanguage::Persian,
        )
        .unwrap();
        assert_ne!(english.title, persian.title);
        assert!(english.title.ends_with(" · Renamed board"));
        assert!(persian.title.ends_with(" · Renamed board"));
        for (english, persian) in english.items.iter().zip(&persian.items) {
            assert_ne!(
                english.label, persian.label,
                "missing translation for {}",
                english.label
            );
        }
    }
}

impl BoardTrayAction {
    pub fn from_command(id: u32) -> Option<Self> {
        Some(match id {
            2201 => Self::Monitor,
            2202 => Self::EngageEmergencyStop,
            2203 => Self::ReleaseEmergencyStop,
            2204 => Self::StopBuzzer,
            2205 => Self::StopEffects,
            2206 => Self::StopStrip,
            2207 => Self::ClearStrip,
            2208 => Self::AutomaticStatusLed,
            _ => return None,
        })
    }
}

/// A menu selection must never follow a board/coordinator switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayBoardTarget {
    host: String,
    profile_identity: String,
    identity: HardwareBoardIdentity,
    port: HardwarePort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTrayItem {
    pub action: BoardTrayAction,
    pub label: String,
    pub enabled: bool,
    pub separator_before: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTrayMenu {
    pub target: TrayBoardTarget,
    pub title: String,
    pub items: Vec<BoardTrayItem>,
}

impl BoardTrayMenu {
    pub fn build(
        hardware: Option<&HardwareCapabilities>,
        coordinator_connected: bool,
        estop: bool,
        board_busy: bool,
        effect_busy: bool,
        language: crate::config::AppLanguage,
    ) -> Option<Self> {
        let hardware = hardware.filter(|hardware| {
            coordinator_connected
                && hardware.board_connected
                && !hardware.host_instance_id.is_empty()
        })?;
        let tr = |text| crate::ui::i18n::tr(language, text);
        // Includes operations started by another client, not just this app.
        let busy = board_busy || effect_busy || hardware.programming_blocked() || hardware.operations.iter().any(|op| op.active());
        let mut items = vec![
            BoardTrayItem {
                action: BoardTrayAction::Monitor,
                label: tr("Hardware Monitor"),
                enabled: true,
                separator_before: false,
            },
            BoardTrayItem {
                action: BoardTrayAction::EngageEmergencyStop,
                label: tr(if estop {
                    "E-STOP ACTIVE"
                } else {
                    "Emergency stop"
                }),
                enabled: !estop,
                separator_before: true,
            },
        ];
        if estop {
            items.push(BoardTrayItem {
                action: BoardTrayAction::ReleaseEmergencyStop,
                label: tr("Release E-STOP"),
                enabled: !hardware.programming_blocked(),
                separator_before: false,
            });
        }
        let mut stops = Vec::new();
        // Buzzer capability bits or the configured catalog advertise controls; do not
        // guess availability from board model names or relay numbers.
        if hardware.supports_buzzer_controls() {
            stops.push((BoardTrayAction::StopBuzzer, "Stop buzzer", !busy && hardware.buzzer.is_playing()));
        }
        let strip_running = hardware.strip_control.as_ref().is_some_and(|strip| strip.running);
        if !hardware.macros.is_empty() || (hardware.supports_addressable_led && hardware.strip_control.is_some()) {
            stops.push((
                BoardTrayAction::StopEffects,
                "Stop board effects",
                !busy && (hardware.macro_running == Some(true) || strip_running),
            ));
        }
        if hardware.supports_addressable_led
            && let Some(strip) = &hardware.strip_control
        {
            stops.push((
                BoardTrayAction::StopStrip,
                "Stop strip animation",
                strip.running && !busy,
            ));
            stops.push((BoardTrayAction::ClearStrip, "Clear LED strip", !busy));
        }
        if hardware.supports_status_led_settings {
            stops.push((
                BoardTrayAction::AutomaticStatusLed,
                "Restore automatic status LED",
                !busy && hardware.status_led_override == Some(true),
            ));
        }
        for (index, (action, label, enabled)) in stops.into_iter().enumerate() {
            items.push(BoardTrayItem {
                action,
                label: tr(label),
                enabled,
                separator_before: index == 0,
            });
        }
        let name: String = hardware
            .board_name
            .chars()
            .filter(|c| !c.is_control())
            .take(64)
            .collect();
        let title = if name.is_empty() || name == "PCController" {
            tr("Hardware")
        } else {
            format!("{} · {name}", tr("Hardware"))
        };
        Some(Self {
            target: TrayBoardTarget {
                host: hardware.host_instance_id.clone(),
                profile_identity: hardware
                    .board_profile
                    .as_ref()
                    .map(|profile| profile.board_identity.clone())
                    .unwrap_or_default(),
                identity: hardware.board_identity.clone(),
                port: hardware.port.clone(),
            },
            title,
            items,
        })
    }

    pub fn allows(&self, action: BoardTrayAction, target: &TrayBoardTarget) -> bool {
        &self.target == target
            && self
                .items
                .iter()
                .any(|item| item.action == action && item.enabled)
    }
}
