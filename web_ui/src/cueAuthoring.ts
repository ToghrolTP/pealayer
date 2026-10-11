export type SequenceStep = { at_us: number; kind: string; target?: number; value?: number; duration_ms?: number; action_ids?: string[]; [key: string]: unknown };

export const sequenceKindLabels: Record<string, string> = {
  motion: 'Seat motion', relay: 'Relay', 'relay-mask': 'Relay mask', 'relays-off': 'All relays off',
  pwm: 'PWM', 'pwm-off': 'All PWM outputs off', display: 'Display', rf: 'RF', beep: 'Buzzer',
  rgb: 'Status RGB', addressable: 'Strip pixel', menu: 'Page', 'menu-action': 'Front-panel action', opcode: 'Raw opcode',
};

export const newCueId = () => {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 15) | 64; bytes[8] = (bytes[8] & 63) | 128;
  const hex = Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
};

export const duplicateStep = (step: SequenceStep, offsetMs = Math.max(1000, step.duration_ms ?? 0)): SequenceStep => ({
  ...structuredClone(step), at_us: step.at_us + offsetMs * 1000,
});

export function moveSequenceStep(step: SequenceStep, atUs: number, lane?: string): SequenceStep {
  const moved = { ...step, at_us: Math.max(0, Math.round(atUs)) };
  if (step.kind === 'motion' && (lane === 'seat.a' || lane === 'seat.b')) {
    moved.target = lane === 'seat.a' ? 0 : 1;
    const action = step.value === 1 ? 'up' : step.value === 2 ? 'down' : 'stop';
    moved.action_ids = [`${lane}.${action}`];
  }
  return moved;
}

export function parseTimeMs(text: string): number | null {
  const match = /^\s*(\d+(?:[.,]\d+)?)\s*(ms|s|sec|min|m|h|us|µs|μs)?\s*$/i.exec(text);
  if (!match) return null;
  const multiplier: Record<string, number> = { ms: 1, s: 1000, sec: 1000, min: 60000, m: 60000, h: 3600000, us: .001, 'µs': .001, 'μs': .001 };
  const value = Number(match[1].replace(',', '.')) * (multiplier[(match[2] ?? 'ms').toLowerCase()] ?? 1);
  return Number.isSafeInteger(Math.round(value * 1000)) ? value : null;
}

export function formatTimeMs(value: number, human = true): string {
  if (value === 0) return '0s';
  if (!human) return `${value}ms`;
  const unit = value >= 3600000 ? ['h', 3600000] as const : value >= 60000 ? ['min', 60000] as const : value >= 1000 ? ['s', 1000] as const : ['ms', 1] as const;
  return `${Number((value / unit[1]).toFixed(6))}${unit[0]}`;
}

/** Keep microsecond cue placement until the final display rounding. */
export function repeatedSequenceDurationMs(steps: SequenceStep[], count = 1, interval = 0): number | null {
  if (!Number.isInteger(count) || count < 1 || count > 1000 || !Number.isInteger(interval) || interval < 0 || interval > 3600000) return null;
  const baseUs = steps.reduce((end, step) => Math.max(end, step.at_us + (step.duration_ms ?? 0) * 1000), 0);
  const periodUs = interval === 0 ? baseUs : interval * 1000;
  if (periodUs < baseUs || (count > 1 && periodUs === 0)) return null;
  const totalUs = baseUs + (count - 1) * periodUs;
  return Number.isSafeInteger(totalUs) && totalUs <= 2147483647 && periodUs <= 2147483647 ? Math.ceil(totalUs / 1000) : null;
}
