import React, { useState } from 'react';
import { DeleteOutlined, LoadingOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Input, Popconfirm, Select, Space, Tag } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { GroupSelect } from './GroupSelect';
import { EffectGroupDialog } from './EffectGroupDialog';
import recordingColors from '../../../assets/themes/recording-colors.json';

interface EffectRecorderProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
  compact?: boolean;
  onNewGroup?: () => void;
}

/**
 * One recorder for every Web surface. PCController owns the take and the
 * finished catalog entry; this component only presents that live state.
 */
export const EffectRecorder: React.FC<EffectRecorderProps> = ({ state, sendCmd, locale, compact = false, onNewGroup }) => {
  const recording = state.effect_recording ?? {
    active: false, id: 0, name: '', mode: '', category: '', color: '', steps: 0,
    preview: [], device_retained: false, overwritten: 0, started_at: '', last_error: '', pending: false,
  };
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [category, setCategory] = useState('Recorded');
  const [newGroupName, setNewGroupName] = useState<string | null>(null);
  const [color, setColor] = useState('violet');
  const [mode, setMode] = useState('automatic');
  const canRecord = state.controller_connected && state.hardware_connected;
  const selectedColor = recording.active && recording.color ? recording.color : color;
  const colorHex = (recordingColors.find((option) => option.id === (selectedColor === 'purple' ? 'violet' : selectedColor)) ?? recordingColors[0]).hex;
  const recordDot = <span className="record-dot" style={{ backgroundColor: colorHex }} />;

  return <section className={`effect-recorder-shell ${compact ? 'is-compact' : ''}`}>
    <div className="effects-toolbar effects-recorder-toolbar">
      <Button
        danger={recording.active}
        disabled={!canRecord && !recording.active}
        icon={recording.pending ? <LoadingOutlined spin /> : recordDot}
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
        {recording.active && <span className="recording-pulse" style={{ backgroundColor: colorHex, boxShadow: `0 0 0 5px ${colorHex}29` }} aria-label={tr(locale, 'Recording')} />}
      </div>

      {!recording.active && <div className="effect-recorder__grid">
        <label><span>{tr(locale, 'Name')}</span><Input value={name} autoFocus onChange={(event) => setName(event.target.value)} placeholder={tr(locale, 'Recorded effect name')} /></label>
        <label><span>{tr(locale, 'Group')}</span><GroupSelect value={category} groups={(state.controller_effect_groups ?? []).map((group) => group.name)} locale={locale} onChange={setCategory} onCreate={onNewGroup ?? (() => setNewGroupName(''))} /></label>
        <label><span>{tr(locale, 'Capture mode')}</span><Select value={mode} onChange={setMode} options={[
          { value: 'automatic', label: tr(locale, 'Automatic · all live sources') },
          { value: 'device-clock', label: tr(locale, 'Device clock') },
          { value: 'board-retained', label: tr(locale, 'Board-retained relay take') },
        ]} /></label>
        <label><span>{tr(locale, 'Color')}</span><Select value={color} onChange={setColor} options={recordingColors.map((option) => ({
          value: option.id,
          label: <span className="recording-color-option"><span className="recording-color-swatch" style={{ backgroundColor: option.hex }} />{tr(locale, option.label)}</span>,
        }))} /></label>
      </div>}

      {recording.active && <Space wrap size={[6, 6]}>
        <Tag>{tr(locale, 'Pealayer / Web / API')}</Tag>
        <Tag>{recording.device_retained ? tr(locale, 'Physical relays / Board RAM') : tr(locale, 'Board / RF / front panel')}</Tag>
        {recording.overwritten > 0 && <Tag color="orange">{recording.overwritten} {tr(locale, 'overwritten')}</Tag>}
      </Space>}
      {recording.active && <div className="effect-recorder__preview" aria-live="polite">
        <strong>{tr(locale, 'Live sequence')}</strong>
        {(recording.preview ?? []).length === 0 ? <span>{tr(locale, 'Waiting for the first captured action…')}</span> : <ol>
          {(recording.preview ?? []).map((step, index) => {
            const detail = step.action_ids?.length
              ? step.action_ids.join(' · ')
              : step.text?.trim()
                ? step.text.replace(/\n/g, ' · ')
                : [step.target === undefined ? '' : `target ${step.target}`, step.value === undefined ? '' : `value ${step.value}`].filter(Boolean).join(' · ');
            return <li key={`${step.at_us}-${index}`}>
              <time>{(step.at_us / 1_000_000).toFixed(3)}s</time>
              <b>{step.kind}</b>
              {detail && <span>{detail}</span>}
            </li>;
          })}
        </ol>}
      </div>}
      {recording.last_error && <div className="effect-recorder__error">{recording.last_error}</div>}

      <div className="effect-recorder__actions">
        {!recording.active ? <Button
          type="primary"
          danger
          disabled={!canRecord || !name.trim() || recording.pending}
          icon={recordDot}
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
    <EffectGroupDialog name={newGroupName} setName={setNewGroupName} locale={locale} create={() => {
      if (!newGroupName?.trim()) return;
      sendCmd('controller_effect.group.create', { name: newGroupName.trim() });
      setNewGroupName(null);
    }} />
  </section>;
};
