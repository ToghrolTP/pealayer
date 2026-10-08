const PRECACHE = 'pealayer-precache-cf98662959c0bb2e';
const RUNTIME = 'pealayer-runtime-cf98662959c0bb2e';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined.js?v=cf98662959c0bb2e",
  "/assets/DeleteOutlined.js?v=cf98662959c0bb2e",
  "/assets/DesktopOutlined.js?v=cf98662959c0bb2e",
  "/assets/EditOutlined.js?v=cf98662959c0bb2e",
  "/assets/EffectsTab.js?v=cf98662959c0bb2e",
  "/assets/ExpandOutlined.js?v=cf98662959c0bb2e",
  "/assets/GroupSelect.js?v=cf98662959c0bb2e",
  "/assets/HardwareTab.js?v=cf98662959c0bb2e",
  "/assets/LinkOutlined.js?v=cf98662959c0bb2e",
  "/assets/MediaLibraryTab.js?v=cf98662959c0bb2e",
  "/assets/MediaTrackSelectors.js?v=cf98662959c0bb2e",
  "/assets/PlayCircleOutlined.js?v=cf98662959c0bb2e",
  "/assets/PlayerInfoTab.js?v=cf98662959c0bb2e",
  "/assets/PlusOutlined.js?v=cf98662959c0bb2e",
  "/assets/PreferencesTab.js?v=cf98662959c0bb2e",
  "/assets/PushpinOutlined.js?v=cf98662959c0bb2e",
  "/assets/RemoteControlTab.js?v=cf98662959c0bb2e",
  "/assets/SafetyCertificateOutlined.js?v=cf98662959c0bb2e",
  "/assets/SettingOutlined.js?v=cf98662959c0bb2e",
  "/assets/StopOutlined.js?v=cf98662959c0bb2e",
  "/assets/StudioTab.js?v=cf98662959c0bb2e",
  "/assets/VideoCameraOutlined.js?v=cf98662959c0bb2e",
  "/assets/WifiOutlined.js?v=cf98662959c0bb2e",
  "/assets/app.css?v=cf98662959c0bb2e",
  "/assets/app.js?v=cf98662959c0bb2e",
  "/assets/card.js?v=cf98662959c0bb2e",
  "/assets/color-picker.js?v=cf98662959c0bb2e",
  "/assets/effectIcons.js?v=cf98662959c0bb2e",
  "/assets/jsx-runtime.js?v=cf98662959c0bb2e",
  "/assets/melodyCatalog.js?v=cf98662959c0bb2e",
  "/assets/popconfirm.js?v=cf98662959c0bb2e",
  "/assets/rolldown-runtime.js?v=cf98662959c0bb2e",
  "/assets/row.js?v=cf98662959c0bb2e",
  "/assets/style.js?v=cf98662959c0bb2e",
  "/assets/typography.js?v=cf98662959c0bb2e",
  "/fuji-loader.css",
  "/fuji-loader.svg",
  "/index.html",
  "/manifest.webmanifest"
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'cf98662959c0bb2e' })),
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
