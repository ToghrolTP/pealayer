import React, { useEffect, useRef, useState } from 'react';
import { DeleteOutlined, LoadingOutlined, StopOutlined } from '@ant-design/icons';
import { Button, Checkbox, Modal, Popconfirm, Select, Space, Tooltip } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

interface EffectRecorderProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
  effect: Record<string, unknown>;
  onSequenceChange: (steps: NonNullable<PlayerState['effect_recording']>['preview'], id: number) => void;
}

/** Capture actions into the effect already being edited, using its identity. */
export const EffectRecorder: React.FC<EffectRecorderProps> = ({ state, sendCmd, locale, effect, onSequenceChange }) => {
  const recording = state.effect_recording;
  const [mode, setMode] = useState('automatic');
  const [choosing, setChoosing] = useState(false);
  const [selection, setSelection] = useState({ all: false, control_keys: [] as string[], opcodes: [] as number[] });
  const capabilities = recording?.capabilities;
  const hasSelection = selection.all || selection.control_keys.length > 0 || selection.opcodes.length > 0;
  const take = useRef<{ steps: NonNullable<PlayerState['effect_recording']>['preview']; active: boolean; discard: boolean } | null>(null);
  const update = useRef(onSequenceChange);
  update.current = onSequenceChange;
  useEffect(() => {
    if (!take.current || !recording) return;
    if (recording.active) {
      take.current.active = true;
      update.current(recording.preview, recording.id);
    } else if (take.current.active && !recording.pending) {
      update.current(take.current.discard ? take.current.steps : recording.preview, recording.id);
      take.current = null;
    }
  }, [recording]);
  const active = Boolean(recording?.active);
  const pending = Boolean(recording?.pending);
  const canRecord = state.controller_connected && state.hardware_connected;
  const recordDot = <span className="record-dot" />;
  return <Space wrap size={8} className="effect-capture-controls">
    <Select aria-label={tr(locale, 'Capture mode')} value={mode} disabled={active || pending} onChange={nextMode => {
      setMode(nextMode); setSelection(current => ({ ...current,
        opcodes: current.opcodes.filter(value => capabilities?.opcodes.some(opcode => opcode.opcode === value && opcode.capture_modes.includes(nextMode))),
        control_keys: current.control_keys.filter(key => capabilities?.controls.some(control => control.key === key && (nextMode !== 'board-retained' || control.kind !== 'pwm'))),
      }));
    }} style={{ width: 160 }} options={[
      { value: 'automatic', label: tr(locale, 'Automatic capture') },
      { value: 'device-clock', label: tr(locale, 'Device clock') },
      { value: 'board-retained', label: tr(locale, 'Board capture') },
    ]} />
    <Button disabled={active || pending || !canRecord} onClick={() => { setChoosing(true); sendCmd('controller_effect.record.capabilities'); }}>{tr(locale, 'Capture selection')}</Button>
    <Modal title={tr(locale, 'Capture selection')} open={choosing} onCancel={() => setChoosing(false)} onOk={() => setChoosing(false)}>
      <p>{tr(locale, 'Choose channels or opcodes; nothing is selected by default')}</p>
      <Checkbox checked={selection.all} onChange={event => setSelection({ ...selection, all: event.target.checked })}>{tr(locale, 'All supported commands (all channels)')}</Checkbox>
      <p className="muted">{tr(locale, 'Opcode selection captures all targets of that command')}</p>
      <Select mode="multiple" style={{ width: '100%' }} aria-label={tr(locale, 'Channels')} placeholder={tr(locale, 'Select channels or opcodes to record')}
        value={selection.control_keys} options={(capabilities?.controls ?? []).filter(control => ['motion', 'relay', 'pwm'].includes(control.kind) && (mode !== 'board-retained' || control.kind !== 'pwm'))
          .map(control => ({ value: control.key, label: control.name }))} onChange={control_keys => setSelection({ ...selection, control_keys })} />
      <Select mode="multiple" style={{ width: '100%', marginTop: 12 }} aria-label={tr(locale, 'Opcodes (all targets)')}
        value={selection.opcodes} options={(capabilities?.opcodes ?? []).filter(opcode => opcode.capture_modes.includes(mode))
          .map(opcode => ({ value: opcode.opcode, label: `0x${opcode.opcode.toString(16).padStart(2, '0').toUpperCase()} · ${opcode.name}` }))}
        onChange={opcodes => setSelection({ ...selection, opcodes })} />
    </Modal>
    {active ? <>
      <span aria-live="polite">{recordDot} {recording?.steps ?? 0} {tr(locale, 'actions')}</span>
      <Button icon={<StopOutlined />} disabled={pending} onClick={() => { if (take.current) take.current.discard = false; sendCmd('controller_effect.record.save'); }}>{tr(locale, 'Finish')}</Button>
      <Popconfirm title={tr(locale, 'Discard this take?')} description={tr(locale, 'Existing sequence steps are retained')} onConfirm={() => { if (take.current) take.current.discard = true; sendCmd('controller_effect.record.discard'); }}>
        <Button icon={<DeleteOutlined />} disabled={pending}>{tr(locale, 'Discard take')}</Button>
      </Popconfirm>
    </> : <Tooltip title={tr(locale, 'Publish the current sequence and capture at its end. Delete existing steps first to replace them.')}>
      <Button type="primary" danger icon={pending ? <LoadingOutlined spin /> : recordDot} disabled={!canRecord || pending || !hasSelection || !String(effect.name ?? '').trim()} onClick={() => {
        const program = effect.program as { steps?: NonNullable<PlayerState['effect_recording']>['preview'] };
        take.current = { steps: structuredClone(program.steps ?? []), active: false, discard: false };
        sendCmd('controller_effect.record.start', { name: effect.name, category: effect.category, color: effect.color, mode, capture_selection: selection, effect });
      }}>{tr(locale, 'Record')}</Button>
    </Tooltip>}
    {recording?.last_error && <span role="alert">{recording.last_error}</span>}
  </Space>;
};
