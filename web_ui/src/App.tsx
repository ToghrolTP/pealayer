import React, { useEffect, useState, useRef } from 'react';
import { ConfigProvider, theme, Layout, Menu } from 'antd';
import {
  AppstoreOutlined,
  ControlOutlined,
  FolderOpenOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import { HeaderBar } from './components/HeaderBar';
import { RemoteControlTab, PlayerState } from './components/RemoteControlTab';
import { MediaLibraryTab } from './components/MediaLibraryTab';
import { PlayerInfoTab } from './components/PlayerInfoTab';
import { StudioTab } from './components/StudioTab';
import { tr } from './i18n';
import './styles.css';

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
  const [activeTab, setActiveTab] = useState<string>('studio');
  const [connected, setConnected] = useState<boolean>(false);
  const [connectionMode, setConnectionMode] = useState<'ws' | 'http'>('http');
  const [state, setState] = useState<PlayerState>({ status: 'initializing' });
  const [runtime, setRuntime] = useState<RuntimeConfig | null>(null);
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>('dark');
  const [quickSeekSeconds, setQuickSeekSeconds] = useState<number>(10);

  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!runtime) return;
    document.documentElement.lang = runtime.locale;
    document.documentElement.dir = runtime.direction;
    document.title = `${runtime.appName} — ${tr(runtime.locale, 'Control Center')}`;
    const media = window.matchMedia('(prefers-color-scheme: light)');
    const applyTheme = () => {
      const nextTheme = runtime.theme === 'system'
        ? (media.matches ? 'light' : 'dark')
        : runtime.theme;
      document.documentElement.dataset.theme = nextTheme;
      setResolvedTheme(nextTheme);
    };
    applyTheme();
    media.addEventListener('change', applyTheme);
    return () => media.removeEventListener('change', applyTheme);
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
    let disposed = false;
    fetch('/api/config')
      .then((response) => response.ok ? response.json() : Promise.reject())
      .then((value) => {
        const seconds = Number(value?.quick_seek_seconds);
        if (!disposed && Number.isFinite(seconds) && seconds > 0) {
          setQuickSeekSeconds(seconds);
        }
      })
      .catch(() => {});
    return () => { disposed = true; };
  }, [runtime]);

  useEffect(() => {
    if (!runtime) return;
    let disposed = false;
    let reconnectTimer: number | undefined;
    const connectWS = () => {
      if (disposed) return;
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${proto}//${window.location.host}${runtime.websocketPath}`;

      try {
        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
          if (disposed) {
            ws.close();
            return;
          }
          setConnected(true);
          setConnectionMode('ws');
        };

        ws.onclose = () => {
          if (disposed) return;
          setConnected(false);
          setConnectionMode('http');
          reconnectTimer = window.setTimeout(connectWS, 3000);
        };

        ws.onmessage = (ev) => {
          try {
            const data = JSON.parse(ev.data);
            setState((prev) => ({ ...prev, ...data }));
            setConnected(true);
          } catch {}
        };
      } catch {
        if (disposed) return;
        setConnected(false);
        setConnectionMode('http');
        reconnectTimer = window.setTimeout(connectWS, 3000);
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
        if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
          setConnected(false);
        }
      }
    }, 500);

    return () => {
      disposed = true;
      clearInterval(httpInterval);
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
      if (wsRef.current) wsRef.current.close();
      wsRef.current = null;
    };
  }, [runtime]);

  const menuItems = [
    {
      key: 'studio',
      icon: <AppstoreOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Studio'),
    },
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
        algorithm: resolvedTheme === 'light' ? theme.defaultAlgorithm : theme.darkAlgorithm,
        token: {
          colorPrimary: '#38d27a',
          colorInfo: '#68a7ff',
          colorSuccess: '#38d27a',
          colorWarning: '#f3b954',
          colorError: '#ff5c68',
          colorBgContainer: 'var(--surface-1)',
          colorBgBase: 'var(--canvas)',
          colorBorder: 'var(--line)',
          colorText: 'var(--text)',
          colorTextSecondary: 'var(--muted)',
          borderRadius: 9,
          fontFamily: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`,
        },
      }}
    >
      <Layout className="app-shell">
        <HeaderBar
          collapsed={collapsed}
          onToggleCollapse={() => setCollapsed(!collapsed)}
          connected={connected}
          connectionMode={connectionMode}
          appName={runtime?.appName}
          locale={runtime?.locale || 'en'}
        />

        <Layout className="app-body">
          <Sider
            trigger={null}
            collapsible
            collapsed={collapsed}
            breakpoint="lg"
            onBreakpoint={(broken) => setCollapsed(broken)}
            className="app-sider"
            width={220}
          >
            <Menu
              mode="inline"
              selectedKeys={[activeTab]}
              onClick={({ key }) => setActiveTab(key)}
              items={menuItems}
              className="app-menu"
            />
          </Sider>

          <Content className={`app-content ${activeTab === 'studio' ? 'app-content--studio' : ''}`}>
            {activeTab === 'studio' && (
              <StudioTab
                state={state}
                sendCmd={sendCmd}
                locale={runtime?.locale || 'en'}
                appName={runtime?.appName || 'Pealayer'}
                quickSeekSeconds={quickSeekSeconds}
              />
            )}
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
