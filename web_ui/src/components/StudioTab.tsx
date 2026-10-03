import React, { useMemo, useState } from 'react';
import {
  AppstoreOutlined,
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
  SoundOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { Button, Empty, Input, InputNumber, message, Modal, Popconfirm, Select, Slider, Tooltip } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

interface StudioTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
  appName: string;
  quickSeekSeconds: number;
  apiBaseUrl: string;
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

export const StudioTab: React.FC<StudioTabProps> = ({ state, sendCmd, locale, appName, quickSeekSeconds, apiBaseUrl, surface = 'studio' }) => {
  const [selectedEffect, setSelectedEffect] = useState<string | null>(null);
  const [effectEditorOpen, setEffectEditorOpen] = useState(false);
  const [effectDraft, setEffectDraft] = useState<Record<string, any> | null>(null);
  const [seekDraft, setSeekDraft] = useState<number | null>(null);
  const effects = state.effects ?? [];
  const controllerEffects = state.controller_effects ?? [];
  const cues = state.cues ?? [];
  const currentSeconds = state.playback_time ?? 0;
  const durationSeconds = state.duration ?? 0;
  const timelineDurationMs = useMemo(() => {
    const cueEnd = cues.reduce((maximum, cue) => Math.max(maximum, cue.start_time_ms + cue.duration_ms), 0);
    return Math.max(durationSeconds * 1000, cueEnd, 1000);
  }, [cues, durationSeconds]);
  const selected = selectedEffect ? controllerEffects.find((effect) => effect.reference === selectedEffect) : undefined;
  const mediaName = state.current_video
    ? state.current_video.split(/[\\/]/).pop()
    : tr(locale, 'No Media Playing');
  const seekPercent = durationSeconds > 0 ? (currentSeconds / durationSeconds) * 100 : 0;
  const activeSeek = seekDraft ?? seekPercent;
  const canRecord = (state.recordable_track_count ?? 0) > 0;
  const timelineLanes = useMemo(() => {
    const order = ['motion', 'relay', 'pwm', 'lighting', 'display', 'rf', 'audio', 'sequence', 'composite'];
    const active = new Set(effects.map((effect) => effect.lane || 'sequence'));
    return order.filter((lane) => active.has(lane));
  }, [effects]);

  const addCue = (reference: string) => {
    sendCmd('controller_effect_cue.add', {
      reference,
      start_time_ms: Math.max(0, Math.round(currentSeconds * 1000)),
    });
  };

  const editEffect = (effect?: typeof controllerEffects[number]) => {
    setEffectDraft(effect ? {
      ...effect,
      programText: JSON.stringify(effect.program ?? {}, null, 2),
      color: 'green',
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      is_new: false,
    } : {
      reference: '', id: '', name: '', category: 'Lighting', description: '',
      kind: 'strip-stream', programText: '{}',
      color: 'green', default_fps: 20, duration_ms: 5000, default_pixels: 100,
      is_new: true,
    });
    setEffectEditorOpen(true);
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

        {canRecord && (
          <div className="effects-toolbar">
            <Button
              danger={state.recording_armed}
              icon={<span className="record-dot" />}
              onClick={() => sendCmd('set_recording', { enabled: !state.recording_armed })}
            >
              {state.recording ? tr(locale, 'Recording') : state.recording_armed ? tr(locale, 'Armed') : tr(locale, 'Record hardware')}
            </Button>
          </div>
        )}

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
              >
                <span className="effect-profile__icon">{effectGlyph(effect.kind === 'sequence' ? 'controller' : 'strip')}</span>
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
          try {
            program = JSON.parse(effectDraft.programText || '{}');
          } catch {
            void message.error(tr(locale, 'Program must be valid JSON'));
            return;
          }
          const { programText, ...payload } = effectDraft;
          sendCmd('controller_effect.save', { ...payload, program });
          setEffectEditorOpen(false);
        }}
      >
        {effectDraft && (
          <div className="effect-editor-grid">
            <label><span>{tr(locale, 'Type')}</span><Select value={effectDraft.kind} options={[{ value: 'strip-stream', label: tr(locale, 'Lighting') }, { value: 'sequence', label: tr(locale, 'Sequence') }]} onChange={(kind) => setEffectDraft({ ...effectDraft, kind })} /></label>
            <label><span>{tr(locale, 'ID')}</span><Input value={effectDraft.id} onChange={(event) => setEffectDraft({ ...effectDraft, id: event.target.value })} /></label>
            <label><span>{tr(locale, 'Name')}</span><Input value={effectDraft.name} onChange={(event) => setEffectDraft({ ...effectDraft, name: event.target.value })} /></label>
            <label><span>{tr(locale, 'Category')}</span><Input value={effectDraft.category} onChange={(event) => setEffectDraft({ ...effectDraft, category: event.target.value })} /></label>
            <label className="effect-editor-grid__wide"><span>{tr(locale, 'Description')}</span><Input value={effectDraft.description} onChange={(event) => setEffectDraft({ ...effectDraft, description: event.target.value })} /></label>
            <label><span>{tr(locale, 'Duration (ms)')}</span><InputNumber min={1} value={effectDraft.duration_ms} onChange={(duration_ms) => setEffectDraft({ ...effectDraft, duration_ms: duration_ms ?? 1 })} /></label>
            {effectDraft.kind === 'strip-stream' && <label><span>{tr(locale, 'Frames per second')}</span><InputNumber min={1} max={120} value={effectDraft.default_fps} onChange={(default_fps) => setEffectDraft({ ...effectDraft, default_fps: default_fps ?? 20 })} /></label>}
            {effectDraft.kind === 'strip-stream' && <label><span>{tr(locale, 'Pixels')}</span><InputNumber min={1} value={effectDraft.default_pixels} onChange={(default_pixels) => setEffectDraft({ ...effectDraft, default_pixels: default_pixels ?? 100 })} /></label>}
            {effectDraft.kind === 'sequence' && <label><span>{tr(locale, 'Color')}</span><Input value={effectDraft.color} onChange={(event) => setEffectDraft({ ...effectDraft, color: event.target.value })} /></label>}
            {effectDraft.kind === 'strip-stream' && <label className="effect-editor-grid__wide"><span>{tr(locale, 'Program')}</span><Input.TextArea autoSize={{ minRows: 7, maxRows: 16 }} value={effectDraft.programText} onChange={(event) => setEffectDraft({ ...effectDraft, programText: event.target.value })} /></label>}
          </div>
        )}
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
          <Slider
            className="studio-scrubber"
            min={0}
            max={100}
            value={activeSeek}
            disabled={!state.seekable || durationSeconds <= 0}
            onChange={(value) => setSeekDraft(value)}
            onChangeComplete={(value) => {
              setSeekDraft(null);
              sendCmd('seek_abs', { percentage: value });
            }}
            tooltip={{ formatter: (value) => formatTime(((value ?? 0) / 100) * durationSeconds) }}
          />
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
            <Tooltip title={tr(locale, 'Show timeline workspace')}>
              <Button
                type={state.workspace === 'nle' ? 'primary' : 'default'}
                icon={<EditOutlined />}
                onClick={() => sendCmd('set_workspace', { nle: true })}
              />
            </Tooltip>
            <Tooltip title={tr(locale, 'Show player workspace')}>
              <Button
                type={state.workspace === 'simple' ? 'primary' : 'default'}
                icon={<DesktopOutlined />}
                onClick={() => sendCmd('set_workspace', { nle: false })}
              />
            </Tooltip>
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

        <div className="timeline-grid">
          <div
            className="timeline-playhead-web"
            style={{ left: `${Math.min(100, (currentSeconds * 1000 / timelineDurationMs) * 100)}%` }}
          />
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
                    <button
                      key={cue.id}
                      className="timeline-cue"
                      style={{
                        left: `${(cue.start_time_ms / timelineDurationMs) * 100}%`,
                        width: `${Math.max(1.2, (cue.duration_ms / timelineDurationMs) * 100)}%`,
                      }}
                      title={`${cue.name} · ${formatTime(cue.start_time_ms / 1000)}`}
                      onClick={() => sendCmd('seek_to', { seconds: cue.start_time_ms / 1000 })}
                    >
                      <span>{cue.name}</span>
                      <Button
                        type="text"
                        size="small"
                        icon={<DeleteOutlined />}
                        onClick={(event) => {
                          event.stopPropagation();
                          sendCmd('remove_effect_cue', { instance_id: cue.id });
                        }}
                      />
                    </button>
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
