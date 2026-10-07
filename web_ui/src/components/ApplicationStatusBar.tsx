import React from 'react';
import { Dropdown } from 'antd';
import {
  ApiOutlined,
  BgColorsOutlined,
  ClockCircleOutlined,
  DashboardOutlined,
  EyeInvisibleOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

export type StatusBarKey = 'media_rate' | 'hardware' | 'telemetry' | 'status_rgb' | 'warnings' | 'workspace';
export type StatusBarVisibility = Record<StatusBarKey, boolean>;

const DEFAULT_VISIBILITY: StatusBarVisibility = {
  media_rate: true,
  hardware: true,
  telemetry: true,
  status_rgb: true,
  warnings: true,
  workspace: true,
};

interface ApplicationStatusBarProps {
  state: PlayerState;
  connected: boolean;
  connectionMode: 'ws' | 'http';
  activeSurface: string;
  locale: UiLocale;
  visibility?: Partial<StatusBarVisibility>;
  onVisibilityChange: (visibility: StatusBarVisibility) => void;
}

const compactTime = (seconds: number) => {
  const value = Math.max(0, Number.isFinite(seconds) ? seconds : 0);
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  const wholeSeconds = Math.floor(value % 60);
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, '0')}:${String(wholeSeconds).padStart(2, '0')}`
    : `${minutes}:${String(wholeSeconds).padStart(2, '0')}`;
};

const telemetryText = (state: PlayerState) => {
  const telemetry = state.hardware_details?.telemetry;
  if (!telemetry) return '';
  const parts: string[] = [];
  const supply = Number(telemetry.supply_mv);
  const current = Number(telemetry.current_ma);
  const temperature = Number(telemetry.led_temperature_centi_c ?? telemetry.audio_temperature_centi_c);
  if (Number.isFinite(supply)) parts.push(`${(supply / 1000).toFixed(2)} V`);
  if (Number.isFinite(current)) parts.push(`${Math.round(current)} mA`);
  if (Number.isFinite(temperature)) parts.push(`${(temperature / 100).toFixed(1)} °C`);
  return parts.join(' · ');
};

const StatusItem: React.FC<{
  itemKey: StatusBarKey;
  label: string;
  locale: UiLocale;
  children: React.ReactNode;
  onHide: (key: StatusBarKey) => void;
  className?: string;
}> = ({ itemKey, label, locale, children, onHide, className }) => <Dropdown
  trigger={['contextMenu']}
  menu={{
    items: [{ key: 'hide', icon: <EyeInvisibleOutlined />, label: `${tr(locale, 'Hide')} ${label}` }],
    onClick: () => onHide(itemKey),
  }}
><span className={className} title={label}>{children}</span></Dropdown>;

export const ApplicationStatusBar: React.FC<ApplicationStatusBarProps> = ({
  state,
  connected,
  connectionMode,
  activeSurface,
  locale,
  visibility: configuredVisibility,
  onVisibilityChange,
}) => {
  const visibility = { ...DEFAULT_VISIBILITY, ...(configuredVisibility ?? {}) };
  const setVisible = (key: StatusBarKey, visible: boolean) => onVisibilityChange({ ...visibility, [key]: visible });
  const telemetry = telemetryText(state);
  const warnings = state.hardware_details?.warnings ?? [];
  const statusLed = state.hardware_details?.status_led;
  const rgb = statusLed
    ? `rgb(${statusLed.red}, ${statusLed.green}, ${statusLed.blue})`
    : null;
  const transientMessage = state.update?.state && state.update.state !== 'idle'
    ? state.update.message
    : state.osd && state.osd.remaining_ms > 0
      ? state.osd.message
      : '';
  const boardName = state.hardware?.board_name || state.hardware_details?.board_name || 'PCController';
  const transport = state.hardware_transport || connectionMode.toUpperCase();
  const menuLabels: Record<StatusBarKey, string> = {
    media_rate: tr(locale, 'Media frame rate'),
    hardware: tr(locale, 'Hardware connection'),
    telemetry: tr(locale, 'Hardware telemetry'),
    status_rgb: tr(locale, 'Physical status RGB'),
    warnings: tr(locale, 'Hardware warnings'),
    workspace: tr(locale, 'Workspace mode'),
  };
  const contextItems = (Object.keys(menuLabels) as StatusBarKey[]).map((key) => ({
    key,
    label: menuLabels[key],
  }));

  return <Dropdown
    trigger={['contextMenu']}
    menu={{
      selectable: true,
      multiple: true,
      selectedKeys: (Object.keys(visibility) as StatusBarKey[]).filter((key) => visibility[key]),
      items: contextItems,
      onClick: ({ key }) => setVisible(key as StatusBarKey, !visibility[key as StatusBarKey]),
    }}
  ><footer className="app-statusbar" aria-label={tr(locale, 'Application status')}>
    {visibility.hardware && <StatusItem itemKey="hardware" label={menuLabels.hardware} locale={locale} onHide={(key) => setVisible(key, false)} className={`app-statusbar__connection ${connected ? 'is-online' : ''}`}>
      <i />{connected ? `${boardName} · ${transport}` : tr(locale, 'Offline')}
    </StatusItem>}
    {visibility.status_rgb && rgb && <StatusItem itemKey="status_rgb" label={menuLabels.status_rgb} locale={locale} onHide={(key) => setVisible(key, false)} className="app-statusbar__rgb">
      <i style={{ backgroundColor: rgb }} /><BgColorsOutlined /> RGB {statusLed?.red}, {statusLed?.green}, {statusLed?.blue}
    </StatusItem>}
    {visibility.telemetry && telemetry && <StatusItem itemKey="telemetry" label={menuLabels.telemetry} locale={locale} onHide={(key) => setVisible(key, false)}>
      <DashboardOutlined />{telemetry}
    </StatusItem>}
    {visibility.media_rate && state.current_video && <StatusItem itemKey="media_rate" label={menuLabels.media_rate} locale={locale} onHide={(key) => setVisible(key, false)}>
      <ClockCircleOutlined />{state.media_fps && state.media_fps > 0 ? `${state.media_fps.toFixed(2)} fps · ` : ''}{state.playing ? tr(locale, 'Playing') : tr(locale, 'Paused')} · {compactTime(state.playback_time ?? 0)}
    </StatusItem>}
    {visibility.warnings && warnings.length > 0 && <StatusItem itemKey="warnings" label={menuLabels.warnings} locale={locale} onHide={(key) => setVisible(key, false)} className="app-statusbar__warning">
      <WarningOutlined />{warnings[0].message}
    </StatusItem>}
    {transientMessage && <span className="app-statusbar__message"><ApiOutlined />{transientMessage}</span>}
    {state.estop_active && <span className="app-statusbar__estop"><WarningOutlined />{tr(locale, 'E-STOP ACTIVE')}</span>}
    {visibility.workspace && <StatusItem itemKey="workspace" label={menuLabels.workspace} locale={locale} onHide={(key) => setVisible(key, false)} className="app-statusbar__workspace">
      {tr(locale, 'Workspace')}: {state.active_workspace_profile || state.workspace || activeSurface}
    </StatusItem>}
  </footer></Dropdown>;
};
