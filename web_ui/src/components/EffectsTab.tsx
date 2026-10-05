import React, { useMemo, useState } from 'react';
import {
  Button,
  Card,
  Collapse,
  ColorPicker,
  Dropdown,
  Empty,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Select,
  Space,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  CheckOutlined,
  ClockCircleOutlined,
  DeleteOutlined,
  EditOutlined,
  FolderAddOutlined,
  MoreOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  SaveOutlined,
  StopOutlined,
  UnorderedListOutlined,
} from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { EffectIconPicker, effectGlyph, effectIconOptions } from '../effectIcons';
import { EffectRecorder } from './EffectRecorder';
import { GroupSelect } from './GroupSelect';

interface EffectsTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
}

type EffectStep = {
  at_us: number;
  kind: string;
  target?: number;
  value?: number;
  duration_ms?: number;
  frequency_hz?: number;
  text?: string;
  destination?: string;
  code?: number;
  bits?: number;
  protocol?: number;
  pulse_us?: number;
  red?: number;
  green?: number;
  blue?: number;
  brightness?: number;
  opcode?: number;
  payload_hex?: string;
  action_ids?: string[];
};

type EffectDraft = {
  reference: string;
  id: string;
  name: string;
  icon: string;
  category: string;
  description: string;
  kind: 'sequence' | 'strip-stream';
  duration_ms: number;
  color: string;
  default_fps: number;
  default_pixels: number;
  steps: EffectStep[];
  properties: Record<string, unknown>;
  programText: string;
  is_new: boolean;
};

type InlineEffectEdit = {
  reference: string;
  name: string;
  icon: string;
};

const defaultStep = (): EffectStep => ({ at_us: 0, kind: 'relay', target: 1, value: 1, action_ids: [] });

const programParts = (program: unknown) => {
  if (Array.isArray(program)) return { steps: program as EffectStep[], properties: {} };
  if (program && typeof program === 'object') {
    const value = program as { steps?: EffectStep[]; properties?: Record<string, unknown> };
    return { steps: Array.isArray(value.steps) ? value.steps : [], properties: value.properties ?? {} };
  }
  return { steps: [], properties: {} };
};

