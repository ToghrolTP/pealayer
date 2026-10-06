const PRECACHE = 'pealayer-precache-d7ee10920d015e8f';
const RUNTIME = 'pealayer-runtime-d7ee10920d015e8f';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=d7ee10920d015e8f",
  "/assets/DesktopOutlined.js?v=d7ee10920d015e8f",
  "/assets/EditOutlined.js?v=d7ee10920d015e8f",
  "/assets/EffectsTab.js?v=d7ee10920d015e8f",
  "/assets/ExperimentOutlined.js?v=d7ee10920d015e8f",
  "/assets/GroupSelect.js?v=d7ee10920d015e8f",
  "/assets/HardwareTab.js?v=d7ee10920d015e8f",
  "/assets/LinkOutlined.js?v=d7ee10920d015e8f",
  "/assets/MediaLibraryTab.js?v=d7ee10920d015e8f",
  "/assets/PlayerInfoTab.js?v=d7ee10920d015e8f",
  "/assets/PlusOutlined.js?v=d7ee10920d015e8f",
  "/assets/PoweroffOutlined.js?v=d7ee10920d015e8f",
  "/assets/PreferencesTab.js?v=d7ee10920d015e8f",
  "/assets/RemoteControlTab.js?v=d7ee10920d015e8f",
  "/assets/SafetyCertificateOutlined.js?v=d7ee10920d015e8f",
  "/assets/SaveOutlined.js?v=d7ee10920d015e8f",
  "/assets/SeekThumbnailPreview.js?v=d7ee10920d015e8f",
  "/assets/StudioTab.js?v=d7ee10920d015e8f",
  "/assets/UnlockOutlined.js?v=d7ee10920d015e8f",
  "/assets/VideoCameraOutlined.js?v=d7ee10920d015e8f",
  "/assets/WifiOutlined.js?v=d7ee10920d015e8f",
  "/assets/app.css?v=d7ee10920d015e8f",
  "/assets/app.js?v=d7ee10920d015e8f",
  "/assets/card.js?v=d7ee10920d015e8f",
  "/assets/color-picker.js?v=d7ee10920d015e8f",
  "/assets/jsx-runtime.js?v=d7ee10920d015e8f",
  "/assets/melodyCatalog.js?v=d7ee10920d015e8f",
  "/assets/popconfirm.js?v=d7ee10920d015e8f",
  "/assets/rolldown-runtime.js?v=d7ee10920d015e8f",
  "/assets/row.js?v=d7ee10920d015e8f",
  "/assets/select.js?v=d7ee10920d015e8f",
  "/assets/slider.js?v=d7ee10920d015e8f",
  "/assets/typography.js?v=d7ee10920d015e8f",
  "/assets/useBreakpoint.js?v=d7ee10920d015e8f",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'd7ee10920d015e8f' })),
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
