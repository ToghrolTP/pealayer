const PRECACHE = 'pealayer-precache-0c81b9e239c5bbfb';
const RUNTIME = 'pealayer-runtime-0c81b9e239c5bbfb';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/ArrowUpOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/ColorPicker.js?v=0c81b9e239c5bbfb",
  "/assets/DeleteOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/DesktopOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/EditOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/EffectsTab.js?v=0c81b9e239c5bbfb",
  "/assets/ExpandOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/GroupSelect.js?v=0c81b9e239c5bbfb",
  "/assets/HardwareTab.js?v=0c81b9e239c5bbfb",
  "/assets/MediaLibraryTab.js?v=0c81b9e239c5bbfb",
  "/assets/MediaTrackSelectors.js?v=0c81b9e239c5bbfb",
  "/assets/PlayCircleOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/PlayerInfoTab.js?v=0c81b9e239c5bbfb",
  "/assets/PlusOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/PreferencesTab.js?v=0c81b9e239c5bbfb",
  "/assets/PushpinOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/RemoteControlTab.js?v=0c81b9e239c5bbfb",
  "/assets/SafetyCertificateOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/SettingOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/StopOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/StudioTab.js?v=0c81b9e239c5bbfb",
  "/assets/WifiOutlined.js?v=0c81b9e239c5bbfb",
  "/assets/app.css?v=0c81b9e239c5bbfb",
  "/assets/app.js?v=0c81b9e239c5bbfb",
  "/assets/card.js?v=0c81b9e239c5bbfb",
  "/assets/effectIcons.js?v=0c81b9e239c5bbfb",
  "/assets/jsx-runtime.js?v=0c81b9e239c5bbfb",
  "/assets/melodyCatalog.js?v=0c81b9e239c5bbfb",
  "/assets/popconfirm.js?v=0c81b9e239c5bbfb",
  "/assets/rolldown-runtime.js?v=0c81b9e239c5bbfb",
  "/assets/row.js?v=0c81b9e239c5bbfb",
  "/assets/style.js?v=0c81b9e239c5bbfb",
  "/assets/typography.js?v=0c81b9e239c5bbfb",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '0c81b9e239c5bbfb' })),
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
