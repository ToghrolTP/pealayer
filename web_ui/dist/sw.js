const PRECACHE = 'pealayer-precache-e88573d186532162';
const RUNTIME = 'pealayer-runtime-e88573d186532162';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=e88573d186532162",
  "/assets/ArrowUpOutlined.js?v=e88573d186532162",
  "/assets/ColorPicker.js?v=e88573d186532162",
  "/assets/DeleteOutlined.js?v=e88573d186532162",
  "/assets/DesktopOutlined.js?v=e88573d186532162",
  "/assets/EffectsTab.js?v=e88573d186532162",
  "/assets/ElapsedTimeInput.js?v=e88573d186532162",
  "/assets/ExpandOutlined.js?v=e88573d186532162",
  "/assets/GroupSelect.js?v=e88573d186532162",
  "/assets/HardwareTab.js?v=e88573d186532162",
  "/assets/MediaLibraryTab.js?v=e88573d186532162",
  "/assets/MelodySelect.js?v=e88573d186532162",
  "/assets/PlayCircleOutlined.js?v=e88573d186532162",
  "/assets/PlayerInfoTab.js?v=e88573d186532162",
  "/assets/PlusOutlined.js?v=e88573d186532162",
  "/assets/PreferencesTab.js?v=e88573d186532162",
  "/assets/PushpinOutlined.js?v=e88573d186532162",
  "/assets/RemoteControlTab.js?v=e88573d186532162",
  "/assets/SafetyCertificateOutlined.js?v=e88573d186532162",
  "/assets/ServerFilePicker.js?v=e88573d186532162",
  "/assets/SettingOutlined.js?v=e88573d186532162",
  "/assets/SoundEffectFields.js?v=e88573d186532162",
  "/assets/StopOutlined.js?v=e88573d186532162",
  "/assets/StudioTab.js?v=e88573d186532162",
  "/assets/WifiOutlined.js?v=e88573d186532162",
  "/assets/app.css?v=e88573d186532162",
  "/assets/app.js?v=e88573d186532162",
  "/assets/card.js?v=e88573d186532162",
  "/assets/jsx-runtime.js?v=e88573d186532162",
  "/assets/popconfirm.js?v=e88573d186532162",
  "/assets/rolldown-runtime.js?v=e88573d186532162",
  "/assets/row.js?v=e88573d186532162",
  "/assets/style.js?v=e88573d186532162",
  "/assets/typography.js?v=e88573d186532162",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'e88573d186532162' })),
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
