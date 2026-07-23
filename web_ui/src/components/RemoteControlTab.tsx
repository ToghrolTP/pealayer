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

const { Title, Text } = Typography;

export interface PlayerState {
  status?: string;
  playing?: boolean;
  volume?: number;
  playback_time?: number;
  duration?: number;
  current_video?: string | null;
}

interface RemoteControlTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, any>) => void;
  onOpenLibraryTab?: () => void;
}

export const RemoteControlTab: React.FC<RemoteControlTabProps> = ({
  state,
  sendCmd,
  onOpenLibraryTab,
}) => {
  const [frameTimestamp, setFrameTimestamp] = useState<number>(Date.now());
  const [isMuted, setIsMuted] = useState<boolean>(false);
  const [previousVolume, setPreviousVolume] = useState<number>(100);

  useEffect(() => {
    if (state.playing) {
      const timer = setInterval(() => {
        setFrameTimestamp(Date.now());
      }, 1000);
      return () => clearInterval(timer);
    }
  }, [state.playing]);

  const formatTime = (sec: number = 0) => {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  const videoName = state.current_video
    ? state.current_video.split('/').pop()?.split('\\').pop() || 'Untitled'
    : 'No Media Playing';

  const handleSeek = (val: number) => {
    if (state.duration && state.duration > 0) {
      const targetSec = (val / 100) * state.duration;
      sendCmd('seek', { seconds: targetSec - (state.playback_time || 0) });
      message.info(`Seeked to ${formatTime(targetSec)}`);
    }
  };

  const handleVolumeChange = (val: number) => {
    sendCmd('set_volume', { level: val });
    if (val === 0) setIsMuted(true);
    else setIsMuted(false);
  };

  const toggleMute = () => {
    if (isMuted) {
      sendCmd('set_volume', { level: previousVolume || 100 });
      setIsMuted(false);
      message.info(`Volume unmuted to ${Math.round(previousVolume || 100)}%`);
    } else {
      setPreviousVolume(state.volume || 100);
      sendCmd('set_volume', { level: 0 });
      setIsMuted(true);
      message.info('Volume muted');
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
            background: '#161b26',
            borderRadius: 16,
            boxShadow: '0 20px 40px rgba(0, 0, 0, 0.5)',
            border: '1px solid rgba(255, 255, 255, 0.08)',
          }}
          bodyStyle={{ padding: 24 }}
        >
          {/* Video Frame Snapshot Preview */}
          <div
            style={{
              width: '100%',
              aspectRatio: '16/9',
              background: '#090d16',
              borderRadius: 12,
              overflow: 'hidden',
              border: '1px solid rgba(255, 255, 255, 0.08)',
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
                alt="Video Preview"
                style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                onError={(e) => {
                  (e.target as HTMLElement).style.display = 'none';
                }}
              />
            ) : (
              <Space direction="vertical" align="center">
                <VideoCameraOutlined style={{ fontSize: 48, color: '#475569' }} />
                <Text type="secondary" style={{ fontSize: 14 }}>
                  No Media Active
                </Text>
                {onOpenLibraryTab && (
                  <Button type="primary" size="small" onClick={onOpenLibraryTab} style={{ marginTop: 8 }}>
                    Browse Media Library
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
              <Card size="small" style={{ background: '#0e121b', border: '1px solid rgba(255,255,255,0.05)' }}>
                <Statistic
                  title={<Text type="secondary" style={{ fontSize: 12 }}>Time</Text>}
                  value={formatTime(state.playback_time)}
                  prefix={<FieldTimeOutlined style={{ color: '#e11d48' }} />}
                  valueStyle={{ fontSize: 16, color: '#f8fafc', fontFamily: 'monospace' }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small" style={{ background: '#0e121b', border: '1px solid rgba(255,255,255,0.05)' }}>
                <Statistic
                  title={<Text type="secondary" style={{ fontSize: 12 }}>Status</Text>}
                  value={state.playing ? 'Playing' : 'Paused'}
                  valueStyle={{ fontSize: 16, color: state.playing ? '#22c55e' : '#f59e0b' }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small" style={{ background: '#0e121b', border: '1px solid rgba(255,255,255,0.05)' }}>
                <Statistic
                  title={<Text type="secondary" style={{ fontSize: 12 }}>Volume</Text>}
                  value={Math.round(state.volume || 100)}
                  suffix="%"
                  prefix={<SoundOutlined style={{ color: '#e11d48' }} />}
                  valueStyle={{ fontSize: 16, color: '#f8fafc', fontFamily: 'monospace' }}
                />
              </Card>
            </Col>
          </Row>

          {/* Seek Slider */}
          <div style={{ marginBottom: 24, padding: '0 4px' }}>
            <Slider
              value={seekPercent}
              onChange={handleSeek}
              tooltip={{ formatter: (val) => `${val?.toFixed(0)}%` }}
              trackStyle={{ backgroundColor: '#e11d48' }}
              handleStyle={{ borderColor: '#e11d48', backgroundColor: '#e11d48' }}
            />
          </div>

          {/* Control Buttons */}
          <Row justify="center" align="middle" gutter={24} style={{ marginBottom: 24 }}>
            <Col>
              <Tooltip title="Seek -10s">
                <Button
                  shape="circle"
                  size="large"
                  icon={<FastBackwardOutlined />}
                  onClick={() => {
                    sendCmd('seek', { seconds: -10 });
                    message.info('Seeked -10 seconds');
                  }}
                  style={{ background: '#1e293b', borderColor: '#334155', color: '#f8fafc' }}
                />
              </Tooltip>
            </Col>
            <Col>
              <Tooltip title={state.playing ? 'Pause' : 'Play'}>
                <Button
                  shape="circle"
                  style={{
                    width: 64,
                    height: 64,
                    background: '#e11d48',
                    borderColor: '#e11d48',
                    color: '#fff',
                    boxShadow: '0 8px 24px rgba(225, 29, 72, 0.4)',
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
                    message.success(state.playing ? 'Paused' : 'Playing');
                  }}
                />
              </Tooltip>
            </Col>
            <Col>
              <Tooltip title="Seek +10s">
                <Button
                  shape="circle"
                  size="large"
                  icon={<FastForwardOutlined />}
                  onClick={() => {
                    sendCmd('seek', { seconds: 10 });
                    message.info('Seeked +10 seconds');
                  }}
                  style={{ background: '#1e293b', borderColor: '#334155', color: '#f8fafc' }}
                />
              </Tooltip>
            </Col>
          </Row>

          {/* Volume Control */}
          <div
            style={{
              background: 'rgba(255, 255, 255, 0.03)',
              padding: '12px 18px',
              borderRadius: 12,
              border: '1px solid rgba(255, 255, 255, 0.08)',
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
                  value={isMuted ? 0 : state.volume || 100}
                  onChange={handleVolumeChange}
                  trackStyle={{ backgroundColor: '#e11d48' }}
                  handleStyle={{ borderColor: '#e11d48', backgroundColor: '#e11d48' }}
                />
              </Col>
              <Col>
                <Text style={{ fontFamily: 'monospace', color: '#cbd5e1', width: 45, display: 'inline-block', textAlign: 'right' }}>
                  {isMuted ? '0%' : `${Math.round(state.volume || 100)}%`}
                </Text>
              </Col>
            </Row>
          </div>
        </Card>
      </Col>
    </Row>
  );
};
