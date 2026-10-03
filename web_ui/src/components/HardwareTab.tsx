import React, { useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  ColorPicker,
  Collapse,
  Empty,
  InputNumber,
  Slider,
  Space,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import {
  AlertOutlined,
  BulbOutlined,
  DashboardOutlined,
  DesktopOutlined,
  DisconnectOutlined,
  ExperimentOutlined,
  PoweroffOutlined,
  ThunderboltOutlined,
  UsbOutlined,
} from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

interface HardwareTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
}

const controlIcon = (kind: string) => {
  if (/pwm|mosfet/i.test(kind)) return <DashboardOutlined />;
  if (/seat|motion/i.test(kind)) return <ExperimentOutlined />;
  if (/light|strip|led/i.test(kind)) return <BulbOutlined />;
  return <ThunderboltOutlined />;
};

const semanticSection = (kind: string, locale: UiLocale) => {
  if (/seat|motion/i.test(kind)) return { key: 'motion', label: tr(locale, 'Motion controls'), order: 0 };
  if (/relay/i.test(kind)) return { key: 'relay', label: tr(locale, 'Relay outputs'), order: 1 };
  if (/pwm|mosfet/i.test(kind)) return { key: 'pwm', label: tr(locale, 'PWM outputs'), order: 2 };
  return { key: `other:${kind}`, label: tr(locale, 'Other controls'), order: 3 };
};

const controlCaption = (
  control: NonNullable<PlayerState['hardware_details']>['controls'][number],
  locale: UiLocale,
) => {
  if (/seat|motion/i.test(control.kind)) return tr(locale, 'Motion control');
  if (/relay/i.test(control.kind)) return control.channel == null ? tr(locale, 'Relay output') : `R${control.channel}`;
  if (/pwm|mosfet/i.test(control.kind)) return control.channel == null ? tr(locale, 'PWM output') : `CH${control.channel + 1}`;
  return control.kind.replace(/-/g, ' ');
};

const telemetryValue = (value: number | boolean | null | undefined, unit = '') => {
  if (value === null || value === undefined) return '—';
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  return `${value}${unit}`;
};

