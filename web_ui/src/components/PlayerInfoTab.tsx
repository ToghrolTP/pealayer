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
import { tr, UiLocale } from '../i18n';

const { Title, Text } = Typography;

interface PlayerInfoTabProps {
  state: PlayerState;
  connectionMode: 'ws' | 'http';
  runtime: RuntimeConfig | null;
  locale: UiLocale;
  apiBaseUrl: string;
  websocketUrl: string;
}

export const PlayerInfoTab: React.FC<PlayerInfoTabProps> = ({ state, connectionMode, runtime, locale, apiBaseUrl, websocketUrl }) => {
  const formatTime = (sec?: number) => {
    if (sec === undefined) return '—';
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
      {/* System Statistics */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} className="info-stat">
            <Statistic
              title={<Text type="secondary">{tr(locale, 'Connection Type')}</Text>}
              value={connectionMode.toUpperCase()}
              prefix={<ApiOutlined />}
              valueStyle={{ fontSize: 18, fontWeight: 700 }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} className="info-stat">
            <Statistic
              title={<Text type="secondary">{tr(locale, 'Playback Duration')}</Text>}
              value={formatTime(state.duration)}
              prefix={<DesktopOutlined />}
              valueStyle={{ fontSize: 18, fontFamily: 'monospace' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} className="info-stat">
            <Statistic
              title={<Text type="secondary">{tr(locale, 'Volume Level')}</Text>}
              value={state.volume === undefined ? '—' : Math.round(state.volume)}
              suffix="%"
              prefix={<SoundOutlined />}
              valueStyle={{ fontSize: 18, fontFamily: 'monospace' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} className="info-stat">
            <Statistic
              title={<Text type="secondary">{tr(locale, 'Player Engine')}</Text>}
              value="MPV Core"
              prefix={<CodeOutlined />}
              valueStyle={{ fontSize: 18 }}
            />
          </Card>
        </Col>
      </Row>

      {/* System & Session Descriptions */}
      <Card bordered={false} className="surface-card info-surface"
        title={
          <Space>
            <InfoCircleOutlined />
            <Title level={4} style={{ margin: 0 }}>
              {tr(locale, 'System & Media Metadata')}
            </Title>
          </Space>
        }
      >
        <Descriptions bordered column={{ xs: 1, sm: 1, md: 2 }} size="middle">
          <Descriptions.Item label={tr(locale, 'Active Video Path')}>
            <Text className="info-surface__path">
              {state.current_video === undefined ? tr(locale, 'Initializing…') : state.current_video || tr(locale, 'None')}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'Playback Status')}>
            {state.playing === true ? (
              <Tag color="success">{tr(locale, 'Active Playback')}</Tag>
            ) : state.playing === false ? (
              <Tag color="warning">{tr(locale, 'Paused / Idle')}</Tag>
            ) : (
              <Tag>{tr(locale, 'Unknown')}</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'HTTP Remote Endpoint')}>
            <Text code>{apiBaseUrl || window.location.origin}</Text>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'WebSocket Remote Endpoint')}>
            <Text code>{runtime ? websocketUrl : 'Initializing…'}</Text>
          </Descriptions.Item>
          <Descriptions.Item label={`${runtime?.appName || tr(locale, 'Application')} ${tr(locale, 'Version')}`}>
            <Text>{runtime?.version || 'Initializing…'}</Text>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'Hardware Acceleration')}>
            <Tag color="blue">mpv libmpv2 render</Tag>
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  );
};
