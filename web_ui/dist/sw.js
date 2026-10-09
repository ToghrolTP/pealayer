const PRECACHE = 'pealayer-precache-8497651db53865ae';
const RUNTIME = 'pealayer-runtime-8497651db53865ae';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=8497651db53865ae",
  "/assets/ArrowUpOutlined.js?v=8497651db53865ae",
  "/assets/ColorPicker.js?v=8497651db53865ae",
  "/assets/DeleteOutlined.js?v=8497651db53865ae",
  "/assets/DesktopOutlined.js?v=8497651db53865ae",
  "/assets/EditOutlined.js?v=8497651db53865ae",
  "/assets/EffectsTab.js?v=8497651db53865ae",
  "/assets/ElapsedTimeInput.js?v=8497651db53865ae",
  "/assets/ExpandOutlined.js?v=8497651db53865ae",
  "/assets/HardwareTab.js?v=8497651db53865ae",
  "/assets/MediaLibraryTab.js?v=8497651db53865ae",
  "/assets/MelodySelect.js?v=8497651db53865ae",
  "/assets/PlayCircleOutlined.js?v=8497651db53865ae",
  "/assets/PlayerInfoTab.js?v=8497651db53865ae",
  "/assets/PlusOutlined.js?v=8497651db53865ae",
  "/assets/PreferencesTab.js?v=8497651db53865ae",
  "/assets/PushpinOutlined.js?v=8497651db53865ae",
  "/assets/RemoteControlTab.js?v=8497651db53865ae",
  "/assets/SafetyCertificateOutlined.js?v=8497651db53865ae",
  "/assets/SettingOutlined.js?v=8497651db53865ae",
  "/assets/StopOutlined.js?v=8497651db53865ae",
  "/assets/StudioTab.js?v=8497651db53865ae",
  "/assets/WifiOutlined.js?v=8497651db53865ae",
  "/assets/app.css?v=8497651db53865ae",
  "/assets/app.js?v=8497651db53865ae",
  "/assets/card.js?v=8497651db53865ae",
  "/assets/effectIcons.js?v=8497651db53865ae",
  "/assets/jsx-runtime.js?v=8497651db53865ae",
  "/assets/melodyCatalog.js?v=8497651db53865ae",
  "/assets/popconfirm.js?v=8497651db53865ae",
  "/assets/rolldown-runtime.js?v=8497651db53865ae",
  "/assets/row.js?v=8497651db53865ae",
  "/assets/style.js?v=8497651db53865ae",
  "/assets/typography.js?v=8497651db53865ae",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '8497651db53865ae' })),
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
