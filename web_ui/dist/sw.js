const PRECACHE = 'pealayer-precache-4abe3ec7e48ed30b';
const RUNTIME = 'pealayer-runtime-4abe3ec7e48ed30b';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/DesktopOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/EditOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/EffectRecorder.js?v=4abe3ec7e48ed30b",
  "/assets/EffectsTab.js?v=4abe3ec7e48ed30b",
  "/assets/ExperimentOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/GroupSelect.js?v=4abe3ec7e48ed30b",
  "/assets/HardwareTab.js?v=4abe3ec7e48ed30b",
  "/assets/MediaLibraryTab.js?v=4abe3ec7e48ed30b",
  "/assets/PlayerInfoTab.js?v=4abe3ec7e48ed30b",
  "/assets/PlusOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/PoweroffOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/PreferencesTab.js?v=4abe3ec7e48ed30b",
  "/assets/RemoteControlTab.js?v=4abe3ec7e48ed30b",
  "/assets/SafetyCertificateOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/SaveOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/SeekThumbnailPreview.js?v=4abe3ec7e48ed30b",
  "/assets/StudioTab.js?v=4abe3ec7e48ed30b",
  "/assets/VideoCameraOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/WifiOutlined.js?v=4abe3ec7e48ed30b",
  "/assets/app.css?v=4abe3ec7e48ed30b",
  "/assets/app.js?v=4abe3ec7e48ed30b",
  "/assets/card.js?v=4abe3ec7e48ed30b",
  "/assets/color-picker.js?v=4abe3ec7e48ed30b",
  "/assets/jsx-runtime.js?v=4abe3ec7e48ed30b",
  "/assets/popconfirm.js?v=4abe3ec7e48ed30b",
  "/assets/rolldown-runtime.js?v=4abe3ec7e48ed30b",
  "/assets/row.js?v=4abe3ec7e48ed30b",
  "/assets/select.js?v=4abe3ec7e48ed30b",
  "/assets/slider.js?v=4abe3ec7e48ed30b",
  "/assets/typography.js?v=4abe3ec7e48ed30b",
  "/assets/useBreakpoint.js?v=4abe3ec7e48ed30b",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '4abe3ec7e48ed30b' })),
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
