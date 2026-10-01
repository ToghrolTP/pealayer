import React, { useEffect, useState } from 'react';
import { Card, Typography, Slider, Button, Row, Col, Statistic, Space, Tooltip, message } from 'antd';
import {
  PlayCircleFilled,
  PauseCircleFilled,
  FastBackwardOutlined,
  FastForwardOutlined,
  SoundOutlined,
  MutedOutlined,
  VideoCameraOutlined,
  FieldTimeOutlined,
} from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

const { Title, Text } = Typography;

export interface PlayerState {
  status?: string;
  playing?: boolean;
  volume?: number;
  playback_time?: number;
  duration?: number;
  current_video?: string | null;
  seekable?: boolean;
  live?: boolean;
  muted?: boolean;
  playback_rate?: number;
  fullscreen?: boolean;
  workspace?: string;
  controller_connected?: boolean;
  hardware_connected?: boolean;
  hardware?: {
    board_name?: string;
    relay_count?: number;
    pwm_count?: number;
    supports_rf_transmit?: boolean;
    supports_addressable_led?: boolean;
    supports_segment_display?: boolean;
    supports_lcd_display?: boolean;
  } | null;
  recording?: boolean;
  recording_armed?: boolean;
  recordable_track_count?: number;
  effects?: Array<{
    id: string;
    name: string;
    duration_ms: number;
    action_count: number;
    target: string;
  }>;
  cues?: Array<{
    id: string;
    effect_id: string;
    name: string;
    start_time_ms: number;
    duration_ms: number;
  }>;
}

interface RemoteControlTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, any>) => void;
  onOpenLibraryTab?: () => void;
  locale: UiLocale;
}

