const PRECACHE = 'pealayer-precache-02a54db3d46927eb';
const RUNTIME = 'pealayer-runtime-02a54db3d46927eb';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=02a54db3d46927eb",
  "/assets/ArrowUpOutlined.js?v=02a54db3d46927eb",
  "/assets/DeleteOutlined.js?v=02a54db3d46927eb",
  "/assets/DesktopOutlined.js?v=02a54db3d46927eb",
  "/assets/EditOutlined.js?v=02a54db3d46927eb",
  "/assets/EffectsTab.js?v=02a54db3d46927eb",
  "/assets/ExpandOutlined.js?v=02a54db3d46927eb",
  "/assets/GroupSelect.js?v=02a54db3d46927eb",
  "/assets/HardwareTab.js?v=02a54db3d46927eb",
  "/assets/MediaLibraryTab.js?v=02a54db3d46927eb",
  "/assets/MediaTrackSelectors.js?v=02a54db3d46927eb",
  "/assets/PlayCircleOutlined.js?v=02a54db3d46927eb",
  "/assets/PlayerInfoTab.js?v=02a54db3d46927eb",
  "/assets/PlusOutlined.js?v=02a54db3d46927eb",
  "/assets/PreferencesTab.js?v=02a54db3d46927eb",
  "/assets/PushpinOutlined.js?v=02a54db3d46927eb",
  "/assets/RemoteControlTab.js?v=02a54db3d46927eb",
  "/assets/SafetyCertificateOutlined.js?v=02a54db3d46927eb",
  "/assets/SettingOutlined.js?v=02a54db3d46927eb",
  "/assets/StopOutlined.js?v=02a54db3d46927eb",
  "/assets/StudioTab.js?v=02a54db3d46927eb",
  "/assets/VideoCameraOutlined.js?v=02a54db3d46927eb",
  "/assets/WifiOutlined.js?v=02a54db3d46927eb",
  "/assets/app.css?v=02a54db3d46927eb",
  "/assets/app.js?v=02a54db3d46927eb",
  "/assets/card.js?v=02a54db3d46927eb",
  "/assets/color-picker.js?v=02a54db3d46927eb",
  "/assets/effectIcons.js?v=02a54db3d46927eb",
  "/assets/jsx-runtime.js?v=02a54db3d46927eb",
  "/assets/melodyCatalog.js?v=02a54db3d46927eb",
  "/assets/popconfirm.js?v=02a54db3d46927eb",
  "/assets/rolldown-runtime.js?v=02a54db3d46927eb",
  "/assets/row.js?v=02a54db3d46927eb",
  "/assets/style.js?v=02a54db3d46927eb",
  "/assets/typography.js?v=02a54db3d46927eb",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '02a54db3d46927eb' })),
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
