const PRECACHE = 'pealayer-precache-b3f4c2cd5d43791d';
const RUNTIME = 'pealayer-runtime-b3f4c2cd5d43791d';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/ArrowUpOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/ColorPicker.js?v=b3f4c2cd5d43791d",
  "/assets/DeleteOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/DesktopOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/EditOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/EffectsTab.js?v=b3f4c2cd5d43791d",
  "/assets/ExpandOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/GroupSelect.js?v=b3f4c2cd5d43791d",
  "/assets/HardwareTab.js?v=b3f4c2cd5d43791d",
  "/assets/MediaLibraryTab.js?v=b3f4c2cd5d43791d",
  "/assets/MediaTrackSelectors.js?v=b3f4c2cd5d43791d",
  "/assets/PlayCircleOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/PlayerInfoTab.js?v=b3f4c2cd5d43791d",
  "/assets/PlusOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/PreferencesTab.js?v=b3f4c2cd5d43791d",
  "/assets/PushpinOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/RemoteControlTab.js?v=b3f4c2cd5d43791d",
  "/assets/SafetyCertificateOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/SettingOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/StopOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/StudioTab.js?v=b3f4c2cd5d43791d",
  "/assets/WifiOutlined.js?v=b3f4c2cd5d43791d",
  "/assets/app.css?v=b3f4c2cd5d43791d",
  "/assets/app.js?v=b3f4c2cd5d43791d",
  "/assets/card.js?v=b3f4c2cd5d43791d",
  "/assets/effectIcons.js?v=b3f4c2cd5d43791d",
  "/assets/jsx-runtime.js?v=b3f4c2cd5d43791d",
  "/assets/melodyCatalog.js?v=b3f4c2cd5d43791d",
  "/assets/popconfirm.js?v=b3f4c2cd5d43791d",
  "/assets/rolldown-runtime.js?v=b3f4c2cd5d43791d",
  "/assets/row.js?v=b3f4c2cd5d43791d",
  "/assets/style.js?v=b3f4c2cd5d43791d",
  "/assets/typography.js?v=b3f4c2cd5d43791d",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'b3f4c2cd5d43791d' })),
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
