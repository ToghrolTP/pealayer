import React, { useEffect, useState, useRef } from 'react';
import { ConfigProvider, theme, Layout, Menu } from 'antd';
import {
  ControlOutlined,
  FolderOpenOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import { HeaderBar } from './components/HeaderBar';
import { RemoteControlTab, PlayerState } from './components/RemoteControlTab';
import { MediaLibraryTab } from './components/MediaLibraryTab';
import { PlayerInfoTab } from './components/PlayerInfoTab';
import { tr } from './i18n';

const { Sider, Content } = Layout;

export interface RuntimeConfig {
  appName: string;
  version: string;
  websocketPath: string;
  locale: 'en' | 'fa';
  direction: 'ltr' | 'rtl';
  theme: 'system' | 'light' | 'dark';
}

const App: React.FC = () => {
  const [collapsed, setCollapsed] = useState<boolean>(false);
  const [activeTab, setActiveTab] = useState<string>('remote');
  const [connected, setConnected] = useState<boolean>(false);
  const [connectionMode, setConnectionMode] = useState<'ws' | 'http'>('http');
  const [state, setState] = useState<PlayerState>({ status: 'initializing' });
  const [runtime, setRuntime] = useState<RuntimeConfig | null>(null);

  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!runtime) return;
    document.documentElement.lang = runtime.locale;
    document.documentElement.dir = runtime.direction;
    document.title = `${runtime.appName} — ${tr(runtime.locale, 'Control Center')}`;
  }, [runtime]);

  const sendCmd = (command: string, payload: Record<string, any> = {}) => {
    const body = JSON.stringify({ command, ...payload });
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(body);
    } else {
      fetch('/api/player/command', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      }).catch(() => {});
    }
  };

  useEffect(() => {
    let disposed = false;
    fetch('/api/runtime/config')
      .then((response) => {
        if (!response.ok) throw new Error(`runtime config ${response.status}`);
        return response.json();
      })
      .then((value: RuntimeConfig) => {
        if (!disposed) setRuntime(value);
      })
      .catch(() => {
        if (!disposed) setRuntime(null);
      });
    return () => { disposed = true; };
  }, []);

  useEffect(() => {
    if (!runtime) return;
    const connectWS = () => {
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${proto}//${window.location.host}${runtime.websocketPath}`;

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
      if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
        return; // Skip HTTP polling when WebSocket is connected
      }
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
  }, [runtime]);

  const menuItems = [
    {
      key: 'remote',
      icon: <ControlOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Remote Control'),
    },
    {
      key: 'library',
      icon: <FolderOpenOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Media Library'),
    },
    {
      key: 'info',
      icon: <InfoCircleOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'System Info'),
    },
  ];

  return (
    <ConfigProvider direction={runtime?.direction}
      theme={{
        algorithm: runtime?.theme === 'light' ? theme.defaultAlgorithm : theme.darkAlgorithm,
        token: {
          colorPrimary: '#1d84b5',
          colorBgContainer: '#132e32',
          colorBgBase: '#0a2239',
          colorBorder: 'rgba(23, 96, 135, 0.3)',
          borderRadius: 12,
          fontFamily: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`,
        },
      }}
    >
      <Layout style={{ minHeight: '100vh', background: '#0a2239' }}>
        <HeaderBar
          collapsed={collapsed}
          onToggleCollapse={() => setCollapsed(!collapsed)}
          connected={connected}
          connectionMode={connectionMode}
          appName={runtime?.appName}
          locale={runtime?.locale || 'en'}
        />

        <Layout style={{ background: '#0a2239' }}>
          <Sider
            trigger={null}
            collapsible
            collapsed={collapsed}
            breakpoint="lg"
            onBreakpoint={(broken) => setCollapsed(broken)}
            style={{
              background: '#132e32',
              borderRight: '1px solid rgba(23, 96, 135, 0.3)',
            }}
            width={220}
          >
            <Menu
              mode="inline"
              selectedKeys={[activeTab]}
              onClick={({ key }) => setActiveTab(key)}
              items={menuItems}
              style={{
                background: 'transparent',
                borderRight: 'none',
                marginTop: 16,
              }}
            />
          </Sider>

          <Content
            style={{
              padding: '24px 16px',
              maxWidth: 1200,
              margin: '0 auto',
              width: '100%',
            }}
          >
            {activeTab === 'remote' && (
              <RemoteControlTab
                state={state}
                sendCmd={sendCmd}
                onOpenLibraryTab={() => setActiveTab('library')}
                locale={runtime?.locale || 'en'}
              />
            )}
            {activeTab === 'library' && (
              <MediaLibraryTab
                sendCmd={sendCmd}
                onMediaPlayStarted={() => setActiveTab('remote')}
                locale={runtime?.locale || 'en'}
              />
            )}
            {activeTab === 'info' && (
              <PlayerInfoTab state={state} connectionMode={connectionMode} runtime={runtime} locale={runtime?.locale || 'en'} />
            )}
          </Content>
        </Layout>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
