const PRECACHE = 'pealayer-precache-b0a85ab25bf04060';
const RUNTIME = 'pealayer-runtime-b0a85ab25bf04060';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=b0a85ab25bf04060",
  "/assets/DesktopOutlined.js?v=b0a85ab25bf04060",
  "/assets/EditOutlined.js?v=b0a85ab25bf04060",
  "/assets/EffectsTab.js?v=b0a85ab25bf04060",
  "/assets/ExperimentOutlined.js?v=b0a85ab25bf04060",
  "/assets/GroupSelect.js?v=b0a85ab25bf04060",
  "/assets/HardwareTab.js?v=b0a85ab25bf04060",
  "/assets/LinkOutlined.js?v=b0a85ab25bf04060",
  "/assets/MediaLibraryTab.js?v=b0a85ab25bf04060",
  "/assets/MediaTrackSelectors.js?v=b0a85ab25bf04060",
  "/assets/PlayCircleOutlined.js?v=b0a85ab25bf04060",
  "/assets/PlayerInfoTab.js?v=b0a85ab25bf04060",
  "/assets/PlusOutlined.js?v=b0a85ab25bf04060",
  "/assets/PreferencesTab.js?v=b0a85ab25bf04060",
  "/assets/RemoteControlTab.js?v=b0a85ab25bf04060",
  "/assets/SafetyCertificateOutlined.js?v=b0a85ab25bf04060",
  "/assets/StopOutlined.js?v=b0a85ab25bf04060",
  "/assets/StudioTab.js?v=b0a85ab25bf04060",
  "/assets/UnlockOutlined.js?v=b0a85ab25bf04060",
  "/assets/VideoCameraOutlined.js?v=b0a85ab25bf04060",
  "/assets/WifiOutlined.js?v=b0a85ab25bf04060",
  "/assets/app.css?v=b0a85ab25bf04060",
  "/assets/app.js?v=b0a85ab25bf04060",
  "/assets/card.js?v=b0a85ab25bf04060",
  "/assets/color-picker.js?v=b0a85ab25bf04060",
  "/assets/effectIcons.js?v=b0a85ab25bf04060",
  "/assets/jsx-runtime.js?v=b0a85ab25bf04060",
  "/assets/melodyCatalog.js?v=b0a85ab25bf04060",
  "/assets/popconfirm.js?v=b0a85ab25bf04060",
  "/assets/rolldown-runtime.js?v=b0a85ab25bf04060",
  "/assets/row.js?v=b0a85ab25bf04060",
  "/assets/slider.js?v=b0a85ab25bf04060",
  "/assets/typography.js?v=b0a85ab25bf04060",
  "/assets/useBreakpoint.js?v=b0a85ab25bf04060",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'b0a85ab25bf04060' })),
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
