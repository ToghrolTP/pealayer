import React from 'react';
import { Layout, Typography, Space, Tag, Button } from 'antd';
import {
  VideoCameraOutlined,
  SyncOutlined,
  CheckCircleOutlined,
  DisconnectOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';

const { Header } = Layout;
const { Title } = Typography;

interface HeaderBarProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  connected: boolean;
  connectionMode: 'ws' | 'http';
}

export const HeaderBar: React.FC<HeaderBarProps> = ({
  collapsed,
  onToggleCollapse,
  connected,
  connectionMode,
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
        <Space size="middle">
          <VideoCameraOutlined style={{ fontSize: 24, color: '#1d84b5' }} />
          <Title level={4} style={{ margin: 0, color: '#f8fafc', fontWeight: 700 }}>
            Pealayer Control Center
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
            {connectionMode === 'ws' ? 'WebSocket Live' : 'HTTP Polling'}
          </Tag>
        ) : (
          <Tag
            icon={<DisconnectOutlined />}
            color="error"
            style={{ borderRadius: 12, padding: '4px 12px', fontSize: 13 }}
          >
            Offline
          </Tag>
        )}
      </div>
    </Header>
  );
};
