import React, { useEffect, useMemo, useRef, useState } from 'react';

// getRandomValues also works on LAN HTTP origins, unlike randomUUID.
const newAudioId = () => {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 15) | 64; bytes[8] = (bytes[8] & 63) | 128;
  const hex = Array.from(bytes, (value) => value.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0,8)}-${hex.slice(8,12)}-${hex.slice(12,16)}-${hex.slice(16,20)}-${hex.slice(20)}`;
};
import { ColorPicker } from './ColorPicker';
import { ColorDropdown } from './ColorDropdown';
import { DesignerFieldLabel } from './DesignerFieldLabel';
import {
  Button,
  Card,
  Collapse,
  ConfigProvider,
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
  SoundOutlined,
  StopOutlined,
  UnorderedListOutlined,
} from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { EffectIconPicker, effectGlyph } from '../effectIcons';
import { EffectRecorder } from './EffectRecorder';
import { GroupSelect } from './GroupSelect';
import { EffectGroupDialog, EffectGroupDraft } from './EffectGroupDialog';
import recordingColors from '../../../assets/themes/recording-colors.json';
import { appendMelodySteps, sequenceDurationMs } from '../melodyCatalog';
import { MelodySelect } from './MelodySelect';
import { SoundEffectFields } from './SoundEffectFields';
import { SequenceTimeline, SequenceTimelineActions } from './SequenceTimeline';
import { StepRepeatFields } from './StepRepeatFields';
import { TimeValueField } from './TimeValueField';
import { formatTimeMs, repeatedSequenceDurationMs, sequenceKindLabels, sequenceExtraKinds, sequenceStepTemplate, moveSequenceStep, sequenceLaneKey, sequenceLaneChoices } from '../cueAuthoring';

interface EffectsTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  locale: UiLocale;
  apiBaseUrl: string;
}

type EffectStep = {
  at_us: number;
  kind: string;
  target?: number;
  value?: number;
  duration_ms?: number;
  repeat_count?: number;
  repeat_interval_ms?: number;
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
  kind: 'sequence' | 'strip-stream' | 'audio';
  duration_ms: number;
  color: string;
  default_fps: number;
  default_pixels: number;
  steps: EffectStep[];
  repeat_count: number;
  repeat_interval_ms: number;
  properties: Record<string, unknown>;
  programText: string;
  is_new: boolean;
};

type InlineEffectEdit = {
  reference: string;
  name: string;
  icon: string;
};

const programParts = (program: unknown) => {
  if (Array.isArray(program)) return { steps: program as EffectStep[], properties: {} };
  if (program && typeof program === 'object') {
    const value = program as { steps?: EffectStep[]; properties?: Record<string, unknown> };
    return { steps: Array.isArray(value.steps) ? value.steps : [], properties: value.properties ?? {} };
  }
  return { steps: [], properties: {} };
};

// Publishing and capture share exactly the same edited effect definition.
const draftPayload = (draft: EffectDraft): Record<string, unknown> => ({
  ...draft,
  program: draft.kind === 'sequence' ? {
    steps: draft.steps.map((step) => ({ ...step, at_us: Math.max(0, Math.round(step.at_us)) })),
    repeat_count: draft.repeat_count,
    repeat_interval_ms: draft.repeat_interval_ms,
    properties: { ...draft.properties, ...(draft.color ? { color: draft.color } : {}) },
  } : JSON.parse(draft.programText || '{}'),
});

export const EffectsTab: React.FC<EffectsTabProps> = ({ state, sendCmd, locale, apiBaseUrl }) => {
  const effects = state.controller_effects ?? [];
  const [draft, setDraft] = useState<EffectDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [activeStep, setActiveStep] = useState('0');
  const previousIdentities = useRef(new Map<string, { name: string; icon: string; category: string }>());
  useEffect(() => {
    const baseline = previousIdentities.current;
    setDraft(current => {
      if (!current || current.is_new) return current;
      const next = effects.find(effect => effect.reference === current.reference);
      const previous = baseline.get(current.reference);
      if (!next || !previous) return current;
      const name = current.name === previous.name ? next.name : current.name;
      const icon = current.icon === previous.icon ? next.icon : current.icon;
      const category = current.category === previous.category ? next.category : current.category;
      return name === current.name && icon === current.icon && category === current.category ? current : { ...current, name, icon, category };
    });
    previousIdentities.current = new Map(effects.map(effect => [effect.reference, { name: effect.name, icon: effect.icon, category: effect.category }]));
  }, [effects]);
  useEffect(() => {
    if (draft?.kind !== 'audio') return;
    const saved = effects.find((effect) => effect.kind === 'audio' && effect.id === draft.id);
    if (saved && (draft.reference !== saved.reference || draft.duration_ms !== saved.duration_ms)) {
      setDraft((current) => current?.id === saved.id ? { ...current, reference: saved.reference, is_new: false, duration_ms: saved.duration_ms } : current);
    }
  }, [effects, draft?.id, draft?.reference, draft?.duration_ms, draft?.kind]);
  const updateAudio = (patch: Record<string, unknown>) => setDraft((current) => current ? { ...current, programText: JSON.stringify({ ...JSON.parse(current.programText || '{}'), ...patch }) } : current);
  const audioProgram = draft?.kind === 'audio' ? JSON.parse(draft.programText || '{}') as { source?: string; volume?: number; output_device?: string } : {};
  const [selected, setSelected] = useState<string | null>(null);
  const sequenceActions = useRef<SequenceTimelineActions | null>(null);
  const [inlineEdit, setInlineEdit] = useState<InlineEffectEdit | null>(null);
  const [groupDraft, setGroupDraft] = useState<EffectGroupDraft | null>(null);
  const [draggingEffect, setDraggingEffect] = useState<string | null>(null);
  const [dropGroup, setDropGroup] = useState<string | null>(null);
  const [movingEffect, setMovingEffect] = useState<string | null>(null);
  const captureBusy = Boolean(state.effect_recording?.active || state.effect_recording?.pending);
  const presentedEffects = useMemo(() => effects.map(effect => draft && !draft.is_new && draft.reference === effect.reference
    ? { ...effect, name: draft.name, icon: draft.icon } : effect), [effects, draft?.reference, draft?.name, draft?.icon, draft?.is_new]);
  const selectedEffect = presentedEffects.find((effect) => effect.reference === selected);
  const draggedEffect = effects.find((effect) => effect.reference === draggingEffect);
  const effectGroupName = (effect: typeof effects[number]) => effect.category || tr(locale, 'Other');
  const grouped = useMemo(() => {
    const groups = new Map<string, { name: string; icon: string; items: typeof effects }>();
    for (const group of state.controller_effect_groups ?? []) {
      groups.set(group.name, { name: group.name, icon: group.icon || 'folder', items: [] });
    }
    for (const effect of presentedEffects) {
      const name = effectGroupName(effect);
      const group = groups.get(name) ?? { name, icon: 'folder', items: [] };
      groups.set(name, { ...group, items: [...group.items, effect] });
    }
    return [...groups.values()];
  }, [presentedEffects, locale, state.controller_effect_groups]);

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
      color: String(parts.properties.color ?? ''),
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      steps: parts.steps,
      repeat_count: Number((effect.program as Record<string, unknown>)?.repeat_count ?? 1),
      repeat_interval_ms: Number((effect.program as Record<string, unknown>)?.repeat_interval_ms ?? 0),
      properties: parts.properties,
      programText: effect.kind !== 'sequence' ? JSON.stringify(effect.program ?? {}, null, 2) : '{}',
      is_new: false,
    } : {
      reference: '', id: String(Array.from({ length: 256 }, (_, id) => id).find((id) => !effects.some((effect) => effect.id === String(id))) ?? ''), name: '', icon: 'plug', category: category ?? 'Motion', description: '', kind: 'sequence',
      duration_ms: 1000, color: 'green', default_fps: 20, default_pixels: 100,
      steps: [], repeat_count: 1, repeat_interval_ms: 0, properties: { mode: 'auto', timing_tolerance_us: 0, keep_outputs_on_cancel: false },
      programText: '{}', is_new: true,
    });
  };

  const openGroupEditor = (group?: { name: string; icon: string }) => {
    setGroupDraft({
      original_name: group?.name ?? '',
      name: group?.name ?? '',
      icon: group?.icon || 'folder',
    });
  };
  const newAudio = () => {
    openEditor();
    setDraft((current) => current ? { ...current, id: newAudioId(), kind: 'audio', category: 'Audio', icon: 'speaker-high', programText: JSON.stringify({ source: '', volume: 100, output_device: '' }) } : current);
  };

  const saveGroup = async () => {
    if (!groupDraft?.name.trim()) return;
    if (draft?.kind === 'audio' && !groupDraft.original_name) {
      setDraft({ ...draft, category: groupDraft.name.trim() }); setGroupDraft(null); return;
    }
    if (await sendCmd('controller_effect.group.save', {
      original_name: groupDraft.original_name,
      name: groupDraft.name.trim(),
      icon: groupDraft.icon || 'folder',
    })) setGroupDraft(null);
  };

  const updateStep = (index: number, patch: Partial<EffectStep>) => {
    if (!draft) return;
    const steps = draft.steps.map((step, stepIndex) => {
      if (stepIndex !== index) return step;
      const next = { ...step, ...patch };
      return patch.target !== undefined || patch.value !== undefined ? moveSequenceStep(next, next.at_us, sequenceLaneKey(next)) as EffectStep : next;
    });
    setDraft({ ...draft, steps });
  };
  const revealStepProperties = (index: number) => {
    setActiveStep(String(index));
    requestAnimationFrame(() => {
      const panel = document.querySelector<HTMLElement>(`[data-effect-step-properties="${index}"]`);
      panel?.scrollIntoView({ block: 'nearest', behavior: document.documentElement.dataset.motion === 'disabled' ? 'auto' : 'smooth' });
      panel?.querySelector<HTMLInputElement>('input')?.focus({ preventScroll: true });
    });
  };
  const moveStep = (index: number, delta: number) => {
    if (!draft) return;
    const destination = index + delta;
    if (destination < 0 || destination >= draft.steps.length) return;
    const steps = [...draft.steps];
    [steps[index], steps[destination]] = [steps[destination], steps[index]];
    setDraft({ ...draft, steps });
  };
  const addMelody = (name: string) => {
    if (!draft) return;
    const melody = state.hardware_details?.melodies?.find((item) => item.name === name);
    if (!melody) return;
    const steps = appendMelodySteps(draft.steps, melody);
    setDraft({ ...draft, steps, duration_ms: sequenceDurationMs(steps) });
  };

  const save = async () => {
    if (!draft || saving) return;
    setSaving(true);
    try {
      if (!await sendCmd('controller_effect.save', draftPayload(draft))) return;
      if (draft.kind !== 'audio') setDraft(null);
    }
    catch { void message.error(tr(locale, 'Program must be valid JSON')); return; }
    finally { setSaving(false); }
  };

  const saveInlineIdentity = async (effect: typeof effects[number]) => {
    if (!inlineEdit || inlineEdit.reference !== effect.reference || !inlineEdit.name.trim()) return;
    const parts = programParts(effect.program);
    if (!await sendCmd('controller_effect.save', {
      reference: effect.reference,
      id: effect.id,
      name: inlineEdit.name.trim(),
      icon: inlineEdit.icon,
      category: effect.category,
      description: effect.description,
      kind: effect.kind,
      duration_ms: effect.duration_ms,
      ...(parts.properties.color ? { color: String(parts.properties.color) } : {}),
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      program: effect.program,
      is_new: false,
    })) return;
    setInlineEdit(null);
  };

  const moveEffectToGroup = async (effect: typeof effects[number], category: string) => {
    if (effectGroupName(effect) === category || movingEffect) return;
    const parts = programParts(effect.program);
    setMovingEffect(effect.reference);
    try {
      await sendCmd('controller_effect.save', {
        reference: effect.reference,
        id: effect.id,
        name: effect.name,
        icon: effect.icon,
        category,
        description: effect.description,
        kind: effect.kind,
        duration_ms: effect.duration_ms,
        ...(parts.properties.color ? { color: String(parts.properties.color) } : {}),
        default_fps: effect.default_fps ?? 20,
        default_pixels: effect.default_pixels ?? 100,
        program: effect.program,
        is_new: false,
      });
    } finally {
      setMovingEffect(null);
      setDraggingEffect(null);
      setDropGroup(null);
    }
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
        <Button icon={<FolderAddOutlined />} onClick={() => openGroupEditor()}>{tr(locale, 'New group')}</Button>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => openEditor()}>{tr(locale, 'New effect')}</Button>
        <Button icon={<SoundOutlined />} onClick={newAudio}>{tr(locale, 'New audio effect')}</Button>
        {Boolean(state.audio_preview_ids?.length) && <Button icon={<StopOutlined />} onClick={() => sendCmd('audio_effect.stop')}>{tr(locale, 'Stop audio preview')}</Button>}
      </Space>
    </header>

    {grouped.length === 0 ? <Card className="surface-card"><Empty description={tr(locale, 'No effects')}><Space wrap><Button type="primary" icon={<PlusOutlined />} onClick={() => openEditor()}>{tr(locale, 'Create effect')}</Button><Button icon={<FolderAddOutlined />} onClick={() => openGroupEditor()}>{tr(locale, 'New group')}</Button></Space></Empty></Card> :
      <Collapse className="effect-groups" defaultActiveKey={grouped.filter((group) => group.items.length > 0).map((group) => group.name)} items={grouped.map((group) => ({
        key: group.name,
        className: `effect-group ${group.items.length === 0 ? 'is-empty' : ''}`,
        showArrow: group.items.length > 0,
        collapsible: group.items.length === 0 ? 'icon' as const : undefined,
        label: <span
          className={`effect-group-title ${dropGroup === group.name ? 'is-drop-target' : ''}`}
          onDragEnter={(event) => {
            if (!draggedEffect || effectGroupName(draggedEffect) === group.name) return;
            event.preventDefault();
            setDropGroup(group.name);
          }}
          onDragOver={(event) => {
            if (!draggedEffect || effectGroupName(draggedEffect) === group.name) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = 'move';
          }}
          onDragLeave={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropGroup((current) => current === group.name ? null : current);
          }}
          onDrop={(event) => {
            const reference = event.dataTransfer.getData('application/x-pealayer-effect') || draggingEffect;
            const effect = effects.find((candidate) => candidate.reference === reference);
            if (!effect || effectGroupName(effect) === group.name) return;
            event.preventDefault();
            event.stopPropagation();
            void moveEffectToGroup(effect, group.name);
          }}
        ><span className="effect-group-icon">{effectGlyph(group.icon || 'folder')}</span><strong>{group.name}</strong><span className="effect-group-count">{group.items.length}</span></span>,
        extra: <Space.Compact className="effect-group-actions">
          <Tooltip title={tr(locale, 'Manage effect group')}><Button type="text" size="small" icon={<EditOutlined />} aria-label={tr(locale, 'Manage effect group')} onClick={(event) => { event.stopPropagation(); openGroupEditor(group); }} /></Tooltip>
          <Tooltip title={tr(locale, 'New effect in this group')}><Button type="text" size="small" icon={<PlusOutlined />} aria-label={tr(locale, 'New effect in this group')} onClick={(event) => { event.stopPropagation(); openEditor(undefined, group.name); }} /></Tooltip>
        </Space.Compact>,
        children: <div className="effect-card-grid">{group.items.map((effect) => {
          const actions = [
            { key: 'rename', label: tr(locale, 'Rename'), icon: <EditOutlined /> },
            { key: 'play', label: tr(locale, 'Play effect'), icon: <PlayCircleOutlined /> },
            { key: 'manage', label: tr(locale, 'Manage'), icon: <EditOutlined /> },
            { key: 'cue', label: tr(locale, 'Add cue at playhead'), icon: <PlusOutlined /> },
            { type: 'divider' as const },
            { key: 'delete', label: tr(locale, 'Delete'), icon: <DeleteOutlined />, danger: true },
          ];
          const run = (key: string) => {
            if (key === 'rename') beginInlineEdit();
            if (key === 'play') sendCmd(effect.kind === 'audio' && state.audio_preview_ids?.includes(effect.id) ? 'audio_effect.stop' : 'controller_effect.play', { reference: effect.reference });
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
          return <Dropdown key={effect.reference} trigger={['contextMenu']} menu={{ items: actions, onClick: ({ key }) => run(key) }}><article
            className={`effect-card ${selected === effect.reference ? 'is-selected' : ''} ${editing ? 'is-editing' : ''} ${draggingEffect === effect.reference ? 'is-dragging' : ''} ${movingEffect === effect.reference ? 'is-moving' : ''}`}
            draggable={!editing && movingEffect === null}
            aria-grabbed={draggingEffect === effect.reference}
            onDragStart={(event) => {
              event.dataTransfer.effectAllowed = 'move';
              event.dataTransfer.setData('application/x-pealayer-effect', effect.reference);
              setDraggingEffect(effect.reference);
            }}
            onDragEnd={() => { setDraggingEffect(null); setDropGroup(null); }}
            onClick={() => setSelected(effect.reference)}
          >
            {editing ? <>
              <EffectIconPicker compact locale={locale}
                className="effect-card__inline-icon"
                value={inlineEdit.icon}
                searchPlaceholder={tr(locale, 'Search icons...')} presetsLabel={tr(locale, 'Presets')} emptyLabel={tr(locale, 'No matching icons')}
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
              <Tooltip title={tr(locale, 'Rename')}><Button type="text" icon={<EditOutlined />} aria-label={tr(locale, 'Rename')} onClick={(event) => { event.stopPropagation(); beginInlineEdit(); }} /></Tooltip>
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
          </article></Dropdown>;
        })}</div>,
      }))} />}

    {selectedEffect && <div className="selection-bar"><span>{selectedEffect.name}</span><Button icon={<EditOutlined />} onClick={() => openEditor(selectedEffect)}>{tr(locale, 'Manage')}</Button></div>}

    <Dropdown trigger={['contextMenu']} menu={{ items: [
      { key: 'effect', icon: <PlusOutlined />, label: tr(locale, 'New effect') },
      { key: 'group', icon: <FolderAddOutlined />, label: tr(locale, 'New group') },
    ], onClick: ({ key }) => { if (key === 'group') openGroupEditor(); else openEditor(); } }}>
      <div className="effects-library-empty-space" style={{ minHeight: 140, flex: 1 }} aria-label={tr(locale, 'Effects Library')} />
    </Dropdown>
    <EffectGroupDialog draft={groupDraft} setDraft={setGroupDraft} save={saveGroup} locale={locale} />

    <Modal
      className="effect-editor-modal"
      title={tr(locale, 'Effects Designer')}
      open={Boolean(draft)}
      onCancel={() => { if (!captureBusy && !saving) setDraft(null); }}
      closable={!captureBusy && !saving}
      maskClosable={!captureBusy && !saving}
      keyboard={!captureBusy && !saving}
      cancelButtonProps={{ disabled: captureBusy || saving }}
      confirmLoading={saving}
      onOk={save}
      okText={tr(locale, 'Save')}
      okButtonProps={{ icon: <SaveOutlined />, disabled: captureBusy || saving || !draft?.name.trim() }}
      width={900}
    >
      {draft && <div className="effect-editor">
        <ConfigProvider componentDisabled={captureBusy}><div className="effect-editor__identity">
          <label><DesignerFieldLabel label="Type" locale={locale} /><Select disabled={!draft.is_new} value={draft.kind} options={[{ value: 'sequence', label: tr(locale, 'Sequence') }, { value: 'strip-stream', label: tr(locale, 'Lighting') }, { value: 'audio', label: tr(locale, 'Audio effect') }]} onChange={(kind) => setDraft({ ...draft, kind, ...(kind === 'audio' ? { id: newAudioId(), icon: 'speaker-high', category: 'Audio', programText: JSON.stringify({ source: '', volume: 100, output_device: '' }) } : {}) })} /></label>
          {draft.kind !== 'audio' && <label><DesignerFieldLabel label="ID" locale={locale} /><Input disabled={!draft.is_new} value={draft.id} onChange={(event) => setDraft({ ...draft, id: event.target.value })} /></label>}
          <label><DesignerFieldLabel label="Name" locale={locale} /><Input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></label>
          <label><DesignerFieldLabel label="Icon" locale={locale} /><EffectIconPicker locale={locale} value={draft.icon} searchPlaceholder={tr(locale, 'Search icons...')} presetsLabel={tr(locale, 'Presets')} emptyLabel={tr(locale, 'No matching icons')} onChange={(icon) => setDraft({ ...draft, icon })} /></label>
          <label><DesignerFieldLabel label="Group" locale={locale} /><GroupSelect value={draft.category} groups={[...new Set([...(state.controller_effect_groups ?? []).map((group) => group.name), ...effects.map((effect) => effect.category)])]} locale={locale} onChange={(category) => setDraft({ ...draft, category })} onCreate={() => openGroupEditor()} /></label>
          <label className="effect-editor__wide"><DesignerFieldLabel label="Description" locale={locale} /><Input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} /></label>
          <label><DesignerFieldLabel label="Length" locale={locale} />{draft.kind === 'audio' ? <Typography.Text type="secondary">{draft.is_new ? tr(locale, 'Read from audio on Save') : formatTimeMs(draft.duration_ms, state.human_readable_time_units !== false)}</Typography.Text> : draft.kind === 'sequence' ? <Typography.Text type="secondary">{repeatedSequenceDurationMs(draft.steps, draft.repeat_count, draft.repeat_interval_ms) === null ? '—' : formatTimeMs(repeatedSequenceDurationMs(draft.steps, draft.repeat_count, draft.repeat_interval_ms)!, state.human_readable_time_units !== false)}</Typography.Text> : <TimeValueField min={1} value={draft.duration_ms} human={state.human_readable_time_units !== false} onChange={duration_ms => setDraft({ ...draft, duration_ms: Math.round(duration_ms) })} />}</label>
          {draft.kind === 'sequence' && <>
            <label><DesignerFieldLabel label="Repeat effect" locale={locale} /><InputNumber min={1} max={1000} value={draft.repeat_count} onChange={value => setDraft({ ...draft, repeat_count: value ?? 1 })} /></label>
            <label><DesignerFieldLabel label="Interval" locale={locale} /><TimeValueField value={draft.repeat_interval_ms} human={state.human_readable_time_units !== false} onChange={value => setDraft({ ...draft, repeat_interval_ms: Math.round(value) })} />
              <Typography.Text type="secondary">{tr(locale, '0s uses the full effect length; interval is start-to-start')}</Typography.Text></label>
            <Typography.Text type={repeatedSequenceDurationMs(draft.steps, draft.repeat_count, draft.repeat_interval_ms) === null ? 'danger' : 'secondary'}>
              {repeatedSequenceDurationMs(draft.steps, draft.repeat_count, draft.repeat_interval_ms) === null ? tr(locale, 'Effect repeat interval cannot be shorter than its length') : formatTimeMs(repeatedSequenceDurationMs(draft.steps, draft.repeat_count, draft.repeat_interval_ms)!, state.human_readable_time_units !== false)}
            </Typography.Text>
            <Button disabled={captureBusy || saving || !state.controller_connected || !state.hardware_connected || Boolean(state.estop_active) || !draft.name.trim() || !draft.steps.length || repeatedSequenceDurationMs(draft.steps,draft.repeat_count,draft.repeat_interval_ms) === null} icon={<PlayCircleOutlined />} onClick={() => { void sendCmd('controller_effect.publish_and_run', draftPayload(draft)); }}>{tr(locale, 'Publish & Run')}</Button>
          </>}
          {draft.kind !== 'audio' && <label><DesignerFieldLabel label="Color" locale={locale} /><ColorDropdown disabled={captureBusy} locale={locale}
            value={draft.color.startsWith('#') ? 'custom' : draft.color}
            customHex={draft.color.startsWith('#') ? draft.color : (recordingColors.find(color => color.id === draft.color)?.hex ?? '#38D27A')}
            options={recordingColors.map(color => ({ value: color.id, label: tr(locale, color.label), color: color.hex }))}
            onChange={(color, hex) => setDraft({ ...draft, color: color === 'custom' ? hex! : color })} /></label>}
        </div></ConfigProvider>
        {draft.kind === 'audio' ? <SoundEffectFields program={audioProgram} onChange={updateAudio} state={state} reference={draft.reference} apiBaseUrl={apiBaseUrl} locale={locale} sendCmd={sendCmd} /> : draft.kind === 'sequence' ? <>
          <div className="effect-editor__toolbar"><strong>{tr(locale, 'Sequence steps')}</strong><Space wrap>
            <EffectRecorder state={state} sendCmd={sendCmd} locale={locale} effect={draftPayload(draft)} onSequenceChange={(steps, id) => setDraft((current) => current ? { ...current, id: String(id), reference: `effect:${id}`, is_new: false, steps } : current)} />
            <Button disabled={captureBusy || (!draft.steps.length && !sequenceLaneChoices(state.hardware_details?.controls ?? [],sequenceExtraKinds(state.hardware_details)).length)} icon={<PlusOutlined />} onClick={() => sequenceActions.current?.addStep()}>{tr(locale, 'Add step')}</Button>
            <MelodySelect
              className="effect-melody-picker"
              disabled={captureBusy || !state.controller_connected}
              placeholder={<><SoundOutlined /> {tr(locale, 'Add melody')}</>}
              value={undefined}
              melodies={state.hardware_details?.melodies ?? []}
              locale={locale}
              refresh={() => { void sendCmd('hardware.catalog.refresh'); }}
              onChange={addMelody}
            />
            <Popconfirm title={tr(locale, 'Delete all sequence steps?')} onConfirm={() => setDraft({ ...draft, steps: [] })}><Button disabled={captureBusy || draft.steps.length === 0} icon={<DeleteOutlined />}>{tr(locale, 'Clear steps')}</Button></Popconfirm>
          </Space></div>
          <ConfigProvider componentDisabled={captureBusy}>
          <SequenceTimeline key={draft.id} steps={draft.steps} locale={locale} controls={state.hardware_details?.controls} disabled={captureBusy}
            human={state.human_readable_time_units !== false} actionsRef={sequenceActions} extraKinds={sequenceExtraKinds(state.hardware_details)} onSelect={index => setActiveStep(String(index))} onEdit={revealStepProperties} onChange={steps => setDraft({ ...draft, steps })} />
          <Collapse className="effect-step-list" activeKey={[activeStep]} onChange={keys => setActiveStep(String(Array.isArray(keys) ? keys[0] ?? '' : keys))} items={draft.steps.map((step, index) => ({
            key: String(index),
            label: <span className="effect-step-title"><Tag>{index + 1}</Tag><strong>{tr(locale, sequenceKindLabels[step.kind] ?? step.kind)}</strong><span>{formatTimeMs(step.at_us / 1000, state.human_readable_time_units !== false)}</span></span>,
            extra: <Space.Compact onClick={(event) => event.stopPropagation()}>
              <Button size="small" icon={<ArrowUpOutlined />} disabled={index === 0} onClick={() => moveStep(index, -1)} />
              <Button size="small" icon={<ArrowDownOutlined />} disabled={index === draft.steps.length - 1} onClick={() => moveStep(index, 1)} />
              <Popconfirm title={tr(locale, 'Delete step?')} onConfirm={() => setDraft({ ...draft, steps: draft.steps.filter((_, stepIndex) => stepIndex !== index) })}><Button size="small" danger icon={<DeleteOutlined />} /></Popconfirm>
            </Space.Compact>,
            children: <div className="effect-step-grid" data-effect-step-properties={index}>
              <StepRepeatFields step={step} locale={locale} human={state.human_readable_time_units !== false} onChange={patch => updateStep(index, patch)} />
              <label><DesignerFieldLabel label="Start" locale={locale} /><TimeValueField value={step.at_us / 1000} human={state.human_readable_time_units !== false} onChange={value => updateStep(index, { at_us: Math.round(value * 1000) })} /></label>
              <label><DesignerFieldLabel label="Command" locale={locale} /><Select value={step.kind} options={Object.entries(sequenceKindLabels).map(([kind, label]) => ({ value: kind, label: tr(locale, label) }))} onChange={kind => setDraft({ ...draft, steps: draft.steps.map((source, item) => item === index ? { ...sequenceStepTemplate(kind), at_us: source.at_us, repeat_count: source.repeat_count, repeat_interval_ms: source.repeat_interval_ms } as EffectStep : source) })} /></label>
              {['motion','relay','pwm','addressable','menu','menu-action'].includes(step.kind) && <label><DesignerFieldLabel label="Target" locale={locale} /><InputNumber min={0} value={step.target} onChange={(target) => updateStep(index, { target: target ?? undefined })} /></label>}
              {['motion','relay','relay-mask','pwm'].includes(step.kind) && <label><DesignerFieldLabel label="Value" locale={locale} /><InputNumber min={0} value={step.value} onChange={(value) => updateStep(index, { value: value ?? undefined })} /></label>}
              {['motion','relay','pwm','beep','display','rgb','addressable'].includes(step.kind) && <label><DesignerFieldLabel label="Length" locale={locale} /><TimeValueField max={65535} value={step.duration_ms ?? 0} human={state.human_readable_time_units !== false} onChange={value => updateStep(index, { duration_ms: Math.round(value) })} /></label>}
              {['motion','relay','pwm','beep','display','rgb','addressable'].includes(step.kind) && <label><DesignerFieldLabel label="Finish" locale={locale} /><TimeValueField min={step.at_us / 1000} max={step.at_us / 1000 + 65535} value={step.at_us / 1000 + (step.duration_ms ?? 0)} human={state.human_readable_time_units !== false} onChange={value => updateStep(index, { duration_ms: Math.round(value - step.at_us / 1000) })} /></label>}
              {step.kind === 'beep' && <label><DesignerFieldLabel label="Frequency" locale={locale} /><InputNumber min={0} addonAfter="Hz" value={step.frequency_hz} onChange={(frequency_hz) => updateStep(index, { frequency_hz: frequency_hz ?? undefined })} /></label>}
              {step.kind === 'display' && <label className="effect-editor__wide"><DesignerFieldLabel label="Text" locale={locale} /><Input value={step.text} onChange={(event) => updateStep(index, { text: event.target.value })} /></label>}
              {step.kind === 'display' && <label><DesignerFieldLabel label="Destination" locale={locale} /><Input value={step.destination} onChange={(event) => updateStep(index, { destination: event.target.value })} /></label>}
              <label><DesignerFieldLabel label="Action IDs" locale={locale} /><Input value={(step.action_ids ?? []).join(', ')} onChange={(event) => updateStep(index, { action_ids: event.target.value.split(',').map((value) => value.trim()).filter(Boolean) })} /></label>
              {step.kind === 'rf' && <label><DesignerFieldLabel label="RF code" locale={locale} /><InputNumber min={0} value={step.code} onChange={(code) => updateStep(index, { code: code ?? undefined })} /></label>}
              {step.kind === 'rf' && <label><DesignerFieldLabel label="RF bits" locale={locale} /><InputNumber min={0} max={64} value={step.bits} onChange={(bits) => updateStep(index, { bits: bits ?? undefined })} /></label>}
              {['rgb','addressable'].includes(step.kind) && <label><DesignerFieldLabel label="RGB" locale={locale} /><ColorPicker locale={locale} disabledAlpha value={`#${[step.red ?? 0, step.green ?? 0, step.blue ?? 0].map((value) => value.toString(16).padStart(2, '0')).join('')}`} onChangeComplete={(color) => { const [red, green, blue] = color.toRgbString().match(/\d+/g)?.map(Number) ?? [0,0,0]; updateStep(index, { red, green, blue }); }} /></label>}
              {['rgb','addressable'].includes(step.kind) && <label><DesignerFieldLabel label="Brightness" locale={locale} /><InputNumber min={0} max={255} value={step.brightness} onChange={(brightness) => updateStep(index, { brightness: brightness ?? undefined })} /></label>}
              {step.kind === 'opcode' && <label><DesignerFieldLabel label="Payload" locale={locale} /><Input value={step.payload_hex} onChange={(event) => updateStep(index, { payload_hex: event.target.value })} /></label>}
            </div>,
          }))} /></ConfigProvider>
        </> : <div className="effect-editor__program">
          <label><DesignerFieldLabel label="Frames per second" locale={locale} /><InputNumber min={1} max={120} value={draft.default_fps} onChange={(default_fps) => setDraft({ ...draft, default_fps: default_fps ?? 20 })} /></label>
          <label><DesignerFieldLabel label="Pixels" locale={locale} /><InputNumber min={1} value={draft.default_pixels} onChange={(default_pixels) => setDraft({ ...draft, default_pixels: default_pixels ?? 100 })} /></label>
          <Collapse items={[{ key: 'technical', label: tr(locale, 'Technical'), children: <label className="effect-editor__wide"><DesignerFieldLabel label="Program" locale={locale} /><Input.TextArea rows={12} value={draft.programText} onChange={(event) => setDraft({ ...draft, programText: event.target.value })} /></label> }]} />
        </div>}
      </div>}
    </Modal>
  </section>;
};
