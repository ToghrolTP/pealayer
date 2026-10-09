import React from 'react';
import type { RfSnapshot } from './RfManager';
import { Button, Select, Slider, Tooltip } from 'antd';
import {
  FastBackwardOutlined,
  FastForwardOutlined,
  StepBackwardOutlined,
  StepForwardOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';
import { mediaBasename } from '../mediaLabel';
import { MediaSurface } from './MediaSurface';
import type { MediaGesturePreferences } from './MediaSurface';
import { SeekThumbnailPreview } from './SeekThumbnailPreview';
import { SeekbarMarkers, useSeekbar } from './seekbar';
import type { AppearanceState } from '../appearance';
import type { TimelineWheelPreferences } from '../timelineWheel';
import type { HardwareMelody } from '../melodyCatalog';
import { MediaTrackSelectors } from './MediaTrackSelectors';
import { PlaybackButton } from './PlaybackButton';
import { VolumeControl } from './VolumeControl';
import { ElapsedTimeInput } from './ElapsedTimeInput';

export interface PlayerState {
  app_icon_revision?: number;
  rf?: RfSnapshot;
  remote_browser?: import('./RemoteLocationDialog').RemoteBrowser;
  status?: string;
  messages?: import('../messaging').ToastSnapshot;
  appearance?: AppearanceState;
  timeline_wheel_preferences?: TimelineWheelPreferences;
  playing?: boolean;
  volume?: number;
  playback_time?: number;
  duration?: number;
  media_fps?: number;
  ui_fps?: number | null;
  current_video?: string | null;
  media_tracks?: Array<{
    id: number;
    kind: 'video' | 'audio' | 'subtitle';
    title?: string | null;
    language?: string | null;
    codec?: string | null;
    selected: boolean;
    is_default: boolean;
    forced: boolean;
    external: boolean;
  }>;
  chapters?: Array<{ index: number; title: string; time_seconds: number }>;
  current_chapter_index?: number | null;
  seek_pending?: boolean;
  settled_seek_revision?: number;
  settled_seek_target?: number | null;
  seekbar_markers?: { chapter_color: string; active_chapter_color: string; keyframe_color: string };
  timeline_keyframes?: Array<{ id: string; time_ms: number; label: string }>;
  seekable?: boolean;
  live?: boolean;
  muted?: boolean;
  playback_rate?: number;
  fullscreen?: boolean;
  workspace?: string;
  active_workspace_profile?: string | null;
  workspace_profiles?: Array<{
    id: string;
    name: string;
    icon: string;
    order: number;
    mode: 'simple' | 'nle';
  }>;
  controller_connected?: boolean;
  hardware_connected?: boolean;
  hardware_endpoint?: string;
  hardware_transport?: string | null;
  hardware_error?: string | null;
  hardware_sync?: { revision: number; prepared_revision: number; error?: string | null; ack_age_ms?: number | null;
    authority_client_id?: string;
    authority?: { owner_id: string; owner_label: string; exclusive: boolean; revision: number;
      owner_endpoint?: string | null;
      pending: Array<{ client_id: string; label: string; requested_at: string }> } | null;
    timeline?: { state?: string; acknowledged?: number; step_count?: number; max_ack_lateness_ms?: number } } | null;
  controller_effect_groups?: Array<{ name: string; icon: string }>;
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
    motion_control_mode?: 'hold' | 'toggle';
    profile?: { key: string; mode: string; configured: boolean; attached: boolean; revision: string; expose_raw_relays: boolean } | null;
    port?: { name: string; display_name: string; friendly_name: string; product: string; manufacturer: string; vid: string; pid: string; serial_number: string };
    identity?: { product_name: string; stored_name: string; build_hash?: number | null; build_timestamp?: string | null };
    controls: Array<{
      key: string; kind: string; order: number; name: string; default_name: string;
      control: string; icon: string; color: string; group: string; hidden: boolean;
      up_color?: string; down_color?: string; indicator_color?: string;
      direction?: 'up' | 'down' | 'stop' | null;
      locked: boolean; channel?: number | null; active?: boolean | null; percent?: number | null;
      actions: Array<{ id: string; verb: string; name: string; icon: string }>;
    }>;
    telemetry?: Record<string, number | boolean | null>;
    status_led?: { red: number; green: number; blue: number } | null;
    warnings?: Array<{ code: string; severity: string; message: string }>;
    melodies?: HardwareMelody[];
    buzzer?: { playing: boolean; melody_id: number; melody_name: string; board_silent: boolean };
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
  effect_recording?: {
    active: boolean;
    id: number;
    name: string;
    mode: string;
    category: string;
    color: string;
    steps: number;
    preview: Array<{
      at_us: number;
      kind: string;
      target?: number;
      value?: number;
      text?: string;
      action_ids?: string[];
    }>;
    device_retained: boolean;
    overwritten: number;
    started_at: string;
    last_error: string;
    pending: boolean;
  };
  cues?: Array<{
    id: string;
    effect_id: string;
    name: string;
    start_time_ms: number;
    duration_ms: number;
    duration_display: string;
    resizable: boolean;
    behavior?: 'set-keep' | 'hold' | 'ramp' | null;
    end_value_basis_points?: number | null;
    control_key?: string | null;
    value_basis_points?: number | null;
  }>;
  timeline_tracks?: Array<{
    key: string;
    name: string;
    detail?: string | null;
    kind: 'video' | 'audio' | 'subtitle' | 'effect' | 'hardware';
    lane?: string | null;
    control_key?: string | null;
    active: boolean;
    enabled: boolean;
    linked: boolean;
    visible: boolean;
    dimmed: boolean;
    selected: boolean;
    muted: boolean;
    soloed: boolean;
    locked: boolean;
    supports_mute: boolean;
    supports_solo: boolean;
    supports_lock: boolean;
    manageable: boolean;
  }>;
  osd?: {
    message: string;
    remaining_ms: number;
    default_position: 'top_left' | 'top_center' | 'top_right' | 'center_left' | 'center' | 'center_right' | 'bottom_left' | 'bottom_center' | 'bottom_right';
    options: {
      position?: 'top_left' | 'top_center' | 'top_right' | 'center_left' | 'center' | 'center_right' | 'bottom_left' | 'bottom_center' | 'bottom_right' | null;
      x_percent?: number | null; y_percent?: number | null; font_size?: number | null;
      icon?: string | null; text_color?: string | null; background_color?: string | null;
      timeout_seconds?: number | null; padding_x?: number | null; padding_y?: number | null;
      corner_radius?: number | null;
    };
  } | null;
  update?: {
    operation_id?: string | null;
    state: string;
    source?: string | null;
    bytes_done: number;
    bytes_total?: number | null;
    sha256?: string | null;
    version?: string | null;
    message: string;
    error?: string | null;
  };
}

interface RemoteControlTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, any>) => Promise<boolean>;
  onOpenLibraryTab?: () => void;
  locale: UiLocale;
  quickSeekSeconds: number;
  apiBaseUrl: string;
  seekbarHoverThumbnails: boolean;
  mediaGestures: MediaGesturePreferences;
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
  seekbarHoverThumbnails,
  mediaGestures,
}) => {
  const seek = useSeekbar(state, sendCmd);

  const videoName = state.current_video
    ? mediaBasename(state.current_video, tr(locale, 'Untitled'))
    : state.current_video === null ? tr(locale, 'No Media Playing') : tr(locale, 'Initializing…');

  return (
    <section className="remote-player">
      <div className="remote-player__preview">
        {state.current_video ? (
          <MediaSurface state={state} apiBaseUrl={apiBaseUrl} emptyLabel={tr(locale, 'Video Preview')} sendCmd={sendCmd} gestures={mediaGestures} locale={locale} />
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
        <ElapsedTimeInput seconds={state.playback_time ?? 0} locale={locale}
          disabled={!state.current_video || !state.seekable || !state.duration}
          mediaIdentity={state.current_video} onCommit={seek.commitSeconds} />
        <SeekThumbnailPreview
          enabled={seekbarHoverThumbnails && Boolean(state.seekable) && Boolean(state.duration)}
          duration={state.duration || 0}
          mediaIdentity={state.current_video}
          apiBaseUrl={apiBaseUrl}
          unavailableLabel={tr(locale, 'Preview unavailable')}
          chapters={state.chapters}
          keyframes={state.timeline_keyframes}
        >
          <Slider
            min={0}
            max={100}
            value={seek.value}
            disabled={!state.current_video || !state.seekable || !state.duration}
            onChange={seek.change}
            onChangeComplete={seek.commit}
            tooltip={seekbarHoverThumbnails ? { open: false } : { formatter: (value) => formatTime(((value || 0) / 100) * (state.duration || 0)) }}
          />
          <SeekbarMarkers state={state} seconds={seek.value / 100 * (state.duration ?? 0)} onChapter={seek.commitSeconds} />
        </SeekThumbnailPreview>
        <span>{state.live ? tr(locale, 'LIVE') : formatTime(state.duration)}</span>
      </div>

      <div className="remote-player__controls">
        {(state.remote_browser?.previous_file || state.remote_browser?.next_file) && <Tooltip title="Previous file"><Button shape="circle" icon={<StepBackwardOutlined />} disabled={!state.remote_browser.previous_file} onClick={() => sendCmd('previous')} /></Tooltip>}
        <Tooltip title={`${tr(locale, 'Seek backward')} ${quickSeekSeconds}s`}>
          <Button shape="circle" icon={<FastBackwardOutlined />} disabled={!state.current_video || !state.seekable} onClick={() => sendCmd('seek', { seconds: -quickSeekSeconds })} />
        </Tooltip>
          <PlaybackButton
            className="remote-player__play"
            playing={state.playing}
            loaded={Boolean(state.current_video)}
            locale={locale}
            onClick={() => sendCmd('toggle_pause')}
          />
        <Tooltip title={`${tr(locale, 'Seek forward')} ${quickSeekSeconds}s`}>
          <Button shape="circle" icon={<FastForwardOutlined />} disabled={!state.current_video || !state.seekable} onClick={() => sendCmd('seek', { seconds: quickSeekSeconds })} />
        </Tooltip>
        {(state.remote_browser?.previous_file || state.remote_browser?.next_file) && <Tooltip title="Next file"><Button shape="circle" icon={<StepForwardOutlined />} disabled={!state.remote_browser.next_file} onClick={() => sendCmd('next')} /></Tooltip>}
      </div>

      {(state.chapters?.length ?? 0) > 0 && (
        <div className="remote-player__chapters">
          <Tooltip title={tr(locale, 'Previous chapter')}>
            <Button icon={<StepBackwardOutlined />} onClick={() => sendCmd('chapter_previous')} />
          </Tooltip>
          <Select
            aria-label={tr(locale, 'Chapter')}
            value={state.current_chapter_index ?? state.chapters?.[0]?.index}
            options={state.chapters?.map((chapter) => ({
              value: chapter.index,
              label: `${formatTime(chapter.time_seconds)} · ${chapter.title}`,
            }))}
            onChange={(index) => sendCmd('set_chapter', { index })}
          />
          <Tooltip title={tr(locale, 'Next chapter')}>
            <Button icon={<StepForwardOutlined />} onClick={() => sendCmd('chapter_next')} />
          </Tooltip>
        </div>
      )}

      <MediaTrackSelectors state={state} sendCmd={sendCmd} locale={locale} />

      <VolumeControl className="remote-player__volume" state={state} sendCmd={sendCmd} locale={locale} />
    </section>
  );
};
