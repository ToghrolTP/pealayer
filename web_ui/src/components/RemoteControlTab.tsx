import React, { useEffect, useState } from 'react';
import { Button, Slider, Tooltip } from 'antd';
import {
  FastBackwardOutlined,
  FastForwardOutlined,
  MutedOutlined,
  PauseOutlined,
  PlayCircleFilled,
  SoundOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

export interface PlayerState {
  status?: string;
  playing?: boolean;
  volume?: number;
  playback_time?: number;
  duration?: number;
  current_video?: string | null;
  seekable?: boolean;
  live?: boolean;
  muted?: boolean;
  playback_rate?: number;
  fullscreen?: boolean;
  workspace?: string;
  controller_connected?: boolean;
  hardware_connected?: boolean;
  estop_active?: boolean;
  hardware?: {
    board_name?: string;
    relay_count?: number;
    pwm_count?: number;
    supports_rf_transmit?: boolean;
    supports_addressable_led?: boolean;
    supports_segment_display?: boolean;
    supports_lcd_display?: boolean;
  } | null;
  hardware_details?: {
    board_name: string;
    capability_bits: number;
    host_instance_id: string;
    profile?: { key: string; mode: string; configured: boolean; attached: boolean; revision: string; expose_raw_relays: boolean } | null;
    port?: { name: string; display_name: string; friendly_name: string; product: string; manufacturer: string; vid: string; pid: string; serial_number: string };
    identity?: { product_name: string; stored_name: string; build_hash?: number | null; build_timestamp?: string | null };
    controls: Array<{
      key: string; kind: string; order: number; name: string; default_name: string;
      control: string; icon: string; color: string; group: string; hidden: boolean;
      locked: boolean; channel?: number | null; active?: boolean | null; percent?: number | null;
      actions: Array<{ id: string; verb: string; name: string; icon: string }>;
    }>;
    telemetry?: Record<string, number | boolean | null>;
    warnings?: Array<{ code: string; severity: string; message: string }>;
    settings?: Record<string, number | boolean> | null;
    front_panel?: {
      raw_segments: number[]; brightness: number; blink: boolean; segments_active: boolean;
      pressed_keys: number; menu_page: number; program_mode: number; lcd_available: boolean;
      lcd_address: number; lcd_line_1: string; lcd_line_2: string;
    } | null;
    strip?: {
      minimum_pixels: number; maximum_pixels: number; default_pixels: number;
      minimum_fps: number; maximum_fps: number; default_fps: number;
      modes: string[]; running: boolean; active_name: string;
    } | null;
    supports?: Record<string, boolean>;
  } | null;
  recording?: boolean;
  recording_armed?: boolean;
  recordable_track_count?: number;
  effects?: Array<{
    id: string;
    name: string;
    duration_ms: number;
    duration_display: string;
    action_count: number;
    target: string;
    lane: string;
  }>;
  controller_effects?: Array<{
    reference: string;
    id: string;
    name: string;
    icon: string;
    category: string;
    description: string;
    kind: 'sequence' | 'strip-stream';
    duration_ms: number;
    duration_display: string;
    action_count: number;
    editable: boolean;
    lane: string;
    program: unknown;
    default_fps?: number | null;
    default_pixels?: number | null;
  }>;
  cues?: Array<{
    id: string;
    effect_id: string;
    name: string;
    start_time_ms: number;
    duration_ms: number;
    duration_display: string;
  }>;
}

interface RemoteControlTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, any>) => void;
  onOpenLibraryTab?: () => void;
  locale: UiLocale;
  quickSeekSeconds: number;
  apiBaseUrl: string;
}

const formatTime = (seconds?: number) => {
  if (seconds === undefined || !Number.isFinite(seconds)) return '—';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const wholeSeconds = Math.floor(seconds % 60);
  return `${hours ? `${hours}:` : ''}${String(minutes).padStart(2, '0')}:${String(wholeSeconds).padStart(2, '0')}`;
};

