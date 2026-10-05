import React, { useMemo, useRef, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  ColorPicker,
  Collapse,
  Divider,
  Dropdown,
  Empty,
  Input,
  InputNumber,
  Modal,
  Segmented,
  Slider,
  Space,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import {
  AlertOutlined,
  ArrowLeftOutlined,
  ArrowDownOutlined,
  ArrowUpOutlined,
  BulbOutlined,
  DashboardOutlined,
  DesktopOutlined,
  DisconnectOutlined,
  ExperimentOutlined,
  MenuOutlined,
  MoreOutlined,
  EyeInvisibleOutlined,
  EyeOutlined,
  LockOutlined,
  PushpinOutlined,
  UnlockOutlined,
  PoweroffOutlined,
  ThunderboltOutlined,
  ToolOutlined,
  UsbOutlined,
} from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

interface HardwareTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
}

type HardwareControl = NonNullable<PlayerState['hardware_details']>['controls'][number];
type HardwareAction = HardwareControl['actions'][number];

const isMotionControl = (control: HardwareControl) =>
  /seat|motion/i.test(control.kind)
  || /motion/i.test(control.control)
  || control.actions.some((action) => /^(up|down|stop)$/i.test(action.verb));

const isStopAction = (action: HardwareAction) => action.verb.toLowerCase() === 'stop';

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

