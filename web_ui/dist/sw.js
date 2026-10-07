const PRECACHE = 'pealayer-precache-7cebd4198f625a69';
const RUNTIME = 'pealayer-runtime-7cebd4198f625a69';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=7cebd4198f625a69",
  "/assets/DesktopOutlined.js?v=7cebd4198f625a69",
  "/assets/EditOutlined.js?v=7cebd4198f625a69",
  "/assets/EffectsTab.js?v=7cebd4198f625a69",
  "/assets/ExperimentOutlined.js?v=7cebd4198f625a69",
  "/assets/GroupSelect.js?v=7cebd4198f625a69",
  "/assets/HardwareTab.js?v=7cebd4198f625a69",
  "/assets/LinkOutlined.js?v=7cebd4198f625a69",
  "/assets/MediaLibraryTab.js?v=7cebd4198f625a69",
  "/assets/MediaTrackSelectors.js?v=7cebd4198f625a69",
  "/assets/PlayCircleOutlined.js?v=7cebd4198f625a69",
  "/assets/PlayerInfoTab.js?v=7cebd4198f625a69",
  "/assets/PlusOutlined.js?v=7cebd4198f625a69",
  "/assets/PreferencesTab.js?v=7cebd4198f625a69",
  "/assets/RemoteControlTab.js?v=7cebd4198f625a69",
  "/assets/SafetyCertificateOutlined.js?v=7cebd4198f625a69",
  "/assets/StopOutlined.js?v=7cebd4198f625a69",
  "/assets/StudioTab.js?v=7cebd4198f625a69",
  "/assets/UnlockOutlined.js?v=7cebd4198f625a69",
  "/assets/VideoCameraOutlined.js?v=7cebd4198f625a69",
  "/assets/WifiOutlined.js?v=7cebd4198f625a69",
  "/assets/app.css?v=7cebd4198f625a69",
  "/assets/app.js?v=7cebd4198f625a69",
  "/assets/card.js?v=7cebd4198f625a69",
  "/assets/color-picker.js?v=7cebd4198f625a69",
  "/assets/effectIcons.js?v=7cebd4198f625a69",
  "/assets/jsx-runtime.js?v=7cebd4198f625a69",
  "/assets/melodyCatalog.js?v=7cebd4198f625a69",
  "/assets/popconfirm.js?v=7cebd4198f625a69",
  "/assets/rolldown-runtime.js?v=7cebd4198f625a69",
  "/assets/row.js?v=7cebd4198f625a69",
  "/assets/slider.js?v=7cebd4198f625a69",
  "/assets/typography.js?v=7cebd4198f625a69",
  "/assets/useBreakpoint.js?v=7cebd4198f625a69",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '7cebd4198f625a69' })),
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
