import React from 'react';
import { Layout, Typography, Tag, Button, Dropdown, Input, Modal, Switch, Tooltip } from 'antd';
import {
  ApiOutlined,
  AppstoreAddOutlined,
  BellOutlined,
  CheckCircleOutlined,
  DisconnectOutlined,
  FullscreenOutlined,
  GlobalOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  MoreOutlined,
  ShareAltOutlined,
  SoundOutlined,
  ThunderboltOutlined,
  UpCircleOutlined,
} from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';
import type { WebPlatformController } from '../webPlatform';

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
  platform: WebPlatformController;
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
  platform,
}) => {
  const [connectionOpen, setConnectionOpen] = React.useState(false);
  const [draftTarget, setDraftTarget] = React.useState(connectionTarget);

  React.useEffect(() => setDraftTarget(connectionTarget), [connectionTarget]);

  return (
    <Header className="studio-header">
      <div className="studio-header__leading">
        <Button
          type="text"
          icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
          onClick={onToggleCollapse}
          className="studio-header__menu-button"
          aria-label={tr(locale, collapsed ? 'Expand navigation' : 'Collapse navigation')}
        />
        <div className="studio-brand">
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
        </div>
      </div>

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
        <Dropdown
          trigger={['click']}
          menu={{
            items: [
              platform.capabilities.install && !platform.standalone ? {
                key: 'install', icon: <AppstoreAddOutlined />, label: tr(locale, 'Install app'),
                onClick: () => void platform.install(),
              } : null,
              platform.updateReady ? {
                key: 'update', icon: <UpCircleOutlined />, label: tr(locale, 'Apply web update'),
                onClick: platform.applyUpdate,
              } : null,
              platform.capabilities.share ? {
                key: 'share', icon: <ShareAltOutlined />, label: tr(locale, 'Share'),
                onClick: () => void platform.share(),
              } : null,
              platform.capabilities.fullscreen ? {
                key: 'fullscreen', icon: <FullscreenOutlined />, label: tr(locale, 'Fullscreen'),
                onClick: () => void platform.toggleFullscreen(),
              } : null,
              platform.capabilities.notifications ? {
                key: 'notifications', icon: <BellOutlined />, label: tr(locale, 'Enable notifications'),
                onClick: () => void platform.enableNotifications(),
              } : null,
              { type: 'divider' },
              platform.capabilities.vibration ? {
                key: 'haptics',
                icon: <ThunderboltOutlined />,
                label: <span className="web-capability-toggle"><span>{tr(locale, 'Haptic feedback')}</span><Switch size="small" checked={platform.hapticsEnabled} onChange={platform.setHapticsEnabled} /></span>,
              } : null,
              platform.capabilities.audio ? {
                key: 'audio-feedback',
                icon: <SoundOutlined />,
                label: <span className="web-capability-toggle"><span>{tr(locale, 'Audio feedback')}</span><Switch size="small" checked={platform.audioFeedbackEnabled} onChange={platform.setAudioFeedbackEnabled} /></span>,
              } : null,
              platform.capabilities.wakeLock ? {
                key: 'wake-lock',
                icon: <ThunderboltOutlined />,
                label: <span className="web-capability-toggle"><span>{tr(locale, 'Keep screen awake while playing')}</span><Switch size="small" checked={platform.keepAwakeEnabled} onChange={platform.setKeepAwakeEnabled} /></span>,
              } : null,
            ].filter(Boolean) as any,
          }}
        >
          <Tooltip title={tr(locale, 'Web app features')}>
            <Button type="text" icon={<MoreOutlined />} className="connection-target-button" aria-label={tr(locale, 'Web app features')} />
          </Tooltip>
        </Dropdown>
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
