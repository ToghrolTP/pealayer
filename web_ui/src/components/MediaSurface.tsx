import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  BookOutlined,
  ExpandOutlined,
  FastForwardOutlined,
  InfoCircleOutlined,
  MutedOutlined,
  PauseOutlined,
  PlayCircleOutlined,
  SoundOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { Dropdown } from 'antd';
import { tr, type UiLocale } from '../i18n';
import type { PlayerState } from './RemoteControlTab';

export type MediaClickAction = 'play_pause' | 'toggle_mute' | 'toggle_fullscreen' | 'context_menu' | 'none';
export type MediaDragAction = 'move_window' | 'seek' | 'temporary_fast_forward' | 'none';
export interface MediaGesturePreferences {
  clickPlayerToToggle: boolean;
  pausedDragAction: MediaDragAction;
  playingDragAction: MediaDragAction;
  middleClickAction: MediaClickAction;
  middleHoldAction: MediaDragAction;
  rightClickAction: MediaClickAction;
  rightHoldAction: MediaDragAction;
  temporaryFastForwardSpeed: number;
  playbackSpeed: number;
}

interface MediaSurfaceProps {
  state: PlayerState;
  apiBaseUrl: string;
  emptyLabel: string;
  className?: string;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  gestures: MediaGesturePreferences;
  locale: UiLocale;
}

interface PointerGesture {
  pointerId: number;
  button: number;
  startX: number;
  lastX: number;
  pendingX: number;
  lastSentAt: number;
  width: number;
  dragAction: MediaDragAction;
  dragging: boolean;
  fastForwarding: boolean;
  timer: number | null;
}

