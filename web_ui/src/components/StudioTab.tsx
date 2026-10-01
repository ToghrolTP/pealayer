import React, { useMemo, useState } from 'react';
import {
  AppstoreOutlined,
  CaretRightFilled,
  ClockCircleOutlined,
  DeleteOutlined,
  FastBackwardOutlined,
  FastForwardOutlined,
  PauseOutlined,
  PlusOutlined,
  RadarChartOutlined,
  SoundOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { Button, Empty, Slider, Tooltip } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

interface StudioTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  locale: UiLocale;
  appName: string;
  quickSeekSeconds: number;
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

export const StudioTab: React.FC<StudioTabProps> = ({ state, sendCmd, locale, appName, quickSeekSeconds }) => {
  const [selectedEffect, setSelectedEffect] = useState<string | null>(null);
  const [seekDraft, setSeekDraft] = useState<number | null>(null);
  const effects = state.effects ?? [];
  const cues = state.cues ?? [];
  const currentSeconds = state.playback_time ?? 0;
  const durationSeconds = state.duration ?? 0;
  const timelineDurationMs = useMemo(() => {
    const cueEnd = cues.reduce((maximum, cue) => Math.max(maximum, cue.start_time_ms + cue.duration_ms), 0);
    return Math.max(durationSeconds * 1000, cueEnd, 1000);
  }, [cues, durationSeconds]);
  const selected = selectedEffect ? effects.find((effect) => effect.id === selectedEffect) : undefined;
  const mediaName = state.current_video
    ? state.current_video.split(/[\\/]/).pop()
    : tr(locale, 'No Media Playing');
  const seekPercent = durationSeconds > 0 ? (currentSeconds / durationSeconds) * 100 : 0;
  const activeSeek = seekDraft ?? seekPercent;

  const addCue = (effectId: string) => {
    sendCmd('add_effect_cue', {
      effect_id: effectId,
      start_time_ms: Math.max(0, Math.round(currentSeconds * 1000)),
    });
  };

  return (
    <div className="studio-suite">
      <section className="studio-panel effects-panel">
        <header className="studio-panel__header">
          <div>
            <span className="eyebrow">{tr(locale, 'Live project')}</span>
            <h2>{tr(locale, 'Effects')}</h2>
          </div>
          <span className="count-badge">{effects.length}</span>
        </header>

        <div className="mode-switch" role="group" aria-label={tr(locale, 'Studio mode')}>
          <button
            className={!state.recording ? 'is-active' : ''}
            onClick={() => sendCmd('set_recording', { enabled: false })}
          >
            <CaretRightFilled /> {tr(locale, 'Play Mode')}
          </button>
          <button
            className={state.recording ? 'is-active is-recording' : ''}
            onClick={() => sendCmd('set_recording', { enabled: true })}
          >
            <span className="record-dot" /> {tr(locale, 'Record Mode')}
          </button>
        </div>

        <div className="effect-list">
          {effects.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={tr(locale, 'No effect profiles are available in the current project.')}
            />
          ) : effects.map((effect, index) => {
            const selectedCard = selectedEffect === effect.id;
            return (
              <article
                key={effect.id}
                className={`effect-profile ${selectedCard ? 'is-selected' : ''}`}
                onClick={() => setSelectedEffect(effect.id)}
              >
                <span className="effect-profile__icon">{effectGlyph(effect.target)}</span>
                <div className="effect-profile__body">
                  <strong>{effect.name}</strong>
                  <span>{effect.action_count} {tr(locale, 'actions')} · {effect.duration_ms} ms</span>
                </div>
                <kbd>F{index + 1}</kbd>
                <Tooltip title={tr(locale, 'Add cue at playhead')}>
                  <Button
                    type="text"
                    icon={<PlusOutlined />}
                    onClick={(event) => {
                      event.stopPropagation();
                      addCue(effect.id);
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
          {selected && <strong>{selected.name}</strong>}
        </footer>
      </section>

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
              src={`/api/player/frame?path=${encodeURIComponent(state.current_video)}`}
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
            <span>{state.workspace === 'nle' ? tr(locale, 'Edit Mode') : tr(locale, 'Play Mode')}</span>
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
            <Button
              type={state.workspace === 'nle' ? 'primary' : 'default'}
              onClick={() => sendCmd('set_workspace', { nle: true })}
            >
              {tr(locale, 'Edit Mode')}
            </Button>
            <Button onClick={() => sendCmd('set_workspace', { nle: false })}>
              {tr(locale, 'Play Mode')}
            </Button>
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
          {effects.length === 0 ? (
            <div className="timeline-empty">{tr(locale, 'Create or discover an effect profile to begin authoring cues.')}</div>
          ) : effects.map((effect) => {
            const effectCues = cues.filter((cue) => cue.effect_id === effect.id);
            return (
              <div className="timeline-row" key={effect.id}>
                <div className="timeline-row__label">
                  <span>{effectGlyph(effect.target)}</span>
                  <strong>{effect.name}</strong>
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
                <span className="timeline-row__duration">{effect.duration_ms} ms</span>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
};
