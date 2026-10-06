const PRECACHE = 'pealayer-precache-d039b3cafcd11df0';
const RUNTIME = 'pealayer-runtime-d039b3cafcd11df0';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=d039b3cafcd11df0",
  "/assets/DesktopOutlined.js?v=d039b3cafcd11df0",
  "/assets/EditOutlined.js?v=d039b3cafcd11df0",
  "/assets/EffectRecorder.js?v=d039b3cafcd11df0",
  "/assets/EffectsTab.js?v=d039b3cafcd11df0",
  "/assets/ExperimentOutlined.js?v=d039b3cafcd11df0",
  "/assets/GroupSelect.js?v=d039b3cafcd11df0",
  "/assets/HardwareTab.js?v=d039b3cafcd11df0",
  "/assets/MediaLibraryTab.js?v=d039b3cafcd11df0",
  "/assets/PlayerInfoTab.js?v=d039b3cafcd11df0",
  "/assets/PlusOutlined.js?v=d039b3cafcd11df0",
  "/assets/PoweroffOutlined.js?v=d039b3cafcd11df0",
  "/assets/PreferencesTab.js?v=d039b3cafcd11df0",
  "/assets/RemoteControlTab.js?v=d039b3cafcd11df0",
  "/assets/SafetyCertificateOutlined.js?v=d039b3cafcd11df0",
  "/assets/SaveOutlined.js?v=d039b3cafcd11df0",
  "/assets/SeekThumbnailPreview.js?v=d039b3cafcd11df0",
  "/assets/StudioTab.js?v=d039b3cafcd11df0",
  "/assets/VideoCameraOutlined.js?v=d039b3cafcd11df0",
  "/assets/WifiOutlined.js?v=d039b3cafcd11df0",
  "/assets/app.css?v=d039b3cafcd11df0",
  "/assets/app.js?v=d039b3cafcd11df0",
  "/assets/card.js?v=d039b3cafcd11df0",
  "/assets/color-picker.js?v=d039b3cafcd11df0",
  "/assets/jsx-runtime.js?v=d039b3cafcd11df0",
  "/assets/message.js?v=d039b3cafcd11df0",
  "/assets/popconfirm.js?v=d039b3cafcd11df0",
  "/assets/rolldown-runtime.js?v=d039b3cafcd11df0",
  "/assets/row.js?v=d039b3cafcd11df0",
  "/assets/select.js?v=d039b3cafcd11df0",
  "/assets/slider.js?v=d039b3cafcd11df0",
  "/assets/typography.js?v=d039b3cafcd11df0",
  "/assets/useBreakpoint.js?v=d039b3cafcd11df0",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'd039b3cafcd11df0' })),
  );
});

self.addEventListener('message', (event) => {
  if (event.data?.type === 'SKIP_WAITING') {
    event.waitUntil(self.skipWaiting());
    return;
  }
  if (event.data?.type === 'CLEAR_RUNTIME_CACHE') {
    event.waitUntil(caches.delete(RUNTIME));
  }
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
