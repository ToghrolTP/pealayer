import React, { useCallback, useEffect, useLayoutEffect, useState, useRef } from 'react';
import { Alert, ConfigProvider, theme, Layout, Menu, message } from 'antd';
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
import { ApplicationStatusBar, StatusBarVisibility } from './components/ApplicationStatusBar';
import { SharedToasts } from './components/SharedToasts';
import { FujiLoader } from './components/FujiLoader';
import { WebViewBoundary } from './components/WebViewBoundary';
import { RemoteLocationDialog } from './components/RemoteLocationDialog';
import './remote-location.css';
import type { PlayerState } from './components/RemoteControlTab';
import type { MediaGesturePreferences } from './components/MediaSurface';
import { tr } from './i18n';
import { useWebPlatform } from './webPlatform';
import './styles.css';
import './shared-toasts.css';
import palettes from '../../assets/themes/palettes.json';
import { accentForeground, mergeAppearance, paletteName as resolvePaletteName, resolvedAccent, resolvedAppearanceTheme } from './appearance';

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
  // Never resurrect transient notifications from offline storage.
  if (key === STORAGE.state && value && typeof value === 'object') {
    const { messages: _messages, remote_browser: _remoteBrowser, ...snapshot } = value as PlayerState;
    value = snapshot;
  }
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

