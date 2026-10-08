import React from 'react';
import { AudioOutlined, StopOutlined, SubnodeOutlined, VideoCameraOutlined } from '@ant-design/icons';
import { Select, Tooltip } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';

type TrackKind = 'video' | 'audio' | 'subtitle';

interface MediaTrackSelectorsProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  locale: UiLocale;
  compact?: boolean;
}

const kinds: Array<{ kind: TrackKind; icon: React.ReactNode; label: string }> = [
  { kind: 'video', icon: <VideoCameraOutlined />, label: 'Video' },
  { kind: 'audio', icon: <AudioOutlined />, label: 'Audio' },
  { kind: 'subtitle', icon: <SubnodeOutlined />, label: 'Subtitles' },
];

const fallbackName = (kind: TrackKind, id: number) =>
  `${kind === 'subtitle' ? 'Subtitle' : kind[0].toUpperCase() + kind.slice(1)} ${id}`;

export const MediaTrackSelectors: React.FC<MediaTrackSelectorsProps> = ({
  state,
  sendCmd,
  locale,
  compact = false,
}) => (
  <div className={`media-track-selectors${compact ? ' media-track-selectors--compact' : ''}`}>
    {kinds.map(({ kind, icon, label }) => {
      const tracks = (state.media_tracks ?? []).filter((track) => track.kind === kind);
      const selected = tracks.find((track) => track.selected)?.id ?? 'none';
      const options = [
        {
          value: 'none',
          label: <span className="media-track-option"><StopOutlined /><span>{tr(locale, 'None')}</span></span>,
        },
        ...tracks.map((track) => {
          const title = track.title?.trim() || fallbackName(kind, track.id);
          const detail = [
            track.language,
            track.codec,
            track.is_default ? tr(locale, 'Default') : '',
            track.forced ? tr(locale, 'Forced') : '',
            track.external ? tr(locale, 'External') : '',
          ].filter(Boolean).join(' · ');
          return {
            value: track.id,
            label: <span className="media-track-option"><strong>{title}</strong>{detail && <small>{detail}</small>}</span>,
          };
        }),
      ];
      return <Tooltip key={kind} title={tr(locale, `${label} track`)}>
        <Select
          className="media-track-select"
          aria-label={tr(locale, `${label} track`)}
          prefix={icon}
          value={selected}
          disabled={!state.current_video || tracks.length === 0}
          options={options}
          optionLabelProp="label"
          popupMatchSelectWidth={compact ? 280 : true}
          onChange={(value) => value === 'none'
            ? sendCmd('pealayer.media.track.disable', { kind })
            : sendCmd('pealayer.media.track.select', { kind, id: value })}
        />
      </Tooltip>;
    })}
  </div>
);
