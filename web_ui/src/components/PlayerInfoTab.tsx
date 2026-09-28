import React from 'react';
import { Card, Descriptions, Tag, Row, Col, Statistic, Typography, Space } from 'antd';
import {
  InfoCircleOutlined,
  DesktopOutlined,
  ApiOutlined,
  CodeOutlined,
  SoundOutlined,
} from '@ant-design/icons';
import { PlayerState } from './RemoteControlTab';
import { RuntimeConfig } from '../App';

const { Title, Text } = Typography;

interface PlayerInfoTabProps {
  state: PlayerState;
  connectionMode: 'ws' | 'http';
  runtime: RuntimeConfig | null;
}

export const PlayerInfoTab: React.FC<PlayerInfoTabProps> = ({ state, connectionMode, runtime }) => {
  const formatTime = (sec: number = 0) => {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
      {/* System Statistics */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Connection Type</Text>}
              value={connectionMode.toUpperCase()}
              prefix={<ApiOutlined style={{ color: '#53a2be' }} />}
              valueStyle={{ color: '#53a2be', fontSize: 18, fontWeight: 700 }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Playback Duration</Text>}
              value={formatTime(state.duration)}
              prefix={<DesktopOutlined style={{ color: '#1d84b5' }} />}
              valueStyle={{ color: '#f8fafc', fontSize: 18, fontFamily: 'monospace' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Volume Level</Text>}
              value={state.volume === undefined ? '—' : Math.round(state.volume)}
              suffix="%"
              prefix={<SoundOutlined style={{ color: '#53a2be' }} />}
              valueStyle={{ color: '#f8fafc', fontSize: 18, fontFamily: 'monospace' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#0a2239', border: '1px solid rgba(23, 96, 135, 0.2)', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Player Engine</Text>}
              value="MPV Core"
              prefix={<CodeOutlined style={{ color: '#22c55e' }} />}
              valueStyle={{ color: '#22c55e', fontSize: 18 }}
            />
          </Card>
        </Col>
      </Row>

      {/* System & Session Descriptions */}
      <Card
        bordered={false}
        title={
          <Space>
            <InfoCircleOutlined style={{ color: '#1d84b5' }} />
            <Title level={4} style={{ margin: 0, color: '#f8fafc' }}>
              System & Media Metadata
            </Title>
          </Space>
        }
        style={{
          background: '#132e32',
          borderRadius: 16,
          border: '1px solid rgba(23, 96, 135, 0.3)',
        }}
      >
        <Descriptions bordered column={{ xs: 1, sm: 1, md: 2 }} size="middle">
          <Descriptions.Item label="Active Video Path">
            <Text style={{ fontFamily: 'monospace', color: '#cbd5e1', wordBreak: 'break-all' }}>
              {state.current_video || 'None'}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label="Playback Status">
            {state.playing ? (
              <Tag color="success">Active Playback</Tag>
            ) : (
              <Tag color="warning">Paused / Idle</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="HTTP Remote Endpoint">
            <Text code>{window.location.origin}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="WebSocket Remote Endpoint">
            <Text code>{runtime ? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.hostname}:${runtime.wsPort}` : 'Initializing…'}</Text>
          </Descriptions.Item>
          <Descriptions.Item label={`${runtime?.appName || 'Application'} Version`}>
            <Text style={{ color: '#f8fafc' }}>{runtime?.version || 'Initializing…'}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Hardware Acceleration">
            <Tag color="blue">mpv libmpv2 render</Tag>
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  );
};
