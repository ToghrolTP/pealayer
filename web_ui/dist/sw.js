const PRECACHE = 'pealayer-precache-25da5e8f93bbedbf';
const RUNTIME = 'pealayer-runtime-25da5e8f93bbedbf';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=25da5e8f93bbedbf",
  "/assets/DesktopOutlined.js?v=25da5e8f93bbedbf",
  "/assets/EditOutlined.js?v=25da5e8f93bbedbf",
  "/assets/EffectsTab.js?v=25da5e8f93bbedbf",
  "/assets/ExperimentOutlined.js?v=25da5e8f93bbedbf",
  "/assets/GroupSelect.js?v=25da5e8f93bbedbf",
  "/assets/HardwareTab.js?v=25da5e8f93bbedbf",
  "/assets/LinkOutlined.js?v=25da5e8f93bbedbf",
  "/assets/MediaLibraryTab.js?v=25da5e8f93bbedbf",
  "/assets/MediaTrackSelectors.js?v=25da5e8f93bbedbf",
  "/assets/PlayCircleOutlined.js?v=25da5e8f93bbedbf",
  "/assets/PlayerInfoTab.js?v=25da5e8f93bbedbf",
  "/assets/PlusOutlined.js?v=25da5e8f93bbedbf",
  "/assets/PreferencesTab.js?v=25da5e8f93bbedbf",
  "/assets/RemoteControlTab.js?v=25da5e8f93bbedbf",
  "/assets/SafetyCertificateOutlined.js?v=25da5e8f93bbedbf",
  "/assets/StopOutlined.js?v=25da5e8f93bbedbf",
  "/assets/StudioTab.js?v=25da5e8f93bbedbf",
  "/assets/UnlockOutlined.js?v=25da5e8f93bbedbf",
  "/assets/VideoCameraOutlined.js?v=25da5e8f93bbedbf",
  "/assets/WifiOutlined.js?v=25da5e8f93bbedbf",
  "/assets/app.css?v=25da5e8f93bbedbf",
  "/assets/app.js?v=25da5e8f93bbedbf",
  "/assets/card.js?v=25da5e8f93bbedbf",
  "/assets/color-picker.js?v=25da5e8f93bbedbf",
  "/assets/effectIcons.js?v=25da5e8f93bbedbf",
  "/assets/jsx-runtime.js?v=25da5e8f93bbedbf",
  "/assets/melodyCatalog.js?v=25da5e8f93bbedbf",
  "/assets/popconfirm.js?v=25da5e8f93bbedbf",
  "/assets/rolldown-runtime.js?v=25da5e8f93bbedbf",
  "/assets/row.js?v=25da5e8f93bbedbf",
  "/assets/slider.js?v=25da5e8f93bbedbf",
  "/assets/typography.js?v=25da5e8f93bbedbf",
  "/assets/useBreakpoint.js?v=25da5e8f93bbedbf",
  "/fuji-loader.css",
  "/fuji-loader.svg",
  "/index.html",
  "/manifest.webmanifest",
  "/api/runtime/app-icon",
  "/api/runtime/app-icon-192.png",
  "/api/runtime/app-icon-512.png"
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '25da5e8f93bbedbf' })),
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
