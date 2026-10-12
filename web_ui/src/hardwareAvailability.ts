import type { PlayerState } from './components/RemoteControlTab';

export type HardwareAvailability = 'ok' | 'programming' | 'programming_latched' | 'programming_recovery' | 'hardware_unavailable' | 'connecting' | 'disconnected';
export const hardwareLabels: Record<HardwareAvailability, string> = {
  ok: 'Connected board', programming: 'Programming board…', programming_latched: 'Programming latch active',
  programming_recovery: 'Programming recovery pending', hardware_unavailable: 'PCController connected; board not connected',
  connecting: 'Connecting…', disconnected: 'Hardware: Disconnected',
};

export function hardwareAvailability(state: PlayerState, peerConnected = true): HardwareAvailability {
  if (!peerConnected || !state.controller_connected) return peerConnected && state.hardware_connection_requested ? 'connecting' : 'disconnected';
  const programming = state.hardware_details?.programming;
  const handoff = programming?.restoration_pending && programming.intentional_uart_release;
  if (state.hardware_connected || handoff) {
    if (programming?.active) return 'programming';
    if (programming?.latch_known && programming.programming_latch) return 'programming_latched';
    if (handoff) return 'programming_recovery';
  }
  return state.hardware_connected ? 'ok' : 'hardware_unavailable';
}

export function hardwareOutputsAvailable(state: PlayerState) {
  return Boolean(state.controller_connected && state.hardware_connected && state.hardware_details?.outputs_available !== false && hardwareAvailability(state) === 'ok');
}
