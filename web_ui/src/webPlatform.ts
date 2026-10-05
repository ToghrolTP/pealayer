import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { PlayerState } from './components/RemoteControlTab';

interface InstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>;
}

interface NavigatorWithStandalone extends Navigator {
  standalone?: boolean;
}

export interface WebPlatformCapabilities {
  install: boolean;
  share: boolean;
  fullscreen: boolean;
  notifications: boolean;
  vibration: boolean;
  audio: boolean;
  mediaSession: boolean;
  wakeLock: boolean;
  badging: boolean;
  serviceWorker: boolean;
}

export interface WebPlatformController {
  capabilities: WebPlatformCapabilities;
  online: boolean;
  standalone: boolean;
  updateReady: boolean;
  hapticsEnabled: boolean;
  audioFeedbackEnabled: boolean;
  keepAwakeEnabled: boolean;
  install: () => Promise<boolean>;
  share: () => Promise<boolean>;
  toggleFullscreen: () => Promise<boolean>;
  enableNotifications: () => Promise<NotificationPermission | 'unsupported'>;
  applyUpdate: () => void;
  setHapticsEnabled: (enabled: boolean) => void;
  setAudioFeedbackEnabled: (enabled: boolean) => void;
  setKeepAwakeEnabled: (enabled: boolean) => void;
  signalInteraction: () => void;
}

type CommandDispatcher = (command: string, payload?: Record<string, unknown>) => void;

const STORAGE_PREFIX = 'pealayer.webPlatform.';
let serviceWorkerRegistration: ServiceWorkerRegistration | null = null;

function storedBoolean(key: string, fallback: boolean): boolean {
  const value = window.localStorage.getItem(`${STORAGE_PREFIX}${key}`);
  return value === null ? fallback : value === 'true';
}

function mediaTitle(state: PlayerState, appName: string): string {
  const source = state.current_video?.trim();
  if (!source) return appName;
  try {
    const url = new URL(source);
    const parts = url.pathname.split('/').filter(Boolean);
    const tail = parts[parts.length - 1] || appName;
    return decodeURIComponent(tail.replace(/\.[^.]+$/, ''));
  } catch {
    const parts = source.split(/[\\/]/).filter(Boolean);
    const tail = parts[parts.length - 1] || appName;
    try { return decodeURIComponent(tail.replace(/\.[^.]+$/, '')); } catch { return tail; }
  }
}

function supportsStandaloneMode(): boolean {
  const navigatorWithStandalone = navigator as NavigatorWithStandalone;
  return window.matchMedia('(display-mode: standalone)').matches || navigatorWithStandalone.standalone === true;
}

