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
        assert_eq!(menu.title, "PCController · Renamed board");
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
        assert_eq!(english.title, persian.title);
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
                enabled: true,
                separator_before: false,
            });
        }
        let mut stops = Vec::new();
        // Buzzer capability bits or the configured catalog advertise controls; do not
        // guess availability from board model names or relay numbers.
        if hardware.supports_buzzer_controls() {
            stops.push((BoardTrayAction::StopBuzzer, "Stop buzzer", !board_busy));
        }
        if !hardware.macros.is_empty() {
            stops.push((
                BoardTrayAction::StopEffects,
                "Stop board effects",
                !effect_busy,
            ));
        }
        if hardware.supports_addressable_led
            && let Some(strip) = &hardware.strip_control
        {
            stops.push((
                BoardTrayAction::StopStrip,
                "Stop strip animation",
                strip.running && !effect_busy,
            ));
            stops.push((BoardTrayAction::ClearStrip, "Clear LED strip", !effect_busy));
        }
        if hardware.supports_status_led_settings {
            stops.push((
                BoardTrayAction::AutomaticStatusLed,
                "Restore automatic status LED",
                !board_busy,
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
            "PCController".into()
        } else {
            format!("PCController · {name}")
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
