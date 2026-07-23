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

const { Title, Text } = Typography;

interface PlayerInfoTabProps {
  state: PlayerState;
  connectionMode: 'ws' | 'http';
}

export const PlayerInfoTab: React.FC<PlayerInfoTabProps> = ({ state, connectionMode }) => {
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
          <Card bordered={false} style={{ background: '#161b26', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Connection Type</Text>}
              value={connectionMode.toUpperCase()}
              prefix={<ApiOutlined style={{ color: '#38bdf8' }} />}
              valueStyle={{ color: '#38bdf8', fontSize: 18, fontWeight: 700 }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#161b26', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Playback Duration</Text>}
              value={formatTime(state.duration)}
              prefix={<DesktopOutlined style={{ color: '#e11d48' }} />}
              valueStyle={{ color: '#f8fafc', fontSize: 18, fontFamily: 'monospace' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#161b26', borderRadius: 12 }}>
            <Statistic
              title={<Text type="secondary">Volume Level</Text>}
              value={Math.round(state.volume || 100)}
              suffix="%"
              prefix={<SoundOutlined style={{ color: '#f59e0b' }} />}
              valueStyle={{ color: '#f8fafc', fontSize: 18, fontFamily: 'monospace' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} style={{ background: '#161b26', borderRadius: 12 }}>
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
            <InfoCircleOutlined style={{ color: '#e11d48' }} />
            <Title level={4} style={{ margin: 0, color: '#f8fafc' }}>
              System & Media Metadata
            </Title>
          </Space>
        }
        style={{
          background: '#161b26',
          borderRadius: 16,
          border: '1px solid rgba(255, 255, 255, 0.08)',
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
            <Text code>http://0.0.0.0:8080</Text>
          </Descriptions.Item>
          <Descriptions.Item label="WebSocket Remote Endpoint">
            <Text code>ws://0.0.0.0:8081</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Pealayer Core Version">
            <Text style={{ color: '#f8fafc' }}>0.1.0 (Rust Edition 2024)</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Hardware Acceleration">
            <Tag color="blue">mpv libmpv2 render</Tag>
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  );
};