const App: React.FC = () => {
  const [collapsed, setCollapsed] = useState<boolean>(() => window.localStorage.getItem('pealayer.sidebarCollapsed') === 'true');
  const [activeTab, setActiveTabState] = useState<SurfaceId>(surfaceFromLocation);
  const [connected, setConnected] = useState<boolean>(false);
  const [connectionMode, setConnectionMode] = useState<'ws' | 'http'>('http');
  const [state, setState] = useState<PlayerState>(() => ({ ...readStoredJson<PlayerState>(STORAGE.state, { status: 'initializing' }), messages: undefined }));
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
  const appearance = state.appearance;

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
      const nextTheme = resolvedAppearanceTheme(preference, media.matches, appearance);
      document.documentElement.dataset.theme = nextTheme;
      setResolvedTheme(nextTheme);
    };
    applyTheme();
    media.addEventListener('change', applyTheme);
    return () => media.removeEventListener('change', applyTheme);
  }, [runtime, appConfig?.theme, appearance?.resolved_theme, activeTab]);

  useEffect(() => {
    if (!appearance) return;
    setAppConfig((previous) => {
      if (previous?.theme === appearance.theme
        && previous?.color_palette === appearance.color_palette
        && previous?.accent_color === appearance.accent_color
        && previous?.custom_accent_color === appearance.custom_accent_color) return previous;
      const next = mergeAppearance(previous ?? {}, appearance);
      persistJson(STORAGE.config, next);
      return next;
    });
  }, [appearance?.theme, appearance?.color_palette, appearance?.accent_color, appearance?.custom_accent_color]);

  const accentColor = resolvedAccent(runtime, appConfig, appearance);
  const accentTextColor = accentForeground(accentColor);
  const paletteName = resolvePaletteName(appConfig);
  const palette = palettes[paletteName][resolvedTheme];
  const mediaGestures: MediaGesturePreferences = {
    clickPlayerToToggle: appConfig?.click_player_to_toggle ?? true,
    pausedDragAction: appConfig?.paused_drag_action ?? 'move_window',
    playingDragAction: appConfig?.playing_drag_action ?? 'temporary_fast_forward',
    middleClickAction: appConfig?.middle_click_action ?? 'none',
    middleHoldAction: appConfig?.middle_hold_action ?? 'none',
    rightClickAction: appConfig?.right_click_action ?? 'context_menu',
    rightHoldAction: appConfig?.right_hold_action ?? 'none',
    temporaryFastForwardSpeed: Number(appConfig?.temporary_fast_forward_speed ?? 2),
    playbackSpeed: Number(appConfig?.playback_speed ?? state.playback_rate ?? 1),
  };

  useLayoutEffect(() => {
    document.documentElement.dataset.palette = paletteName;
    for (const [role, value] of Object.entries(palette)) {
      document.documentElement.style.setProperty(`--${role}`, value);
    }
  }, [paletteName, palette]);

  useEffect(() => {
    document.documentElement.style.setProperty('--accent', accentColor);
    document.documentElement.style.setProperty('--accent-text', accentTextColor);
    document.documentElement.style.setProperty('--accent-soft', `color-mix(in srgb, ${accentColor} 16%, transparent)`);
  }, [accentColor, accentTextColor]);

  const nextRequestId = useRef(1);
  const pendingRequests = useRef(new Map<number, { finish: (ok: boolean) => void; timer: number }>());
  const apiEndpoint = useCallback((path: string) => {
    if (!connectionTarget) return path;
    const address = connectionTarget.replace(/^pealayer:\/\//i, 'http://');
    const target = new URL(address.includes('://') ? address : `http://${address}`);
    if (target.protocol === 'ws:') target.protocol = 'http:';
    if (target.protocol === 'wss:') target.protocol = 'https:';
    target.pathname = path;
    target.search = '';
    target.hash = '';
    return target.toString();
  }, [connectionTarget]);
  const completeRequest = useCallback((response: any) => {
    const pending = pendingRequests.current.get(Number(response.id));
    if (!pending) return;
    window.clearTimeout(pending.timer);
    pendingRequests.current.delete(Number(response.id));
    const error = response.error?.message || response.error || (response.result?.accepted === false ? 'Command dispatcher unavailable' : null);
    if (error) void message.error(String(error));
    pending.finish(!error);
  }, []);
  const rawSendCmd = useCallback((command: string, payload: Record<string, any> = {}): Promise<boolean> => {
    const id = nextRequestId.current++;
    const request = { jsonrpc: '2.0', id, method: command, params: payload };
    return new Promise((finish) => {
      const timer = window.setTimeout(() => {
        completeRequest({ id, error: { message: 'Command acknowledgement timed out; check the current state before retrying.' } });
      }, 15_000);
      pendingRequests.current.set(id, { finish, timer });
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      try { wsRef.current.send(JSON.stringify(request)); }
      catch (error) { completeRequest({ id, error: { message: String(error) } }); }
    } else {
      // Identical method, parameters and correlation ID on both transports.
      try { void fetch(apiEndpoint('/api/rpc'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }).then(async (response) => {
        const result = await response.json();
        if (!response.ok && !result.error) result.error = { message: `Command rejected (${response.status})` };
        completeRequest({ ...result, id });
      }).catch((error) => completeRequest({ id, error: { message: String(error) } })); }
      catch (error) { completeRequest({ id, error: { message: String(error) } }); }
    }
    });
  }, [apiEndpoint, completeRequest]);

  const playbackIconState = !state.current_video ? 'stopped' : state.playing ? 'playing' : 'paused';
  const stateAppIconPath = `${runtime?.appIconPath || '/api/runtime/app-icon'}?state=${playbackIconState}`;
  const platform = useWebPlatform(
    state,
    rawSendCmd,
    runtime?.appName || 'Pealayer',
    stateAppIconPath,
  );

  const signalInteraction = platform.signalInteraction;

  const sendCmd = useCallback((command: string, payload: Record<string, any> = {}) => {
    if (!platform.online && !connected) {
      void message.error(tr(runtime?.locale || 'en', 'Pealayer backend unavailable'));
      return Promise.resolve(false);
    }
    signalInteraction();
    return rawSendCmd(command, payload);
  }, [connected, platform.online, rawSendCmd, signalInteraction, runtime?.locale]);

  const updateStatusBarVisibility = useCallback((visibility: StatusBarVisibility) => {
    setAppConfig((previous) => {
      const next = { ...(previous ?? {}), status_bar: visibility };
      persistJson(STORAGE.config, next);
      return next;
    });
    void fetch(apiEndpoint('/api/config'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status_bar: visibility }),
    }).then(async (response) => {
      if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body.error || `Status bar update failed (${response.status})`);
      }
    }).catch((error) => void message.error(String(error)));
  }, [apiEndpoint]);

  const resolveWebSocketUrl = useCallback(() => {
    const fallbackProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    if (!connectionTarget) {
      return `${fallbackProtocol}//${window.location.host}${runtime?.websocketPath || '/ws'}`;
    }
    try {
      const address = connectionTarget.replace(/^pealayer:\/\//i, 'ws://');
      const target = new URL(address.includes('://') ? address : `ws://${address}`);
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
    fetch(apiEndpoint('/api/runtime/config'))
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
  }, [apiEndpoint]);

  useEffect(() => {
    if (!runtime) return;
    let disposed = false;
    fetch(apiEndpoint('/api/config'))
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
  }, [runtime, apiEndpoint]);

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
            if (data && data.jsonrpc === '2.0') { completeRequest(data); return; }
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
      if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
        return; // Skip HTTP polling when WebSocket is connected
      }
      try {
        const res = await fetch(apiEndpoint('/api/player/status'));
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
  }, [runtime, connectionTarget, resolveWebSocketUrl, apiEndpoint, completeRequest]);

  useEffect(() => () => {
    for (const pending of pendingRequests.current.values()) {
      window.clearTimeout(pending.timer);
      pending.finish(false);
    }
    pendingRequests.current.clear();
  }, []);

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
          // Ant Design derives additional colors from these tokens. Supply
          // actual RGB values, not CSS var() strings its color math cannot parse.
          colorInfo: palette.blue,
          colorSuccess: palette.green,
          colorWarning: palette.amber,
          colorError: palette.red,
          colorBgContainer: palette['surface-1'],
          colorBgElevated: palette['surface-1'],
          colorBgBase: palette.canvas,
          colorBorder: palette.line,
          colorText: palette.text,
          colorTextSecondary: palette.muted,
          borderRadius: 9,
          fontFamily: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`,
        },
        components: {
          Modal: {
            contentBg: palette['surface-1'],
            headerBg: 'transparent',
            footerBg: 'transparent',
            titleColor: palette.text,
          },
        },
      }}
    >
      <SharedToasts snapshot={state.messages} connected={connected} dismiss={id => sendCmd('pealayer.toast.dismiss', { id })} />
      <RemoteLocationDialog state={state.remote_browser} connected={connected} base={apiBaseUrl} sendCmd={sendCmd} />
      <Layout className="app-shell">
        <HeaderBar
          collapsed={collapsed}
          onToggleCollapse={() => setCollapsed(!collapsed)}
          connected={connected}
          connectionMode={connectionMode}
          appName={runtime?.appName}
          appIconPath={stateAppIconPath}
          locale={runtime?.locale || 'en'}
          connectionTarget={connectionTarget}
          onConnectionTargetChange={changeConnectionTarget}
          platform={platform}
          onBrowseRemote={() => sendCmd('pealayer.remote.browse', { target: '' })}
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
            <WebViewBoundary key={activeTab} locale={runtime?.locale || 'en'}>
            <React.Suspense fallback={<FujiLoader locale={runtime?.locale || 'en'} />}>
            {activeTab === 'timeline' && (
              <StudioTab
                state={state}
                sendCmd={sendCmd}
                locale={runtime?.locale || 'en'}
                appName={runtime?.appName || 'Pealayer'}
                quickSeekSeconds={quickSeekSeconds}
                apiBaseUrl={apiBaseUrl}
                seekbarHoverThumbnails={Boolean(appConfig?.nle_seekbar_hover_thumbnails)}
                surface="timeline"
                timelineWheelPreferences={state.timeline_wheel_preferences ?? {
                  plain: appConfig?.timeline_plain_wheel_action ?? 'zoom',
                  ctrl: appConfig?.timeline_ctrl_wheel_action ?? 'vertical_scroll',
                  shift: appConfig?.timeline_shift_wheel_action ?? 'horizontal_scroll',
                  alt: appConfig?.timeline_alt_wheel_action ?? 'zoom',
                }}
                mediaGestures={mediaGestures}
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
                mediaGestures={mediaGestures}
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
                appearance={appearance}
                timelineWheelPreferences={state.timeline_wheel_preferences}
                onConfigChange={(values) => {
                  setAppConfig(values);
                  persistJson(STORAGE.config, values);
                  const seconds = Number(values.quick_seek_seconds);
                  if (Number.isFinite(seconds) && seconds > 0) setQuickSeekSeconds(seconds);
                }}
              />
            )}
            </React.Suspense>
            </WebViewBoundary>
          </Content>
        </Layout>
        <ApplicationStatusBar
          state={state}
          connected={connected}
          connectionMode={connectionMode}
          activeSurface={activeTab === 'timeline' ? tr(runtime?.locale || 'en', 'Timeline') : tr(runtime?.locale || 'en', menuItems.find(item => item?.key === activeTab)?.label as string || activeTab)}
          locale={runtime?.locale || 'en'}
          visibility={appConfig?.status_bar}
          onVisibilityChange={updateStatusBarVisibility}
        />
      </Layout>
    </ConfigProvider>
  );
};

export default App;
