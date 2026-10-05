import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  AppstoreOutlined,
  ArrowDownOutlined,
  ArrowUpOutlined,
  BookOutlined,
  CaretRightFilled,
  ClockCircleOutlined,
  DeleteOutlined,
  DesktopOutlined,
  EditOutlined,
  FastBackwardOutlined,
  FastForwardOutlined,
  PauseOutlined,
  PlusOutlined,
  RadarChartOutlined,
  SaveOutlined,
  SettingOutlined,
  SoundOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { Button, Divider, Dropdown, Empty, Input, InputNumber, message, Modal, Popconfirm, Select, Slider, Space, Tooltip } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { effectGlyph as configuredEffectGlyph, effectIconOptions } from '../effectIcons';
import { EffectRecorder } from './EffectRecorder';
import { mediaBasename } from '../mediaLabel';
import { SeekThumbnailPreview } from './SeekThumbnailPreview';

interface StudioTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
  appName: string;
  quickSeekSeconds: number;
  apiBaseUrl: string;
  seekbarHoverThumbnails: boolean;
  surface?: 'studio' | 'timeline';
}

const formatTime = (seconds = 0, showMilliseconds = true) => {
  if (!Number.isFinite(seconds) || seconds < 0) return '--:--';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const wholeSeconds = Math.floor(seconds % 60);
  const milliseconds = Math.floor((seconds % 1) * 1000);
  const prefix = hours > 0 ? `${String(hours).padStart(2, '0')}:` : '';
  return `${prefix}${String(minutes).padStart(2, '0')}:${String(wholeSeconds).padStart(2, '0')}${showMilliseconds ? `.${String(milliseconds).padStart(3, '0')}` : ''}`;
};

const effectGlyph = (target: string) => {
  if (target.startsWith('relay:')) return <RadarChartOutlined />;
  if (target === 'controller') return <AppstoreOutlined />;
  return <ClockCircleOutlined />;
};

const workspaceIconOptions = [
  { value: 'monitor', label: 'Monitor' },
  { value: 'timeline', label: 'Timeline' },
  { value: 'tabs', label: 'Tabs' },
  { value: 'window', label: 'Window' },
  { value: 'video', label: 'Video' },
  { value: 'hardware', label: 'Hardware' },
  { value: 'effects', label: 'Effects' },
  { value: 'layout', label: 'Layout' },
];

const workspaceGlyph = (icon?: string) => {
  switch (icon) {
    case 'timeline': return <EditOutlined />;
    case 'hardware': return <RadarChartOutlined />;
    case 'effects': return <AppstoreOutlined />;
    case 'tabs': return <AppstoreOutlined />;
    default: return <DesktopOutlined />;
  }
};