export const EffectsTab: React.FC<EffectsTabProps> = ({ state, sendCmd, locale }) => {
  const effects = state.controller_effects ?? [];
  const [draft, setDraft] = useState<EffectDraft | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [inlineEdit, setInlineEdit] = useState<InlineEffectEdit | null>(null);
  const [newGroupName, setNewGroupName] = useState<string | null>(null);
  const selectedEffect = effects.find((effect) => effect.reference === selected);
  const grouped = useMemo(() => {
    const groups = new Map<string, typeof effects>();
    for (const effect of effects) groups.set(effect.category || tr(locale, 'Other'), [...(groups.get(effect.category || tr(locale, 'Other')) ?? []), effect]);
    return [...groups.entries()];
  }, [effects, locale]);

  const openEditor = (effect?: typeof effects[number], category?: string) => {
    const parts = programParts(effect?.program);
    setDraft(effect ? {
      reference: effect.reference,
      id: effect.id,
      name: effect.name,
      icon: effect.icon || 'plug',
      category: effect.category,
      description: effect.description,
      kind: effect.kind,
      duration_ms: effect.duration_ms,
      color: String(parts.properties.color ?? '#38D27A'),
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      steps: parts.steps,
      properties: parts.properties,
      programText: effect.kind === 'strip-stream' ? JSON.stringify(effect.program ?? {}, null, 2) : '{}',
      is_new: false,
    } : {
      reference: '', id: '', name: '', icon: 'plug', category: category ?? 'Motion', description: '', kind: 'sequence',
      duration_ms: 1000, color: '#38D27A', default_fps: 20, default_pixels: 100,
      steps: [defaultStep()], properties: { mode: 'host', timing_tolerance_us: 0, keep_outputs_on_cancel: false },
      programText: '{}', is_new: true,
    });
  };

  const createGroupEffect = () => {
    const category = newGroupName?.trim();
    if (!category) return;
    openEditor(undefined, category);
    setNewGroupName(null);
  };

  const updateStep = (index: number, patch: Partial<EffectStep>) => {
    if (!draft) return;
    const steps = draft.steps.map((step, stepIndex) => stepIndex === index ? { ...step, ...patch } : step);
    setDraft({ ...draft, steps });
  };
  const moveStep = (index: number, delta: number) => {
    if (!draft) return;
    const destination = index + delta;
    if (destination < 0 || destination >= draft.steps.length) return;
    const steps = [...draft.steps];
    [steps[index], steps[destination]] = [steps[destination], steps[index]];
    setDraft({ ...draft, steps });
  };

  const save = () => {
    if (!draft) return;
    let program: unknown;
    if (draft.kind === 'sequence') {
      program = {
        steps: draft.steps.map((step) => ({ ...step, at_us: Math.max(0, Math.round(step.at_us)) })),
        properties: { ...draft.properties, color: draft.color },
      };
    } else {
      try { program = JSON.parse(draft.programText || '{}'); }
      catch { void message.error(tr(locale, 'Program must be valid JSON')); return; }
    }
    sendCmd('controller_effect.save', {
      reference: draft.reference,
      id: draft.id,
      name: draft.name,
      icon: draft.icon,
      category: draft.category,
      description: draft.description,
      kind: draft.kind,
      duration_ms: draft.duration_ms,
      color: draft.color,
      default_fps: draft.default_fps,
      default_pixels: draft.default_pixels,
      program,
      is_new: draft.is_new,
    });
    setDraft(null);
  };

  const saveInlineIdentity = (effect: typeof effects[number]) => {
    if (!inlineEdit || inlineEdit.reference !== effect.reference || !inlineEdit.name.trim()) return;
    const parts = programParts(effect.program);
    sendCmd('controller_effect.save', {
      reference: effect.reference,
      id: effect.id,
      name: inlineEdit.name.trim(),
      icon: inlineEdit.icon,
      category: effect.category,
      description: effect.description,
      kind: effect.kind,
      duration_ms: effect.duration_ms,
      color: String(parts.properties.color ?? '#38D27A'),
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      program: effect.program,
      is_new: false,
    });
    setInlineEdit(null);
  };

  return <section className="surface-page effects-library-page">
    <header className="surface-page__header">
      <div>
        <span className="eyebrow">{tr(locale, 'PCController catalog')}</span>
        <Typography.Title level={2}>{tr(locale, 'Effects Library')}</Typography.Title>
        <Typography.Text type="secondary">{state.hardware?.board_name || tr(locale, 'No hardware')}</Typography.Text>
      </div>
      <Space>
        {state.hardware_details?.strip?.running && <Button icon={<StopOutlined />} onClick={() => sendCmd('controller_effect.stop')}>{tr(locale, 'Stop preview')}</Button>}
        <Button icon={<FolderAddOutlined />} onClick={() => setNewGroupName('')}>{tr(locale, 'New group')}</Button>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => openEditor()}>{tr(locale, 'New effect')}</Button>
      </Space>
    </header>

    {effects.length === 0 ? <Card className="surface-card"><Empty description={tr(locale, 'No effects')}><Space wrap><Button type="primary" icon={<PlusOutlined />} onClick={() => openEditor()}>{tr(locale, 'Create effect')}</Button><Button icon={<FolderAddOutlined />} onClick={() => setNewGroupName('')}>{tr(locale, 'New group')}</Button></Space></Empty></Card> :
      <Collapse className="effect-groups" defaultActiveKey={grouped.map(([group]) => group)} items={grouped.map(([group, items]) => ({
        key: group,
        label: <span className="effect-group-title"><strong>{group}</strong><span className="effect-group-count">{items.length}</span></span>,
        extra: <Tooltip title={tr(locale, 'New effect in this group')}><Button type="text" size="small" icon={<PlusOutlined />} aria-label={tr(locale, 'New effect in this group')} onClick={(event) => { event.stopPropagation(); openEditor(undefined, group); }} /></Tooltip>,
        children: <div className="effect-card-grid">{items.map((effect) => {
          const actions = [
            { key: 'play', label: tr(locale, 'Play effect'), icon: <PlayCircleOutlined /> },
            { key: 'manage', label: tr(locale, 'Manage'), icon: <EditOutlined /> },
            { key: 'cue', label: tr(locale, 'Add cue at playhead'), icon: <PlusOutlined /> },
            { type: 'divider' as const },
            { key: 'delete', label: tr(locale, 'Delete'), icon: <DeleteOutlined />, danger: true },
          ];
          const run = (key: string) => {
            if (key === 'play') sendCmd('controller_effect.play', { reference: effect.reference });
            if (key === 'manage') openEditor(effect);
            if (key === 'cue') sendCmd('controller_effect_cue.add', { reference: effect.reference, start_time_ms: Math.max(0, Math.round((state.playback_time ?? 0) * 1000)) });
            if (key === 'delete') sendCmd('controller_effect.delete', { reference: effect.reference });
          };
          const editing = inlineEdit?.reference === effect.reference;
          const beginInlineEdit = () => setInlineEdit({
            reference: effect.reference,
            name: effect.name,
            icon: effect.icon || 'plug',
          });
          return <article className={`effect-card ${selected === effect.reference ? 'is-selected' : ''} ${editing ? 'is-editing' : ''}`} key={effect.reference} onClick={() => setSelected(effect.reference)}>
            {editing ? <>
              <Select
                className="effect-card__inline-icon"
                value={inlineEdit.icon}
                options={effectIconOptions}
                popupMatchSelectWidth={false}
                showSearch
                optionFilterProp="value"
                aria-label={tr(locale, 'Icon')}
                onClick={(event) => event.stopPropagation()}
                onChange={(icon) => setInlineEdit({ ...inlineEdit, icon })}
              />
              <Input
                className="effect-card__inline-name"
                value={inlineEdit.name}
                autoFocus
                maxLength={64}
                aria-label={tr(locale, 'Name')}
                onClick={(event) => event.stopPropagation()}
                onChange={(event) => setInlineEdit({ ...inlineEdit, name: event.target.value })}
                onPressEnter={() => saveInlineIdentity(effect)}
                onKeyDown={(event) => {
                  if (event.key === 'Escape') setInlineEdit(null);
                }}
              />
              <Tooltip title={tr(locale, 'Save')}><Button type="text" icon={<CheckOutlined />} disabled={!inlineEdit.name.trim()} onClick={(event) => { event.stopPropagation(); saveInlineIdentity(effect); }} /></Tooltip>
              <Tooltip title={tr(locale, 'Cancel')}><Button type="text" icon={<StopOutlined />} onClick={(event) => { event.stopPropagation(); setInlineEdit(null); }} /></Tooltip>
            </> : <>
              <Tooltip title={tr(locale, 'Change icon')}>
                <button type="button" className="effect-card__icon" onClick={(event) => { event.stopPropagation(); beginInlineEdit(); }}>{effectGlyph(effect.icon)}</button>
              </Tooltip>
              <button type="button" className="effect-card__caption" onClick={(event) => { event.stopPropagation(); beginInlineEdit(); }}>
                <strong>{effect.name}</strong>
              </button>
              <Tooltip title={tr(locale, 'Play effect')}><Button type="text" icon={<PlayCircleOutlined />} onClick={(event) => { event.stopPropagation(); run('play'); }} /></Tooltip>
              <Dropdown menu={{ items: actions, onClick: ({ key }) => run(key) }} trigger={['click']}>
                <Button type="text" icon={<MoreOutlined />} onClick={(event) => event.stopPropagation()} aria-label={tr(locale, 'Actions')} />
              </Dropdown>
            </>}
            <div className="effect-card__metadata">
              <div className="effect-card__badges">
                {effect.lane && <span className="effect-card__badge effect-card__kind" title={effect.lane}>{effect.lane}</span>}
                <span className="effect-card__badge" title={tr(locale, 'Actions')}><UnorderedListOutlined /><span>{effect.action_count} {tr(locale, effect.action_count === 1 ? 'action' : 'actions')}</span></span>
              </div>
              <span className="effect-card__duration" title={tr(locale, 'Total duration')}><ClockCircleOutlined /><span>{effect.duration_display}</span></span>
            </div>
          </article>;
        })}</div>,
      }))} />}

    {selectedEffect && <div className="selection-bar"><span>{selectedEffect.name}</span><Button icon={<EditOutlined />} onClick={() => openEditor(selectedEffect)}>{tr(locale, 'Manage')}</Button></div>}

    <Modal
      title={<Space><FolderAddOutlined />{tr(locale, 'New group')}</Space>}
      open={newGroupName !== null}
      width={360}
      destroyOnHidden
      okText={tr(locale, 'Create effect')}
      cancelText={tr(locale, 'Cancel')}
      okButtonProps={{ disabled: !newGroupName?.trim(), icon: <PlusOutlined /> }}
      onCancel={() => setNewGroupName(null)}
      onOk={createGroupEffect}
    >
      <Input autoFocus aria-label={tr(locale, 'Group name')} placeholder={tr(locale, 'Group name')} value={newGroupName ?? ''} onChange={(event) => setNewGroupName(event.target.value)} onPressEnter={createGroupEffect} />
    </Modal>

    <Modal
      className="effect-editor-modal"
      title={draft?.is_new ? tr(locale, 'New effect') : tr(locale, 'Manage effect')}
      open={Boolean(draft)}
      onCancel={() => setDraft(null)}
      onOk={save}
      okText={tr(locale, 'Save')}
      okButtonProps={{ icon: <SaveOutlined /> }}
      width={900}
    >
      {draft && <div className="effect-editor">
        {draft.is_new && draft.kind === 'sequence' && <EffectRecorder state={state} sendCmd={sendCmd} locale={locale} />}
        <div className="effect-editor__identity">
          <label><span>{tr(locale, 'Type')}</span><Select value={draft.kind} options={[{ value: 'sequence', label: tr(locale, 'Sequence') }, { value: 'strip-stream', label: tr(locale, 'Lighting') }]} onChange={(kind) => setDraft({ ...draft, kind })} /></label>
          <label><span>{tr(locale, 'ID')}</span><Input value={draft.id} onChange={(event) => setDraft({ ...draft, id: event.target.value })} /></label>
          <label><span>{tr(locale, 'Name')}</span><Input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></label>
          <label><span>{tr(locale, 'Icon')}</span><EffectIconPicker value={draft.icon} searchPlaceholder={tr(locale, 'Search icons...')} presetsLabel={tr(locale, 'Presets')} emptyLabel={tr(locale, 'No matching icons')} onChange={(icon) => setDraft({ ...draft, icon })} /></label>
          <label><span>{tr(locale, 'Category')}</span><GroupSelect value={draft.category} groups={effects.map((effect) => effect.category)} locale={locale} onChange={(category) => setDraft({ ...draft, category })} /></label>
          <label className="effect-editor__wide"><span>{tr(locale, 'Description')}</span><Input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} /></label>
          <label><span>{tr(locale, 'Duration')}</span><InputNumber min={1} addonAfter="ms" value={draft.duration_ms} onChange={(duration_ms) => setDraft({ ...draft, duration_ms: duration_ms ?? 1 })} /></label>
          <label><span>{tr(locale, 'Color')}</span><ColorPicker value={draft.color} disabledAlpha onChangeComplete={(color) => setDraft({ ...draft, color: color.toHexString().toUpperCase() })} /></label>
        </div>
        {draft.kind === 'sequence' ? <>
          <div className="effect-editor__toolbar"><strong>{tr(locale, 'Sequence steps')}</strong><Button icon={<PlusOutlined />} onClick={() => setDraft({ ...draft, steps: [...draft.steps, defaultStep()] })}>{tr(locale, 'Add step')}</Button></div>
          <Collapse className="effect-step-list" defaultActiveKey={draft.steps.map((_, index) => String(index))} items={draft.steps.map((step, index) => ({
            key: String(index),
            label: <span className="effect-step-title"><Tag>{index + 1}</Tag><strong>{step.kind}</strong><span>{(step.at_us / 1000).toLocaleString()} ms</span></span>,
            extra: <Space.Compact onClick={(event) => event.stopPropagation()}>
              <Button size="small" icon={<ArrowUpOutlined />} disabled={index === 0} onClick={() => moveStep(index, -1)} />
              <Button size="small" icon={<ArrowDownOutlined />} disabled={index === draft.steps.length - 1} onClick={() => moveStep(index, 1)} />
              <Popconfirm title={tr(locale, 'Delete step?')} onConfirm={() => setDraft({ ...draft, steps: draft.steps.filter((_, stepIndex) => stepIndex !== index) })}><Button size="small" danger icon={<DeleteOutlined />} /></Popconfirm>
            </Space.Compact>,
            children: <div className="effect-step-grid">
              <label><span>{tr(locale, 'Time')}</span><InputNumber min={0} addonAfter="ms" value={step.at_us / 1000} onChange={(value) => updateStep(index, { at_us: Math.round((value ?? 0) * 1000) })} /></label>
              <label><span>{tr(locale, 'Command')}</span><Select value={step.kind} options={['relay','relay-mask','pwm','display','rf','beep','rgb','opcode'].map((kind) => ({ value: kind, label: kind }))} onChange={(kind) => updateStep(index, { kind })} /></label>
              <label><span>{tr(locale, 'Target')}</span><InputNumber min={0} value={step.target} onChange={(target) => updateStep(index, { target: target ?? undefined })} /></label>
              <label><span>{tr(locale, 'Value')}</span><InputNumber min={0} value={step.value} onChange={(value) => updateStep(index, { value: value ?? undefined })} /></label>
              <label><span>{tr(locale, 'Duration')}</span><InputNumber min={0} addonAfter="ms" value={step.duration_ms} onChange={(duration_ms) => updateStep(index, { duration_ms: duration_ms ?? undefined })} /></label>
              <label><span>{tr(locale, 'Frequency')}</span><InputNumber min={0} addonAfter="Hz" value={step.frequency_hz} onChange={(frequency_hz) => updateStep(index, { frequency_hz: frequency_hz ?? undefined })} /></label>
              <label className="effect-editor__wide"><span>{tr(locale, 'Text')}</span><Input value={step.text} onChange={(event) => updateStep(index, { text: event.target.value })} /></label>
              <label><span>{tr(locale, 'Destination')}</span><Input value={step.destination} onChange={(event) => updateStep(index, { destination: event.target.value })} /></label>
              <label><span>{tr(locale, 'Action IDs')}</span><Input value={(step.action_ids ?? []).join(', ')} onChange={(event) => updateStep(index, { action_ids: event.target.value.split(',').map((value) => value.trim()).filter(Boolean) })} /></label>
              <label><span>RF code</span><InputNumber min={0} value={step.code} onChange={(code) => updateStep(index, { code: code ?? undefined })} /></label>
              <label><span>RF bits</span><InputNumber min={0} max={64} value={step.bits} onChange={(bits) => updateStep(index, { bits: bits ?? undefined })} /></label>
              <label><span>RGB</span><ColorPicker disabledAlpha value={`#${[step.red ?? 0, step.green ?? 0, step.blue ?? 0].map((value) => value.toString(16).padStart(2, '0')).join('')}`} onChangeComplete={(color) => { const [red, green, blue] = color.toRgbString().match(/\d+/g)?.map(Number) ?? [0,0,0]; updateStep(index, { red, green, blue }); }} /></label>
              <label><span>{tr(locale, 'Brightness')}</span><InputNumber min={0} max={255} value={step.brightness} onChange={(brightness) => updateStep(index, { brightness: brightness ?? undefined })} /></label>
              <label><span>{tr(locale, 'Payload')}</span><Input value={step.payload_hex} onChange={(event) => updateStep(index, { payload_hex: event.target.value })} /></label>
            </div>,
          }))} />
        </> : <div className="effect-editor__program">
          <label><span>{tr(locale, 'Frames per second')}</span><InputNumber min={1} max={120} value={draft.default_fps} onChange={(default_fps) => setDraft({ ...draft, default_fps: default_fps ?? 20 })} /></label>
          <label><span>{tr(locale, 'Pixels')}</span><InputNumber min={1} value={draft.default_pixels} onChange={(default_pixels) => setDraft({ ...draft, default_pixels: default_pixels ?? 100 })} /></label>
          <label className="effect-editor__wide"><span>{tr(locale, 'Program')}</span><Input.TextArea rows={12} value={draft.programText} onChange={(event) => setDraft({ ...draft, programText: event.target.value })} /></label>
        </div>}
      </div>}
    </Modal>
  </section>;
};
