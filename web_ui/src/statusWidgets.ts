export const WIDGETS = ['fps', 'hot', 'estop', 'connection', 'status_rgb', 'voltage', 'current', 'temperature', 'warnings', 'timeline', 'progress', 'chart', 'message', 'workspace'] as const;
export type StatusWidget = typeof WIDGETS[number];
export type FpsMode = 'media' | 'ui' | 'auto';
export type ChartMetric = 'voltage' | 'current' | 'temperature';
export type StatusBarKey = 'media_rate' | 'hardware' | 'telemetry' | Exclude<StatusWidget, 'fps' | 'connection'>;
export type StatusBarVisibility = Record<StatusBarKey, boolean> & { fps_mode: FpsMode; chart_metric: ChartMetric; order: StatusWidget[] };
export const DEFAULT_VISIBILITY: StatusBarVisibility = {
  media_rate: true, hardware: true, telemetry: true, status_rgb: true, warnings: true, workspace: true,
  voltage: true, current: true, temperature: true, hot: true, estop: true, timeline: true, message: true,
  progress: true, chart: false, fps_mode: 'auto', chart_metric: 'voltage', order: [...WIDGETS],
};
export function orderedWidgets(order?: readonly string[]): StatusWidget[] {
  return [...new Set([...(order ?? []), ...WIDGETS].filter((value): value is StatusWidget => WIDGETS.includes(value as StatusWidget)))];
}
export function widgetKey(widget: StatusWidget): StatusBarKey {
  return widget === 'fps' ? 'media_rate' : widget === 'connection' ? 'hardware' : widget;
}
export function widgetVisible(config: StatusBarVisibility, widget: StatusWidget): boolean {
  return config[widgetKey(widget)] && (!['voltage', 'current', 'temperature'].includes(widget) || config.telemetry);
}
export function setWidgetVisible(config: StatusBarVisibility, widget: StatusWidget, visible: boolean): StatusBarVisibility {
  return { ...config, [widgetKey(widget)]: visible, ...(visible && ['voltage', 'current', 'temperature'].includes(widget) ? { telemetry: true } : {}) };
}
export function reorderWidgets(order: readonly string[], source: StatusWidget, target: StatusWidget, after: boolean): StatusWidget[] {
  const complete = orderedWidgets(order);
  if (source === target) return complete;
  const result = complete.filter((widget) => widget !== source);
  result.splice(result.indexOf(target) + Number(after), 0, source);
  return result;
}
export const nextFpsMode = (mode: FpsMode): FpsMode => mode === 'media' ? 'ui' : mode === 'ui' ? 'auto' : 'media';
export const screenRateSelected = (mode: FpsMode, playing: boolean, loaded: boolean): boolean => mode === 'ui' || (mode === 'auto' && (!playing || !loaded));
export const numericTelemetry = (value: unknown): number | undefined => typeof value === 'number' && Number.isFinite(value) ? value : undefined;
export type ChartSample = { time: number; values: (number | undefined)[] };
export function appendSample(samples: readonly ChartSample[], sample: ChartSample): ChartSample[] {
  if (samples.length && sample.time - samples[samples.length - 1].time < 500) return [...samples];
  return [...samples, sample].slice(-240);
}
export function chartPaths(samples: readonly ChartSample[], metric: number, width: number, height: number): string[] {
  const values = samples.flatMap((sample) => sample.values[metric] === undefined ? [] : [sample.values[metric]!]);
  if (!values.length) return [];
  const low = Math.min(...values), high = Math.max(...values), pad = Math.max((high - low) * .1, .1);
  const start = samples[0].time, span = Math.max(samples[samples.length - 1].time - start, 500);
  const paths: string[] = []; let current = '';
  for (const sample of samples) {
    const value = sample.values[metric];
    if (value === undefined) { if (current) paths.push(current); current = ''; continue; }
    const x = (sample.time - start) / span * width, y = height - (value - low + pad) / (high - low + pad * 2) * height;
    current += `${current ? ' L' : 'M'}${x.toFixed(2)},${y.toFixed(2)}`;
  }
  if (current) paths.push(current);
  return paths;
}