export const StudioTab: React.FC<StudioTabProps> = ({ state, sendCmd, locale, appName, quickSeekSeconds, apiBaseUrl, seekbarHoverThumbnails, surface = 'studio' }) => {
  const [selectedEffect, setSelectedEffect] = useState<string | null>(null);
  const [effectEditorOpen, setEffectEditorOpen] = useState(false);
  const [effectDraft, setEffectDraft] = useState<Record<string, any> | null>(null);
  const [seekDraft, setSeekDraft] = useState<number | null>(null);
  const [workspaceManagerOpen, setWorkspaceManagerOpen] = useState(false);
  const [workspaceName, setWorkspaceName] = useState('');
  const [workspaceIcon, setWorkspaceIcon] = useState('window');
  const [cuePreviews, setCuePreviews] = useState<Record<string, { start_time_ms: number; duration_ms: number }>>({});
  const [activeCueDrag, setActiveCueDrag] = useState<null | {
    id: string;
    mode: 'move' | 'resize-left' | 'resize-right';
    originX: number;
    laneWidth: number;
    startTimeMs: number;
    durationMs: number;
    moved: boolean;
  }>(null);
  const suppressCueClick = useRef<string | null>(null);
  const effects = state.effects ?? [];
  const controllerEffects = state.controller_effects ?? [];
  const cues = state.cues ?? [];
  const chapters = state.chapters ?? [];
  const currentSeconds = state.playback_time ?? 0;
  const durationSeconds = state.duration ?? 0;
  const timelineDurationMs = useMemo(() => {
    const cueEnd = cues.reduce((maximum, cue) => Math.max(maximum, cue.start_time_ms + cue.duration_ms), 0);
    return Math.max(durationSeconds * 1000, cueEnd, 1000);
  }, [cues, durationSeconds]);
  const selected = selectedEffect ? controllerEffects.find((effect) => effect.reference === selectedEffect) : undefined;
  const mediaName = state.current_video
    ? mediaBasename(state.current_video, tr(locale, 'Untitled'))
    : tr(locale, 'No Media Playing');
  const seekPercent = durationSeconds > 0 ? (currentSeconds / durationSeconds) * 100 : 0;
  const activeSeek = seekDraft ?? seekPercent;
  const timelineLanes = useMemo(() => {
    const order = ['motion', 'relay', 'pwm', 'lighting', 'display', 'rf', 'audio', 'sequence', 'composite'];
    const active = new Set(effects.map((effect) => effect.lane || 'sequence'));
    return order.filter((lane) => active.has(lane));
  }, [effects]);
  const workspaceProfiles = useMemo(
    () => [...(state.workspace_profiles ?? [])].sort((left, right) => left.order - right.order || left.name.localeCompare(right.name)),
    [state.workspace_profiles],
  );
  const activeWorkspace = workspaceProfiles.find((profile) => profile.id === state.active_workspace_profile);

  useEffect(() => {
    try {
      const stored = window.localStorage.getItem('pealayer.effect-working-draft');
      if (stored) setEffectDraft(JSON.parse(stored));
    } catch {
      window.localStorage.removeItem('pealayer.effect-working-draft');
    }
  }, []);

  useEffect(() => {
    if (effectDraft) {
      window.localStorage.setItem('pealayer.effect-working-draft', JSON.stringify(effectDraft));
    } else {
      window.localStorage.removeItem('pealayer.effect-working-draft');
    }
  }, [effectDraft]);

  useEffect(() => {
    if (!activeCueDrag) return undefined;
    const onPointerMove = (event: PointerEvent) => {
      const deltaMs = Math.round(((event.clientX - activeCueDrag.originX) / Math.max(1, activeCueDrag.laneWidth)) * timelineDurationMs);
      let startTimeMs = activeCueDrag.startTimeMs;
      let durationMs = activeCueDrag.durationMs;
      if (activeCueDrag.mode === 'move') {
        startTimeMs = Math.max(0, activeCueDrag.startTimeMs + deltaMs);
      } else if (activeCueDrag.mode === 'resize-left') {
        const requestedStart = Math.max(0, activeCueDrag.startTimeMs + deltaMs);
        const fixedEnd = activeCueDrag.startTimeMs + activeCueDrag.durationMs;
        startTimeMs = Math.min(fixedEnd - 1, requestedStart);
        durationMs = Math.max(1, fixedEnd - startTimeMs);
      } else {
        durationMs = Math.max(1, activeCueDrag.durationMs + deltaMs);
      }
      setCuePreviews((current) => ({ ...current, [activeCueDrag.id]: { start_time_ms: startTimeMs, duration_ms: durationMs } }));
      if (Math.abs(event.clientX - activeCueDrag.originX) > 2 && !activeCueDrag.moved) {
        setActiveCueDrag((current) => current ? { ...current, moved: true } : current);
      }
    };
    const onPointerUp = () => {
      const preview = cuePreviews[activeCueDrag.id] ?? {
        start_time_ms: activeCueDrag.startTimeMs,
        duration_ms: activeCueDrag.durationMs,
      };
      if (activeCueDrag.moved) {
        suppressCueClick.current = activeCueDrag.id;
        sendCmd('effect_cue.update', {
          instance_id: activeCueDrag.id,
          start_time_ms: preview.start_time_ms,
          duration_ms: preview.duration_ms,
        });
      }
      setActiveCueDrag(null);
    };
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp, { once: true });
    return () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
    };
  }, [activeCueDrag, cuePreviews, sendCmd, timelineDurationMs]);

  const addCue = (reference: string) => {
    sendCmd('controller_effect_cue.add', {
      reference,
      start_time_ms: Math.max(0, Math.round(currentSeconds * 1000)),
    });
  };

  const editEffect = (effect?: typeof controllerEffects[number]) => {
    const program = effect?.program && typeof effect.program === 'object'
      ? effect.program as Record<string, any>
      : {};
    setEffectDraft(effect ? {
      ...effect,
      programText: JSON.stringify(program, null, 2),
      steps: Array.isArray(program.steps) ? program.steps : [],
      color: program.properties?.color ?? 'green',
      engine: program.properties?.mode ?? 'auto',
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      is_new: false,
    } : {
      reference: '', id: '', name: '', icon: 'sparkle', category: 'Effects', description: '',
      kind: 'sequence', programText: '{}', steps: [], engine: 'auto',
      color: 'violet', default_fps: 20, duration_ms: 1, default_pixels: 100,
      is_new: true,
    });
    setEffectEditorOpen(true);
  };

  const addSequenceStep = () => {
    if (!effectDraft) return;
    const previous = effectDraft.steps?.[effectDraft.steps.length - 1];
    setEffectDraft({
      ...effectDraft,
      steps: [...(effectDraft.steps ?? []), {
        at_us: previous ? Number(previous.at_us ?? 0) + 250_000 : 0,
        kind: 'relay', target: 4, value: 1,
      }],
    });
  };

  return (
    <div className={`studio-suite studio-suite--${surface}`}>
      <section className="studio-panel effects-panel">
        <header className="studio-panel__header">
          <div>
            <span className="eyebrow">{tr(locale, 'Live project')}</span>
            <h2>{tr(locale, 'Effects')}</h2>
          </div>
          <div className="effects-heading-actions">
            <Button type="text" icon={<PlusOutlined />} onClick={() => editEffect()} aria-label={tr(locale, 'New effect')} />
            <span className="count-badge">{controllerEffects.length}</span>
          </div>
        </header>

        <EffectRecorder state={state} sendCmd={sendCmd} locale={locale} compact />

        <div className="effect-list">
          {controllerEffects.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={tr(locale, 'No effects')}
            />
          ) : controllerEffects.map((effect) => {
            const selectedCard = selectedEffect === effect.reference;
            return (
              <article
                key={effect.reference}
                className={`effect-profile ${selectedCard ? 'is-selected' : ''}`}
                onClick={() => setSelectedEffect(effect.reference)}
                draggable
                onDragStart={(event) => {
                  event.dataTransfer.effectAllowed = 'copy';
                  event.dataTransfer.setData('application/x-pealayer-effect', effect.reference);
                  event.dataTransfer.setData('text/plain', effect.name);
                }}
              >
                <span className="effect-profile__icon">{configuredEffectGlyph(effect.icon)}</span>
                <div className="effect-profile__body">
                  <strong>{effect.name}</strong>
                  <span>{effect.category} · {effect.action_count} {tr(locale, 'actions')} · {effect.duration_display}</span>
                </div>
                <Tooltip title={tr(locale, 'Play effect')}>
                  <Button
                    type="text"
                    icon={<CaretRightFilled />}
                    onClick={(event) => {
                      event.stopPropagation();
                      sendCmd('controller_effect.play', { reference: effect.reference });
                    }}
                  />
                </Tooltip>
                <Tooltip title={tr(locale, 'Add cue at playhead')}>
                  <Button
                    type="text"
                    icon={<PlusOutlined />}
                    onClick={(event) => {
                      event.stopPropagation();
                      addCue(effect.reference);
                    }}
                  />
                </Tooltip>
              </article>
            );
          })}
        </div>

        <footer className="effects-panel__footer">
          <span className={`status-light ${state.hardware_connected ? 'is-online' : ''}`} />
          <span>{state.hardware?.board_name || tr(locale, 'No hardware')}</span>
          {selected && (
            <span className="effect-selection-actions">
              {selected.editable && <Button type="text" size="small" icon={<EditOutlined />} onClick={() => editEffect(selected)} />}
              <Popconfirm
                title={tr(locale, 'Delete effect?')}
                onConfirm={() => sendCmd('controller_effect.delete', { reference: selected.reference })}
              >
                <Button type="text" danger size="small" icon={<DeleteOutlined />} />
              </Popconfirm>
              <strong>{selected.name}</strong>
            </span>
          )}
        </footer>
      </section>

      <Modal
        title={effectDraft?.is_new ? tr(locale, 'New effect') : tr(locale, 'Effect properties')}
        open={effectEditorOpen}
        onCancel={() => setEffectEditorOpen(false)}
        okText={tr(locale, 'Save')}
        width={640}
        onOk={() => {
          if (!effectDraft) return;
          let program = {};
          if (effectDraft.kind === 'sequence') {
            program = {
              steps: effectDraft.steps ?? [],
              properties: { mode: effectDraft.engine ?? 'auto', color: effectDraft.color ?? 'violet' },
            };
          } else {
            try {
              program = JSON.parse(effectDraft.programText || '{}');
            } catch {
              void message.error(tr(locale, 'Program must be valid JSON'));
              return;
            }
          }
          const { programText, steps, engine, ...payload } = effectDraft;
          sendCmd('controller_effect.save', { ...payload, program });
          window.localStorage.removeItem('pealayer.effect-working-draft');
          setEffectEditorOpen(false);
        }}
      >
        {effectDraft && (
          <div className="effect-editor-grid">
            <label><span>{tr(locale, 'Type')}</span><Select value={effectDraft.kind} options={[{ value: 'sequence', label: tr(locale, 'Timed sequence') }, { value: 'strip-stream', label: tr(locale, 'Addressable lighting') }]} onChange={(kind) => setEffectDraft({ ...effectDraft, kind })} /></label>
            <label><span>{tr(locale, 'ID')}</span><Input value={effectDraft.id} onChange={(event) => setEffectDraft({ ...effectDraft, id: event.target.value })} /></label>
            <label><span>{tr(locale, 'Name')}</span><Input value={effectDraft.name} onChange={(event) => setEffectDraft({ ...effectDraft, name: event.target.value })} /></label>
            <label><span>{tr(locale, 'Icon')}</span><Select showSearch optionFilterProp="value" value={effectDraft.icon} options={effectIconOptions} onChange={(icon) => setEffectDraft({ ...effectDraft, icon })} /></label>
            <label><span>{tr(locale, 'Category')}</span><Input value={effectDraft.category} onChange={(event) => setEffectDraft({ ...effectDraft, category: event.target.value })} /></label>
            <label className="effect-editor-grid__wide"><span>{tr(locale, 'Description')}</span><Input value={effectDraft.description} onChange={(event) => setEffectDraft({ ...effectDraft, description: event.target.value })} /></label>
            <label><span>{tr(locale, 'Duration (ms)')}</span><InputNumber min={1} value={effectDraft.duration_ms} onChange={(duration_ms) => setEffectDraft({ ...effectDraft, duration_ms: duration_ms ?? 1 })} /></label>
            {effectDraft.kind === 'strip-stream' && <label><span>{tr(locale, 'Frames per second')}</span><InputNumber min={1} max={120} value={effectDraft.default_fps} onChange={(default_fps) => setEffectDraft({ ...effectDraft, default_fps: default_fps ?? 20 })} /></label>}
            {effectDraft.kind === 'strip-stream' && <label><span>{tr(locale, 'Pixels')}</span><InputNumber min={1} value={effectDraft.default_pixels} onChange={(default_pixels) => setEffectDraft({ ...effectDraft, default_pixels: default_pixels ?? 100 })} /></label>}
            {effectDraft.kind === 'sequence' && <label><span>{tr(locale, 'Color')}</span><Input value={effectDraft.color} onChange={(event) => setEffectDraft({ ...effectDraft, color: event.target.value })} /></label>}
            {effectDraft.kind === 'sequence' && <label><span>{tr(locale, 'Execution')}</span><Select value={effectDraft.engine ?? 'auto'} options={[
              { value: 'auto', label: tr(locale, 'Automatic (recommended)') },
              { value: 'host', label: tr(locale, 'Host clock') },
              { value: 'mcu', label: tr(locale, 'Device clock') },
            ]} onChange={(engine) => setEffectDraft({ ...effectDraft, engine })} /></label>}
            {effectDraft.kind === 'strip-stream' && <label className="effect-editor-grid__wide"><span>{tr(locale, 'Program')}</span><Input.TextArea autoSize={{ minRows: 7, maxRows: 16 }} value={effectDraft.programText} onChange={(event) => setEffectDraft({ ...effectDraft, programText: event.target.value })} /></label>}
            {effectDraft.kind === 'sequence' && (
              <div className="effect-editor-grid__wide sequence-editor-web">
                <Divider titlePlacement="start" plain>{tr(locale, 'Sequence steps')}</Divider>
                <div className="sequence-editor-web__toolbar">
                  <Button icon={<PlusOutlined />} onClick={addSequenceStep}>{tr(locale, 'Add step')}</Button>
                  <span>{(effectDraft.steps ?? []).length} {tr(locale, 'actions')}</span>
                </div>
                <div className="sequence-step-list">
                  {(effectDraft.steps ?? []).length === 0 ? (
                    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={tr(locale, 'No actions yet')} />
                  ) : (effectDraft.steps ?? []).map((step: Record<string, any>, index: number) => (
                    <div className="sequence-step-row" key={`${index}:${step.at_us}:${step.kind}`}>
                      <span className="sequence-step-row__index">{index + 1}</span>
                      <label><span>{tr(locale, 'Time')}</span><InputNumber min={0} step={10} addonAfter="ms" value={Math.round(Number(step.at_us ?? 0) / 1000)} onChange={(value) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, at_us: Math.max(0, Number(value ?? 0)) * 1000 }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      <label><span>{tr(locale, 'Action')}</span><Select value={step.kind} options={[
                        { value: 'relay', label: tr(locale, 'Relay') },
                        { value: 'motion', label: tr(locale, 'Seat motion') },
                        { value: 'pwm', label: tr(locale, 'PWM') },
                        { value: 'status-led', label: tr(locale, 'Status light') },
                        { value: 'addressable', label: tr(locale, 'Strip pixel') },
                        { value: 'display', label: tr(locale, 'Display text') },
                        { value: 'rf', label: tr(locale, 'RF code') },
                        { value: 'beep', label: tr(locale, 'Buzzer') },
                      ]} onChange={(kind) => {
                        const steps = [...effectDraft.steps]; steps[index] = { at_us: step.at_us ?? 0, kind, target: 0, value: kind === 'motion' ? 1 : 0 }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      {['relay', 'motion', 'pwm', 'addressable'].includes(step.kind) && <label><span>{tr(locale, step.kind === 'motion' ? 'Side' : 'Channel')}</span><InputNumber min={0} max={step.kind === 'relay' ? 7 : step.kind === 'motion' ? 1 : 15} value={step.target ?? 0} onChange={(target) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, target: Number(target ?? 0) }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>}
                      {['relay', 'motion', 'pwm'].includes(step.kind) && <label><span>{tr(locale, 'Value')}</span><InputNumber min={0} max={step.kind === 'relay' ? 1 : step.kind === 'motion' ? 2 : 4095} value={step.value ?? 0} onChange={(value) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, value: Number(value ?? 0) }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>}
                      <label><span>{tr(locale, 'Duration')}</span><InputNumber min={0} step={50} addonAfter="ms" value={step.duration_ms ?? 0} onChange={(duration_ms) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, duration_ms: Number(duration_ms ?? 0) || undefined }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      <label><span>{tr(locale, 'Repeat')}</span><InputNumber min={0} max={1000} value={step.repeat_count ?? 0} onChange={(repeat_count) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, repeat_count: Number(repeat_count ?? 0) || undefined }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      <Space.Compact className="sequence-step-row__actions">
                        <Button icon={<ArrowUpOutlined />} disabled={index === 0} onClick={() => {
                          const steps = [...effectDraft.steps]; [steps[index - 1], steps[index]] = [steps[index], steps[index - 1]]; setEffectDraft({ ...effectDraft, steps });
                        }} />
                        <Button icon={<ArrowDownOutlined />} disabled={index + 1 === effectDraft.steps.length} onClick={() => {
                          const steps = [...effectDraft.steps]; [steps[index + 1], steps[index]] = [steps[index], steps[index + 1]]; setEffectDraft({ ...effectDraft, steps });
                        }} />
                        <Button danger icon={<DeleteOutlined />} onClick={() => setEffectDraft({ ...effectDraft, steps: effectDraft.steps.filter((_: unknown, stepIndex: number) => stepIndex !== index) })} />
                      </Space.Compact>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </Modal>

      <Modal
        title={tr(locale, 'Workspaces')}
        open={workspaceManagerOpen}
        footer={null}
        width={720}
        onCancel={() => setWorkspaceManagerOpen(false)}
      >
        <div className="workspace-create-row">
          <Input
            value={workspaceName}
            placeholder={tr(locale, 'Workspace name')}
            onChange={(event) => setWorkspaceName(event.target.value)}
          />
          <Select
            value={workspaceIcon}
            options={workspaceIconOptions.map((option) => ({
              ...option,
              label: <span className="workspace-icon-option">{workspaceGlyph(option.value)} {tr(locale, option.label)}</span>,
            }))}
            onChange={setWorkspaceIcon}
          />
          <Button
            type="primary"
            icon={<PlusOutlined />}
            disabled={!workspaceName.trim()}
            onClick={() => {
              sendCmd('workspace.create', { name: workspaceName.trim(), icon: workspaceIcon });
              setWorkspaceName('');
            }}
          >
            {tr(locale, 'Add workspace')}
          </Button>
        </div>
        <div className="workspace-profile-list">
          {workspaceProfiles.map((profile, index) => (
            <div className={`workspace-profile-row ${profile.id === state.active_workspace_profile ? 'is-active' : ''}`} key={profile.id}>
              <span className="workspace-profile-row__state">{workspaceGlyph(profile.icon)}</span>
              <Input
                defaultValue={profile.name}
                key={`${profile.id}:${profile.name}`}
                onBlur={(event) => {
                  const name = event.target.value.trim();
                  if (name && name !== profile.name) {
                    sendCmd('workspace.update', { id: profile.id, name, icon: profile.icon, capture: false });
                  }
                }}
                onPressEnter={(event) => event.currentTarget.blur()}
              />
              <Select
                value={profile.icon}
                options={workspaceIconOptions.map((option) => ({
                  ...option,
                  label: <span className="workspace-icon-option">{workspaceGlyph(option.value)} {tr(locale, option.label)}</span>,
                }))}
                onChange={(icon) => sendCmd('workspace.update', {
                  id: profile.id,
                  name: profile.name,
                  icon,
                  capture: false,
                })}
              />
              <Tooltip title={tr(locale, 'Restore')}>
                <Button icon={workspaceGlyph(profile.icon)} onClick={() => sendCmd('set_workspace', { profile: profile.id })} />
              </Tooltip>
              <Tooltip title={tr(locale, 'Replace with current workspace')}>
                <Button icon={<SaveOutlined />} onClick={() => sendCmd('workspace.update', {
                  id: profile.id,
                  name: profile.name,
                  icon: profile.icon,
                  capture: true,
                })} />
              </Tooltip>
              <Button
                icon={<ArrowUpOutlined />}
                disabled={index === 0}
                onClick={() => sendCmd('workspace.move', { id: profile.id, direction: -1 })}
              />
              <Button
                icon={<ArrowDownOutlined />}
                disabled={index + 1 === workspaceProfiles.length}
                onClick={() => sendCmd('workspace.move', { id: profile.id, direction: 1 })}
              />
              <Popconfirm
                title={tr(locale, 'Delete workspace?')}
                onConfirm={() => sendCmd('workspace.delete', { id: profile.id })}
              >
                <Button danger icon={<DeleteOutlined />} />
              </Popconfirm>
            </div>
          ))}
        </div>
      </Modal>

      <section className="studio-panel player-panel-web">
        <header className="studio-panel__header">
          <div>
            <span className="eyebrow">{tr(locale, 'Program monitor')}</span>
            <h2>{mediaName}</h2>
          </div>
          <span className={`transport-state ${state.playing ? 'is-playing' : ''}`}>
            {state.playing ? tr(locale, 'Playing') : tr(locale, 'Paused')}
          </span>
        </header>

        <div className="program-viewer">
          {state.current_video ? (
            <img
              src={`${apiBaseUrl}/api/player/frame`}
              alt={tr(locale, 'Video Preview')}
            />
          ) : (
            <div className="program-viewer__empty">
              <VideoCameraOutlined />
              <strong>{tr(locale, 'No Media Playing')}</strong>
            </div>
          )}
          <div className="program-viewer__chrome">
            <span>{appName}</span>
            <span>{state.workspace === 'nle' ? tr(locale, 'Timeline') : tr(locale, 'Player')}</span>
          </div>
        </div>

        <div className="studio-transport">
          <Tooltip title={`${tr(locale, 'Seek backward')} ${quickSeekSeconds}s`}>
            <Button icon={<FastBackwardOutlined />} onClick={() => sendCmd('seek', { seconds: -quickSeekSeconds })} />
          </Tooltip>
          <Button
            className="studio-transport__play"
            icon={state.playing ? <PauseOutlined /> : <CaretRightFilled />}
            onClick={() => sendCmd('toggle_pause')}
          />
          <Tooltip title={`${tr(locale, 'Seek forward')} ${quickSeekSeconds}s`}>
            <Button icon={<FastForwardOutlined />} onClick={() => sendCmd('seek', { seconds: quickSeekSeconds })} />
          </Tooltip>
          <span className="studio-timecode">{formatTime(currentSeconds)}</span>
          <SeekThumbnailPreview
            enabled={seekbarHoverThumbnails && Boolean(state.seekable) && durationSeconds > 0}
            duration={durationSeconds}
            mediaIdentity={state.current_video}
            apiBaseUrl={apiBaseUrl}
            unavailableLabel={tr(locale, 'Preview unavailable')}
            className="studio-scrubber"
          >
            <Slider
              min={0}
              max={100}
              value={activeSeek}
              disabled={!state.seekable || durationSeconds <= 0}
              onChange={(value) => setSeekDraft(value)}
              onChangeComplete={(value) => {
                setSeekDraft(null);
                sendCmd('seek_abs', { percentage: value });
              }}
              tooltip={seekbarHoverThumbnails ? { open: false } : { formatter: (value) => formatTime(((value ?? 0) / 100) * durationSeconds) }}
            />
          </SeekThumbnailPreview>
          <span className="studio-timecode studio-timecode--muted">
            {state.live ? tr(locale, 'LIVE') : formatTime(durationSeconds)}
          </span>
          <SoundOutlined className="volume-icon" />
          <Slider
            className="studio-volume"
            min={0}
            max={130}
            value={state.muted ? 0 : (state.volume ?? 0)}
            onChange={(value) => sendCmd('set_volume', { value })}
          />
        </div>
      </section>

      <section className="studio-panel timeline-panel-web">
        <header className="timeline-toolbar">
          <div>
            <span className="eyebrow">{tr(locale, 'Effect timeline')}</span>
            <h2>{tr(locale, 'Cues')}</h2>
          </div>
          <div className="timeline-toolbar__actions">
            <Dropdown
              menu={{
                selectedKeys: state.active_workspace_profile ? [state.active_workspace_profile] : [],
                items: workspaceProfiles.map((profile) => ({
                  key: profile.id,
                  icon: workspaceGlyph(profile.icon),
                  label: profile.name,
                })),
                onClick: ({ key }) => sendCmd('set_workspace', { profile: key }),
              }}
            >
              <Button icon={workspaceGlyph(activeWorkspace?.icon)}>
                {activeWorkspace?.name ?? tr(locale, 'Workspaces')}
              </Button>
            </Dropdown>
            <Tooltip title={tr(locale, 'Manage workspaces')}>
              <Button icon={<SettingOutlined />} onClick={() => setWorkspaceManagerOpen(true)} />
            </Tooltip>
            {chapters.length > 0 && (
              <Dropdown
                menu={{
                  selectedKeys: state.current_chapter_index === null || state.current_chapter_index === undefined
                    ? []
                    : [String(state.current_chapter_index)],
                  items: chapters.map((chapter) => ({
                    key: String(chapter.index),
                    label: `${formatTime(chapter.time_seconds, false)} · ${chapter.title}`,
                  })),
                  onClick: ({ key }) => sendCmd('set_chapter', { index: Number(key) }),
                }}
              >
                <Button icon={<BookOutlined />}>{tr(locale, 'Chapters')}</Button>
              </Dropdown>
            )}
            <span className="timeline-meta">{cues.length} {tr(locale, 'cues')}</span>
          </div>
        </header>

        <div className="timeline-ruler">
          <span>{formatTime(0, false)}</span>
          <span>{formatTime((timelineDurationMs / 1000) * .25, false)}</span>
          <span>{formatTime((timelineDurationMs / 1000) * .5, false)}</span>
          <span>{formatTime((timelineDurationMs / 1000) * .75, false)}</span>
          <span>{formatTime(timelineDurationMs / 1000, false)}</span>
        </div>

        <div
          className="timeline-grid"
          onDragOver={(event) => {
            if (event.dataTransfer.types.includes('application/x-pealayer-effect')) {
              event.preventDefault();
              event.dataTransfer.dropEffect = 'copy';
            }
          }}
          onDrop={(event) => {
            const reference = event.dataTransfer.getData('application/x-pealayer-effect');
            if (!reference) return;
            event.preventDefault();
            const rect = event.currentTarget.getBoundingClientRect();
            const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / Math.max(1, rect.width)));
            sendCmd('controller_effect_cue.add', { reference, start_time_ms: Math.round(fraction * timelineDurationMs) });
          }}
        >
          <div
            className="timeline-playhead-web"
            style={{ left: `${Math.min(100, (currentSeconds * 1000 / timelineDurationMs) * 100)}%` }}
          />
          {chapters.map((chapter) => (
            <button
              key={chapter.index}
              type="button"
              className={`timeline-chapter-web ${state.current_chapter_index === chapter.index ? 'is-active' : ''}`}
              style={{ left: `${Math.min(100, (chapter.time_seconds * 1000 / timelineDurationMs) * 100)}%` }}
              title={`${chapter.title} · ${formatTime(chapter.time_seconds)}`}
              aria-label={`${tr(locale, 'Chapter')}: ${chapter.title}`}
              onClick={() => sendCmd('set_chapter', { index: chapter.index })}
            />
          ))}
          {timelineLanes.length === 0 ? (
            <div className="timeline-empty">{tr(locale, 'No effects')}</div>
          ) : timelineLanes.map((lane) => {
            const laneEffects = effects.filter((effect) => (effect.lane || 'sequence') === lane);
            const laneEffectIds = new Set(laneEffects.map((effect) => effect.id));
            const effectCues = cues.filter((cue) => laneEffectIds.has(cue.effect_id));
            return (
              <div className="timeline-row" key={lane}>
                <div className="timeline-row__label">
                  <span>{effectGlyph(lane === 'relay' || lane === 'motion' ? 'relay:lane' : lane === 'sequence' ? 'controller' : lane)}</span>
                  <strong>{lane.charAt(0).toUpperCase() + lane.slice(1)}</strong>
                </div>
                <div className="timeline-lane">
                  {effectCues.map((cue) => (
                    (() => {
                      const placement = cuePreviews[cue.id] ?? cue;
                      return (
                    <button
                      key={cue.id}
                      className="timeline-cue"
                      style={{
                        left: `${(placement.start_time_ms / timelineDurationMs) * 100}%`,
                        width: `${Math.max(1.2, (placement.duration_ms / timelineDurationMs) * 100)}%`,
                      }}
                      title={`${cue.name} · ${formatTime(placement.start_time_ms / 1000)} · ${tr(locale, 'Drag to move; use the edges to resize')}`}
                      onPointerDown={(event) => {
                        if (event.button !== 0 || (event.target as HTMLElement).closest('.timeline-cue__delete')) return;
                        event.preventDefault();
                        const cueRect = event.currentTarget.getBoundingClientRect();
                        const laneRect = event.currentTarget.parentElement?.getBoundingClientRect();
                        const edge = Math.min(12, cueRect.width * .3);
                        const localX = event.clientX - cueRect.left;
                        const mode = localX <= edge
                          ? 'resize-left'
                          : localX >= cueRect.width - edge
                            ? 'resize-right'
                            : 'move';
                        setActiveCueDrag({
                          id: cue.id,
                          mode,
                          originX: event.clientX,
                          laneWidth: laneRect?.width ?? 1,
                          startTimeMs: placement.start_time_ms,
                          durationMs: placement.duration_ms,
                          moved: false,
                        });
                      }}
                      onClick={() => {
                        if (suppressCueClick.current === cue.id) {
                          suppressCueClick.current = null;
                          return;
                        }
                        sendCmd('seek_to', { seconds: placement.start_time_ms / 1000 });
                      }}
                    >
                      <span className="timeline-cue__resize timeline-cue__resize--left" aria-hidden="true" />
                      <span>{cue.name}</span>
                      <Button
                        className="timeline-cue__delete"
                        type="text"
                        size="small"
                        icon={<DeleteOutlined />}
                        onClick={(event) => {
                          event.stopPropagation();
                          sendCmd('remove_effect_cue', { instance_id: cue.id });
                        }}
                      />
                      <span className="timeline-cue__resize timeline-cue__resize--right" aria-hidden="true" />
                    </button>
                      );
                    })()
                  ))}
                </div>
                <span className="timeline-row__duration">{effectCues.length}</span>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
};
