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
import { insideMediaPicture } from '../mediaPicture';
import { extraMouseAction, videoMousePolicy, videoReleaseIntent } from '../mouseBindings';
import type { MouseAction, MouseBindings } from '../mouseBindings';
import { useSeekbar } from './seekbar';

export type MediaClickAction = 'play_pause' | 'toggle_mute' | 'toggle_fullscreen' | 'boss_key' | 'context_menu' | 'none';
export type MediaDragAction = 'move_window' | 'seek' | 'temporary_fast_forward' | 'none';
export interface MediaGesturePreferences {
  clickPlayerToToggle: boolean;
  videoClickFrameOnly: boolean;
  pausedDragAction: MediaDragAction;
  playingDragAction: MediaDragAction;
  middleClickAction: MediaClickAction;
  middleHoldAction: MediaDragAction;
  rightClickAction: MediaClickAction;
  rightHoldAction: MediaDragAction;
  temporaryFastForwardSpeed: number;
  playbackSpeed: number;
  mouseBindings?: MouseBindings;
  frameStepCount: number;
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
  target: string;
  startTime: number;
  targetTime: number;
  seekStarted: boolean;
  pointerId: number;
  button: number;
  startX: number;
  startY: number;
  moved: boolean;
  clickAction: MediaClickAction;
  extraAction: MouseAction;
  forceMenu: boolean;
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

const isDirectPlayable = (target: string, tracks?: PlayerState['media_tracks']) => {
  if (!target) return false;
  if (/\.(mkv|avi|flv|wmv|ts|m2ts|vob|iso|rmvb)$/i.test(target)) return false;
  const videoTrack = tracks?.find((t) => t.kind === 'video' && t.selected);
  if (videoTrack?.codec) {
    const codec = videoTrack.codec.toLowerCase();
    if (codec.includes('hevc') || codec.includes('h265') || codec.includes('prores')) {
      return false;
    }
  }
  return true;
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
  const target = state.current_video ?? '';
  const targetRef=useRef(target);targetRef.current=target;
  const seek=useSeekbar(state,sendCmd);
  const seekRef=useRef(seek); seekRef.current=seek;
  const playable = useMemo(() => isDirectPlayable(target, state.media_tracks), [target, state.media_tracks]);
  const useFallback = fallback || !playable;
  const [fallbackStamp, setFallbackStamp] = useState(() => Math.max(0, Math.round(state.playback_time ?? 0)));
  const lastFallbackRef = useRef(0);
  const [gestureActive, setGestureActive] = useState(false);
  const [contextMenuOpen, setContextMenuOpen] = useState(false);
  const [contextPoint, setContextPoint] = useState({x:0,y:0});
  const source = useMemo(() => target ? mediaSource(target, apiBaseUrl) : '', [target, apiBaseUrl]);
  const insidePicture = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const hit=document.elementFromPoint(event.clientX,event.clientY);
    if(hit && !event.currentTarget.contains(hit)) return false;
    if (!gestures.videoClickFrameOnly) return true;
    const element = event.currentTarget.querySelector('video, img') as HTMLVideoElement | HTMLImageElement | null;
    if (!element) return false;
    const width = element instanceof HTMLVideoElement ? element.videoWidth : element.naturalWidth;
    const height = element instanceof HTMLVideoElement ? element.videoHeight : element.naturalHeight;
    if (!(width > 0 && height > 0)) return false;
    return insideMediaPicture(event.clientX, event.clientY, element.getBoundingClientRect(), width, height);
  }, [gestures.videoClickFrameOnly]);

  useEffect(() => {
    if (!useFallback) return;
    const now = Date.now();
    const time = state.playback_time ?? 0;
    const interval = state.playing ? 1000 : 250;
    if (now - lastFallbackRef.current >= interval) {
      lastFallbackRef.current = now;
      setFallbackStamp(Math.max(0, Math.round(time)));
    } else {
      const remaining = interval - (now - lastFallbackRef.current);
      const timer = window.setTimeout(() => {
        lastFallbackRef.current = Date.now();
        setFallbackStamp(Math.max(0, Math.round(state.playback_time ?? 0)));
      }, remaining);
      return () => window.clearTimeout(timer);
    }
  }, [useFallback, state.playback_time, state.playing]);

  const invokeClickAction = useCallback((action: MediaClickAction) => {
    if (!target || action === 'none' || action === 'context_menu') return;
    if (action === 'play_pause') void sendCmd('toggle_pause');
    else if (action === 'toggle_mute') void sendCmd('toggle_mute');
    else if (action === 'toggle_fullscreen') void sendCmd('toggle_fullscreen');
    else if (action === 'boss_key') void sendCmd('pause').then(() => sendCmd('minimize'));
  }, [sendCmd, target]);

  const invokeExtraAction = useCallback((action: MouseAction) => {
    if (action === 'seek_backward' || action === 'seek_forward') void sendCmd('seek', {seconds:(action === 'seek_backward' ? -1 : 1) * (state.quick_seek_seconds ?? 10)});
    else if (action === 'previous_chapter' || action === 'next_chapter') void sendCmd(action);
    else if (action === 'previous_file' || action === 'next_file') void sendCmd(action === 'previous_file' ? 'previous' : 'next');
    else if (action === 'frame_backward' || action === 'frame_forward') void sendCmd('frame_step', {frames:(action === 'frame_backward' ? -1 : 1) * gestures.frameStepCount});
    else if (['play_pause','toggle_mute','toggle_fullscreen','boss_key'].includes(action)) invokeClickAction(action as MediaClickAction);
  }, [sendCmd,state.quick_seek_seconds,gestures.frameStepCount,invokeClickAction]);

  const flushSeek = useCallback((gesture: PointerGesture) => {
    if (gesture.target!==target || gesture.pendingX === 0 || !state.seekable || !state.duration) return;
    const seconds = Math.max(0,Math.min(state.duration,gesture.startTime+(gesture.lastX-gesture.startX)/12));
    gesture.pendingX = 0;
    gesture.targetTime=seconds;gesture.seekStarted=true;
    seek.change(seconds/state.duration*100);
  }, [seek.change,target,state.duration,state.seekable]);

  const activateDrag = useCallback((gesture: PointerGesture) => {
    if(gesture.target!==targetRef.current) return;
    if (gesture.dragging || gesture.dragAction === 'none') return;
    if (gesture.dragAction === 'seek' && (!state.seekable || !state.duration)) return;
    gesture.dragging = true;
    setGestureActive(true);
    if (gesture.dragAction === 'temporary_fast_forward') {
      gesture.fastForwarding = true;
      void sendCmd('set_rate', { rate: Math.max(.05, gestures.temporaryFastForwardSpeed) });
    }
  }, [gestures.temporaryFastForwardSpeed,sendCmd,state.seekable,state.duration]);

  const finishPointer = useCallback((event: React.PointerEvent<HTMLDivElement>, cancelled = false) => {
    const gesture = pointerGesture.current;
    if (!gesture || gesture.pointerId !== event.pointerId || !cancelled && gesture.button !== event.button) return;
    pointerGesture.current = null;
    if (gesture.timer !== null) window.clearTimeout(gesture.timer);
    const sameSource=gesture.target===target;
    if (!cancelled && sameSource && gesture.dragging && gesture.dragAction === 'seek') flushSeek(gesture);
    if (sameSource && gesture.seekStarted) seek.commitSeconds(gesture.targetTime);
    if (gesture.fastForwarding) void sendCmd('set_rate', { rate: Math.max(.05, gestures.playbackSpeed) });
    const intent=videoReleaseIntent(gesture,cancelled,insidePicture(event),sameSource);
    if (intent!=='none') {
      if (intent==='menu') {
        const box = event.currentTarget.getBoundingClientRect();
        setContextPoint({x:event.clientX - box.left,y:event.clientY - box.top}); setContextMenuOpen(true);
      } else {
        if (gesture.button === 3 || gesture.button === 4) invokeExtraAction(gesture.extraAction);
        else invokeClickAction(gesture.clickAction);
      }
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    pointerGesture.current = null;
    setGestureActive(false);
  }, [flushSeek,seek.commitSeconds,target,gestures.playbackSpeed,invokeClickAction,invokeExtraAction,insidePicture,sendCmd]);

  const pointerHandlers = target ? {
    onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => {
      if (!insidePicture(event)) return;
      if (pointerGesture.current || ![0, 1, 2, 3, 4].includes(event.button)) return;
      const baseDrag = event.button === 0
        ? (state.playing ? gestures.playingDragAction : gestures.pausedDragAction)
        : event.button === 1 ? gestures.middleHoldAction : event.button === 2 ? gestures.rightHoldAction : 'none';
      const baseClick = event.button === 0 ? gestures.clickPlayerToToggle ? 'play_pause' : 'none'
        : event.button === 1 ? gestures.middleClickAction : event.button === 2 ? gestures.rightClickAction : 'none';
      const policy = videoMousePolicy(event.button,event,gestures.mouseBindings,baseClick,baseDrag);
      const dragAction = policy.drag as MediaDragAction;
      setContextMenuOpen(false);
      const gesture: PointerGesture = {
        target,startTime:state.playback_time??0,targetTime:state.playback_time??0,seekStarted:false,
        pointerId: event.pointerId,
        button: event.button,
        startX: event.clientX,
        startY: event.clientY,
        moved: false,
        clickAction: policy.click as MediaClickAction,
        extraAction: extraMouseAction(gestures.mouseBindings?.video,event.button,event),
        forceMenu: policy.forceMenu,
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
      if (event.button !== 0 || event.ctrlKey || event.metaKey) event.preventDefault();
    },
    onPointerMove: (event: React.PointerEvent<HTMLDivElement>) => {
      const gesture = pointerGesture.current;
      if (!gesture || gesture.pointerId !== event.pointerId) return;
      if(gesture.target!==target){finishPointer(event,true);return;}
      gesture.moved ||= Math.hypot(event.clientX - gesture.startX,event.clientY - gesture.startY) >= 6;
      const delta = event.clientX - gesture.lastX;
      gesture.lastX = event.clientX;
      gesture.pendingX += delta;
      if (!gesture.dragging && gesture.moved) activateDrag(gesture);
      if (gesture.dragging && gesture.dragAction === 'seek' && performance.now() - gesture.lastSentAt >= 34) {
        gesture.lastSentAt = performance.now();
        flushSeek(gesture);
      }
      if (gesture.dragging) event.preventDefault();
    },
    onPointerUp: (event: React.PointerEvent<HTMLDivElement>) => finishPointer(event),
    onPointerCancel: (event: React.PointerEvent<HTMLDivElement>) => finishPointer(event, true),
    onLostPointerCapture: (event: React.PointerEvent<HTMLDivElement>) => { if (pointerGesture.current?.pointerId === event.pointerId) finishPointer(event, true); },
    onAuxClick: (event: React.MouseEvent<HTMLDivElement>) => event.preventDefault(),
    onContextMenu: (event: React.MouseEvent<HTMLDivElement>) => {
      event.preventDefault();
      // Keyboard Context Menu / Shift+F10 has no secondary-button gesture.
      if (event.button === 0 && !pointerGesture.current) {
        const box = event.currentTarget.getBoundingClientRect();
        setContextPoint({x:box.width/2,y:box.height/2}); setContextMenuOpen(true);
      }
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
      setContextMenuOpen(false);
      if (key === 'play_pause') invokeClickAction('play_pause');
      else if (key === 'mute') invokeClickAction('toggle_mute');
      else if (key === 'fullscreen') invokeClickAction('toggle_fullscreen');
      else if (key === 'information') void sendCmd('open_media_information');
    },
  };

  useEffect(() => setFallback(false), [source]);

  useEffect(() => {
    const cancel = () => {
      const gesture = pointerGesture.current;
      pointerGesture.current = null;
      if (!gesture) return;
      if (gesture.timer !== null) window.clearTimeout(gesture.timer);
      if (gesture.fastForwarding) void sendCmd('set_rate', { rate: Math.max(.05, gestures.playbackSpeed) });
      setGestureActive(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      const gesture = pointerGesture.current;
      if (gesture?.target === target && gesture.seekStarted) seekRef.current.commitSeconds(gesture.targetTime);
      cancel();
    };
    // useSeekbar commits its last preview on blur; this only cancels the hold.
    window.addEventListener('blur', cancel);
    window.addEventListener('keydown', escape);
    return () => {
      window.removeEventListener('blur', cancel); window.removeEventListener('keydown', escape);
      cancel();
    };
  }, [gestures.playbackSpeed, sendCmd,target]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video || fallback) return;
    const requested = state.playback_time ?? 0;
    if (Number.isFinite(requested) && Math.abs(video.currentTime - requested) > .65) {
      try { video.currentTime = requested; } catch { /* metadata may not be ready yet */ }
    }
    if (state.playing) void video.play().catch(() => undefined);
    else video.pause();
  }, [useFallback, state.playback_time, state.playing]);

  if (!target) {
    return <div className={`media-surface media-surface--empty ${className ?? ''}`}>
      <VideoCameraOutlined />
      <strong>{emptyLabel}</strong>
    </div>;
  }

  let surface: React.ReactElement;
  if (useFallback) {
    surface = <div className={`media-surface media-surface--fallback media-surface--interactive ${gestureActive ? 'is-gesture-active' : ''} ${className ?? ''}`} {...pointerHandlers}>
      <img src={`${apiBaseUrl}/api/player/frame?t=${fallbackStamp}`} alt={emptyLabel} />
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

  return <Dropdown trigger={[]} open={contextMenuOpen} onOpenChange={setContextMenuOpen}
    align={{points:['tl','tl'],offset:[contextPoint.x,contextPoint.y]}} menu={contextMenu}>{surface}</Dropdown>;
};
