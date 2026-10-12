import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { chapterAt } from './seekbar';
import { formatPreviewTime, hoveredSeekTime, previewPosition } from '../seekPreview';
import { tr, type UiLocale } from '../i18n';
import type { PlayerState } from './RemoteControlTab';

interface SeekThumbnailPreviewProps {
  /** Only the image is optional; one shared card always owns hover metadata. */
  enabled: boolean;
  seekable: boolean;
  live?: boolean;
  showSubseconds: boolean;
  locale: UiLocale;
  duration: number;
  mediaIdentity?: string | null;
  apiBaseUrl: string;
  unavailableLabel: string;
  className?: string;
  chapters?: PlayerState['chapters'];
  keyframes?: PlayerState['timeline_keyframes'];
  children: React.ReactNode;
}

interface HoverPosition {
  x: number;
  barTop: number;
  barBottom: number;
  seconds: number | null;
  chapter: string;
  keyframe: string;
}

export const SeekThumbnailPreview: React.FC<SeekThumbnailPreviewProps> = ({
  enabled, seekable, live, showSubseconds, locale, duration, mediaIdentity,
  apiBaseUrl, unavailableLabel, className, chapters = [], keyframes = [], children,
}) => {
  const [hover, setHover] = useState<HoverPosition | null>(null);
  const [requestedSecond, setRequestedSecond] = useState<number | null>(null);
  const [loadedSecond, setLoadedSecond] = useState<number | null>(null);
  const [failedSecond, setFailedSecond] = useState<number | null>(null);
  const [seeking, setSeeking] = useState(false);
  const card = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  const showImage = enabled && seekable && Boolean(mediaIdentity) && Boolean(hover && hover.seconds !== null);
  const second = hover?.seconds == null ? null : Math.floor(hover.seconds);

  useEffect(() => {
    setHover(null); setRequestedSecond(null); setLoadedSecond(null); setFailedSecond(null); setSeeking(false);
  }, [mediaIdentity]);

  useEffect(() => {
    // Don't leave a detached card at the old seekbar after scrolling/resizing.
    const clear = () => setHover(null);
    window.addEventListener('scroll', clear, true);
    window.addEventListener('resize', clear);
    window.addEventListener('blur', clear);
    return () => {
      window.removeEventListener('scroll', clear, true);
      window.removeEventListener('resize', clear);
      window.removeEventListener('blur', clear);
    };
  }, []);

  useEffect(() => {
    if (!seeking) return;
    const finishSeek = () => setSeeking(false);
    window.addEventListener('pointerup', finishSeek);
    window.addEventListener('pointercancel', finishSeek);
    return () => {
      window.removeEventListener('pointerup', finishSeek);
      window.removeEventListener('pointercancel', finishSeek);
    };
  }, [seeking]);

  useEffect(() => {
    if (!showImage || second === null) return;
    const timer = window.setTimeout(() => {
      setRequestedSecond(second); setLoadedSecond(null); setFailedSecond(null);
    }, 120);
    return () => window.clearTimeout(timer);
  }, [showImage, second, mediaIdentity]);

  useLayoutEffect(() => {
    if (!hover || !card.current) return;
    const rect = card.current.getBoundingClientRect();
    setPosition(previewPosition(hover.x, hover.barTop, hover.barBottom, rect.width, rect.height, window.innerWidth, window.innerHeight));
  }, [hover, showImage, locale, showSubseconds]);

  const thumbnailUrl = useMemo(() => {
    if (requestedSecond === null) return null;
    const query = new URLSearchParams({ seconds: String(requestedSecond), media: mediaIdentity || '' });
    return `${apiBaseUrl}/api/player/seek-thumbnail?${query.toString()}`;
  }, [apiBaseUrl, mediaIdentity, requestedSecond]);
  const previewReady = requestedSecond === second && loadedSecond === second;
  const previewFailed = requestedSecond === second && failedSecond === second;
  const reason = !mediaIdentity ? tr(locale, mediaIdentity === null ? 'Open media to seek.' : 'Duration is still being determined; seeking will become available when MPV reports a timeline.')
    : !(duration > 0) ? tr(locale, live ? 'This live or duration-less source has no fixed seek range.' : 'Duration is still being determined; seeking will become available when MPV reports a timeline.')
    : !seekable ? tr(locale, 'This media reports a duration but does not support seeking.') : '';

  const preview = !seeking && hover ? createPortal(
    <div ref={card} className="seek-thumbnail-popover" style={position} role="tooltip">
      {showImage && <div className="seek-thumbnail-popover__image">
        {thumbnailUrl && <img key={`${mediaIdentity || ''}:${requestedSecond}`} src={thumbnailUrl}
          className={previewReady ? 'is-ready' : ''} alt=""
          onLoad={(event) => { if (event.currentTarget.naturalWidth > 0) setLoadedSecond(requestedSecond); }}
          onError={() => setFailedSecond(requestedSecond)} />}
        {!previewReady && !previewFailed && <span className="seek-thumbnail-popover__loading" />}
        {previewFailed && <span className="seek-thumbnail-popover__unavailable">{unavailableLabel}</span>}
      </div>}
      {hover.seconds !== null && <strong className="seek-thumbnail-popover__caption">{formatPreviewTime(hover.seconds, showSubseconds, duration >= 3600)}</strong>}
      {hover.chapter && <span className="seek-thumbnail-popover__detail">{hover.chapter}</span>}
      {hover.keyframe && <span className="seek-thumbnail-popover__detail">{hover.keyframe}</span>}
      {reason && <span className="seek-thumbnail-popover__detail">{reason}</span>}
    </div>, document.body,
  ) : null;

  const moveRaf = useRef<number | null>(null);

  useEffect(() => () => {
    if (moveRaf.current !== null) {
      window.cancelAnimationFrame(moveRaf.current);
      moveRaf.current = null;
    }
  }, []);

  return <div className={`seek-thumbnail-host${className ? ` ${className}` : ''}`}
    onPointerDown={(event) => {
      if (event.button === 0) {
        if (moveRaf.current !== null) {
          window.cancelAnimationFrame(moveRaf.current);
          moveRaf.current = null;
        }
        setSeeking(true);
        setHover(null);
      }
    }}
    onPointerMove={(event) => {
      if (seeking || event.buttons !== 0 || event.pointerType === 'touch') return;
      const clientX = event.clientX;
      const currentTarget = event.currentTarget;
      if (moveRaf.current !== null) return;
      moveRaf.current = window.requestAnimationFrame(() => {
        moveRaf.current = null;
        const bounds = currentTarget.getBoundingClientRect();
        const rail = currentTarget.querySelector('.ant-slider-rail')?.getBoundingClientRect() ?? bounds;
        const rtl = window.getComputedStyle(currentTarget).direction === 'rtl';
        const seconds = hoveredSeekTime(rail.left, rail.width, clientX, duration, rtl);
        const chapter = seconds === null ? null : chapterAt(chapters, seconds);
        const nearest = seconds === null ? null : keyframes.filter((marker) => marker.time_ms >= 0 && marker.time_ms / 1000 <= duration)
          .map((marker) => ({ marker, distance: Math.abs((rtl ? 1 - marker.time_ms / 1000 / duration : marker.time_ms / 1000 / duration) * rail.width - (clientX - rail.left)) }))
          .filter((entry) => entry.distance <= 6).sort((a, b) => a.distance - b.distance)[0]?.marker;
        setHover({ x: clientX, barTop: bounds.top, barBottom: bounds.bottom, seconds,
          chapter: chapter ? `${tr(locale, 'Chapter')} ${chapter.index + 1}${chapter.title.trim() ? ` · ${chapter.title}` : ''}` : '',
          keyframe: nearest ? `${tr(locale, 'Keyframe')}${nearest.label ? ` · ${nearest.label}` : ''}` : '' });
      });
    }}
    onPointerLeave={() => {
      if (moveRaf.current !== null) {
        window.cancelAnimationFrame(moveRaf.current);
        moveRaf.current = null;
      }
      setHover(null);
    }}>
    {children}{preview}
  </div>;
};
