const PRECACHE = 'pealayer-precache-fd0e46f9001c7ccd';
const RUNTIME = 'pealayer-runtime-fd0e46f9001c7ccd';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/DesktopOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/EditOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/EffectsTab.js?v=fd0e46f9001c7ccd",
  "/assets/ExperimentOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/GroupSelect.js?v=fd0e46f9001c7ccd",
  "/assets/HardwareTab.js?v=fd0e46f9001c7ccd",
  "/assets/LinkOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/MediaLibraryTab.js?v=fd0e46f9001c7ccd",
  "/assets/MediaTrackSelectors.js?v=fd0e46f9001c7ccd",
  "/assets/PlayCircleOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/PlayerInfoTab.js?v=fd0e46f9001c7ccd",
  "/assets/PlusOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/PreferencesTab.js?v=fd0e46f9001c7ccd",
  "/assets/RemoteControlTab.js?v=fd0e46f9001c7ccd",
  "/assets/SafetyCertificateOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/StopOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/StudioTab.js?v=fd0e46f9001c7ccd",
  "/assets/UnlockOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/VideoCameraOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/WifiOutlined.js?v=fd0e46f9001c7ccd",
  "/assets/app.css?v=fd0e46f9001c7ccd",
  "/assets/app.js?v=fd0e46f9001c7ccd",
  "/assets/card.js?v=fd0e46f9001c7ccd",
  "/assets/color-picker.js?v=fd0e46f9001c7ccd",
  "/assets/effectIcons.js?v=fd0e46f9001c7ccd",
  "/assets/jsx-runtime.js?v=fd0e46f9001c7ccd",
  "/assets/melodyCatalog.js?v=fd0e46f9001c7ccd",
  "/assets/popconfirm.js?v=fd0e46f9001c7ccd",
  "/assets/rolldown-runtime.js?v=fd0e46f9001c7ccd",
  "/assets/row.js?v=fd0e46f9001c7ccd",
  "/assets/slider.js?v=fd0e46f9001c7ccd",
  "/assets/typography.js?v=fd0e46f9001c7ccd",
  "/assets/useBreakpoint.js?v=fd0e46f9001c7ccd",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'fd0e46f9001c7ccd' })),
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
