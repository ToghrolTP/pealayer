import React from 'react';
import { Layout, Typography, Space, Tag, Button, Input, Modal, Tooltip } from 'antd';
import {
  ApiOutlined,
  CheckCircleOutlined,
  DisconnectOutlined,
  GlobalOutlined,
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
  appIconPath?: string;
  locale: UiLocale;
  connectionTarget: string;
  onConnectionTargetChange: (target: string) => void;
}

export const HeaderBar: React.FC<HeaderBarProps> = ({
  collapsed,
  onToggleCollapse,
  connected,
  connectionMode,
  appName,
  appIconPath,
  locale,
  connectionTarget,
  onConnectionTargetChange,
}) => {
  const [connectionOpen, setConnectionOpen] = React.useState(false);
  const [draftTarget, setDraftTarget] = React.useState(connectionTarget);

  React.useEffect(() => setDraftTarget(connectionTarget), [connectionTarget]);

  return (
    <Header className="studio-header">
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
              src={appIconPath || '/api/runtime/app-icon'}
              alt={appName ? `${appName} ${tr(locale, 'Application logo')}` : tr(locale, 'Application logo')}
            />
          </span>
          <div className="studio-brand__copy">
            <Title level={4}>{appName || 'Pealayer'}</Title>
            <span>{connected ? tr(locale, 'Connected workspace') : tr(locale, 'Connecting')}</span>
          </div>
        </Space>
      </Space>

      <div className="connection-cluster" aria-live="polite">
        <Tooltip title={tr(locale, 'Connect to another Pealayer')}>
          <Button
            type="text"
            icon={<GlobalOutlined />}
            className="connection-target-button"
            onClick={() => setConnectionOpen(true)}
          />
        </Tooltip>
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

      <Modal
        title={tr(locale, 'Pealayer connection')}
        open={connectionOpen}
        onCancel={() => setConnectionOpen(false)}
        okText={tr(locale, 'Connect')}
        onOk={() => {
          onConnectionTargetChange(draftTarget.trim());
          setConnectionOpen(false);
        }}
        destroyOnClose={false}
      >
        <label className="connection-target-field">
          <span>{tr(locale, 'WebSocket endpoint')}</span>
          <Input
            value={draftTarget}
            onChange={(event) => setDraftTarget(event.target.value)}
            placeholder="ws://host:port/ws"
            allowClear
            onPressEnter={() => {
              onConnectionTargetChange(draftTarget.trim());
              setConnectionOpen(false);
            }}
          />
        </label>
        <button
          type="button"
          className="connection-target-local"
          onClick={() => setDraftTarget('')}
        >
          {tr(locale, 'Use this instance')}
        </button>
      </Modal>
    </Header>
  );
};
