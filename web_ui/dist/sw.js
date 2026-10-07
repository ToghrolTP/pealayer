const PRECACHE = 'pealayer-precache-eaf1c59a0192e7e0';
const RUNTIME = 'pealayer-runtime-eaf1c59a0192e7e0';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/DesktopOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/EditOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/EffectsTab.js?v=eaf1c59a0192e7e0",
  "/assets/ExperimentOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/GroupSelect.js?v=eaf1c59a0192e7e0",
  "/assets/HardwareTab.js?v=eaf1c59a0192e7e0",
  "/assets/LinkOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/MediaLibraryTab.js?v=eaf1c59a0192e7e0",
  "/assets/MediaTrackSelectors.js?v=eaf1c59a0192e7e0",
  "/assets/PlayCircleOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/PlayerInfoTab.js?v=eaf1c59a0192e7e0",
  "/assets/PlusOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/PreferencesTab.js?v=eaf1c59a0192e7e0",
  "/assets/RemoteControlTab.js?v=eaf1c59a0192e7e0",
  "/assets/SafetyCertificateOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/StopOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/StudioTab.js?v=eaf1c59a0192e7e0",
  "/assets/UnlockOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/VideoCameraOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/WifiOutlined.js?v=eaf1c59a0192e7e0",
  "/assets/app.css?v=eaf1c59a0192e7e0",
  "/assets/app.js?v=eaf1c59a0192e7e0",
  "/assets/card.js?v=eaf1c59a0192e7e0",
  "/assets/color-picker.js?v=eaf1c59a0192e7e0",
  "/assets/effectIcons.js?v=eaf1c59a0192e7e0",
  "/assets/jsx-runtime.js?v=eaf1c59a0192e7e0",
  "/assets/melodyCatalog.js?v=eaf1c59a0192e7e0",
  "/assets/popconfirm.js?v=eaf1c59a0192e7e0",
  "/assets/rolldown-runtime.js?v=eaf1c59a0192e7e0",
  "/assets/row.js?v=eaf1c59a0192e7e0",
  "/assets/slider.js?v=eaf1c59a0192e7e0",
  "/assets/typography.js?v=eaf1c59a0192e7e0",
  "/assets/useBreakpoint.js?v=eaf1c59a0192e7e0",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'eaf1c59a0192e7e0' })),
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