const mediaSource = (target: string, apiBaseUrl: string) => {
  if (/^https?:\/\//i.test(target)) return target;
  return `${apiBaseUrl}/api/fs/file?path=${encodeURIComponent(target)}`;
};

const safeColor = (value?: string | null) => value
  ? (value.startsWith('#') ? value : `#${value}`)
  : undefined;

const osdIcon = (name?: string | null) => {
  switch ((name ?? '').trim().toLowerCase()) {
    case 'play': return <PlayCircleOutlined />;
    case 'pause': return <PauseOutlined />;
    case 'volume': case 'sound': return <SoundOutlined />;
    case 'mute': case 'muted': return <MutedOutlined />;
    case 'speed': case 'fast-forward': return <FastForwardOutlined />;
    case 'chapter': case 'book': return <BookOutlined />;
    case 'video': return <VideoCameraOutlined />;
    case 'info': return <InfoCircleOutlined />;
    default: return null;
  }
};

const OsdOverlay: React.FC<{ state: PlayerState }> = ({ state }) => {
  const osd = state.osd;
  if (!osd?.message || osd.remaining_ms <= 0) return null;
  const options = osd.options ?? {};
  const customPoint = Number.isFinite(options.x_percent) && Number.isFinite(options.y_percent);
  const position = options.position ?? osd.default_position;
  const style = {
    ...(customPoint ? { left: `${options.x_percent}%`, top: `${options.y_percent}%` } : {}),
    color: safeColor(options.text_color),
    backgroundColor: safeColor(options.background_color),
    fontSize: options.font_size ? `${options.font_size}px` : undefined,
    paddingInline: options.padding_x !== null && options.padding_x !== undefined ? `${options.padding_x}px` : undefined,
    paddingBlock: options.padding_y !== null && options.padding_y !== undefined ? `${options.padding_y}px` : undefined,
    borderRadius: options.corner_radius !== null && options.corner_radius !== undefined ? `${options.corner_radius}px` : undefined,
  } as React.CSSProperties;
  return <div className={`media-osd media-osd--${position} ${customPoint ? 'media-osd--custom' : ''}`} style={style} role="status">
    {osdIcon(options.icon)}
    <span>{osd.message}</span>
  </div>;
};

/**
 * Browser-native playback is the primary Web surface. It consumes the same
 * byte-range endpoint used by remote Pealayer peers and follows the Rust/mpv
 * clock without emitting duplicate transport commands. Containers/codecs the
 * browser cannot decode fall back to the backend frame surface.
 */
export const MediaSurface: React.FC<MediaSurfaceProps> = ({ state, apiBaseUrl, emptyLabel, className, sendCmd, gestures, locale }) => {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const pointerGesture = useRef<PointerGesture | null>(null);
  const [fallback, setFallback] = useState(false);
  const [gestureActive, setGestureActive] = useState(false);
  const target = state.current_video ?? '';
  const source = useMemo(() => target ? mediaSource(target, apiBaseUrl) : '', [target, apiBaseUrl]);

  const invokeClickAction = useCallback((action: MediaClickAction) => {
    if (!target || action === 'none' || action === 'context_menu') return;
    if (action === 'play_pause') void sendCmd('toggle_pause');
    else if (action === 'toggle_mute') void sendCmd('toggle_mute');
    else if (action === 'toggle_fullscreen') void sendCmd('toggle_fullscreen');
  }, [sendCmd, target]);

  const flushSeek = useCallback((gesture: PointerGesture) => {
    if (gesture.pendingX === 0 || !state.seekable || !state.duration) return;
    const seconds = gesture.pendingX / Math.max(1, gesture.width) * state.duration;
    gesture.pendingX = 0;
    if (Math.abs(seconds) >= .001) void sendCmd('seek', { seconds });
  }, [sendCmd, state.duration, state.seekable]);

  const activateDrag = useCallback((gesture: PointerGesture) => {
    if (gesture.dragging || gesture.dragAction === 'none') return;
    gesture.dragging = true;
    setGestureActive(true);
    if (gesture.dragAction === 'temporary_fast_forward') {
      gesture.fastForwarding = true;
      void sendCmd('set_rate', { rate: Math.max(.05, gestures.temporaryFastForwardSpeed) });
    }
  }, [gestures.temporaryFastForwardSpeed, sendCmd]);

  const finishPointer = useCallback((event: React.PointerEvent<HTMLDivElement>, cancelled = false) => {
    const gesture = pointerGesture.current;
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    if (gesture.timer !== null) window.clearTimeout(gesture.timer);
    if (gesture.dragging && gesture.dragAction === 'seek') flushSeek(gesture);
    if (gesture.fastForwarding) void sendCmd('set_rate', { rate: Math.max(.05, gestures.playbackSpeed) });
    if (!cancelled && !gesture.dragging) {
      if (gesture.button === 0 && gestures.clickPlayerToToggle) invokeClickAction('play_pause');
      else if (gesture.button === 1) invokeClickAction(gestures.middleClickAction);
      else if (gesture.button === 2) invokeClickAction(gestures.rightClickAction);
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    pointerGesture.current = null;
    setGestureActive(false);
  }, [flushSeek, gestures.clickPlayerToToggle, gestures.middleClickAction, gestures.playbackSpeed, gestures.rightClickAction, invokeClickAction, sendCmd]);

  const pointerHandlers = target ? {
    onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => {
      if (pointerGesture.current || ![0, 1, 2].includes(event.button)) return;
      const dragAction = event.button === 0
        ? (state.playing ? gestures.playingDragAction : gestures.pausedDragAction)
        : event.button === 1 ? gestures.middleHoldAction : gestures.rightHoldAction;
      const gesture: PointerGesture = {
        pointerId: event.pointerId,
        button: event.button,
        startX: event.clientX,
        lastX: event.clientX,
        pendingX: 0,
        lastSentAt: performance.now(),
        width: event.currentTarget.getBoundingClientRect().width,
        dragAction,
        dragging: false,
        fastForwarding: false,
        timer: null,
      };
      pointerGesture.current = gesture;
      event.currentTarget.setPointerCapture(event.pointerId);
      // Holding for fast-forward is useful without requiring pointer travel.
      if (dragAction === 'temporary_fast_forward') {
        gesture.timer = window.setTimeout(() => activateDrag(gesture), 180);
      }
      if (event.button === 1) event.preventDefault();
    },
    onPointerMove: (event: React.PointerEvent<HTMLDivElement>) => {
      const gesture = pointerGesture.current;
      if (!gesture || gesture.pointerId !== event.pointerId) return;
      const delta = event.clientX - gesture.lastX;
      gesture.lastX = event.clientX;
      gesture.pendingX += delta;
      if (!gesture.dragging && Math.abs(event.clientX - gesture.startX) >= 5) activateDrag(gesture);
      if (gesture.dragging && gesture.dragAction === 'seek' && performance.now() - gesture.lastSentAt >= 34) {
        gesture.lastSentAt = performance.now();
        flushSeek(gesture);
      }
      if (gesture.dragging) event.preventDefault();
    },
    onPointerUp: (event: React.PointerEvent<HTMLDivElement>) => finishPointer(event),
    onPointerCancel: (event: React.PointerEvent<HTMLDivElement>) => finishPointer(event, true),
    onContextMenu: (event: React.MouseEvent<HTMLDivElement>) => {
      if (gestures.rightClickAction !== 'context_menu') event.preventDefault();
    },
  } : {};

  const contextMenu = {
    items: [
      { key: 'play_pause', icon: state.playing ? <PauseOutlined /> : <PlayCircleOutlined />, label: tr(locale, state.playing ? 'Pause' : 'Play') },
      { key: 'mute', icon: state.muted ? <SoundOutlined /> : <MutedOutlined />, label: tr(locale, state.muted ? 'Unmute' : 'Mute') },
      { key: 'fullscreen', icon: <ExpandOutlined />, label: tr(locale, 'Toggle fullscreen') },
      { type: 'divider' as const },
      { key: 'information', icon: <InfoCircleOutlined />, label: tr(locale, 'Media information') },
    ],
    onClick: ({ key }: { key: string }) => {
      if (key === 'play_pause') invokeClickAction('play_pause');
      else if (key === 'mute') invokeClickAction('toggle_mute');
      else if (key === 'fullscreen') invokeClickAction('toggle_fullscreen');
      else if (key === 'information') void sendCmd('open_media_information');
    },
  };

  useEffect(() => setFallback(false), [source]);

  useEffect(() => () => {
    const gesture = pointerGesture.current;
    if (!gesture) return;
    if (gesture.timer !== null) window.clearTimeout(gesture.timer);
    if (gesture.fastForwarding) void sendCmd('set_rate', { rate: Math.max(.05, gestures.playbackSpeed) });
    pointerGesture.current = null;
  }, [gestures.playbackSpeed, sendCmd]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video || fallback) return;
    const requested = state.playback_time ?? 0;
    if (Number.isFinite(requested) && Math.abs(video.currentTime - requested) > .65) {
      try { video.currentTime = requested; } catch { /* metadata may not be ready yet */ }
    }
    if (state.playing) void video.play().catch(() => undefined);
    else video.pause();
  }, [fallback, state.playback_time, state.playing]);

  if (!target) {
    return <div className={`media-surface media-surface--empty ${className ?? ''}`}>
      <VideoCameraOutlined />
      <strong>{emptyLabel}</strong>
    </div>;
  }

  let surface: React.ReactElement;
  if (fallback) {
    const stamp = Math.max(0, Math.round((state.playback_time ?? 0) * 1000));
    surface = <div className={`media-surface media-surface--fallback media-surface--interactive ${gestureActive ? 'is-gesture-active' : ''} ${className ?? ''}`} {...pointerHandlers}>
      <img src={`${apiBaseUrl}/api/player/frame?t=${stamp}`} alt={emptyLabel} />
      <OsdOverlay state={state} />
    </div>;
  } else surface = <div className={`media-surface media-surface--interactive ${gestureActive ? 'is-gesture-active' : ''} ${className ?? ''}`} {...pointerHandlers}>
    <video
      ref={videoRef}
      key={source}
      src={source}
      muted
      playsInline
      preload="auto"
      aria-label={emptyLabel}
      onLoadedMetadata={() => {
        const video = videoRef.current;
        if (!video) return;
        const requested = state.playback_time ?? 0;
        if (Number.isFinite(requested)) video.currentTime = requested;
        if (state.playing) void video.play().catch(() => undefined);
      }}
      onError={() => setFallback(true)}
    />
    <OsdOverlay state={state} />
  </div>;

  return <Dropdown trigger={['contextMenu']} menu={contextMenu}>{surface}</Dropdown>;
};
