const PRECACHE = 'pealayer-precache-b4ffac9d174c0a7e';
const RUNTIME = 'pealayer-runtime-b4ffac9d174c0a7e';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/DesktopOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/EditOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/EffectRecorder.js?v=b4ffac9d174c0a7e",
  "/assets/EffectsTab.js?v=b4ffac9d174c0a7e",
  "/assets/ExperimentOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/GroupSelect.js?v=b4ffac9d174c0a7e",
  "/assets/HardwareTab.js?v=b4ffac9d174c0a7e",
  "/assets/MediaLibraryTab.js?v=b4ffac9d174c0a7e",
  "/assets/PlayerInfoTab.js?v=b4ffac9d174c0a7e",
  "/assets/PlusOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/PoweroffOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/PreferencesTab.js?v=b4ffac9d174c0a7e",
  "/assets/RemoteControlTab.js?v=b4ffac9d174c0a7e",
  "/assets/SafetyCertificateOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/SaveOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/SeekThumbnailPreview.js?v=b4ffac9d174c0a7e",
  "/assets/StudioTab.js?v=b4ffac9d174c0a7e",
  "/assets/VideoCameraOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/WifiOutlined.js?v=b4ffac9d174c0a7e",
  "/assets/app.css?v=b4ffac9d174c0a7e",
  "/assets/app.js?v=b4ffac9d174c0a7e",
  "/assets/card.js?v=b4ffac9d174c0a7e",
  "/assets/color-picker.js?v=b4ffac9d174c0a7e",
  "/assets/jsx-runtime.js?v=b4ffac9d174c0a7e",
  "/assets/message.js?v=b4ffac9d174c0a7e",
  "/assets/popconfirm.js?v=b4ffac9d174c0a7e",
  "/assets/rolldown-runtime.js?v=b4ffac9d174c0a7e",
  "/assets/row.js?v=b4ffac9d174c0a7e",
  "/assets/select.js?v=b4ffac9d174c0a7e",
  "/assets/slider.js?v=b4ffac9d174c0a7e",
  "/assets/typography.js?v=b4ffac9d174c0a7e",
  "/assets/useBreakpoint.js?v=b4ffac9d174c0a7e",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'b4ffac9d174c0a7e' })),
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
