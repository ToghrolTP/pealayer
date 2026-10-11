const PRECACHE = 'pealayer-precache-d9933ac1a13a0854';
const RUNTIME = 'pealayer-runtime-d9933ac1a13a0854';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=d9933ac1a13a0854",
  "/assets/ArrowDownOutlined.js?v=d9933ac1a13a0854",
  "/assets/ArrowUpOutlined.js?v=d9933ac1a13a0854",
  "/assets/ColorPicker.js?v=d9933ac1a13a0854",
  "/assets/DeleteOutlined.js?v=d9933ac1a13a0854",
  "/assets/DesktopOutlined.js?v=d9933ac1a13a0854",
  "/assets/DownloadsTab.js?v=d9933ac1a13a0854",
  "/assets/EffectsTab.js?v=d9933ac1a13a0854",
  "/assets/ElapsedTimeInput.js?v=d9933ac1a13a0854",
  "/assets/ExpandOutlined.js?v=d9933ac1a13a0854",
  "/assets/GroupSelect.js?v=d9933ac1a13a0854",
  "/assets/HardwareTab.js?v=d9933ac1a13a0854",
  "/assets/MediaLibraryTab.js?v=d9933ac1a13a0854",
  "/assets/MelodySelect.js?v=d9933ac1a13a0854",
  "/assets/PauseOutlined.js?v=d9933ac1a13a0854",
  "/assets/PlayCircleOutlined.js?v=d9933ac1a13a0854",
  "/assets/PlayerInfoTab.js?v=d9933ac1a13a0854",
  "/assets/PlusOutlined.js?v=d9933ac1a13a0854",
  "/assets/PreferencesTab.js?v=d9933ac1a13a0854",
  "/assets/PushpinOutlined.js?v=d9933ac1a13a0854",
  "/assets/RemoteControlTab.js?v=d9933ac1a13a0854",
  "/assets/SafetyCertificateOutlined.js?v=d9933ac1a13a0854",
  "/assets/ServerFilePicker.js?v=d9933ac1a13a0854",
  "/assets/SettingOutlined.js?v=d9933ac1a13a0854",
  "/assets/StopOutlined.js?v=d9933ac1a13a0854",
  "/assets/StudioTab.js?v=d9933ac1a13a0854",
  "/assets/TimeValueField.js?v=d9933ac1a13a0854",
  "/assets/WifiOutlined.js?v=d9933ac1a13a0854",
  "/assets/app.css?v=d9933ac1a13a0854",
  "/assets/app.js?v=d9933ac1a13a0854",
  "/assets/card.js?v=d9933ac1a13a0854",
  "/assets/dropdown.js?v=d9933ac1a13a0854",
  "/assets/input-number.js?v=d9933ac1a13a0854",
  "/assets/jsx-runtime.js?v=d9933ac1a13a0854",
  "/assets/popconfirm.js?v=d9933ac1a13a0854",
  "/assets/popover.js?v=d9933ac1a13a0854",
  "/assets/progress.js?v=d9933ac1a13a0854",
  "/assets/rolldown-runtime.js?v=d9933ac1a13a0854",
  "/assets/row.js?v=d9933ac1a13a0854",
  "/assets/slider.js?v=d9933ac1a13a0854",
  "/assets/style.js?v=d9933ac1a13a0854",
  "/assets/table.js?v=d9933ac1a13a0854",
  "/assets/typography.js?v=d9933ac1a13a0854",
  "/assets/useBreakpoint.js?v=d9933ac1a13a0854",
  "/fuji-loader.css",
  "/fuji-loader.svg",
  "/index.html",
  "/manifest.webmanifest"
];

const notifyClients = async (message) => {
  const clients = await self.clients.matchAll({ includeUncontrolled: true, type: 'window' });
  for (const client of clients) client.postMessage(message);
};

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(PRECACHE)
      .then((cache) => cache.addAll(PRECACHE_URLS.map((url) => new Request(url, { cache: 'reload' }))))
      .then(() => (self.registration.active ? undefined : self.skipWaiting())),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys
        .filter((key) => key.startsWith('pealayer-') && ![PRECACHE, RUNTIME].includes(key))
        .map((key) => caches.delete(key))))
      .then(() => self.clients.claim())
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'd9933ac1a13a0854' })),
  );
});

self.addEventListener('message', (event) => {
  // Message shape is not authorization: verify the sending window as well.
  if (event.origin !== self.location.origin || !event.source?.id) return;
  const sourceId = event.source.id;
  event.waitUntil((async () => {
    const client = await self.clients.get(sourceId);
    if (!client || client.type !== 'window' || new URL(client.url).origin !== self.location.origin) return;
    if (event.data?.type === 'SKIP_WAITING') await self.skipWaiting();
    if (event.data?.type === 'CLEAR_RUNTIME_CACHE') await caches.delete(RUNTIME);
  })());
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') return;
  const url = new URL(request.url);
  const precached = PRECACHE_URLS.includes(url.pathname);
  if (
    url.origin !== self.location.origin
    || url.pathname === '/ws'
    || (url.pathname.startsWith('/api/') && !precached)
  ) return;

  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then((response) => {
          if (response.ok) void caches.open(RUNTIME).then((cache) => cache.put('/', response.clone()));
          return response;
        })
        .catch(async () => (await (await caches.open(RUNTIME)).match('/'))
          || (await (await caches.open(PRECACHE)).match('/index.html')) || Response.error()),
    );
    return;
  }

  const versionedAsset = url.pathname.startsWith('/assets/') && url.searchParams.has('v');
  if (versionedAsset) {
    // Exact versioned URL lookup, never ignoreSearch or cross-generation
    // caches.match: clean filenames must not return bytes from another build.
    event.respondWith(caches.open(PRECACHE).then(cache => cache.match(request)).then(cached => cached || fetch(request)));
    return;
  }
  if (precached || url.pathname.startsWith('/assets/')) {
    event.respondWith(fetch(request, { cache: 'no-cache' })
      .catch(async () => (await (await caches.open(PRECACHE)).match(request)) || Response.error()));
    return;
  }

  event.respondWith(
    fetch(request)
      .then((response) => {
        if (response.ok) void caches.open(RUNTIME).then((cache) => cache.put(request, response.clone()));
        return response;
      })
      .catch(() => caches.match(request)),
  );
});
