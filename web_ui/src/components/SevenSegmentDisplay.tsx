import React from 'react';

const SEGMENTS = [
  ['a', 8, 5, 28, 5],
  ['b', 31, 8, 31, 27],
  ['c', 31, 32, 31, 51],
  ['d', 8, 54, 28, 54],
  ['e', 5, 32, 5, 51],
  ['f', 5, 8, 5, 27],
  ['g', 8, 29.5, 28, 29.5],
] as const;

interface SevenSegmentDisplayProps {
  segments: number[];
  brightness: number;
  active: boolean;
  blinking: boolean;
  label: string;
}

export const SevenSegmentDisplay: React.FC<SevenSegmentDisplayProps> = ({
  segments,
  brightness,
  active,
  blinking,
  label,
}) => {
  const masks = [...segments.slice(0, 4), ...Array(Math.max(0, 4 - segments.length)).fill(0)];
  const intensity = active ? Math.max(.2, Math.min(1, brightness / 7)) : .08;
  return <div
    className={`seven-segment-display${blinking ? ' is-blinking' : ''}`}
    aria-label={label}
    role="img"
    style={{ '--segment-intensity': intensity } as React.CSSProperties}
  >
    {masks.map((mask, digit) => <svg key={digit} viewBox="0 0 36 60" aria-hidden="true">
      {SEGMENTS.map(([name, x1, y1, x2, y2], bit) => <line
        key={name}
        className={(mask & (1 << bit)) !== 0 ? 'is-lit' : undefined}
        x1={x1}
        y1={y1}
        x2={x2}
        y2={y2}
      />)}
      <circle className={(mask & 0x80) !== 0 ? 'is-lit' : undefined} cx="32" cy="54" r="2.4" />
    </svg>)}
  </div>;
};
