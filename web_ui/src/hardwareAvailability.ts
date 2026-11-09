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

/** Cached capabilities do not authorize live Monitor cards or RF controls. */
export function hardwareMonitorReady(state: PlayerState) {
  return Boolean(state.controller_connected && state.hardware_connected && state.hardware_details);
}
export function hardwareMonitorRfAvailable(state: PlayerState) {
  return hardwareMonitorReady(state) && hardwareOutputsAvailable(state) && state.hardware?.supports_rf_transmit === true;
}
export function hardwareMonitorEmpty(state: PlayerState) {
  const availability = hardwareAvailability(state);
  if (!state.controller_connected) {
    const progress = Boolean(state.hardware_connection_requested);
    return { title: progress ? 'Connecting to PCController…' : 'Hardware Disconnected',
    detail: state.hardware_error?.trim() ? "Couldn't connect to PCController.\nCheck that it is running, then try again." : (progress ? 'Waiting for the configured PCController connection.' : 'Connect to PCController to discover live board controls.'),
      diagnostics: state.hardware_error?.trim() || null,
      progress, primary: !progress && Boolean(state.hardware_endpoint?.trim()) ? 'connect' as const : null };
  }
  if (!state.hardware_details) return { title: state.hardware_error ? 'Hardware catalog unavailable' : 'Loading hardware…',
    detail: state.hardware_error?.trim() ? "Couldn't load the hardware controls. Try refreshing the connection." : 'Waiting for PCController to advertise the board capabilities.',
    diagnostics: state.hardware_error?.trim() || null,
    progress: !state.hardware_error, primary: 'refresh' as const };
  const programming = ['programming', 'programming_latched', 'programming_recovery'].includes(availability);
  return { title: programming ? hardwareLabels[availability] : 'No board connected',
    detail: programming ? 'Board controls are unavailable while programming or recovery is in progress.' : 'PCController is reachable, but no board is connected. Connect the board and check its USB cable, power, and port.',
    diagnostics: state.hardware_error?.trim() || null,
    progress: programming, primary: 'refresh' as const };
}
