import React, { useEffect, useMemo, useState } from 'react';

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
  second: number;
}

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

  return (
    <div
      className={`seek-thumbnail-host${className ? ` ${className}` : ''}`}
      onPointerMove={(event) => {
        if (!enabled || !Number.isFinite(duration) || duration <= 0) return;
        const bounds = event.currentTarget.getBoundingClientRect();
        if (bounds.width <= 0) return;
        const fraction = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
        const second = Math.floor(Math.min(duration * fraction, Math.max(0, duration - 0.001)));
        const halfPreview = 106;
        const x = Math.min(
          Math.max(event.clientX - bounds.left, halfPreview),
          Math.max(halfPreview, bounds.width - halfPreview),
        );
        setHover((current) => current?.second === second && current.x === x ? current : { x, second });
      }}
      onPointerLeave={() => setHover(null)}
    >
      {children}
      {enabled && hover && (
        <div className="seek-thumbnail-popover" style={{ left: hover.x }} role="status" aria-live="polite">
          <div className="seek-thumbnail-popover__image">
            {thumbnailUrl && (
              <img
                key={`${mediaIdentity || ''}:${requestedSecond}`}
                src={thumbnailUrl}
                className={previewReady ? 'is-ready' : ''}
                alt=""
                onLoad={() => setLoadedSecond(requestedSecond)}
                onError={() => setFailedSecond(requestedSecond)}
              />
            )}
            {!previewReady && !previewFailed && <span className="seek-thumbnail-popover__loading" />}
            {previewFailed && <span className="seek-thumbnail-popover__unavailable">{unavailableLabel}</span>}
          </div>
          <strong>{formatPreviewTime(hover.second)}</strong>
        </div>
      )}
    </div>
  );
};