const controlIdentity = (
  control: NonNullable<PlayerState['hardware_details']>['controls'][number],
) => {
  if (/relay/i.test(control.kind) && control.channel != null) return `R${control.channel}`;
  if (/pwm|mosfet/i.test(control.kind) && control.channel != null) return `CH${control.channel}`;
  if (control.key === 'seat.a') return 'A';
  if (control.key === 'seat.b') return 'B';
  const parts = control.key.split('.');
  return parts[parts.length - 1]?.toUpperCase() || control.key;
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
  const [managerOpen, setManagerOpen] = useState(false);
  const [detailKey, setDetailKey] = useState<string | null>(null);
  const [renamingKey, setRenamingKey] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState('');
  const [dragKey, setDragKey] = useState<string | null>(null);
  const [dropKey, setDropKey] = useState<string | null>(null);
  const heldMotionPointers = useRef(new Map<string, number>());
  const controls = useMemo(
    () => [...(details?.controls ?? [])].sort((left, right) => left.order - right.order || left.key.localeCompare(right.key)),
    [details],
  );
  const updatePresentation = (key: string, fields: Record<string, unknown>) =>
    sendCmd('hardware.presentation.update', { key, fields });
  const actionInputProps = (control: HardwareControl, action: HardwareAction) => {
    const invoke = () => sendCmd('hardware.action.invoke', { action_id: action.id });
    if (!isMotionControl(control)) return { onClick: invoke };

    const keyboardInvoke = (event: React.MouseEvent<HTMLElement>) => {
      if (event.detail === 0) invoke();
    };
    if (isStopAction(action) || (details?.motion_control_mode ?? 'hold') === 'toggle') {
      return {
        onPointerDown: (event: React.PointerEvent<HTMLElement>) => {
          if (event.button !== 0) return;
          event.preventDefault();
          invoke();
        },
        onClick: keyboardInvoke,
      };
    }

    const stop = control.actions.find(isStopAction);
    if (!stop) return { onPointerDown: (event: React.PointerEvent<HTMLElement>) => {
      if (event.button === 0) invoke();
    }, onClick: keyboardInvoke };
    const release = (pointerId?: number) => {
      if (pointerId !== undefined && heldMotionPointers.current.get(control.key) !== pointerId) return;
      if (!heldMotionPointers.current.has(control.key)) return;
      heldMotionPointers.current.delete(control.key);
      sendCmd('hardware.action.invoke', { action_id: stop.id });
    };
    return {
      onPointerDown: (event: React.PointerEvent<HTMLElement>) => {
        if (event.button !== 0) return;
        event.preventDefault();
        heldMotionPointers.current.set(control.key, event.pointerId);
        event.currentTarget.setPointerCapture(event.pointerId);
        invoke();
      },
      onPointerUp: (event: React.PointerEvent<HTMLElement>) => release(event.pointerId),
      onPointerCancel: (event: React.PointerEvent<HTMLElement>) => release(event.pointerId),
      onKeyDown: (event: React.KeyboardEvent<HTMLElement>) => {
        if ((event.key === 'Enter' || event.key === ' ') && !event.repeat && !heldMotionPointers.current.has(control.key)) {
          heldMotionPointers.current.set(control.key, -1);
          invoke();
        }
      },
      onKeyUp: (event: React.KeyboardEvent<HTMLElement>) => {
        if (event.key === 'Enter' || event.key === ' ') release(-1);
      },
      onClick: (event: React.MouseEvent<HTMLElement>) => event.preventDefault(),
    };
  };
  const visibleActions = (control: HardwareControl) => control.actions.filter(
    (action) => !isStopAction(action) || Boolean(control.active),
  );
  const moveControl = (key: string, delta: number) => {
    const source = controls.find((control) => control.key === key);
    if (!source) return;
    const peers = controls.filter((control) => control.kind === source.kind);
    const index = peers.findIndex((control) => control.key === key);
    const next = Math.max(0, Math.min(peers.length - 1, index + delta));
    if (index !== next) updatePresentation(key, { order: next });
  };
  const dropControl = (sourceKey: string, targetKey: string) => {
    const source = controls.find((control) => control.key === sourceKey);
    const target = controls.find((control) => control.key === targetKey);
    if (!source || !target || source.kind !== target.kind || source.key === target.key) return;
    const peers = controls.filter((control) => control.kind === source.kind);
    updatePresentation(sourceKey, { order: peers.findIndex((control) => control.key === targetKey) });
  };
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
          return <article
            className={`hardware-control ${control.locked ? 'is-locked' : ''} ${dragKey === control.key ? 'is-dragging' : ''} ${dropKey === control.key ? 'is-drop-target' : ''}`}
            key={control.key}
            onDragOver={(event) => {
              if (dragKey && dragKey !== control.key) {
                event.preventDefault();
                setDropKey(control.key);
              }
            }}
            onDragLeave={() => setDropKey((current) => current === control.key ? null : current)}
            onDrop={(event) => {
              event.preventDefault();
              if (dragKey) dropControl(dragKey, control.key);
              setDragKey(null);
              setDropKey(null);
            }}
          >
            <span className="hardware-control__icon">{controlIcon(control.kind)}</span>
            <span
              className="hardware-control__drag"
              draggable
              title={tr(locale, 'Drag to reorder channel')}
              onDragStart={(event) => {
                const card = event.currentTarget.closest<HTMLElement>('.hardware-control');
                if (card) {
                  const rect = card.getBoundingClientRect();
                  event.dataTransfer.setDragImage(card, event.clientX - rect.left, event.clientY - rect.top);
                }
                event.dataTransfer.effectAllowed = 'move';
                event.dataTransfer.setData('text/plain', control.key);
                setDragKey(control.key);
              }}
              onDragEnd={() => {
                setDragKey(null);
                setDropKey(null);
              }}
            ><MenuOutlined /></span>
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
              {visibleActions(control).map((action) => <Tooltip title={action.name || action.verb} key={action.id}>
                <Button
                  disabled={control.locked || !state.hardware_connected || (state.estop_active && action.verb !== 'stop')}
                  danger={action.verb === 'stop'}
                  {...actionInputProps(control, action)}
                >
                  {action.name || action.verb}
                </Button>
              </Tooltip>)}
            </Space.Compact>
          </article>;
        })}
      </div>,
    }));
  }, [details, locale, pwmDrafts, sendCmd, state.estop_active, state.hardware_connected, dragKey, dropKey, controls]);

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
        {state.hardware_connected && <Button icon={<ToolOutlined />} onClick={() => setManagerOpen(true)}>{tr(locale, 'Manage channels')}</Button>}
        <Tag color={state.hardware_connected ? 'success' : 'warning'}>{state.hardware_connected ? tr(locale, 'Connected') : tr(locale, 'Board unavailable')}</Tag>
        <Button
          danger
          type={state.estop_active ? 'primary' : 'default'}
          icon={<PoweroffOutlined />}
          onClick={() => sendCmd('pealayer.estop.set', { active: !state.estop_active })}
        >{tr(locale, 'E-STOP')}</Button>
      </Space>
    </header>

    <Modal
      className="channel-manager-modal"
      title={detailKey
        ? <Space><Button type="text" size="small" icon={<ArrowLeftOutlined />} aria-label={tr(locale, 'All board channels')} onClick={() => setDetailKey(null)} /><ToolOutlined />{tr(locale, 'Manage')}</Space>
        : <Space><ToolOutlined />{tr(locale, 'Manage channels')}</Space>}
      open={managerOpen && Boolean(state.hardware_connected)}
      width={860}
      footer={null}
      onCancel={() => {
        setDetailKey(null);
        setManagerOpen(false);
      }}
    >
      {detailKey ? controls.filter((control) => control.key === detailKey).map((control) => <div className="channel-detail" key={control.key}>
        <div className="channel-detail__identity"><span>{controlIcon(control.kind)}</span><div><strong>{control.name || control.default_name}</strong><code>{controlIdentity(control)}</code></div></div>
        <label><span>{tr(locale, 'Name')}</span><Input defaultValue={control.name || control.default_name} onPressEnter={(event) => updatePresentation(control.key, { name: event.currentTarget.value.trim() })} /></label>
        <dl className="detail-list">
          <div><dt>{tr(locale, 'Order')}</dt><dd>{control.order + 1}</dd></div>
          <div><dt>{tr(locale, 'Type')}</dt><dd>{control.kind}</dd></div>
          <div><dt>{tr(locale, 'Control')}</dt><dd>{control.control}</dd></div>
          <div><dt>{tr(locale, 'Stable key')}</dt><dd><code>{control.key}</code></dd></div>
        </dl>
        <Space wrap>{visibleActions(control).map((action) => <Button key={action.id} disabled={control.locked || (state.estop_active && action.verb !== 'stop')} {...actionInputProps(control, action)}>{action.name || action.verb}</Button>)}</Space>
      </div>) : <>
        <div className="channel-manager__summary">
          <strong>{details.board_name || tr(locale, 'PCController')}</strong>
          <Typography.Text type="secondary">{tr(locale, 'All board channels')} · {controls.length}</Typography.Text>
        </div>
        <div className="channel-manager__list">
          {[...new Set(controls.map((control) => semanticSection(control.kind, locale).key))].map((sectionKey) => {
            const peers = controls.filter((control) => semanticSection(control.kind, locale).key === sectionKey);
            const kind = peers[0]?.kind ?? sectionKey;
            return <section className="channel-manager__group" key={sectionKey}>
              <header>
                <span>{controlIcon(kind)}</span><strong>{semanticSection(kind, locale).label}</strong><Tag>{peers.length}</Tag>
                {sectionKey === 'motion' && <Segmented
                  size="small"
                  value={details.motion_control_mode ?? 'hold'}
                  options={[
                    { label: tr(locale, 'Push'), value: 'hold' },
                    { label: tr(locale, 'Toggle'), value: 'toggle' },
                  ]}
                  onChange={(value) => sendCmd('pealayer.config.update', { motion_control_mode: value })}
                />}
              </header>
              {peers.map((control, index) => {
                const active = Boolean(control.active);
                const onAction = control.actions.find((action) => action.verb.toLowerCase() === 'on');
                const offAction = control.actions.find((action) => action.verb.toLowerCase() === 'off');
                const baseLiveActions = onAction && offAction
                  ? [onAction, offAction]
                  : control.actions.filter((action) => action.verb.toLowerCase() !== 'stop').slice(0, 2);
                const stopAction = control.actions.find(isStopAction);
                const liveActions = active && stopAction ? [...baseLiveActions, stopAction] : baseLiveActions;
                return <React.Fragment key={control.key}>
                  <div
                    className={`channel-manager__row ${control.hidden ? 'is-hidden' : ''} ${dragKey === control.key ? 'is-dragging' : ''} ${dropKey === control.key ? 'is-drop-target' : ''}`}
                    onDragOver={(event) => {
                      if (dragKey && dragKey !== control.key) {
                        event.preventDefault();
                        setDropKey(control.key);
                      }
                    }}
                    onDragLeave={() => setDropKey((current) => current === control.key ? null : current)}
                    onDrop={(event) => {
                      event.preventDefault();
                      if (dragKey) dropControl(dragKey, control.key);
                      setDragKey(null);
                      setDropKey(null);
                    }}
                  >
                    <span className={`hardware-control__indicator ${active ? 'is-on' : ''}`} />
                    <span
                      className="channel-manager__drag"
                      draggable
                      title={tr(locale, 'Drag to reorder channel')}
                      onDragStart={(event) => {
                        const row = event.currentTarget.closest<HTMLElement>('.channel-manager__row');
                        if (row) {
                          const rect = row.getBoundingClientRect();
                          event.dataTransfer.setDragImage(row, event.clientX - rect.left, event.clientY - rect.top);
                        }
                        event.dataTransfer.effectAllowed = 'move';
                        event.dataTransfer.setData('text/plain', control.key);
                        setDragKey(control.key);
                      }}
                      onDragEnd={() => {
                        setDragKey(null);
                        setDropKey(null);
                      }}
                    ><MenuOutlined /></span>
                    <span className="channel-manager__identity" title={control.key}>{controlIdentity(control)}</span>
                    {renamingKey === control.key ? <Input
                      autoFocus
                      size="small"
                      value={renameDraft}
                      onChange={(event) => setRenameDraft(event.target.value)}
                      onPressEnter={() => {
                        updatePresentation(control.key, { name: renameDraft.trim() });
                        setRenamingKey(null);
                      }}
                      onBlur={() => {
                        if (renameDraft.trim() && renameDraft.trim() !== control.name) {
                          updatePresentation(control.key, { name: renameDraft.trim() });
                        }
                        setRenamingKey(null);
                      }}
                    /> : <button
                      type="button"
                      className="channel-manager__name"
                      onClick={() => {
                        setRenameDraft(control.name || control.default_name);
                        setRenamingKey(control.key);
                      }}
                    >{control.name || control.default_name}</button>}
                    <Space.Compact className="channel-manager__live">
                      {liveActions.map((action) => {
                        const verb = action.verb.toLowerCase();
                        const selected = (verb === 'on' && active) || (verb === 'off' && !active);
                        const icon = /up/.test(verb) ? <ArrowUpOutlined /> : /down/.test(verb) ? <ArrowDownOutlined /> : verb === 'on' ? <ThunderboltOutlined /> : <PoweroffOutlined />;
                        const label = verb === 'on' ? tr(locale, 'On') : verb === 'off' ? tr(locale, 'Off') : action.name || action.verb;
                        return <Tooltip title={action.name || action.verb} key={action.id}><Button size="small" icon={icon} type={selected ? 'primary' : 'default'} danger={verb === 'stop'} disabled={control.locked || (state.estop_active && verb !== 'stop')} {...actionInputProps(control, action)}>{label}</Button></Tooltip>;
                      })}
                    </Space.Compact>
                    <div className="channel-manager__ordering">
                      <InputNumber size="small" controls={false} min={1} max={peers.length} value={control.order + 1} aria-label={tr(locale, 'Order')} onChange={(value) => value != null && updatePresentation(control.key, { order: value - 1 })} />
                      <Tooltip title={tr(locale, 'Move up')}><Button size="small" icon={<ArrowUpOutlined />} disabled={index === 0} onClick={() => moveControl(control.key, -1)} /></Tooltip>
                      <Tooltip title={tr(locale, 'Move down')}><Button size="small" icon={<ArrowDownOutlined />} disabled={index + 1 === peers.length} onClick={() => moveControl(control.key, 1)} /></Tooltip>
                    </div>
                    <Dropdown
                      trigger={['click']}
                      menu={{
                        items: [
                          { key: 'manage', icon: <ToolOutlined />, label: tr(locale, 'Manage') },
                          { key: 'pin', icon: <PushpinOutlined />, label: tr(locale, 'Pin to top') },
                          { type: 'divider' },
                          { key: 'visibility', icon: control.hidden ? <EyeOutlined /> : <EyeInvisibleOutlined />, label: tr(locale, control.hidden ? 'Show in Hardware Monitor' : 'Hide from Hardware Monitor') },
                          { key: 'lock', icon: control.locked ? <UnlockOutlined /> : <LockOutlined />, label: tr(locale, control.locked ? 'Unlock channel' : 'Lock channel') },
                        ],
                        onClick: ({ key }) => {
                          if (key === 'manage') setDetailKey(control.key);
                          if (key === 'pin') updatePresentation(control.key, { order: 0 });
                          if (key === 'visibility') updatePresentation(control.key, { hidden: !control.hidden });
                          if (key === 'lock') updatePresentation(control.key, { locked: !control.locked });
                        },
                      }}
                    ><Button size="small" icon={<MoreOutlined />} aria-label={tr(locale, 'Channel actions')} /></Dropdown>
                  </div>
                  {index + 1 < peers.length && <Divider />}
                </React.Fragment>;
              })}
            </section>;
          })}
        </div>
      </>}
    </Modal>

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
