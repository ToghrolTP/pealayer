const PRECACHE = 'pealayer-precache-e838af1036cef17a';
const RUNTIME = 'pealayer-runtime-e838af1036cef17a';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=e838af1036cef17a",
  "/assets/DesktopOutlined.js?v=e838af1036cef17a",
  "/assets/EditOutlined.js?v=e838af1036cef17a",
  "/assets/EffectsTab.js?v=e838af1036cef17a",
  "/assets/ExperimentOutlined.js?v=e838af1036cef17a",
  "/assets/GroupSelect.js?v=e838af1036cef17a",
  "/assets/HardwareTab.js?v=e838af1036cef17a",
  "/assets/MediaLibraryTab.js?v=e838af1036cef17a",
  "/assets/PlayerInfoTab.js?v=e838af1036cef17a",
  "/assets/PlusOutlined.js?v=e838af1036cef17a",
  "/assets/PoweroffOutlined.js?v=e838af1036cef17a",
  "/assets/PreferencesTab.js?v=e838af1036cef17a",
  "/assets/RemoteControlTab.js?v=e838af1036cef17a",
  "/assets/SafetyCertificateOutlined.js?v=e838af1036cef17a",
  "/assets/SaveOutlined.js?v=e838af1036cef17a",
  "/assets/SeekThumbnailPreview.js?v=e838af1036cef17a",
  "/assets/StudioTab.js?v=e838af1036cef17a",
  "/assets/VideoCameraOutlined.js?v=e838af1036cef17a",
  "/assets/WifiOutlined.js?v=e838af1036cef17a",
  "/assets/app.css?v=e838af1036cef17a",
  "/assets/app.js?v=e838af1036cef17a",
  "/assets/card.js?v=e838af1036cef17a",
  "/assets/color-picker.js?v=e838af1036cef17a",
  "/assets/jsx-runtime.js?v=e838af1036cef17a",
  "/assets/melodyCatalog.js?v=e838af1036cef17a",
  "/assets/popconfirm.js?v=e838af1036cef17a",
  "/assets/rolldown-runtime.js?v=e838af1036cef17a",
  "/assets/row.js?v=e838af1036cef17a",
  "/assets/select.js?v=e838af1036cef17a",
  "/assets/slider.js?v=e838af1036cef17a",
  "/assets/typography.js?v=e838af1036cef17a",
  "/assets/useBreakpoint.js?v=e838af1036cef17a",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'e838af1036cef17a' })),
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
