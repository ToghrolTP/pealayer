const PRECACHE = 'pealayer-precache-eeda9cb8f7aa49ea';
const RUNTIME = 'pealayer-runtime-eeda9cb8f7aa49ea';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/ArrowUpOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/ColorPicker.js?v=eeda9cb8f7aa49ea",
  "/assets/DeleteOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/DesktopOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/EditOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/EffectsTab.js?v=eeda9cb8f7aa49ea",
  "/assets/ExpandOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/GroupSelect.js?v=eeda9cb8f7aa49ea",
  "/assets/HardwareTab.js?v=eeda9cb8f7aa49ea",
  "/assets/MediaLibraryTab.js?v=eeda9cb8f7aa49ea",
  "/assets/PlayCircleOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/PlayerInfoTab.js?v=eeda9cb8f7aa49ea",
  "/assets/PlusOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/PreferencesTab.js?v=eeda9cb8f7aa49ea",
  "/assets/PushpinOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/RemoteControlTab.js?v=eeda9cb8f7aa49ea",
  "/assets/SafetyCertificateOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/SettingOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/StopOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/StudioTab.js?v=eeda9cb8f7aa49ea",
  "/assets/VolumeControl.js?v=eeda9cb8f7aa49ea",
  "/assets/WifiOutlined.js?v=eeda9cb8f7aa49ea",
  "/assets/app.css?v=eeda9cb8f7aa49ea",
  "/assets/app.js?v=eeda9cb8f7aa49ea",
  "/assets/card.js?v=eeda9cb8f7aa49ea",
  "/assets/effectIcons.js?v=eeda9cb8f7aa49ea",
  "/assets/jsx-runtime.js?v=eeda9cb8f7aa49ea",
  "/assets/melodyCatalog.js?v=eeda9cb8f7aa49ea",
  "/assets/popconfirm.js?v=eeda9cb8f7aa49ea",
  "/assets/rolldown-runtime.js?v=eeda9cb8f7aa49ea",
  "/assets/row.js?v=eeda9cb8f7aa49ea",
  "/assets/style.js?v=eeda9cb8f7aa49ea",
  "/assets/typography.js?v=eeda9cb8f7aa49ea",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'eeda9cb8f7aa49ea' })),
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
