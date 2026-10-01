import React from 'react';
import { Layout, Typography, Space, Tag, Button } from 'antd';
import {
  ApiOutlined,
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
      className="studio-header"
    >
      <Space size={14} className="studio-header__leading">
        <Button
          type="text"
          icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
          onClick={onToggleCollapse}
          className="studio-header__menu-button"
        />
        <Space size={10} align="center" className="studio-brand">
          <span className="studio-brand__mark">
            <img
              src="/pealayer-icon.svg"
              alt={appName ? `${appName} ${tr(locale, 'Application logo')}` : tr(locale, 'Application logo')}
            />
          </span>
          <div className="studio-brand__copy">
            <Title level={4}>{appName || 'Pealayer'}</Title>
            <span>{tr(locale, 'Control Center')}</span>
          </div>
        </Space>
      </Space>

      <div className="connection-cluster" aria-live="polite">
        {connected ? (
          <Tag
            icon={connectionMode === 'ws' ? <ApiOutlined /> : <CheckCircleOutlined />}
            className="connection-pill connection-pill--online"
          >
            {connectionMode === 'ws' ? tr(locale, 'Live') : tr(locale, 'Polling')}
          </Tag>
        ) : (
          <Tag
            icon={<DisconnectOutlined />}
            className="connection-pill connection-pill--offline"
          >
            {tr(locale, 'Offline')}
          </Tag>
        )}
      </div>
    </Header>
  );
};
