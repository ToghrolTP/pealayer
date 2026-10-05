import React, { useState } from 'react';
import { DeleteOutlined, LoadingOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Input, Popconfirm, Select, Space, Tag } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { GroupSelect } from './GroupSelect';

interface EffectRecorderProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
  compact?: boolean;
}

/**
 * One recorder for every Web surface. PCController owns the take and the
 * finished catalog entry; this component only presents that live state.
 */
export const EffectRecorder: React.FC<EffectRecorderProps> = ({ state, sendCmd, locale, compact = false }) => {
  const recording = state.effect_recording ?? {
    active: false, id: 0, name: '', mode: '', category: '', color: '', steps: 0,
    device_retained: false, overwritten: 0, started_at: '', last_error: '', pending: false,
  };
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [category, setCategory] = useState('Recorded');
  const [color, setColor] = useState('violet');
  const [mode, setMode] = useState('automatic');
  const canRecord = state.controller_connected && state.hardware_connected;

  return <section className={`effect-recorder-shell ${compact ? 'is-compact' : ''}`}>
    <div className="effects-toolbar effects-recorder-toolbar">
      <Button
        danger={recording.active}
        disabled={!canRecord && !recording.active}
        icon={recording.pending ? <LoadingOutlined spin /> : <span className="record-dot" />}
        onClick={() => setOpen((value) => !value)}
      >
        {recording.active ? `${tr(locale, 'Recording')} · ${recording.steps}` : tr(locale, 'Record effect')}
      </Button>
      {recording.active && <Tag color="red">{recording.mode || mode}</Tag>}
    </div>

    {(open || recording.active) && <div className={`effect-recorder ${recording.active ? 'is-recording' : ''}`}>
      <div className="effect-recorder__header">
        <div>
          <strong>{recording.active ? recording.name : tr(locale, 'New recorded effect')}</strong>
          <span>{recording.active ? `${recording.steps} ${tr(locale, 'captured actions')}` : tr(locale, 'Capture board, RF, front-panel, Pealayer, Web and API actions together')}</span>
        </div>
        {recording.active && <span className="recording-pulse" aria-label={tr(locale, 'Recording')} />}
      </div>

      {!recording.active && <div className="effect-recorder__grid">
        <label><span>{tr(locale, 'Name')}</span><Input value={name} autoFocus onChange={(event) => setName(event.target.value)} placeholder={tr(locale, 'Recorded effect name')} /></label>
        <label><span>{tr(locale, 'Category')}</span><GroupSelect value={category} groups={(state.controller_effects ?? []).map((effect) => effect.category)} locale={locale} onChange={setCategory} /></label>
        <label><span>{tr(locale, 'Capture mode')}</span><Select value={mode} onChange={setMode} options={[
          { value: 'automatic', label: tr(locale, 'Automatic · all live sources') },
          { value: 'device-clock', label: tr(locale, 'Device clock') },
          { value: 'board-retained', label: tr(locale, 'Board-retained relay take') },
        ]} /></label>
        <label><span>{tr(locale, 'Color')}</span><Select value={color} onChange={setColor} options={['violet', 'green', 'blue', 'red', 'white'].map((value) => ({ value, label: value }))} /></label>
      </div>}

      {recording.active && <Space wrap size={[6, 6]}>
        <Tag>{tr(locale, 'Pealayer / Web / API')}</Tag>
        <Tag>{recording.device_retained ? tr(locale, 'Physical relays / Board RAM') : tr(locale, 'Board / RF / front panel')}</Tag>
        {recording.overwritten > 0 && <Tag color="orange">{recording.overwritten} {tr(locale, 'overwritten')}</Tag>}
      </Space>}
      {recording.last_error && <div className="effect-recorder__error">{recording.last_error}</div>}

      <div className="effect-recorder__actions">
        {!recording.active ? <Button
          type="primary"
          danger
          disabled={!canRecord || !name.trim() || recording.pending}
          icon={<span className="record-dot" />}
          onClick={() => sendCmd('controller_effect.record.start', { name: name.trim(), category: category.trim() || 'Recorded', color, mode })}
        >{tr(locale, 'Start recording')}</Button> : <>
          <Button type="primary" icon={<SaveOutlined />} disabled={recording.pending || recording.steps === 0} onClick={() => sendCmd('controller_effect.record.save')}>
            {tr(locale, 'Finish and edit')}
          </Button>
          <Popconfirm title={tr(locale, 'Discard this take?')} onConfirm={() => sendCmd('controller_effect.record.discard')}>
            <Button danger icon={<DeleteOutlined />} disabled={recording.pending}>{tr(locale, 'Discard take')}</Button>
          </Popconfirm>
          <Button icon={<LoadingOutlined />} disabled={recording.pending} onClick={() => sendCmd('controller_effect.record.status')}>{tr(locale, 'Refresh')}</Button>
        </>}
      </div>
    </div>}
  </section>;
};