export const RemoteControlTab: React.FC<RemoteControlTabProps> = ({
  state,
  sendCmd,
  onOpenLibraryTab,
  locale,
  quickSeekSeconds,
  apiBaseUrl,
}) => {
  const [frameTimestamp, setFrameTimestamp] = useState(Date.now());
  const [seekDraft, setSeekDraft] = useState<number | null>(null);

  useEffect(() => {
    if (!state.playing) return;
    const timer = window.setInterval(() => setFrameTimestamp(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [state.playing]);

  const videoName = state.current_video
    ? state.current_video.split(/[\\/]/).pop() || tr(locale, 'Untitled')
    : state.current_video === null ? tr(locale, 'No Media Playing') : tr(locale, 'Initializing…');
  const seekPercent = state.duration && state.duration > 0
    ? ((state.playback_time || 0) / state.duration) * 100
    : 0;

  return (
    <section className="remote-player">
      <div className="remote-player__preview">
        {state.current_video ? (
          <img src={`${apiBaseUrl}/api/player/frame?t=${frameTimestamp}`} alt={tr(locale, 'Video Preview')} />
        ) : (
          <div className="remote-player__empty">
            <VideoCameraOutlined />
            <strong>{videoName}</strong>
            {onOpenLibraryTab && <Button onClick={onOpenLibraryTab}>{tr(locale, 'Browse Media Library')}</Button>}
          </div>
        )}
      </div>

      <header className="remote-player__title">
        <div>
          <span className="eyebrow">{state.live ? tr(locale, 'LIVE') : tr(locale, 'Now playing')}</span>
          <h2 title={videoName}>{videoName}</h2>
        </div>
        <span className={`transport-state ${state.playing ? 'is-playing' : ''}`}>
          {state.playing ? tr(locale, 'Playing') : tr(locale, 'Paused')}
        </span>
      </header>

      <div className="remote-player__timeline">
        <span>{formatTime(state.playback_time)}</span>
        <Slider
          min={0}
          max={100}
          value={seekDraft ?? seekPercent}
          disabled={!state.seekable || !state.duration}
          onChange={setSeekDraft}
          onChangeComplete={(value) => {
            setSeekDraft(null);
            sendCmd('seek_abs', { percentage: value });
          }}
          tooltip={{ formatter: (value) => formatTime(((value || 0) / 100) * (state.duration || 0)) }}
        />
        <span>{state.live ? tr(locale, 'LIVE') : formatTime(state.duration)}</span>
      </div>

      <div className="remote-player__controls">
        <Tooltip title={`${tr(locale, 'Seek backward')} ${quickSeekSeconds}s`}>
          <Button shape="circle" icon={<FastBackwardOutlined />} onClick={() => sendCmd('seek', { seconds: -quickSeekSeconds })} />
        </Tooltip>
        <Tooltip title={state.playing ? tr(locale, 'Pause') : tr(locale, 'Play')}>
          <Button
            shape="circle"
            className="remote-player__play"
            icon={state.playing ? <PauseOutlined /> : <PlayCircleFilled />}
            onClick={() => sendCmd('toggle_pause')}
          />
        </Tooltip>
        <Tooltip title={`${tr(locale, 'Seek forward')} ${quickSeekSeconds}s`}>
          <Button shape="circle" icon={<FastForwardOutlined />} onClick={() => sendCmd('seek', { seconds: quickSeekSeconds })} />
        </Tooltip>
      </div>

      <div className="remote-player__volume">
        <Button
          type="text"
          aria-label={state.muted ? tr(locale, 'Unmute') : tr(locale, 'Mute')}
          icon={state.muted || state.volume === 0 ? <MutedOutlined /> : <SoundOutlined />}
          onClick={() => sendCmd('set_mute', { muted: !state.muted })}
        />
        <Slider
          min={0}
          max={130}
          value={state.muted ? 0 : (state.volume ?? 0)}
          disabled={state.volume === undefined}
          onChange={(value) => sendCmd('set_volume', { value })}
        />
        <output>{state.volume === undefined ? '—' : `${Math.round(state.muted ? 0 : state.volume)}%`}</output>
      </div>
    </section>
  );
};
