import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, Descriptions, Input, Progress, Tag, Row, Col, Statistic, Typography, Space } from 'antd';
import {
  InfoCircleOutlined,
  DesktopOutlined,
  ApiOutlined,
  CodeOutlined,
  SoundOutlined,
  CloudDownloadOutlined,
  SafetyCertificateOutlined,
  MobileOutlined,
  CloudSyncOutlined,
} from '@ant-design/icons';
import { PlayerState } from './RemoteControlTab';
import { RuntimeConfig } from '../App';
import { tr, UiLocale } from '../i18n';
import type { WebPlatformController } from '../webPlatform';

const { Title, Text } = Typography;

interface PlayerInfoTabProps {
  state: PlayerState;
  connectionMode: 'ws' | 'http';
  runtime: RuntimeConfig | null;
  locale: UiLocale;
  apiBaseUrl: string;
  websocketUrl: string;
  platform: WebPlatformController;
}

export const PlayerInfoTab: React.FC<PlayerInfoTabProps> = ({ state, connectionMode, runtime, locale, apiBaseUrl, websocketUrl, platform }) => {
  const [updateUrl, setUpdateUrl] = useState('');
  const [expectedHash, setExpectedHash] = useState('');
  const [updateStatus, setUpdateStatus] = useState(state.update);
  const [requestingUpdate, setRequestingUpdate] = useState(false);

  useEffect(() => {
    if (state.update) setUpdateStatus(state.update);
  }, [state.update]);

  useEffect(() => {
    if (!updateStatus || !['downloading', 'receiving', 'verifying', 'staged', 'restarting'].includes(updateStatus.state)) return;
    const timer = window.setInterval(() => {
      fetch(`${apiBaseUrl}/api/update/status`)
        .then((response) => response.ok ? response.json() : Promise.reject())
        .then(setUpdateStatus)
        .catch(() => {});
    }, 350);
    return () => window.clearInterval(timer);
  }, [apiBaseUrl, updateStatus?.state]);

  const startUrlUpdate = async () => {
    if (!updateUrl.trim()) return;
    setRequestingUpdate(true);
    try {
      const response = await fetch(`${apiBaseUrl}/api/update/from-url`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          url: updateUrl.trim(),
          sha256: expectedHash.trim() || null,
        }),
      });
      const body = await response.json();
      if (!response.ok) throw new Error(body.error || `Update request failed (${response.status})`);
      setUpdateStatus(body);
    } catch (error) {
      setUpdateStatus({
        state: 'failed', bytes_done: 0, message: 'Update request failed',
        error: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setRequestingUpdate(false);
    }
  };

  const updatePercent = updateStatus?.bytes_total
    ? Math.min(100, Math.round(updateStatus.bytes_done * 100 / updateStatus.bytes_total))
    : undefined;

  const formatBytes = (bytes?: number | null) => {
    if (bytes == null) return '—';
    const units = ['B', 'KiB', 'MiB', 'GiB'];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit + 1 < units.length) { value /= 1024; unit += 1; }
    return unit === 0 ? `${bytes} B` : `${value.toFixed(1)} ${units[unit]}`;
  };
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

      <Card bordered={false} className="surface-card info-surface"
        title={<Space><MobileOutlined /><Title level={4} style={{ margin: 0 }}>{tr(locale, 'Web application runtime')}</Title></Space>}
      >
        <Descriptions bordered column={{ xs: 1, sm: 2, md: 3 }} size="small">
          <Descriptions.Item label={tr(locale, 'Network')}>
            <Tag color={platform.online ? 'success' : 'warning'}>{platform.online ? tr(locale, 'Online') : tr(locale, 'Offline mode')}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'Installation')}>
            <Tag icon={<MobileOutlined />} color={platform.standalone ? 'success' : 'default'}>
              {platform.standalone ? tr(locale, 'Installed app') : tr(locale, 'Browser tab')}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'Offline shell')}>
            <Tag icon={<CloudSyncOutlined />} color={platform.capabilities.serviceWorker ? 'success' : 'default'}>
              {platform.capabilities.serviceWorker ? tr(locale, 'Available') : tr(locale, 'Unsupported')}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'OS media controls')}>
            <Tag color={platform.capabilities.mediaSession ? 'success' : 'default'}>{platform.capabilities.mediaSession ? tr(locale, 'Available') : tr(locale, 'Unsupported')}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'Screen wake lock')}>
            <Tag color={platform.capabilities.wakeLock ? 'success' : 'default'}>{platform.capabilities.wakeLock ? tr(locale, 'Available') : tr(locale, 'Unsupported')}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={tr(locale, 'Haptics / audio')}>
            <Tag color={platform.capabilities.vibration || platform.capabilities.audio ? 'success' : 'default'}>
              {platform.capabilities.vibration || platform.capabilities.audio ? tr(locale, 'Available') : tr(locale, 'Unsupported')}
            </Tag>
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card bordered={false} className="surface-card info-surface"
        title={<Space><CloudDownloadOutlined /><Title level={4} style={{ margin: 0 }}>{tr(locale, 'Application update')}</Title></Space>}
      >
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Input
            value={updateUrl}
            onChange={(event) => setUpdateUrl(event.target.value)}
            placeholder="Release, CI artifact, peer manifest, or executable URL"
            prefix={<CloudDownloadOutlined />}
            onPressEnter={() => void startUrlUpdate()}
          />
          <Input
            value={expectedHash}
            onChange={(event) => setExpectedHash(event.target.value)}
            placeholder="Expected SHA-256 (optional when the manifest supplies it)"
            prefix={<SafetyCertificateOutlined />}
            maxLength={64}
          />
          <Button type="primary" icon={<CloudDownloadOutlined />} loading={requestingUpdate}
            disabled={!updateUrl.trim() || Boolean(updateStatus && ['downloading', 'receiving', 'verifying', 'staged', 'restarting'].includes(updateStatus.state))}
            onClick={() => void startUrlUpdate()}>
            {tr(locale, 'Download, verify and restart')}
          </Button>
          {updateStatus && updateStatus.state !== 'idle' && (
            <div aria-live="polite">
              <Space wrap>
                <Tag color={updateStatus.state === 'failed' ? 'error' : updateStatus.state === 'completed' || updateStatus.state === 'current' ? 'success' : 'processing'}>
                  {updateStatus.state}
                </Tag>
                <Typography.Text>{updateStatus.message}</Typography.Text>
              </Space>
              {updatePercent !== undefined && <Progress percent={updatePercent} status={updateStatus.state === 'failed' ? 'exception' : 'active'} />}
              <Typography.Text type="secondary">
                {formatBytes(updateStatus.bytes_done)}{updateStatus.bytes_total ? ` / ${formatBytes(updateStatus.bytes_total)}` : ''}
                {updateStatus.sha256 ? ` · SHA-256 ${updateStatus.sha256.slice(0, 12)}` : ''}
              </Typography.Text>
              {updateStatus.error && <Alert type="error" showIcon message={updateStatus.error} style={{ marginTop: 12 }} />}
            </div>
          )}
        </Space>
      </Card>
    </Space>
  );
};
