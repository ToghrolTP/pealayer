import React from 'react';
import { Layout, Typography, Space, Tag, Button } from 'antd';
import {
  SyncOutlined,
  CheckCircleOutlined,
  DisconnectOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

const { Header } = Layout;
const { Title } = Typography;

interface HeaderBarProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  connected: boolean;
  connectionMode: 'ws' | 'http';
  appName?: string;
  locale: UiLocale;
}

export const HeaderBar: React.FC<HeaderBarProps> = ({
  collapsed,
  onToggleCollapse,
  connected,
  connectionMode,
  appName,
  locale,
}) => {
  return (
    <Header
      style={{
        padding: '0 24px',
        background: '#0a2239',
        borderBottom: '1px solid rgba(23, 96, 135, 0.4)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        position: 'sticky',
        top: 0,
        zIndex: 100,
        height: 64,
      }}
    >
      <Space size="large">
        <Button
          type="text"
          icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
          onClick={onToggleCollapse}
          style={{ fontSize: 18, color: '#53a2be' }}
        />
        <Space size="middle" align="center">
          <img
            src="/pealayer-icon.svg"
            alt={appName ? `${appName} ${tr(locale, 'Application logo')}` : tr(locale, 'Application logo')}
            style={{ width: 28, height: 28, borderRadius: 6, objectFit: 'contain' }}
          />
          <Title level={4} style={{ margin: 0, color: '#f8fafc', fontWeight: 700 }}>
            {appName ? `${appName} — ${tr(locale, 'Control Center')}` : tr(locale, 'Control Center')}
          </Title>
        </Space>
      </Space>

      <div>
        {connected ? (
          <Tag
            icon={connectionMode === 'ws' ? <SyncOutlined spin /> : <CheckCircleOutlined />}
            color="success"
            style={{ borderRadius: 12, padding: '4px 12px', fontSize: 13 }}
          >
            {connectionMode === 'ws' ? tr(locale, 'WebSocket Live') : tr(locale, 'HTTP Polling')}
          </Tag>
        ) : (
          <Tag
            icon={<DisconnectOutlined />}
            color="error"
            style={{ borderRadius: 12, padding: '4px 12px', fontSize: 13 }}
          >
            {tr(locale, 'Offline')}
          </Tag>
        )}
      </div>
    </Header>
  );
};
