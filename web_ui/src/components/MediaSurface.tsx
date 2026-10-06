import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  BookOutlined,
  FastForwardOutlined,
  InfoCircleOutlined,
  MutedOutlined,
  PauseOutlined,
  PlayCircleOutlined,
  SoundOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';

interface MediaSurfaceProps {
  state: PlayerState;
  apiBaseUrl: string;
  emptyLabel: string;
  className?: string;
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
export const MediaSurface: React.FC<MediaSurfaceProps> = ({ state, apiBaseUrl, emptyLabel, className }) => {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const [fallback, setFallback] = useState(false);
  const target = state.current_video ?? '';
  const source = useMemo(() => target ? mediaSource(target, apiBaseUrl) : '', [target, apiBaseUrl]);

  useEffect(() => setFallback(false), [source]);

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

  if (fallback) {
    const stamp = Math.max(0, Math.round((state.playback_time ?? 0) * 1000));
    return <div className={`media-surface media-surface--fallback ${className ?? ''}`}>
      <img src={`${apiBaseUrl}/api/player/frame?t=${stamp}`} alt={emptyLabel} />
      <OsdOverlay state={state} />
    </div>;
  }

  return <div className={`media-surface ${className ?? ''}`}>
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
};
