const PRECACHE = 'pealayer-precache-f01f333bf9e39fbc';
const RUNTIME = 'pealayer-runtime-f01f333bf9e39fbc';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=f01f333bf9e39fbc",
  "/assets/ArrowUpOutlined.js?v=f01f333bf9e39fbc",
  "/assets/ColorPicker.js?v=f01f333bf9e39fbc",
  "/assets/DeleteOutlined.js?v=f01f333bf9e39fbc",
  "/assets/DesktopOutlined.js?v=f01f333bf9e39fbc",
  "/assets/EditOutlined.js?v=f01f333bf9e39fbc",
  "/assets/EffectsTab.js?v=f01f333bf9e39fbc",
  "/assets/ElapsedTimeInput.js?v=f01f333bf9e39fbc",
  "/assets/ExpandOutlined.js?v=f01f333bf9e39fbc",
  "/assets/GroupSelect.js?v=f01f333bf9e39fbc",
  "/assets/HardwareTab.js?v=f01f333bf9e39fbc",
  "/assets/MediaLibraryTab.js?v=f01f333bf9e39fbc",
  "/assets/PlayCircleOutlined.js?v=f01f333bf9e39fbc",
  "/assets/PlayerInfoTab.js?v=f01f333bf9e39fbc",
  "/assets/PlusOutlined.js?v=f01f333bf9e39fbc",
  "/assets/PreferencesTab.js?v=f01f333bf9e39fbc",
  "/assets/PushpinOutlined.js?v=f01f333bf9e39fbc",
  "/assets/RemoteControlTab.js?v=f01f333bf9e39fbc",
  "/assets/SafetyCertificateOutlined.js?v=f01f333bf9e39fbc",
  "/assets/SettingOutlined.js?v=f01f333bf9e39fbc",
  "/assets/StopOutlined.js?v=f01f333bf9e39fbc",
  "/assets/StudioTab.js?v=f01f333bf9e39fbc",
  "/assets/WifiOutlined.js?v=f01f333bf9e39fbc",
  "/assets/app.css?v=f01f333bf9e39fbc",
  "/assets/app.js?v=f01f333bf9e39fbc",
  "/assets/card.js?v=f01f333bf9e39fbc",
  "/assets/effectIcons.js?v=f01f333bf9e39fbc",
  "/assets/jsx-runtime.js?v=f01f333bf9e39fbc",
  "/assets/melodyCatalog.js?v=f01f333bf9e39fbc",
  "/assets/popconfirm.js?v=f01f333bf9e39fbc",
  "/assets/rolldown-runtime.js?v=f01f333bf9e39fbc",
  "/assets/row.js?v=f01f333bf9e39fbc",
  "/assets/style.js?v=f01f333bf9e39fbc",
  "/assets/typography.js?v=f01f333bf9e39fbc",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'f01f333bf9e39fbc' })),
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