export const RemoteControlTab: React.FC<RemoteControlTabProps> = ({
  state,
  sendCmd,
  onOpenLibraryTab,
  locale,
}) => {
  const [frameTimestamp, setFrameTimestamp] = useState<number>(Date.now());
  const [isMuted, setIsMuted] = useState<boolean>(false);
  const [previousVolume, setPreviousVolume] = useState<number | undefined>();

  useEffect(() => {
    if (state.playing) {
      const timer = setInterval(() => {
        setFrameTimestamp(Date.now());
      }, 1000);
      return () => clearInterval(timer);
    }
  }, [state.playing]);

  const formatTime = (sec?: number) => {
    if (sec === undefined) return '—';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  const videoName = state.current_video
    ? state.current_video.split('/').pop()?.split('\\').pop() || tr(locale, 'Untitled')
    : state.current_video === null
      ? tr(locale, 'No Media Playing')
      : tr(locale, 'Initializing…');

  const [isDraggingSeek, setIsDraggingSeek] = useState<boolean>(false);
  const [dragSeekVal, setDragSeekVal] = useState<number>(0);

  const handleSeekChange = (val: number) => {
    setIsDraggingSeek(true);
    setDragSeekVal(val);
  };

  const handleSeekAfterChange = (val: number) => {
    setIsDraggingSeek(false);
    sendCmd('seek_abs', { percentage: val });
    if (state.duration && state.duration > 0) {
      const targetSec = (val / 100) * state.duration;
      message.info(`${tr(locale, 'Seeked to')} ${formatTime(targetSec)}`);
    }
  };

  const handleVolumeChange = (val: number) => {
    sendCmd('set_volume', { level: val });
    if (val === 0) setIsMuted(true);
    else setIsMuted(false);
  };

  const toggleMute = () => {
    if (isMuted) {
      if (previousVolume === undefined) return;
      sendCmd('set_volume', { level: previousVolume });
      setIsMuted(false);
      message.info(`${tr(locale, 'Volume unmuted to')} ${Math.round(previousVolume)}%`);
    } else {
      if (state.volume === undefined) return;
      setPreviousVolume(state.volume);
      sendCmd('set_volume', { level: 0 });
      setIsMuted(true);
      message.info(tr(locale, 'Volume muted'));
    }
  };

  const seekPercent =
    state.duration && state.duration > 0
      ? Number((( (state.playback_time || 0) / state.duration) * 100).toFixed(1))
      : 0;

  return (
    <Row justify="center" style={{ width: '100%' }}>
      <Col xs={24} sm={22} md={20} lg={16} xl={14}>
        <Card
          bordered={false}
          style={{
            background: '#132e32',
            borderRadius: 16,
            boxShadow: '0 20px 40px rgba(0, 0, 0, 0.5)',
            border: '1px solid rgba(23, 96, 135, 0.3)',
          }}
          bodyStyle={{ padding: 24 }}
        >
          {/* Video Frame Snapshot Preview */}
          <div
            style={{
              width: '100%',
              aspectRatio: '16/9',
              background: '#0a2239',
              borderRadius: 12,
              overflow: 'hidden',
              border: '1px solid rgba(23, 96, 135, 0.3)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              marginBottom: 20,
              position: 'relative',
            }}
          >
            {state.current_video ? (
              <img
                src={`/api/player/frame?t=${frameTimestamp}`}
                alt={tr(locale, 'Video Preview')}
                style={{ width: '100%', height: '100%', objectFit: 'contain' }}
                onError={(e) => {
                  (e.target as HTMLElement).style.display = 'none';
                }}
              />
            ) : (
              <Space direction="vertical" align="center">
                <VideoCameraOutlined style={{ fontSize: 48, color: '#53a2be' }} />
                <Text type="secondary" style={{ fontSize: 14 }}>
                  {state.current_video === null ? tr(locale, 'No Media Active') : tr(locale, 'Initializing…')}
                </Text>
                {onOpenLibraryTab && (
                  <Button type="primary" size="small" onClick={onOpenLibraryTab} style={{ marginTop: 8, backgroundColor: '#1d84b5' }}>
                    {tr(locale, 'Browse Media Library')}
                  </Button>
                )}
              </Space>
            )}
          </div>

          {/* Video Title */}
          <div style={{ textAlign: 'center', marginBottom: 20 }}>
            <Title level={4} style={{ color: '#f8fafc', marginBottom: 4 }} ellipsis={{ tooltip: videoName }}>
              {videoName}
            </Title>
            <Text type="secondary" style={{ fontFamily: 'monospace', fontSize: 14 }}>
              {formatTime(state.playback_time)} / {formatTime(state.duration)}
            </Text>
          </div>

          {/* Quick Statistics Row */}
          <Row gutter={16} style={{ marginBottom: 24, textAlign: 'center' }}>
            <Col span={8}>
              <Card size="small" style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)' }}>
                <Statistic
                  title={<Text type="secondary" style={{ fontSize: 12 }}>{tr(locale, 'Time')}</Text>}
                  value={formatTime(state.playback_time)}
                  prefix={<FieldTimeOutlined style={{ color: '#53a2be' }} />}
                  valueStyle={{ fontSize: 16, color: '#f8fafc', fontFamily: 'monospace' }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small" style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)' }}>
                <Statistic
                  title={<Text type="secondary" style={{ fontSize: 12 }}>{tr(locale, 'Status')}</Text>}
                  value={state.playing === true ? tr(locale, 'Playing') : state.playing === false ? tr(locale, 'Paused') : tr(locale, 'Initializing…')}
                  valueStyle={{ fontSize: 16, color: state.playing ? '#22c55e' : '#f59e0b' }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small" style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)' }}>
                <Statistic
                  title={<Text type="secondary" style={{ fontSize: 12 }}>{tr(locale, 'Volume')}</Text>}
                  value={state.volume === undefined ? '—' : Math.round(state.volume)}
                  suffix="%"
                  prefix={<SoundOutlined style={{ color: '#1d84b5' }} />}
                  valueStyle={{ fontSize: 16, color: '#f8fafc', fontFamily: 'monospace' }}
                />
              </Card>
            </Col>
          </Row>

          {/* Seek Slider */}
          <div style={{ marginBottom: 24, padding: '0 4px' }}>
            <Slider
              value={isDraggingSeek ? dragSeekVal : seekPercent}
              disabled={state.duration === undefined || state.playback_time === undefined}
              onChange={handleSeekChange}
              onAfterChange={handleSeekAfterChange}
              tooltip={{ formatter: (val) => `${val?.toFixed(0)}%` }}
              trackStyle={{ backgroundColor: '#1d84b5' }}
              handleStyle={{ borderColor: '#53a2be', backgroundColor: '#1d84b5' }}
            />
          </div>

          {/* Control Buttons */}
          <Row justify="center" align="middle" gutter={24} style={{ marginBottom: 24 }}>
            <Col>
              <Tooltip title={tr(locale, 'Seek -10s')}>
                <Button
                  shape="circle"
                  size="large"
                  icon={<FastBackwardOutlined />}
                  onClick={() => {
                    sendCmd('seek', { seconds: -10 });
                    message.info(tr(locale, 'Seeked -10 seconds'));
                  }}
                  style={{ background: '#0a2239', borderColor: '#176087', color: '#f8fafc' }}
                />
              </Tooltip>
            </Col>
            <Col>
              <Tooltip title={state.playing ? tr(locale, 'Pause') : tr(locale, 'Play')}>
                <Button
                  shape="circle"
                  style={{
                    width: 64,
                    height: 64,
                    background: '#1d84b5',
                    borderColor: '#1d84b5',
                    color: '#fff',
                    boxShadow: '0 8px 24px rgba(29, 132, 181, 0.4)',
                  }}
                  icon={
                    state.playing ? (
                      <PauseCircleFilled style={{ fontSize: 32 }} />
                    ) : (
                      <PlayCircleFilled style={{ fontSize: 32 }} />
                    )
                  }
                  onClick={() => {
                    sendCmd('toggle_pause');
                    message.success(state.playing ? tr(locale, 'Paused') : tr(locale, 'Playing'));
                  }}
                />
              </Tooltip>
            </Col>
            <Col>
              <Tooltip title={tr(locale, 'Seek +10s')}>
                <Button
                  shape="circle"
                  size="large"
                  icon={<FastForwardOutlined />}
                  onClick={() => {
                    sendCmd('seek', { seconds: 10 });
                    message.info(tr(locale, 'Seeked +10 seconds'));
                  }}
                  style={{ background: '#0a2239', borderColor: '#176087', color: '#f8fafc' }}
                />
              </Tooltip>
            </Col>
          </Row>

          {/* Volume Control */}
          <div
            style={{
              background: '#0a2239',
              padding: '12px 18px',
              borderRadius: 12,
              border: '1px solid rgba(23, 96, 135, 0.3)',
            }}
          >
            <Row align="middle" gutter={16}>
              <Col>
                <Button
                  type="text"
                  icon={isMuted || state.volume === 0 ? <MutedOutlined style={{ color: '#ef4444' }} /> : <SoundOutlined style={{ color: '#94a3b8' }} />}
                  onClick={toggleMute}
                  style={{ fontSize: 18 }}
                />
              </Col>
              <Col flex="auto">
                <Slider
                  min={0}
                  max={130}
                  value={isMuted ? 0 : (state.volume ?? 0)}
                  disabled={state.volume === undefined}
                  onChange={handleVolumeChange}
                  trackStyle={{ backgroundColor: '#e11d48' }}
                  handleStyle={{ borderColor: '#e11d48', backgroundColor: '#e11d48' }}
                />
              </Col>
              <Col>
                <Text style={{ fontFamily: 'monospace', color: '#cbd5e1', width: 45, display: 'inline-block', textAlign: 'right' }}>
                  {state.volume === undefined ? '—' : (isMuted ? '0%' : `${Math.round(state.volume)}%`)}
                </Text>
              </Col>
            </Row>
          </div>
        </Card>
      </Col>
    </Row>
  );
};
