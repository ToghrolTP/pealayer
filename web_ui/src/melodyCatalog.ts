export type HardwareMelody = {
  name: string;
  duration_ms: number;
  notes: Array<{ frequency_hz: number; duration_ms: number; gap_ms?: number }>;
};

export const sequenceDurationMs = (steps: Array<Record<string, any>>): number => steps.reduce(
  (maximum, step) => Math.max(
    maximum,
    Math.ceil(Number(step.at_us ?? 0) / 1000) + Math.max(0, Number(step.duration_ms ?? 0)),
  ),
  1,
);

/** Expands a named coordinator melody into portable buzzer/rest macro steps. */
export const appendMelodySteps = <T extends Record<string, any>>(
  steps: T[],
  melody: HardwareMelody,
): T[] => {
  const result = [...steps];
  const baseUs = (steps.length === 0 ? 0 : sequenceDurationMs(steps)) * 1000;
  let offsetMs = 0;
  for (const note of melody.notes) {
    result.push({
      at_us: baseUs + offsetMs * 1000,
      kind: 'beep',
      frequency_hz: note.frequency_hz,
      duration_ms: note.duration_ms,
    } as unknown as T);
    offsetMs += note.duration_ms;
    const gapMs = note.gap_ms ?? 0;
    if (gapMs > 0) {
      result.push({
        at_us: baseUs + offsetMs * 1000,
        kind: 'beep',
        frequency_hz: 0,
        duration_ms: gapMs,
      } as unknown as T);
      offsetMs += gapMs;
    }
  }
  return result;
};
