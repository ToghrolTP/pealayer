use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammingDeviceIdentity {
    pub port: String,
    #[serde(default)]
    pub vid: String,
    #[serde(default)]
    pub pid: String,
    #[serde(default)]
    pub serial_number: String,
    #[serde(default)]
    pub friendly_name: String,
    #[serde(default)]
    pub instance_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProgrammingState {
    pub active: bool,
    pub phase: String,
    pub restoration_pending: bool,
    pub intentional_uart_release: bool,
    pub latch_known: bool,
    pub programming_latch: bool,
    pub device: ProgrammingDeviceIdentity,
    pub connection_generation: u64,
    pub revision: u64,
    pub updated_at: String,
}

impl HardwareProgrammingState {
    pub fn parse(value: &serde_json::Value) -> Result<Self, String> {
        let state: Self = serde_json::from_value(value.clone())
            .map_err(|error| format!("Invalid PCController programming state: {error}"))?;
        if state.revision > 0 && state.updated_at.trim().is_empty()
            || state.intentional_uart_release && !state.restoration_pending
            || (state.latch_known || state.active) && state.revision == 0
        {
            return Err("Invalid PCController programming state ordering/knowledge".into());
        }
        Ok(state)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareAvailability {
    Ready,
    Programming,
    ProgrammingLatch,
    ProgrammingRecovery,
    BoardUnavailable,
    Connecting,
    Disconnected,
}

impl HardwareAvailability {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ready => "ok",
            Self::Programming => "programming",
            Self::ProgrammingLatch => "programming_latched",
            Self::ProgrammingRecovery => "programming_recovery",
            Self::BoardUnavailable => "hardware_unavailable",
            Self::Connecting => "connecting",
            Self::Disconnected => "disconnected",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Connected board",
            Self::Programming => "Programming board…",
            Self::ProgrammingLatch => "Programming latch active",
            Self::ProgrammingRecovery => "Programming recovery pending",
            Self::BoardUnavailable => "PCController connected; board not connected",
            Self::Connecting => "Connecting…",
            Self::Disconnected => "Hardware: Disconnected",
        }
    }
}

pub fn hardware_availability(
    connected: bool,
    requested: bool,
    hardware: Option<&super::controller::HardwareCapabilities>,
) -> HardwareAvailability {
    if !connected {
        return if requested {
            HardwareAvailability::Connecting
        } else {
            HardwareAvailability::Disconnected
        };
    }
    let Some(hardware) = hardware else {
        return HardwareAvailability::BoardUnavailable;
    };
    let handoff = hardware
        .programming
        .as_ref()
        .is_some_and(|state| state.restoration_pending && state.intentional_uart_release);
    if hardware.board_connected || handoff {
        if hardware
            .programming
            .as_ref()
            .is_some_and(|state| state.active)
        {
            return HardwareAvailability::Programming;
        }
        if hardware.programming_latched() {
            return HardwareAvailability::ProgrammingLatch;
        }
        if handoff {
            return HardwareAvailability::ProgrammingRecovery;
        }
    }
    if hardware.board_connected {
        HardwareAvailability::Ready
    } else {
        HardwareAvailability::BoardUnavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latch_handoff_unknown_and_real_connection_loss_are_distinct() {
        let mut caps = super::super::controller::HardwareCapabilities {
            board_connected: true,
            programming: Some(HardwareProgrammingState {
                latch_known: true,
                programming_latch: true,
                revision: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            hardware_availability(true, false, Some(&caps)),
            HardwareAvailability::ProgrammingLatch
        );
        assert!(!caps.outputs_available());
        caps.programming.as_mut().unwrap().active = true;
        assert_eq!(
            hardware_availability(true, false, Some(&caps)),
            HardwareAvailability::Programming
        );
        caps.board_connected = false;
        caps.programming.as_mut().unwrap().intentional_uart_release = true;
        caps.programming.as_mut().unwrap().restoration_pending = true;
        assert_eq!(
            hardware_availability(true, false, Some(&caps)),
            HardwareAvailability::Programming
        );
        assert_eq!(
            hardware_availability(false, false, Some(&caps)),
            HardwareAvailability::Disconnected
        );
        let state = caps.programming.as_mut().unwrap();
        state.active = false;
        state.latch_known = false;
        assert_eq!(
            hardware_availability(true, false, Some(&caps)),
            HardwareAvailability::ProgrammingRecovery
        );
        caps.programming.as_mut().unwrap().intentional_uart_release = false;
        assert_eq!(
            hardware_availability(true, false, Some(&caps)),
            HardwareAvailability::BoardUnavailable
        );
        caps.board_connected = true;
        assert_eq!(
            hardware_availability(true, false, Some(&caps)),
            HardwareAvailability::Ready,
            "unknown historical latch is not an active latch claim"
        );
        assert!(caps.outputs_available());
    }
    #[test]
    fn malformed_or_unordered_programming_data_is_not_false_clear_state() {
        assert!(HardwareProgrammingState::parse(&serde_json::json!({"active":false})).is_err());
        let mut state = HardwareProgrammingState::default();
        state.latch_known = true;
        assert!(HardwareProgrammingState::parse(&serde_json::to_value(&state).unwrap()).is_err());
        state.revision = 1;
        state.updated_at = "2026-10-12T16:30:00Z".into();
        assert!(HardwareProgrammingState::parse(&serde_json::to_value(&state).unwrap()).is_ok());
        state.intentional_uart_release = true;
        assert!(HardwareProgrammingState::parse(&serde_json::to_value(&state).unwrap()).is_err());
    }
}
