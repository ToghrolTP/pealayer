import React, { useCallback, useEffect, useState, useRef } from 'react';
import { Alert, ConfigProvider, theme, Layout, Menu, Spin } from 'antd';
import {
  AppstoreOutlined,
  BulbOutlined,
  ControlOutlined,
  DashboardOutlined,
  FolderOpenOutlined,
  InfoCircleOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { HeaderBar } from './components/HeaderBar';
import type { PlayerState } from './components/RemoteControlTab';
import { tr } from './i18n';
import { useWebPlatform } from './webPlatform';
import './styles.css';

const { Sider, Content } = Layout;
const RemoteControlTab = React.lazy(() => import('./components/RemoteControlTab').then((module) => ({ default: module.RemoteControlTab })));
const MediaLibraryTab = React.lazy(() => import('./components/MediaLibraryTab').then((module) => ({ default: module.MediaLibraryTab })));
const PlayerInfoTab = React.lazy(() => import('./components/PlayerInfoTab').then((module) => ({ default: module.PlayerInfoTab })));
const StudioTab = React.lazy(() => import('./components/StudioTab').then((module) => ({ default: module.StudioTab })));
const PreferencesTab = React.lazy(() => import('./components/PreferencesTab').then((module) => ({ default: module.PreferencesTab })));
const EffectsTab = React.lazy(() => import('./components/EffectsTab').then((module) => ({ default: module.EffectsTab })));
const HardwareTab = React.lazy(() => import('./components/HardwareTab').then((module) => ({ default: module.HardwareTab })));

const SURFACE_IDS = ['player', 'timeline', 'effects', 'hardware', 'library', 'about', 'preferences'] as const;
type SurfaceId = typeof SURFACE_IDS[number];

const STORAGE = {
  runtime: 'pealayer.lastKnownRuntime',
  config: 'pealayer.lastKnownConfig',
  state: 'pealayer.lastKnownState',
} as const;

function readStoredJson<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(key);
    return raw ? JSON.parse(raw) as T : fallback;
  } catch {
    return fallback;
  }
}

function persistJson(key: string, value: unknown): void {
  try { window.localStorage.setItem(key, JSON.stringify(value)); } catch { /* Storage can be disabled or full. */ }
}