export const HardwareTab: React.FC<HardwareTabProps> = ({ state, sendCmd, locale }) => {
  const details = state.hardware_details;
  const [stripPixels, setStripPixels] = useState<number | null>(null);
  const [stripColor, setStripColor] = useState('#38D27A');
  const [stripBrightness, setStripBrightness] = useState(255);
  const [pwmDrafts, setPwmDrafts] = useState<Record<string, number>>({});
  const grouped = useMemo(() => {
    const result = new Map<string, { label: string; order: number; controls: NonNullable<typeof details>['controls'] }>();
    for (const control of details?.controls ?? []) {
      if (control.hidden) continue;
      const section = semanticSection(control.kind, locale);
      const current = result.get(section.key) ?? { label: section.label, order: section.order, controls: [] };
      current.controls.push(control);
      result.set(section.key, current);
    }
    return [...result.entries()].sort(([, left], [, right]) => left.order - right.order).map(([key, section]) => ({
      key,
      label: <span className="hardware-section-title">{section.label}<Tag>{section.controls.length}</Tag></span>,
      children: <div className="hardware-control-grid">
        {section.controls.sort((left, right) => left.order - right.order).map((control) => {
          const isPwm = /pwm|mosfet/i.test(control.kind);
          const value = pwmDrafts[control.key] ?? control.percent ?? 0;
          return <article className={`hardware-control ${control.locked ? 'is-locked' : ''}`} key={control.key}>
            <span className="hardware-control__icon">{controlIcon(control.kind)}</span>
            <div className="hardware-control__identity">
              <strong>{control.name || control.default_name}</strong>
              <span>{controlCaption(control, locale)}</span>
            </div>
            {isPwm ? <div className="hardware-control__pwm">
              <Slider
                min={0}
                max={100}
                step={0.1}
                value={value}
                disabled={control.locked || !state.hardware_connected}
                onChange={(percent) => {
                  setPwmDrafts((current) => ({ ...current, [control.key]: percent }));
                  if (control.channel !== null && control.channel !== undefined) {
                    sendCmd('hardware.pwm.set', { channel: control.channel, percent });
                  }
                }}
              />
              <output>{value.toFixed(1)}%</output>
            </div> : <span
              className={`hardware-control__indicator ${control.active ? 'is-on' : ''}`}
              aria-label={control.active ? tr(locale, 'On') : tr(locale, 'Off')}
            />}
            <Space.Compact className="hardware-control__actions">
              {control.actions.map((action) => <Tooltip title={action.name || action.verb} key={action.id}>
                <Button
                  disabled={control.locked || !state.hardware_connected || (state.estop_active && action.verb !== 'stop')}
                  danger={action.verb === 'stop'}
                  onClick={() => sendCmd('hardware.action.invoke', { action_id: action.id })}
                >
                  {action.name || action.verb}
                </Button>
              </Tooltip>)}
            </Space.Compact>
          </article>;
        })}
      </div>,
    }));
  }, [details, locale, pwmDrafts, sendCmd, state.estop_active, state.hardware_connected]);

  if (!details || !state.controller_connected) {
    return <section className="surface-page hardware-page">
      <Empty
        image={<DisconnectOutlined />}
        description={tr(locale, state.controller_connected ? 'No board is connected or advertising capabilities' : 'Connecting to PCController…')}
      />
    </section>;
  }

  const strip = details.strip;
  const pixels = stripPixels ?? strip?.default_pixels ?? 1;
  const color = /^#([0-9a-f]{6})$/i.exec(stripColor)?.[1] ?? '000000';
  const rgb = [0, 2, 4].map((offset) => Number.parseInt(color.slice(offset, offset + 2), 16));

  return <section className="surface-page hardware-page">
    <header className="surface-page__header">
      <div>
        <span className="eyebrow">{tr(locale, 'Hardware Monitor')}</span>
        <Typography.Title level={2}>{details.board_name || tr(locale, 'PCController')}</Typography.Title>
        <Typography.Text type="secondary">
          <UsbOutlined /> {details.port?.display_name || details.port?.friendly_name || details.port?.name || tr(locale, 'No board port')}
        </Typography.Text>
      </div>
      <Space>
        <Tag color={state.hardware_connected ? 'success' : 'warning'}>{state.hardware_connected ? tr(locale, 'Connected') : tr(locale, 'Board unavailable')}</Tag>
        <Button
          danger
          type={state.estop_active ? 'primary' : 'default'}
          icon={<PoweroffOutlined />}
          onClick={() => sendCmd('pealayer.estop.set', { active: !state.estop_active })}
        >{tr(locale, 'E-STOP')}</Button>
      </Space>
    </header>

    {(details.warnings ?? []).map((warning) => <Alert
      key={`${warning.code}:${warning.message}`}
      type={warning.severity === 'error' ? 'error' : 'warning'}
      showIcon
      icon={<AlertOutlined />}
      message={warning.message}
      description={warning.code}
    />)}

    <div className="hardware-summary-grid">
      <Card title={tr(locale, 'Board')} className="surface-card">
        <dl className="detail-list">
          <div><dt>{tr(locale, 'Profile')}</dt><dd>{details.profile?.key || '—'}</dd></div>
          <div><dt>{tr(locale, 'Mode')}</dt><dd>{details.profile?.mode || '—'}</dd></div>
          <div><dt>{tr(locale, 'Firmware')}</dt><dd>{details.identity?.build_timestamp || '—'}</dd></div>
          <div><dt>{tr(locale, 'Serial')}</dt><dd>{details.port?.serial_number || '—'}</dd></div>
        </dl>
      </Card>
      <Card title={tr(locale, 'Live telemetry')} className="surface-card">
        <dl className="detail-list">
          <div><dt>{tr(locale, 'Supply')}</dt><dd>{telemetryValue(details.telemetry?.supply_mv as number | null, ' mV')}</dd></div>
          <div><dt>{tr(locale, 'Current')}</dt><dd>{telemetryValue(details.telemetry?.current_ma as number | null, ' mA')}</dd></div>
          <div><dt>{tr(locale, 'Power')}</dt><dd>{telemetryValue(details.telemetry?.power_mw as number | null, ' mW')}</dd></div>
          <div><dt>{tr(locale, 'Door')}</dt><dd>{telemetryValue(details.telemetry?.door_open)}</dd></div>
        </dl>
      </Card>
    </div>

    <Collapse className="hardware-sections" defaultActiveKey={grouped.map((item) => item.key)} items={grouped} />

    {strip && <Card className="surface-card strip-control" title={<Space><BulbOutlined />{tr(locale, 'Addressable lighting')}</Space>}>
      <div className="strip-control__grid">
        <label><span>{tr(locale, 'Pixels')}</span><InputNumber min={strip.minimum_pixels} max={strip.maximum_pixels} value={pixels} onChange={(value) => setStripPixels(value)} /></label>
        <label><span>{tr(locale, 'Color')}</span><ColorPicker value={stripColor} disabledAlpha onChangeComplete={(value) => setStripColor(value.toHexString().toUpperCase())} /></label>
        <label><span>{tr(locale, 'Brightness')}</span><Slider min={0} max={255} value={stripBrightness} onChange={setStripBrightness} /></label>
        <Space wrap>
          <Button onClick={() => sendCmd('hardware.strip.configure', { pixels })}>{tr(locale, 'Apply pixel count')}</Button>
          <Button type="primary" onClick={() => sendCmd('hardware.strip.fill', { red: rgb[0], green: rgb[1], blue: rgb[2], brightness: stripBrightness })}>{tr(locale, 'Fill strip')}</Button>
          <Button onClick={() => sendCmd('hardware.strip.clear')}>{tr(locale, 'Clear')}</Button>
        </Space>
      </div>
    </Card>}

    {details.front_panel && <Card className="surface-card front-panel-card" title={<Space><DesktopOutlined />{tr(locale, 'Front panel')}</Space>}>
      <div className="segment-display" aria-label={tr(locale, 'Seven segment display')}>
        {details.front_panel.raw_segments.map((segment, index) => <span key={index}>{segment.toString(16).padStart(2, '0').toUpperCase()}</span>)}
      </div>
      <Space wrap>{['K1', 'K2', 'K3', 'K4'].map((key) => <Button key={key} onClick={() => sendCmd('hardware.front_panel.press', { key })}>{key}</Button>)}</Space>
      {details.front_panel.lcd_available && <div className="lcd-display"><span>{details.front_panel.lcd_line_1}</span><span>{details.front_panel.lcd_line_2}</span></div>}
    </Card>}
  </section>;
};