export function useWebPlatform(
  state: PlayerState,
  dispatchCommand: CommandDispatcher,
  appName: string,
  appIconPath: string,
): WebPlatformController {
  const [online, setOnline] = useState(navigator.onLine);
  const [standalone, setStandalone] = useState(supportsStandaloneMode);
  const [installPrompt, setInstallPrompt] = useState<InstallPromptEvent | null>(null);
  const [updateReady, setUpdateReady] = useState(false);
  const [hapticsEnabled, setHapticsEnabledState] = useState(() => storedBoolean('haptics', true));
  const [audioFeedbackEnabled, setAudioFeedbackEnabledState] = useState(() => storedBoolean('audioFeedback', false));
  const [keepAwakeEnabled, setKeepAwakeEnabledState] = useState(() => storedBoolean('keepAwake', true));
  const wakeLockRef = useRef<WakeLockSentinel | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);

  const capabilities = useMemo<WebPlatformCapabilities>(() => {
    return {
      install: Boolean(installPrompt),
      share: typeof navigator.share === 'function',
      fullscreen: typeof document.documentElement.requestFullscreen === 'function',
      notifications: 'Notification' in window,
      vibration: typeof navigator.vibrate === 'function',
      audio: 'AudioContext' in window || 'webkitAudioContext' in window,
      mediaSession: 'mediaSession' in navigator,
      wakeLock: 'wakeLock' in navigator,
      badging: typeof navigator.setAppBadge === 'function',
      serviceWorker: 'serviceWorker' in navigator,
    };
  }, [installPrompt]);

  useEffect(() => {
    const onOnline = () => setOnline(true);
    const onOffline = () => setOnline(false);
    const onInstallPrompt = (event: Event) => {
      event.preventDefault();
      setInstallPrompt(event as InstallPromptEvent);
    };
    const onInstalled = () => {
      setInstallPrompt(null);
      setStandalone(true);
    };
    const displayMode = window.matchMedia('(display-mode: standalone)');
    const onDisplayMode = () => setStandalone(supportsStandaloneMode());
    const onServiceWorkerMessage = (event: MessageEvent) => {
      if (event.data?.type === 'PEALAYER_SW_READY') setUpdateReady(Boolean(event.data.update));
    };
    const onServiceWorkerUpdate = () => setUpdateReady(true);

    window.addEventListener('online', onOnline);
    window.addEventListener('offline', onOffline);
    window.addEventListener('beforeinstallprompt', onInstallPrompt);
    window.addEventListener('appinstalled', onInstalled);
    displayMode.addEventListener('change', onDisplayMode);
    navigator.serviceWorker?.addEventListener('message', onServiceWorkerMessage);
    window.addEventListener('pealayer-sw-update', onServiceWorkerUpdate);
    return () => {
      window.removeEventListener('online', onOnline);
      window.removeEventListener('offline', onOffline);
      window.removeEventListener('beforeinstallprompt', onInstallPrompt);
      window.removeEventListener('appinstalled', onInstalled);
      displayMode.removeEventListener('change', onDisplayMode);
      navigator.serviceWorker?.removeEventListener('message', onServiceWorkerMessage);
      window.removeEventListener('pealayer-sw-update', onServiceWorkerUpdate);
    };
  }, []);

  useEffect(() => {
    if (!('mediaSession' in navigator)) return;
    navigator.mediaSession.metadata = new MediaMetadata({
      title: mediaTitle(state, appName),
      artist: appName,
      album: state.live ? 'Live stream' : 'Pealayer media',
      artwork: [
        { src: appIconPath || '/api/runtime/app-icon-192.png', sizes: '192x192', type: 'image/png' },
        { src: '/api/runtime/app-icon-512.png', sizes: '512x512', type: 'image/png' },
      ],
    });
    navigator.mediaSession.playbackState = state.playing ? 'playing' : 'paused';
    const actions: Array<[MediaSessionAction, MediaSessionActionHandler]> = [
      ['play', () => dispatchCommand('play')],
      ['pause', () => dispatchCommand('pause')],
      ['stop', () => dispatchCommand('pause')],
      ['seekbackward', (details) => dispatchCommand('seek', { seconds: -(details.seekOffset || 10) })],
      ['seekforward', (details) => dispatchCommand('seek', { seconds: details.seekOffset || 10 })],
      ['seekto', (details) => {
        if (typeof details.seekTime === 'number') dispatchCommand('seek_to', { seconds: details.seekTime });
      }],
      ['previoustrack', () => dispatchCommand('chapter_previous')],
      ['nexttrack', () => dispatchCommand('chapter_next')],
    ];
    for (const [action, handler] of actions) {
      try { navigator.mediaSession.setActionHandler(action, handler); } catch { /* Browser exposes only a subset. */ }
    }
    if (Number.isFinite(state.duration) && Number.isFinite(state.playback_time) && (state.duration ?? 0) > 0) {
      try {
        navigator.mediaSession.setPositionState({
          duration: state.duration!,
          playbackRate: state.playback_rate || 1,
          position: Math.min(Math.max(0, state.playback_time || 0), state.duration!),
        });
      } catch { /* Live streams and partially-known media may reject position data. */ }
    }
  }, [appIconPath, appName, dispatchCommand, state.current_video, state.duration, state.live, state.playback_rate, state.playback_time, state.playing]);

  useEffect(() => {
    const shouldHold = keepAwakeEnabled && state.playing && document.visibilityState === 'visible';
    let cancelled = false;
    const updateWakeLock = async () => {
      if (!('wakeLock' in navigator)) return;
      if (!shouldHold) {
        await wakeLockRef.current?.release().catch(() => undefined);
        wakeLockRef.current = null;
        return;
      }
      if (!wakeLockRef.current) {
        try {
          const sentinel = await navigator.wakeLock.request('screen');
          if (cancelled) await sentinel.release();
          else wakeLockRef.current = sentinel;
        } catch { /* Permission, battery, or visibility can deny the request. */ }
      }
    };
    void updateWakeLock();
    const onVisibility = () => void updateWakeLock();
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      cancelled = true;
      document.removeEventListener('visibilitychange', onVisibility);
      if (!shouldHold) void wakeLockRef.current?.release().catch(() => undefined);
    };
  }, [keepAwakeEnabled, state.playing]);

  useEffect(() => {
    if (!navigator.setAppBadge) return;
    if (!online) void navigator.setAppBadge(1).catch(() => undefined);
    else void navigator.clearAppBadge().catch(() => undefined);
  }, [online]);

  const install = useCallback(async () => {
    if (!installPrompt) return false;
    await installPrompt.prompt();
    const choice = await installPrompt.userChoice;
    if (choice.outcome === 'accepted') setInstallPrompt(null);
    return choice.outcome === 'accepted';
  }, [installPrompt]);

  const share = useCallback(async () => {
    if (!navigator.share) return false;
    try {
      await navigator.share({ title: appName, text: `Control ${appName}`, url: window.location.href });
      return true;
    } catch { return false; }
  }, [appName]);

  const toggleFullscreen = useCallback(async () => {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
      return true;
    } catch { return false; }
  }, []);

  const enableNotifications = useCallback(async () => {
    if (!('Notification' in window)) return 'unsupported' as const;
    return Notification.requestPermission();
  }, []);

  const applyUpdate = useCallback(() => {
    const worker = serviceWorkerRegistration?.waiting;
    if (worker) worker.postMessage({ type: 'SKIP_WAITING' });
    else window.location.reload();
  }, []);

  const setHapticsEnabled = useCallback((enabled: boolean) => {
    setHapticsEnabledState(enabled);
    window.localStorage.setItem(`${STORAGE_PREFIX}haptics`, String(enabled));
  }, []);
  const setAudioFeedbackEnabled = useCallback((enabled: boolean) => {
    setAudioFeedbackEnabledState(enabled);
    window.localStorage.setItem(`${STORAGE_PREFIX}audioFeedback`, String(enabled));
  }, []);
  const setKeepAwakeEnabled = useCallback((enabled: boolean) => {
    setKeepAwakeEnabledState(enabled);
    window.localStorage.setItem(`${STORAGE_PREFIX}keepAwake`, String(enabled));
  }, []);

  const signalInteraction = useCallback(() => {
    if (hapticsEnabled) navigator.vibrate?.(12);
    if (!audioFeedbackEnabled) return;
    try {
      const AudioContextClass = window.AudioContext || (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AudioContextClass) return;
      const context = audioContextRef.current ?? new AudioContextClass();
      audioContextRef.current = context;
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = 'sine';
      oscillator.frequency.value = 520;
      gain.gain.setValueAtTime(0.028, context.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.045);
      oscillator.connect(gain).connect(context.destination);
      oscillator.start();
      oscillator.stop(context.currentTime + 0.05);
    } catch { /* Audio feedback is best-effort and always user-configurable. */ }
  }, [audioFeedbackEnabled, hapticsEnabled]);

  return {
    capabilities,
    online,
    standalone,
    updateReady,
    hapticsEnabled,
    audioFeedbackEnabled,
    keepAwakeEnabled,
    install,
    share,
    toggleFullscreen,
    enableNotifications,
    applyUpdate,
    setHapticsEnabled,
    setAudioFeedbackEnabled,
    setKeepAwakeEnabled,
    signalInteraction,
  };
}

export async function registerPealayerServiceWorker(): Promise<void> {
  if (!('serviceWorker' in navigator) || !import.meta.env.PROD) return;
  const registration = await navigator.serviceWorker.register('/sw.js', { scope: '/', updateViaCache: 'none' });
  serviceWorkerRegistration = registration;
  await registration.update().catch(() => undefined);
  if (registration.waiting) {
    window.dispatchEvent(new Event('pealayer-sw-update'));
  }
  registration.addEventListener('updatefound', () => {
    const worker = registration.installing;
    worker?.addEventListener('statechange', () => {
      if (worker.state === 'installed' && navigator.serviceWorker.controller) {
        window.dispatchEvent(new Event('pealayer-sw-update'));
      }
    });
  });
  let refreshing = false;
  navigator.serviceWorker.addEventListener('controllerchange', () => {
    if (refreshing) return;
    refreshing = true;
    window.location.reload();
  });
}
