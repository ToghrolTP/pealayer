import React, { useEffect, useState } from 'react';
import { Dropdown } from 'antd';
import type { MenuProps } from 'antd';
import { ApiOutlined, BgColorsOutlined, ClockCircleOutlined, DashboardOutlined, EyeInvisibleOutlined, LineChartOutlined, VideoCameraOutlined, WarningOutlined, ThunderboltOutlined, SyncOutlined } from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { DEFAULT_VISIBILITY, WIDGETS, orderedWidgets, widgetVisible, setWidgetVisible, reorderWidgets, nextFpsMode, screenRateSelected, numericTelemetry, appendSample, chartPaths } from '../statusWidgets';
import type { StatusWidget, StatusBarVisibility, FpsMode, ChartMetric, ChartSample } from '../statusWidgets';
export type { StatusBarVisibility } from '../statusWidgets';

interface ApplicationStatusBarProps {
  state: PlayerState; connected: boolean; connectionMode: 'ws' | 'http'; activeSurface: string; locale: UiLocale;
  visibility?: Partial<StatusBarVisibility>; onVisibilityChange: (visibility: StatusBarVisibility) => void;
}
const LABELS: Record<StatusWidget, string> = {
  fps: 'Frame rate', hot: 'HOT alerts', estop: 'E-STOP', connection: 'Hardware connection', status_rgb: 'Physical status RGB',
  voltage: 'Voltage', current: 'Current draw', temperature: 'Temperature', warnings: 'Hardware warnings', timeline: 'Hardware timing',
  progress: 'Operation progress', chart: 'Telemetry log chart', message: 'Status message', workspace: 'Workspace mode',
};
const ICONS: Record<StatusWidget, React.ReactNode> = {
  fps: <VideoCameraOutlined />, hot: <WarningOutlined />, estop: <WarningOutlined />, connection: <ApiOutlined />, status_rgb: <BgColorsOutlined />,
  voltage: <ThunderboltOutlined />, current: <ThunderboltOutlined />, temperature: <DashboardOutlined />, warnings: <WarningOutlined />,
  timeline: <ClockCircleOutlined />, progress: <SyncOutlined />, chart: <LineChartOutlined />, message: <ApiOutlined />, workspace: <DashboardOutlined />,
};
const METRICS: ChartMetric[] = ['voltage', 'current', 'temperature'];
const MODE_LABELS: Record<FpsMode, string> = { media: 'Media', ui: 'Screen', auto: 'Auto' };

