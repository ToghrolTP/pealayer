export type SequenceStep = { at_us: number; kind: string; target?: number; value?: number; duration_ms?: number; repeat_count?: number; repeat_interval_ms?: number; action_ids?: string[]; [key: string]: unknown };

export function stepRepetition(step: SequenceStep): { count: number; intervalUs: number; endUs: number } | null {
  const count = step.repeat_count ?? 1, interval = step.repeat_interval_ms ?? 0, duration = step.duration_ms ?? 0;
  if (!Number.isInteger(count) || count < 1 || count > 1000 || !Number.isInteger(interval) || interval < 0 || interval > 3600000 || !Number.isInteger(duration) || duration < 0 || duration > 65535 || !Number.isInteger(step.at_us) || step.at_us < 0) return null;
  const intervalUs = (interval === 0 ? Math.max(duration, 1) : interval) * 1000;
  const endUs = step.at_us + duration * 1000 + (count - 1) * intervalUs;
  return Number.isSafeInteger(endUs) && endUs <= 2147483647 && intervalUs <= 2147483647 ? { count, intervalUs, endUs } : null;
}

/** Repeated instances always bind to the one persisted source index. */
export function stepInstances(step: SequenceStep, source: number, fromUs = 0, toUs = 2147483647, pinnedIteration?: number) {
  const repeat = stepRepetition(step);
  if (!repeat) return [{ source, iteration: 0, at_us: step.at_us }];
  const first = Math.max(1, Math.ceil((fromUs - step.at_us - (step.duration_ms ?? 0) * 1000) / repeat.intervalUs));
  const last = Math.min(repeat.count - 1, Math.floor((toUs - step.at_us) / repeat.intervalUs));
  const ghosts = [];
  for (let iteration = last; iteration >= first; iteration--) ghosts.push({ source, iteration, at_us: step.at_us + iteration * repeat.intervalUs });
  if (pinnedIteration != null && pinnedIteration > 0 && pinnedIteration < repeat.count && !ghosts.some(instance => instance.iteration === pinnedIteration)) {
    ghosts.push({ source, iteration: pinnedIteration, at_us: step.at_us + pinnedIteration * repeat.intervalUs });
  }
  return [...ghosts, { source, iteration: 0, at_us: step.at_us }];
}

export function editStepInstance<T extends SequenceStep>(step: T, iteration: number, deltaMs: number, mode: 'move' | 'length'): T {
  const next = structuredClone(step);
  if (mode === 'length') next.duration_ms = Math.max(1, Math.min(65535, Math.round((step.duration_ms ?? 0) + deltaMs)));
  else if (iteration > 0) {
    const period = stepRepetition(step)?.intervalUs ?? 1000;
    const repeats = Math.max(1, (step.repeat_count ?? 1) - 1);
    const roomMs = Math.floor((2147483647 - step.at_us - (step.duration_ms ?? 0) * 1000) / repeats / 1000);
    next.repeat_interval_ms = Math.max(1, Math.min(roomMs, 3600000, Math.round(period / 1000 + deltaMs / iteration)));
  } else next.at_us = Math.max(0, Math.round(step.at_us + deltaMs * 1000));
  return next;
}

export const sequenceKindLabels: Record<string, string> = {
  motion: 'Seat motion', relay: 'Relay', 'relay-mask': 'Relay mask', 'relays-off': 'All relays off',
  pwm: 'PWM', 'pwm-off': 'All PWM outputs off', display: 'Display', rf: 'RF', beep: 'Buzzer',
  rgb: 'Status RGB', addressable: 'Strip pixel', menu: 'Page', 'menu-action': 'Front-panel action', opcode: 'Raw opcode',
};

/** Catalog titles are user-authored; never translate or invent seat names. */
export function sequenceLaneName(lane: string, controls: ReadonlyArray<{ key: string; name: string; default_name?: string }>): string {
  const control = controls.find(control => control.key === lane);
  return control?.name.trim() || control?.default_name?.trim() || lane;
}

export const newCueId = () => {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 15) | 64; bytes[8] = (bytes[8] & 63) | 128;
  const hex = Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
};

export const duplicateStep = (step: SequenceStep, offsetMs = Math.max(1000, step.duration_ms ?? 0)): SequenceStep => ({
  ...structuredClone(step), at_us: step.at_us + offsetMs * 1000,
});

export function selectCueIndices(selected: Iterable<number>, index: number, anchor: number, toggle = false, range = false): number[] {
  const next = new Set(toggle ? selected : []);
  if (range) for (let cue = Math.min(anchor, index); cue <= Math.max(anchor, index); cue++) next.add(cue);
  else if (!toggle || !next.delete(index)) next.add(index);
  return [...next].sort((a, b) => a - b);
}

export function duplicateSequenceCues<T extends SequenceStep>(steps: T[], indices: Iterable<number>): { steps: T[]; indices: number[] } {
  const selected = [...new Set(indices)].filter(index => Number.isInteger(index) && index >= 0 && index < steps.length).sort((a, b) => a - b);
  if (!selected.length) return { steps, indices: [] };
  const start = selected.reduce((min, index) => Math.min(min, steps[index].at_us), Infinity);
  const end = selected.reduce((max, index) => Math.max(max, stepRepetition(steps[index])?.endUs ?? steps[index].at_us), 0);
  const offsetUs = Math.max(1_000_000, end - start);
  const copies = selected.map(index => { const copy = structuredClone(steps[index]); copy.at_us += offsetUs; return copy; });
  return { steps: [...steps, ...copies], indices: copies.map((_, index) => steps.length + index) };
}

export function deleteSequenceCues<T extends SequenceStep>(steps: T[], indices: Iterable<number>): { steps: T[]; primary: number } {
  const selected = new Set([...indices].filter(index => Number.isInteger(index) && index >= 0 && index < steps.length));
  const remaining = steps.filter((_, index) => !selected.has(index));
  const first = selected.size ? [...selected].reduce((min, index) => Math.min(min, index), Infinity) : 0;
  return { steps: remaining, primary: Math.max(0, Math.min(first, remaining.length - 1)) };
}

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
  const plans = steps.map(stepRepetition);
  if (plans.some(plan => plan === null)) return null;
  const baseUs = plans.reduce((end, plan) => Math.max(end, plan!.endUs), 0);
  const periodUs = interval === 0 ? baseUs : interval * 1000;
  if (periodUs < baseUs || (count > 1 && periodUs === 0)) return null;
  const totalUs = baseUs + (count - 1) * periodUs;
  return Number.isSafeInteger(totalUs) && totalUs <= 2147483647 && periodUs <= 2147483647 ? Math.ceil(totalUs / 1000) : null;
}
