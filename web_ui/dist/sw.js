const PRECACHE = 'pealayer-precache-f7119937572e64ab';
const RUNTIME = 'pealayer-runtime-f7119937572e64ab';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=f7119937572e64ab",
  "/assets/DesktopOutlined.js?v=f7119937572e64ab",
  "/assets/EditOutlined.js?v=f7119937572e64ab",
  "/assets/EffectRecorder.js?v=f7119937572e64ab",
  "/assets/EffectsTab.js?v=f7119937572e64ab",
  "/assets/ExperimentOutlined.js?v=f7119937572e64ab",
  "/assets/GroupSelect.js?v=f7119937572e64ab",
  "/assets/HardwareTab.js?v=f7119937572e64ab",
  "/assets/MediaLibraryTab.js?v=f7119937572e64ab",
  "/assets/PlayerInfoTab.js?v=f7119937572e64ab",
  "/assets/PlusOutlined.js?v=f7119937572e64ab",
  "/assets/PoweroffOutlined.js?v=f7119937572e64ab",
  "/assets/PreferencesTab.js?v=f7119937572e64ab",
  "/assets/RemoteControlTab.js?v=f7119937572e64ab",
  "/assets/SafetyCertificateOutlined.js?v=f7119937572e64ab",
  "/assets/SaveOutlined.js?v=f7119937572e64ab",
  "/assets/SeekThumbnailPreview.js?v=f7119937572e64ab",
  "/assets/StudioTab.js?v=f7119937572e64ab",
  "/assets/VideoCameraOutlined.js?v=f7119937572e64ab",
  "/assets/WifiOutlined.js?v=f7119937572e64ab",
  "/assets/app.css?v=f7119937572e64ab",
  "/assets/app.js?v=f7119937572e64ab",
  "/assets/card.js?v=f7119937572e64ab",
  "/assets/color-picker.js?v=f7119937572e64ab",
  "/assets/jsx-runtime.js?v=f7119937572e64ab",
  "/assets/popconfirm.js?v=f7119937572e64ab",
  "/assets/rolldown-runtime.js?v=f7119937572e64ab",
  "/assets/row.js?v=f7119937572e64ab",
  "/assets/select.js?v=f7119937572e64ab",
  "/assets/slider.js?v=f7119937572e64ab",
  "/assets/typography.js?v=f7119937572e64ab",
  "/assets/useBreakpoint.js?v=f7119937572e64ab",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'f7119937572e64ab' })),
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
