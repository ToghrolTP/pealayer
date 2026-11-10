import React from 'react';
import { Button, Slider, Tooltip } from 'antd';
import { MutedOutlined, SoundOutlined } from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

// Both workspaces use the same authoritative volume/mute controls. Keep the
// stored level visible while muted so unmuting does not silently reset it.
export const VolumeControl = React.memo(function VolumeControl({ state, sendCmd, locale, className = '', showMute = true }: {
  state: PlayerState;
  sendCmd: (command: string, extra?: Record<string, unknown>) => void;
  locale: UiLocale;
  className?: string;
  showMute?: boolean;
}) {
  const label = tr(locale, state.muted ? 'Unmute' : 'Mute');
  return (
    <div className={`media-volume ${className}`}>
      {showMute && <Tooltip title={label}>
        <Button
          type="text"
          className={`transport-action ${state.muted ? 'transport-action--pause' : ''}`}
          aria-pressed={Boolean(state.muted)}
          aria-label={label}
          disabled={!state.current_video || state.muted === undefined}
          icon={state.muted || state.volume === 0 ? <MutedOutlined /> : <SoundOutlined />}
          onClick={() => sendCmd('set_mute', { muted: !state.muted })}
        />
      </Tooltip>}
      <Slider
        aria-label={tr(locale, 'Volume')}
        min={0}
        max={130}
        value={state.volume ?? 0}
        disabled={!state.current_video || state.volume === undefined}
        tooltip={{ formatter: (value) => `${value}%` }}
        onChange={(value) => sendCmd('set_volume', { value })}
      />
      <output title={tr(locale, 'Volume')}>
        {state.volume === undefined ? '—' : `${Math.round(state.volume)}%`}
      </output>
    </div>
  );
}, (prev, next) => (
  prev.locale === next.locale
  && prev.className === next.className
  && prev.sendCmd === next.sendCmd
  && prev.state.muted === next.state.muted
  && prev.state.volume === next.state.volume
  && prev.state.current_video === next.state.current_video
));
