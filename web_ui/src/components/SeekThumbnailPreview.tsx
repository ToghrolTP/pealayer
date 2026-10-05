import React, { useEffect, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';

interface SeekThumbnailPreviewProps {
  enabled: boolean;
  duration: number;
  mediaIdentity?: string | null;
  apiBaseUrl: string;
  unavailableLabel: string;
  className?: string;
  children: React.ReactNode;
}

interface HoverPosition {
  x: number;
  bottom: number;
  second: number;
}

const PREVIEW_WIDTH = 174;
const PREVIEW_EDGE_GAP = 8;

const formatPreviewTime = (seconds: number) => {
  const whole = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const remainder = whole % 60;
  return `${hours > 0 ? `${String(hours).padStart(2, '0')}:` : ''}${String(minutes).padStart(2, '0')}:${String(remainder).padStart(2, '0')}`;
};

export const SeekThumbnailPreview: React.FC<SeekThumbnailPreviewProps> = ({
  enabled,
  duration,
  mediaIdentity,
  apiBaseUrl,
  unavailableLabel,
  className,
  children,
}) => {
  const [hover, setHover] = useState<HoverPosition | null>(null);
  const [requestedSecond, setRequestedSecond] = useState<number | null>(null);
  const [loadedSecond, setLoadedSecond] = useState<number | null>(null);
  const [failedSecond, setFailedSecond] = useState<number | null>(null);

  useEffect(() => {
    setHover(null);
    setRequestedSecond(null);
    setLoadedSecond(null);
    setFailedSecond(null);
  }, [mediaIdentity]);

  useEffect(() => {
    if (!enabled || !hover) return;
    const timer = window.setTimeout(() => {
      setRequestedSecond(hover.second);
      setLoadedSecond(null);
      setFailedSecond(null);
    }, 120);
    return () => window.clearTimeout(timer);
  }, [enabled, hover?.second, mediaIdentity]);

  const thumbnailUrl = useMemo(() => {
    if (requestedSecond === null) return null;
    const query = new URLSearchParams({
      seconds: String(requestedSecond),
      media: mediaIdentity || '',
    });
    return `${apiBaseUrl}/api/player/seek-thumbnail?${query.toString()}`;
  }, [apiBaseUrl, mediaIdentity, requestedSecond]);

  const previewReady = hover && requestedSecond === hover.second && loadedSecond === hover.second;
  const previewFailed = hover && requestedSecond === hover.second && failedSecond === hover.second;

  const preview = enabled && hover ? createPortal(
    <div
      className="seek-thumbnail-popover"
      style={{ left: hover.x, bottom: hover.bottom }}
      role="status"
      aria-live="polite"
    >
      <div className="seek-thumbnail-popover__image">
        {thumbnailUrl && (
          <img
            key={`${mediaIdentity || ''}:${requestedSecond}`}
            src={thumbnailUrl}
            className={previewReady ? 'is-ready' : ''}
            alt=""
            onLoad={(event) => {
              if (event.currentTarget.naturalWidth > 0) setLoadedSecond(requestedSecond);
            }}
            onError={() => setFailedSecond(requestedSecond)}
          />
        )}
        {!previewReady && !previewFailed && <span className="seek-thumbnail-popover__loading" />}
        {previewFailed && <span className="seek-thumbnail-popover__unavailable">{unavailableLabel}</span>}
      </div>
      <strong className="seek-thumbnail-popover__caption">{formatPreviewTime(hover.second)}</strong>
    </div>,
    document.body,
  ) : null;

  return (
    <div
      className={`seek-thumbnail-host${className ? ` ${className}` : ''}`}
      onPointerMove={(event) => {
        if (!enabled || !Number.isFinite(duration) || duration <= 0) return;
        const bounds = event.currentTarget.getBoundingClientRect();
        if (bounds.width <= 0) return;
        const fraction = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
        const second = Math.floor(Math.min(duration * fraction, Math.max(0, duration - 0.001)));
        const halfPreview = PREVIEW_WIDTH / 2;
        const x = Math.min(
          Math.max(event.clientX, halfPreview + PREVIEW_EDGE_GAP),
          Math.max(halfPreview + PREVIEW_EDGE_GAP, window.innerWidth - halfPreview - PREVIEW_EDGE_GAP),
        );
        const bottom = Math.max(PREVIEW_EDGE_GAP, window.innerHeight - bounds.top + PREVIEW_EDGE_GAP);
        setHover((current) => (
          current?.second === second && current.x === x && current.bottom === bottom
            ? current
            : { x, bottom, second }
        ));
      }}
      onPointerLeave={() => setHover(null)}
    >
      {children}
      {preview}
    </div>
  );
};
