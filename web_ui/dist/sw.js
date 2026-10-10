const PRECACHE = 'pealayer-precache-ed5d66f8404d696d';
const RUNTIME = 'pealayer-runtime-ed5d66f8404d696d';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=ed5d66f8404d696d",
  "/assets/ArrowDownOutlined.js?v=ed5d66f8404d696d",
  "/assets/ArrowUpOutlined.js?v=ed5d66f8404d696d",
  "/assets/ColorPicker.js?v=ed5d66f8404d696d",
  "/assets/DeleteOutlined.js?v=ed5d66f8404d696d",
  "/assets/DesktopOutlined.js?v=ed5d66f8404d696d",
  "/assets/DownloadsTab.js?v=ed5d66f8404d696d",
  "/assets/EffectsTab.js?v=ed5d66f8404d696d",
  "/assets/ElapsedTimeInput.js?v=ed5d66f8404d696d",
  "/assets/ExpandOutlined.js?v=ed5d66f8404d696d",
  "/assets/GroupSelect.js?v=ed5d66f8404d696d",
  "/assets/HardwareTab.js?v=ed5d66f8404d696d",
  "/assets/MediaLibraryTab.js?v=ed5d66f8404d696d",
  "/assets/MelodySelect.js?v=ed5d66f8404d696d",
  "/assets/PauseOutlined.js?v=ed5d66f8404d696d",
  "/assets/PlayCircleOutlined.js?v=ed5d66f8404d696d",
  "/assets/PlayerInfoTab.js?v=ed5d66f8404d696d",
  "/assets/PlusOutlined.js?v=ed5d66f8404d696d",
  "/assets/PreferencesTab.js?v=ed5d66f8404d696d",
  "/assets/PushpinOutlined.js?v=ed5d66f8404d696d",
  "/assets/RemoteControlTab.js?v=ed5d66f8404d696d",
  "/assets/SafetyCertificateOutlined.js?v=ed5d66f8404d696d",
  "/assets/ServerFilePicker.js?v=ed5d66f8404d696d",
  "/assets/SettingOutlined.js?v=ed5d66f8404d696d",
  "/assets/SoundEffectFields.js?v=ed5d66f8404d696d",
  "/assets/StopOutlined.js?v=ed5d66f8404d696d",
  "/assets/StudioTab.js?v=ed5d66f8404d696d",
  "/assets/WifiOutlined.js?v=ed5d66f8404d696d",
  "/assets/app.css?v=ed5d66f8404d696d",
  "/assets/app.js?v=ed5d66f8404d696d",
  "/assets/card.js?v=ed5d66f8404d696d",
  "/assets/dropdown.js?v=ed5d66f8404d696d",
  "/assets/input-number.js?v=ed5d66f8404d696d",
  "/assets/jsx-runtime.js?v=ed5d66f8404d696d",
  "/assets/popconfirm.js?v=ed5d66f8404d696d",
  "/assets/popover.js?v=ed5d66f8404d696d",
  "/assets/progress.js?v=ed5d66f8404d696d",
  "/assets/rolldown-runtime.js?v=ed5d66f8404d696d",
  "/assets/row.js?v=ed5d66f8404d696d",
  "/assets/slider.js?v=ed5d66f8404d696d",
  "/assets/style.js?v=ed5d66f8404d696d",
  "/assets/table.js?v=ed5d66f8404d696d",
  "/assets/typography.js?v=ed5d66f8404d696d",
  "/assets/useBreakpoint.js?v=ed5d66f8404d696d",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'ed5d66f8404d696d' })),
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