export const ApplicationStatusBar: React.FC<ApplicationStatusBarProps> = ({ state, connected, connectionMode, activeSurface, locale, visibility: configuredVisibility, onVisibilityChange }) => {
  const config = { ...DEFAULT_VISIBILITY, ...configuredVisibility, order: orderedWidgets(configuredVisibility?.order) };
  const [samples, setSamples] = useState<ChartSample[]>([]);
  const [dragged, setDragged] = useState<StatusWidget | null>(null);
  const [dropTarget, setDropTarget] = useState<{ widget: StatusWidget; after: boolean } | null>(null);
  const hardwareLive = connected && state.hardware_connected === true;
  const telemetry = hardwareLive ? state.hardware_details?.telemetry : undefined;
  const voltage = numericTelemetry(telemetry?.bus_mv), current = numericTelemetry(telemetry?.current_ma), temperature = numericTelemetry(telemetry?.led_temperature_centi_c);
  useEffect(() => {
    setSamples((previous) => appendSample(previous, { time: Date.now(), values: [voltage === undefined ? undefined : voltage / 1000, current, temperature === undefined ? undefined : temperature / 100] }));
  }, [state, voltage, current, temperature]);
  const label = (widget: StatusWidget) => tr(locale, LABELS[widget]);
  const show = (widget: StatusWidget, visible: boolean) => onVisibilityChange(setWidgetVisible(config, widget, visible));
  const changeMode = (fps_mode: FpsMode) => onVisibilityChange({ ...config, fps_mode });
  const move = (source: StatusWidget, target: StatusWidget, after: boolean) => onVisibilityChange({ ...config, order: reorderWidgets(config.order, source, target, after) });
  const operations = connected ? state.status_operations ?? [] : [];
  const warnings = hardwareLive ? state.hardware_details?.warnings ?? [] : [];
  const screenRate = screenRateSelected(config.fps_mode, state.playing === true, Boolean(state.current_video));
  const rate = screenRate ? state.ui_fps : state.media_fps;
  const metricIndex = METRICS.indexOf(config.chart_metric);
  const metricUnit = ['V', 'mA', '°C'][metricIndex];
  const metricValues = samples.flatMap((sample) => sample.values[metricIndex] === undefined ? [] : [sample.values[metricIndex]!]);
  const chartTitle = `${label('chart')} · ${label(config.chart_metric)} (${metricUnit}) · ${samples.length}/240 ${tr(locale, 'samples')} · RAM${metricValues.length ? ` · ${Math.min(...metricValues).toFixed(2)}–${Math.max(...metricValues).toFixed(2)} ${metricUnit}` : ''}`;
  const menu = (widget: StatusWidget): MenuProps => {
    const index = config.order.indexOf(widget);
    const items: MenuProps['items'] = [{ key: 'title', label: label(widget), icon: ICONS[widget], disabled: true }, { type: 'divider' }];
    if (widget === 'fps') for (const mode of ['media', 'ui', 'auto'] as FpsMode[]) items.push({ key: `mode:${mode}`, label: tr(locale, MODE_LABELS[mode]), icon: config.fps_mode === mode ? '✓' : undefined });
    if (widget === 'chart') {
      for (const metric of METRICS) items.push({ key: `metric:${metric}`, label: label(metric), icon: config.chart_metric === metric ? '✓' : undefined });
      items.push({ key: 'clear', label: tr(locale, 'Clear chart history') });
    }
    items.push({ key: 'earlier', label: tr(locale, 'Move earlier'), disabled: index === 0 }, { key: 'later', label: tr(locale, 'Move later'), disabled: index === config.order.length - 1 },
      { type: 'divider' }, { key: 'hide', icon: <EyeInvisibleOutlined />, label: <>{tr(locale, 'Hide')} {ICONS[widget]} {label(widget)}</> });
    return { items, onClick: ({ key }) => {
      if (key === 'hide') show(widget, false);
      else if (key.startsWith('mode:')) changeMode(key.slice(5) as FpsMode);
      else if (key.startsWith('metric:')) onVisibilityChange({ ...config, chart_metric: key.slice(7) as ChartMetric });
      else if (key === 'clear') setSamples([]);
      else if (key === 'earlier' && index > 0) move(widget, config.order[index - 1], false);
      else if (key === 'later' && index < config.order.length - 1) move(widget, config.order[index + 1], true);
    } };
  };
  const content = (widget: StatusWidget): React.ReactNode => {
    switch (widget) {
      case 'fps': return <button type="button" className="app-statusbar__fps" onClick={() => changeMode(nextFpsMode(config.fps_mode))}
        title={`${tr(locale, MODE_LABELS[config.fps_mode])} · ${tr(locale, screenRate ? 'Screen render rate' : 'Media frame rate')} · ${tr(locale, 'Click to switch frame rate source')}`}>
        {screenRate ? <DashboardOutlined /> : <VideoCameraOutlined />}{tr(locale, MODE_LABELS[config.fps_mode])} · {rate != null && Number.isFinite(rate) && rate > 0 && (screenRate || state.current_video) ? `${rate.toFixed(2)} fps` : '— fps'}</button>;
      case 'connection': return <><i className={`status-connection-dot ${hardwareLive ? 'is-online' : ''}`} />{hardwareLive ? `${state.hardware?.board_name || state.hardware_details?.board_name || 'PCController'} · ${state.hardware_transport || connectionMode.toUpperCase()}` : connected && state.controller_connected ? tr(locale, 'PCController connected; board not connected') : tr(locale, 'Hardware: Disconnected')}</>;
      case 'hot': return hardwareLive && telemetry?.hot === true ? <><WarningOutlined /> HOT · {tr(locale, 'Temperature alert')}</> : null;
      case 'estop': return state.estop_active ? <><WarningOutlined />{tr(locale, 'E-STOP ACTIVE')}</> : null;
      case 'voltage': return voltage === undefined ? null : `${(voltage / 1000).toFixed(2)} V`;
      case 'current': return current === undefined ? null : `${current.toFixed(0)} mA`;
      case 'temperature': return temperature === undefined ? null : `${(temperature / 100).toFixed(1)} °C`;
      case 'status_rgb': { const led = hardwareLive ? state.hardware_details?.status_led : null; return led ? <i className="status-rgb-dot" style={{ backgroundColor: `rgb(${led.red},${led.green},${led.blue})` }} title={`RGB ${led.red}, ${led.green}, ${led.blue}`} /> : null; }
      case 'warnings': return warnings.length ? <><WarningOutlined />{warnings[0].message}</> : null;
      case 'timeline': return connected && state.controller_connected && state.hardware_sync?.timeline?.state ? <><ClockCircleOutlined />{state.hardware_sync.timeline.state}</> : null;
      case 'progress': { const op = operations[0]; if (!op) return null;
        const fraction = typeof op.fraction === 'number' && Number.isFinite(op.fraction) ? Math.max(0, Math.min(1, op.fraction)) : undefined;
        return <span className="status-progress" role="progressbar" aria-label={`${op.label} · ${op.phase}`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={fraction === undefined ? undefined : Math.round(fraction * 100)}
          title={operations.map((operation) => `${operation.label} · ${operation.phase}\n${operation.detail}`).join('\n')}>
          {op.label} · {op.phase}{fraction === undefined ? ' …' : ` · ${(fraction * 100).toFixed(0)}%`}<i style={{ width: fraction === undefined ? '100%' : `${fraction * 100}%` }} className={fraction === undefined ? 'is-indeterminate' : ''} /></span>;
      }
      case 'chart': return <svg className="status-chart" width="100" height="20" viewBox="0 0 100 20" role="img" aria-label={chartTitle}><title>{chartTitle}</title>
        {metricValues.length ? chartPaths(samples, metricIndex, 100, 20).map((path, index) => <path key={index} d={path} fill="none" stroke="currentColor" strokeWidth="1.5" />) : <text x="50" y="14" textAnchor="middle">—</text>}</svg>;
      case 'message': return state.osd && state.osd.remaining_ms > 0 ? <><ApiOutlined />{state.osd.message}</> : null;
      case 'workspace': return <>{tr(locale, 'Workspace')}: {state.active_workspace_profile || state.workspace || activeSurface}</>;
    }
  };
  const contextItems: MenuProps['items'] = WIDGETS.map((widget) => ({ key: widget, label: label(widget), icon: widgetVisible(config, widget) ? '✓' : ICONS[widget] }));
  contextItems.push({ type: 'divider' }, { key: 'reset', label: tr(locale, 'Reset widget order') });
  return <Dropdown trigger={['contextMenu']} menu={{ items: contextItems, onClick: ({ key }) => key === 'reset' ? onVisibilityChange({ ...config, order: [...WIDGETS] }) : show(key as StatusWidget, !widgetVisible(config, key as StatusWidget)) }}>
    <footer className="app-statusbar" aria-label={tr(locale, 'Application status')}>
      {config.order.filter((widget) => widgetVisible(config, widget)).map((widget) => {
        const child = content(widget); if (child == null) return null;
        return <Dropdown key={widget} trigger={['contextMenu']} menu={menu(widget)}><span
          className={`status-widget status-widget--${widget}${dropTarget?.widget === widget ? ` is-drop-${dropTarget.after ? 'after' : 'before'}` : ''}${dragged === widget ? ' is-dragging' : ''}`}
          data-status-widget={widget} title={label(widget)} draggable onContextMenu={(event) => event.stopPropagation()}
          onDragStart={(event) => { const rect = event.currentTarget.getBoundingClientRect(); event.dataTransfer.setData('application/x-pealayer-status-widget', widget); event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setDragImage(event.currentTarget, event.clientX - rect.left, event.clientY - rect.top); setDragged(widget); }}
          onDragOver={(event) => { if (!dragged || dragged === widget) return; event.preventDefault(); event.dataTransfer.dropEffect = 'move'; const rect = event.currentTarget.getBoundingClientRect(); setDropTarget({ widget, after: (event.clientX > rect.left + rect.width / 2) !== (getComputedStyle(event.currentTarget).direction === 'rtl') }); }}
          onDrop={(event) => { event.preventDefault(); const source = event.dataTransfer.getData('application/x-pealayer-status-widget'); if (WIDGETS.includes(source as StatusWidget) && dropTarget?.widget === widget) move(source as StatusWidget, widget, dropTarget.after); setDragged(null); setDropTarget(null); }}
          onDragLeave={() => setDropTarget(null)} onDragEnd={() => { setDragged(null); setDropTarget(null); }}>{child}</span></Dropdown>;
      })}
    </footer>
  </Dropdown>;
};
