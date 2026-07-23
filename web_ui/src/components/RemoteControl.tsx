import React, { useEffect, useState, useRef } from 'react';
import { Card, Typography, Slider, Button, Tag, Space, Row, Col, Layout } from 'antd';
import {
  PlayCircleFilled,
  PauseCircleFilled,
  FastBackwardOutlined,
  FastForwardOutlined,
  SoundOutlined,
  VideoCameraOutlined,
  CheckCircleOutlined,
  DisconnectOutlined,
  SyncOutlined,
} from '@ant-design/icons';

const { Title, Text } = Typography;
const { Header, Content } = Layout;

interface PlayerState {
  status?: string;
  playing?: boolean;
  volume?: number;
  playback_time?: number;
  duration?: number;
  current_video?: string | null;
}

export const RemoteControl: React.FC = () => {
  const [state, setState] = useState<PlayerState>({
    playing: false,
    volume: 100,
    playback_time: 0,
    duration: 0,
    current_video: null,
  });
  const [connected, setConnected] = useState<boolean>(false);
  const [connectionMode, setConnectionMode] = useState<'ws' | 'http'>('http');
  const [frameTimestamp, setFrameTimestamp] = useState<number>(Date.now());
  const wsRef = useRef<WebSocket | null>(null);

  const sendCmd = (command: string, payload: Record<string, any> = {}) => {
    const body = JSON.stringify({ command, ...payload });
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(body);
    }
    fetch('/api/player/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    }).catch(() => {});
  };

  useEffect(() => {
    const connectWS = () => {
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${proto}//${window.location.hostname}:8081`;

      try {
        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
          setConnected(true);
          setConnectionMode('ws');
        };

        ws.onclose = () => {
          setConnectionMode('http');
          setTimeout(connectWS, 3000);
        };

        ws.onmessage = (ev) => {
          try {
            const data = JSON.parse(ev.data);
            setState((prev) => ({ ...prev, ...data }));
            setConnected(true);
          } catch {}
        };
      } catch {
        setConnectionMode('http');
      }
    };

    connectWS();

    const httpInterval = setInterval(async () => {
      try {
        const res = await fetch('/api/player/status');
        if (res.ok) {
          const data = await res.json();
          setState((prev) => ({ ...prev, ...data }));
          setConnected(true);
        }
      } catch {
        if (connectionMode === 'http') setConnected(false);
      }
    }, 500);

    return () => {
      clearInterval(httpInterval);
      if (wsRef.current) wsRef.current.close();
    };
  }, []);

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
    }
  };

  const handleVolumeChange = (val: number) => {
    sendCmd('set_volume', { level: val });
    setState((prev) => ({ ...prev, volume: val }));
  };

  const seekPercent =
    state.duration && state.duration > 0
      ? Number((( (state.playback_time || 0) / state.duration) * 100).toFixed(1))
      : 0;

  return (
    <Layout style={{ minHeight: '100vh', background: '#0b0c10' }}>
      <Header
        style={{
          background: 'rgba(15, 23, 42, 0.8)',
          backdropFilter: 'blur(12px)',
          borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
          padding: '0 24px',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          position: 'sticky',
          top: 0,
          zIndex: 100,
        }}
      >
        <Space size="middle">
          <VideoCameraOutlined style={{ fontSize: 24, color: '#e11d48' }} />
          <Title level={4} style={{ margin: 0, color: '#f8fafc', fontWeight: 700 }}>
            Pealayer Control Center
          </Title>
        </Space>
        {connected ? (
          <Tag
            icon={connectionMode === 'ws' ? <SyncOutlined spin /> : <CheckCircleOutlined />}
            color="success"
            style={{ borderRadius: 12, padding: '2px 10px', fontSize: 12 }}
          >
            {connectionMode === 'ws' ? 'WebSocket Live' : 'HTTP Connected'}
          </Tag>
        ) : (
          <Tag icon={<DisconnectOutlined />} color="error" style={{ borderRadius: 12, padding: '2px 10px', fontSize: 12 }}>
            Offline
          </Tag>
        )}
      </Header>

      <Content style={{ padding: '32px 16px', display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Card
          style={{
            width: '100%',
            maxWidth: 620,
            background: '#151922',
            borderColor: 'rgba(255, 255, 255, 0.12)',
            borderRadius: 16,
            boxShadow: '0 20px 40px rgba(0, 0, 0, 0.6)',
          }}
          bodyStyle={{ padding: 24 }}
        >
          {/* Video Preview Frame */}
          <div
            style={{
              width: '100%',
              aspectRatio: '16/9',
              background: '#07090e',
              borderRadius: 12,
              overflow: 'hidden',
              border: '1px solid rgba(255,255,255,0.08)',
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
                alt="Video Frame"
                style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                onError={(e) => {
                  (e.target as HTMLElement).style.display = 'none';
                }}
              />
            ) : (
              <Space direction="vertical" align="center">
                <VideoCameraOutlined style={{ fontSize: 48, color: '#475569' }} />
                <Text type="secondary">No Video File Loaded</Text>
              </Space>
            )}
          </div>

          {/* Video Info */}
          <div style={{ textAlign: 'center', marginBottom: 16 }}>
            <Title level={4} style={{ color: '#f8fafc', marginBottom: 4 }} ellipsis={{ tooltip: videoName }}>
              {videoName}
            </Title>
            <Text type="secondary" style={{ fontFamily: 'monospace', fontSize: 14 }}>
              {formatTime(state.playback_time)} / {formatTime(state.duration)}
            </Text>
          </div>

          {/* Seek Slider */}
          <div style={{ marginBottom: 24, padding: '0 8px' }}>
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
              <Button
                shape="circle"
                size="large"
                icon={<FastBackwardOutlined />}
                onClick={() => sendCmd('seek', { seconds: -10 })}
                style={{ background: '#1e293b', borderColor: '#334155', color: '#f8fafc' }}
                title="Seek -10s"
              />
            </Col>
            <Col>
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
                onClick={() => sendCmd('toggle_pause')}
                title={state.playing ? 'Pause' : 'Play'}
              />
            </Col>
            <Col>
              <Button
                shape="circle"
                size="large"
                icon={<FastForwardOutlined />}
                onClick={() => sendCmd('seek', { seconds: 10 })}
                style={{ background: '#1e293b', borderColor: '#334155', color: '#f8fafc' }}
                title="Seek +10s"
              />
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
                <SoundOutlined style={{ color: '#94a3b8', fontSize: 18 }} />
              </Col>
              <Col flex="auto">
                <Slider
                  min={0}
                  max={130}
                  value={state.volume || 100}
                  onChange={handleVolumeChange}
                  trackStyle={{ backgroundColor: '#e11d48' }}
                  handleStyle={{ borderColor: '#e11d48', backgroundColor: '#e11d48' }}
                />
              </Col>
              <Col>
                <Text style={{ fontFamily: 'monospace', color: '#cbd5e1', width: 45, display: 'inline-block', textAlign: 'right' }}>
                  {Math.round(state.volume || 100)}%
                </Text>
              </Col>
            </Row>
          </div>
        </Card>
      </Content>
    </Layout>
  );
};
