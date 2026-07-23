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

const { Sider, Content } = Layout;

const App: React.FC = () => {
  const [collapsed, setCollapsed] = useState<boolean>(false);
  const [activeTab, setActiveTab] = useState<string>('remote');
  const [connected, setConnected] = useState<boolean>(false);
  const [connectionMode, setConnectionMode] = useState<'ws' | 'http'>('http');
  const [state, setState] = useState<PlayerState>({
    playing: false,
    volume: 100,
    playback_time: 0,
    duration: 0,
    current_video: null,
  });

  const wsRef = useRef<WebSocket | null>(null);

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
    const connectWS = () => {
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${proto}//${window.location.hostname}:8081`;

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
  }, []);

  const menuItems = [
    {
      key: 'remote',
      icon: <ControlOutlined style={{ fontSize: 18 }} />,
      label: 'Remote Control',
    },
    {
      key: 'library',
      icon: <FolderOpenOutlined style={{ fontSize: 18 }} />,
      label: 'Media Library',
    },
    {
      key: 'info',
      icon: <InfoCircleOutlined style={{ fontSize: 18 }} />,
      label: 'System Info',
    },
  ];

  return (
    <ConfigProvider
      theme={{
        algorithm: theme.darkAlgorithm,
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
              />
            )}
            {activeTab === 'library' && (
              <MediaLibraryTab
                sendCmd={sendCmd}
                onMediaPlayStarted={() => setActiveTab('remote')}
              />
            )}
            {activeTab === 'info' && (
              <PlayerInfoTab state={state} connectionMode={connectionMode} />
            )}
          </Content>
        </Layout>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