function surfaceFromLocation(): SurfaceId {
  const requested = window.location.hash.replace(/^#\/?/, '') || window.localStorage.getItem('pealayer.webTab') || 'player';
  return SURFACE_IDS.includes(requested as SurfaceId) ? requested as SurfaceId : 'player';
}

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

function accentForeground(accent: string): string {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(accent);
  if (!match) return '#ffffff';
  const [red, green, blue] = match.slice(1).map((value) => Number.parseInt(value, 16));
  return (red * 299 + green * 587 + blue * 114) > 150_000 ? '#141414' : '#ffffff';
}

const App: React.FC = () => {
  const [collapsed, setCollapsed] = useState<boolean>(() => window.localStorage.getItem('pealayer.sidebarCollapsed') === 'true');
  const [activeTab, setActiveTabState] = useState<SurfaceId>(surfaceFromLocation);
  const [connected, setConnected] = useState<boolean>(false);
  const [connectionMode, setConnectionMode] = useState<'ws' | 'http'>('http');
  const [state, setState] = useState<PlayerState>(() => readStoredJson<PlayerState>(STORAGE.state, { status: 'initializing' }));
  const [runtime, setRuntime] = useState<RuntimeConfig | null>(() => readStoredJson<RuntimeConfig | null>(STORAGE.runtime, null));
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>('dark');
  const [quickSeekSeconds, setQuickSeekSeconds] = useState<number>(10);
  const [appConfig, setAppConfig] = useState<Record<string, any> | null>(() => readStoredJson<Record<string, any> | null>(STORAGE.config, null));
  const [hasCachedState] = useState(() => window.localStorage.getItem(STORAGE.state) !== null);
  const [connectionTarget, setConnectionTarget] = useState<string>(() => {
    const query = new URLSearchParams(window.location.search).get('connect');
    return query ?? window.localStorage.getItem('pealayer.connectionTarget') ?? '';
  });

  const wsRef = useRef<WebSocket | null>(null);
  const siderRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    window.localStorage.setItem('pealayer.sidebarCollapsed', String(collapsed));
  }, [collapsed]);

  useEffect(() => {
    const restoreScroll = () => {
      const sider = siderRef.current;
      const content = contentRef.current;
      if (sider) {
        sider.scrollTop = Number(window.localStorage.getItem('pealayer.scroll.sider') || 0);
      }
      if (content) {
        content.scrollTop = Number(window.localStorage.getItem(`pealayer.scroll.${activeTab}.top`) || 0);
        content.scrollLeft = Number(window.localStorage.getItem(`pealayer.scroll.${activeTab}.left`) || 0);
      }
    };
    const frame = window.requestAnimationFrame(() => window.requestAnimationFrame(restoreScroll));
    const afterLazySurface = window.setTimeout(restoreScroll, 100);
    return () => {
      window.cancelAnimationFrame(frame);
      window.clearTimeout(afterLazySurface);
    };
  }, [activeTab]);

  const setActiveTab = useCallback((requested: string) => {
    const tab = SURFACE_IDS.includes(requested as SurfaceId) ? requested as SurfaceId : 'player';
    setActiveTabState(tab);
    window.localStorage.setItem('pealayer.webTab', tab);
    const nextLocation = `${window.location.pathname}${window.location.search}#/${tab}`;
    if (`${window.location.pathname}${window.location.search}${window.location.hash}` !== nextLocation) {
      window.history.pushState({ pealayerSurface: tab }, '', nextLocation);
    }
  }, []);

  useEffect(() => {
    const followLocation = () => {
      const tab = surfaceFromLocation();
      setActiveTabState(tab);
      window.localStorage.setItem('pealayer.webTab', tab);
    };
    window.addEventListener('hashchange', followLocation);
    window.addEventListener('popstate', followLocation);
    return () => {
      window.removeEventListener('hashchange', followLocation);
      window.removeEventListener('popstate', followLocation);
    };
  }, []);

  useEffect(() => {
    if (!runtime) return;
    document.documentElement.lang = runtime.locale;
    document.documentElement.dir = runtime.direction;
    const surfaceNames: Record<SurfaceId, string> = {
      player: 'Player', timeline: 'Timeline', effects: 'Effects Library', hardware: 'Hardware Monitor',
      library: 'Media Library', about: 'About and system', preferences: 'Preferences',
    };
    document.title = `${tr(runtime.locale, surfaceNames[activeTab])} — ${runtime.appName}`;
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
  }, [runtime, appConfig?.theme, activeTab]);

  const accentColor = resolvedAccent(runtime, appConfig);
  const accentTextColor = accentForeground(accentColor);

  useEffect(() => {
    document.documentElement.style.setProperty('--accent', accentColor);
    document.documentElement.style.setProperty('--accent-text', accentTextColor);
    document.documentElement.style.setProperty('--accent-soft', `color-mix(in srgb, ${accentColor} 16%, transparent)`);
  }, [accentColor, accentTextColor]);

  const nextRequestId = useRef(1);
  const rawSendCmd = useCallback((command: string, payload: Record<string, any> = {}) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      const methodAliases: Record<string, string> = {
        add_effect_cue: 'pealayer.timeline.effect.add',
        remove_effect_cue: 'pealayer.timeline.effect.remove',
        set_recording: 'pealayer.recording.set',
      };
      wsRef.current.send(JSON.stringify({
        jsonrpc: '2.0',
        id: nextRequestId.current++,
        method: methodAliases[command] || command,
        params: payload,
      }));
    } else {
      const body = JSON.stringify({ command, ...payload });
      let endpoint = '/api/player/command';
      if (connectionTarget) {
        try {
          const target = new URL(connectionTarget.includes('://') ? connectionTarget : `http://${connectionTarget}`);
          if (target.protocol === 'ws:') target.protocol = 'http:';
          if (target.protocol === 'wss:') target.protocol = 'https:';
          target.pathname = '/api/player/command';
          target.search = '';
          target.hash = '';
          endpoint = target.toString();
        } catch {
          return;
        }
      }
      fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      }).catch(() => {});
    }
  }, [connectionTarget]);

  const platform = useWebPlatform(
    state,
    rawSendCmd,
    runtime?.appName || 'Pealayer',
    runtime?.appIconPath || '/api/runtime/app-icon-192.png',
  );

  const signalInteraction = platform.signalInteraction;

  const sendCmd = useCallback((command: string, payload: Record<string, any> = {}) => {
    if (!platform.online && !connected) return;
    signalInteraction();
    rawSendCmd(command, payload);
  }, [connected, platform.online, rawSendCmd, signalInteraction]);

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
        if (!disposed) {
          setRuntime(value);
          persistJson(STORAGE.runtime, value);
        }
      })
      .catch(() => {});
    return () => { disposed = true; };
  }, []);

  useEffect(() => {
    if (!runtime) return;
    let disposed = false;
    fetch('/api/config')
      .then((response) => response.ok ? response.json() : Promise.reject())
      .then((value) => {
        if (!disposed && value && typeof value === 'object') {
          setAppConfig(value);
          persistJson(STORAGE.config, value);
        }
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
    let reconnectAttempt = Number(window.sessionStorage.getItem('pealayer.wsReconnectAttempt') || 0);
    const scheduleReconnect = () => {
      if (disposed) return;
      reconnectAttempt += 1;
      window.sessionStorage.setItem('pealayer.wsReconnectAttempt', String(reconnectAttempt));
      const delay = Math.min(15_000, 500 * (2 ** Math.min(reconnectAttempt - 1, 5)));
      reconnectTimer = window.setTimeout(connectWS, delay);
    };
    const connectWS = () => {
      if (disposed) return;
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
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
          reconnectAttempt = 0;
          window.sessionStorage.setItem('pealayer.wsReconnectAttempt', '0');
          window.sessionStorage.setItem('pealayer.wsLastConnectedAt', new Date().toISOString());
        };

        ws.onclose = () => {
          if (disposed) return;
          setConnected(false);
          setConnectionMode('http');
          scheduleReconnect();
        };

        ws.onmessage = (ev) => {
          try {
            const data = JSON.parse(ev.data);
            if (data && data.jsonrpc === '2.0') return;
            const nextState = data?.type === 'state' ? data.state : data;
            if (nextState && typeof nextState === 'object' && typeof nextState.status === 'string') {
              setState((prev) => {
                const merged = { ...prev, ...nextState };
                persistJson(STORAGE.state, merged);
                return merged;
              });
              setConnected(true);
            }
          } catch {}
        };
      } catch {
        if (disposed) return;
        setConnected(false);
        setConnectionMode('http');
        scheduleReconnect();
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
          setState((prev) => {
            const merged = { ...prev, ...data };
            persistJson(STORAGE.state, merged);
            return merged;
          });
          setConnected(true);
        }
      } catch {
        if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
          setConnected(false);
        }
      }
    }, 500);

    const reconnectNow = () => {
      reconnectAttempt = 0;
      wsRef.current?.close();
      connectWS();
    };
    window.addEventListener('online', reconnectNow);

    return () => {
      disposed = true;
      clearInterval(httpInterval);
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
      if (wsRef.current) wsRef.current.close();
      wsRef.current = null;
      window.removeEventListener('online', reconnectNow);
    };
  }, [runtime, connectionTarget, resolveWebSocketUrl]);

  const changeConnectionTarget = (target: string) => {
    setConnectionTarget(target);
    if (target) window.localStorage.setItem('pealayer.connectionTarget', target);
    else window.localStorage.removeItem('pealayer.connectionTarget');
  };

  const menuItems = [
    {
      key: 'player',
      icon: <ControlOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Player'),
    },
    {
      key: 'timeline',
      icon: <AppstoreOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Timeline'),
    },
    {
      key: 'effects',
      icon: <BulbOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Effects Library'),
    },
    {
      key: 'hardware',
      icon: <DashboardOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Hardware Monitor'),
    },
    {
      key: 'library',
      icon: <FolderOpenOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'Media Library'),
    },
    {
      key: 'about',
      icon: <InfoCircleOutlined style={{ fontSize: 18 }} />,
      label: tr(runtime?.locale || 'en', 'About and system'),
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
          colorTextLightSolid: accentTextColor,
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
          platform={platform}
        />

        {!connected && (
          <Alert
            className="offline-state-banner"
            type={platform.online ? 'warning' : 'info'}
            showIcon
            message={platform.online
              ? tr(runtime?.locale || 'en', 'Pealayer backend unavailable')
              : tr(runtime?.locale || 'en', 'Offline mode')}
            description={hasCachedState
              ? tr(runtime?.locale || 'en', 'Showing the last known state. Live controls resume when the connection returns.')
              : tr(runtime?.locale || 'en', 'The app shell is available, but live controls require a Pealayer backend connection.')}
          />
        )}

        <Layout className="app-body">
          <Sider
            ref={siderRef}
            trigger={null}
            collapsible
            collapsed={collapsed}
            breakpoint="lg"
            onBreakpoint={(broken) => setCollapsed(broken)}
            className="app-sider"
            width={220}
            onScroll={(event) => window.localStorage.setItem('pealayer.scroll.sider', String(event.currentTarget.scrollTop))}
          >
            <Menu
              mode="inline"
              selectedKeys={[activeTab]}
              onClick={({ key }) => setActiveTab(key)}
              items={menuItems}
              className="app-menu"
            />
          </Sider>

          <Content
            ref={contentRef}
            className={`app-content ${activeTab === 'timeline' ? 'app-content--studio' : ''}`}
            onScroll={(event) => {
              window.localStorage.setItem(`pealayer.scroll.${activeTab}.top`, String(event.currentTarget.scrollTop));
              window.localStorage.setItem(`pealayer.scroll.${activeTab}.left`, String(event.currentTarget.scrollLeft));
            }}
          >
            <React.Suspense fallback={<div className="surface-loading"><Spin size="large" /></div>}>
            {activeTab === 'timeline' && (
              <StudioTab
                state={state}
                sendCmd={sendCmd}
                locale={runtime?.locale || 'en'}
                appName={runtime?.appName || 'Pealayer'}
                quickSeekSeconds={quickSeekSeconds}
                apiBaseUrl={apiBaseUrl}
                seekbarHoverThumbnails={Boolean(appConfig?.seekbar_hover_thumbnails)}
                surface="timeline"
              />
            )}
            {activeTab === 'player' && (
              <RemoteControlTab
                state={state}
                sendCmd={sendCmd}
                onOpenLibraryTab={() => setActiveTab('library')}
                locale={runtime?.locale || 'en'}
                quickSeekSeconds={quickSeekSeconds}
                apiBaseUrl={apiBaseUrl}
                seekbarHoverThumbnails={Boolean(appConfig?.seekbar_hover_thumbnails)}
              />
            )}
            {activeTab === 'library' && (
              <MediaLibraryTab
                sendCmd={sendCmd}
                onMediaPlayStarted={() => setActiveTab('player')}
                locale={runtime?.locale || 'en'}
                apiBaseUrl={apiBaseUrl}
              />
            )}
            {activeTab === 'effects' && <EffectsTab state={state} sendCmd={sendCmd} locale={runtime?.locale || 'en'} />}
            {activeTab === 'hardware' && <HardwareTab state={state} sendCmd={sendCmd} locale={runtime?.locale || 'en'} />}
            {activeTab === 'about' && (
              <PlayerInfoTab state={state} connectionMode={connectionMode} runtime={runtime} locale={runtime?.locale || 'en'} apiBaseUrl={apiBaseUrl} websocketUrl={resolveWebSocketUrl()} platform={platform} />
            )}
            {activeTab === 'preferences' && (
              <PreferencesTab
                apiBaseUrl={apiBaseUrl}
                locale={runtime?.locale || 'en'}
                onConfigChange={(values) => {
                  setAppConfig(values);
                  persistJson(STORAGE.config, values);
                  const seconds = Number(values.quick_seek_seconds);
                  if (Number.isFinite(seconds) && seconds > 0) setQuickSeekSeconds(seconds);
                }}
              />
            )}
            </React.Suspense>
          </Content>
        </Layout>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
