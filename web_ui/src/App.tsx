import React, { useCallback, useEffect, useState, useRef } from 'react';
import { ConfigProvider, theme, Layout, Menu } from 'antd';
import {
  AppstoreOutlined,
  ControlOutlined,
  FolderOpenOutlined,
  InfoCircleOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { HeaderBar } from './components/HeaderBar';
import { RemoteControlTab, PlayerState } from './components/RemoteControlTab';
import { MediaLibraryTab } from './components/MediaLibraryTab';
import { PlayerInfoTab } from './components/PlayerInfoTab';
import { StudioTab } from './components/StudioTab';
import { PreferencesTab } from './components/PreferencesTab';
import { tr } from './i18n';
import './styles.css';

const { Sider, Content } = Layout;

export interface RuntimeConfig {
  appName: string;
  version: string;
  websocketPath: string;
  appIconPath: string;
  locale: 'en' | 'fa';
  direction: 'ltr' | 'rtl';
  theme: 'system' | 'light' | 'dark';
  accentColor: string;
}

function resolvedAccent(runtime: RuntimeConfig | null, config: Record<string, any> | null): string {
  switch (config?.accent_color) {
    case 'pealayer_green': return '#38d27a';
    case 'windows_blue': return '#0078d4';
    case 'macos_blue': return '#0a84ff';
    case 'custom': return /^#[0-9a-f]{6}$/i.test(config?.custom_accent_color ?? '')
      ? config.custom_accent_color
      : (runtime?.accentColor ?? '#0078d4');
    default: return runtime?.accentColor ?? '#0078d4';
  }
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
  const [appConfig, setAppConfig] = useState<Record<string, any> | null>(null);
  const [connectionTarget, setConnectionTarget] = useState<string>(() => {
    const query = new URLSearchParams(window.location.search).get('connect');
    return query ?? window.localStorage.getItem('pealayer.connectionTarget') ?? '';
  });

  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!runtime) return;
    document.documentElement.lang = runtime.locale;
    document.documentElement.dir = runtime.direction;
    document.title = `${runtime.appName} — ${tr(runtime.locale, 'Web Studio')}`;
    const media = window.matchMedia('(prefers-color-scheme: light)');
    const applyTheme = () => {
      const preference = appConfig?.theme ?? runtime.theme;
      const nextTheme = preference === 'system'
        ? (media.matches ? 'light' : 'dark')
        : preference;
      document.documentElement.dataset.theme = nextTheme;
      setResolvedTheme(nextTheme);
    };
    applyTheme();
    media.addEventListener('change', applyTheme);
    return () => media.removeEventListener('change', applyTheme);
  }, [runtime, appConfig?.theme]);

  const accentColor = resolvedAccent(runtime, appConfig);

  useEffect(() => {
    const accent = resolvedAccent(runtime, appConfig);
    document.documentElement.style.setProperty('--accent', accent);
    document.documentElement.style.setProperty('--accent-soft', `color-mix(in srgb, ${accent} 16%, transparent)`);
  }, [runtime, appConfig?.accent_color, appConfig?.custom_accent_color]);

  const nextRequestId = useRef(1);
  const sendCmd = useCallback((command: string, payload: Record<string, any> = {}) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      const methodAliases: Record<string, string> = {
        add_effect_cue: 'pealayer.timeline.effect.add',
        remove_effect_cue: 'pealayer.timeline.effect.remove',
        set_recording: 'pealayer.recording.set',
      };
      const params = command === 'set_workspace'
        ? { workspace: payload.nle ? 'nle' : 'simple' }
        : payload;
      wsRef.current.send(JSON.stringify({
        jsonrpc: '2.0',
        id: nextRequestId.current++,
        method: methodAliases[command] || command,
        params,
      }));
    } else if (!connectionTarget) {
      const body = JSON.stringify({ command, ...payload });
      fetch('/api/player/command', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      }).catch(() => {});
    }
  }, [connectionTarget]);

  const resolveWebSocketUrl = useCallback(() => {
    const fallbackProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    if (!connectionTarget) {
      return `${fallbackProtocol}//${window.location.host}${runtime?.websocketPath || '/ws'}`;
    }
    try {
      const target = new URL(connectionTarget.includes('://') ? connectionTarget : `ws://${connectionTarget}`);
      if (target.protocol === 'http:') target.protocol = 'ws:';
      if (target.protocol === 'https:') target.protocol = 'wss:';
      if (!target.pathname || target.pathname === '/') target.pathname = runtime?.websocketPath || '/ws';
      return target.toString();
    } catch {
      return connectionTarget;
    }
  }, [connectionTarget, runtime]);

  const apiBaseUrl = (() => {
    try {
      const url = new URL(resolveWebSocketUrl());
      url.protocol = url.protocol === 'wss:' ? 'https:' : 'http:';
      url.pathname = '';
      url.search = '';
      url.hash = '';
      return url.toString().replace(/\/$/, '');
    } catch {
      return '';
    }
  })();

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
        if (!disposed && value && typeof value === 'object') setAppConfig(value);
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
      const wsUrl = resolveWebSocketUrl();

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
            if (data && data.jsonrpc === '2.0') return;
            const nextState = data?.type === 'state' ? data.state : data;
            if (nextState && typeof nextState === 'object' && typeof nextState.status === 'string') {
              setState((prev) => ({ ...prev, ...nextState }));
              setConnected(true);
            }
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
      if (connectionTarget || (wsRef.current && wsRef.current.readyState === WebSocket.OPEN)) {
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
  }, [runtime, connectionTarget, resolveWebSocketUrl]);

  const changeConnectionTarget = (target: string) => {
    setConnectionTarget(target);
    if (target) window.localStorage.setItem('pealayer.connectionTarget', target);
    else window.localStorage.removeItem('pealayer.connectionTarget');
  };

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
    {
      key: 'preferences',
      icon: <SettingOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Preferences'),
    },
  ];

  return (
    <ConfigProvider direction={runtime?.direction}
      theme={{
        algorithm: resolvedTheme === 'light' ? theme.defaultAlgorithm : theme.darkAlgorithm,
        token: {
          colorPrimary: accentColor,
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
          appIconPath={runtime?.appIconPath}
          locale={runtime?.locale || 'en'}
          connectionTarget={connectionTarget}
          onConnectionTargetChange={changeConnectionTarget}
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
                apiBaseUrl={apiBaseUrl}
              />
            )}
            {activeTab === 'remote' && (
              <RemoteControlTab
                state={state}
                sendCmd={sendCmd}
                onOpenLibraryTab={() => setActiveTab('library')}
                locale={runtime?.locale || 'en'}
                quickSeekSeconds={quickSeekSeconds}
                apiBaseUrl={apiBaseUrl}
              />
            )}
            {activeTab === 'library' && (
              <MediaLibraryTab
                sendCmd={sendCmd}
                onMediaPlayStarted={() => setActiveTab('remote')}
                locale={runtime?.locale || 'en'}
                apiBaseUrl={apiBaseUrl}
              />
            )}
            {activeTab === 'info' && (
              <PlayerInfoTab state={state} connectionMode={connectionMode} runtime={runtime} locale={runtime?.locale || 'en'} apiBaseUrl={apiBaseUrl} websocketUrl={resolveWebSocketUrl()} />
            )}
            {activeTab === 'preferences' && (
              <PreferencesTab
                apiBaseUrl={apiBaseUrl}
                locale={runtime?.locale || 'en'}
                onConfigChange={(values) => {
                  setAppConfig(values);
                  const seconds = Number(values.quick_seek_seconds);
                  if (Number.isFinite(seconds) && seconds > 0) setQuickSeekSeconds(seconds);
                }}
              />
            )}
          </Content>
        </Layout>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
