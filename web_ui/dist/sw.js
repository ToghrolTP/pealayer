const PRECACHE = 'pealayer-precache-0a27d16edb5538c8';
const RUNTIME = 'pealayer-runtime-0a27d16edb5538c8';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=0a27d16edb5538c8",
  "/assets/ArrowUpOutlined.js?v=0a27d16edb5538c8",
  "/assets/ColorPicker.js?v=0a27d16edb5538c8",
  "/assets/DeleteOutlined.js?v=0a27d16edb5538c8",
  "/assets/DesktopOutlined.js?v=0a27d16edb5538c8",
  "/assets/EffectsTab.js?v=0a27d16edb5538c8",
  "/assets/ElapsedTimeInput.js?v=0a27d16edb5538c8",
  "/assets/ExpandOutlined.js?v=0a27d16edb5538c8",
  "/assets/GroupSelect.js?v=0a27d16edb5538c8",
  "/assets/HardwareTab.js?v=0a27d16edb5538c8",
  "/assets/MediaLibraryTab.js?v=0a27d16edb5538c8",
  "/assets/MelodySelect.js?v=0a27d16edb5538c8",
  "/assets/PlayCircleOutlined.js?v=0a27d16edb5538c8",
  "/assets/PlayerInfoTab.js?v=0a27d16edb5538c8",
  "/assets/PlusOutlined.js?v=0a27d16edb5538c8",
  "/assets/PreferencesTab.js?v=0a27d16edb5538c8",
  "/assets/PushpinOutlined.js?v=0a27d16edb5538c8",
  "/assets/RemoteControlTab.js?v=0a27d16edb5538c8",
  "/assets/SafetyCertificateOutlined.js?v=0a27d16edb5538c8",
  "/assets/ServerFilePicker.js?v=0a27d16edb5538c8",
  "/assets/SettingOutlined.js?v=0a27d16edb5538c8",
  "/assets/SoundEffectFields.js?v=0a27d16edb5538c8",
  "/assets/StopOutlined.js?v=0a27d16edb5538c8",
  "/assets/StudioTab.js?v=0a27d16edb5538c8",
  "/assets/WifiOutlined.js?v=0a27d16edb5538c8",
  "/assets/app.css?v=0a27d16edb5538c8",
  "/assets/app.js?v=0a27d16edb5538c8",
  "/assets/card.js?v=0a27d16edb5538c8",
  "/assets/jsx-runtime.js?v=0a27d16edb5538c8",
  "/assets/popconfirm.js?v=0a27d16edb5538c8",
  "/assets/rolldown-runtime.js?v=0a27d16edb5538c8",
  "/assets/row.js?v=0a27d16edb5538c8",
  "/assets/style.js?v=0a27d16edb5538c8",
  "/assets/typography.js?v=0a27d16edb5538c8",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '0a27d16edb5538c8' })),
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
