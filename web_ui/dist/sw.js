const PRECACHE = 'pealayer-precache-72a7af6b0ce6d43b';
const RUNTIME = 'pealayer-runtime-72a7af6b0ce6d43b';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/ArrowDownOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/ArrowUpOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/ColorPicker.js?v=72a7af6b0ce6d43b",
  "/assets/DeleteOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/DesktopOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/DownloadsTab.js?v=72a7af6b0ce6d43b",
  "/assets/EffectsTab.js?v=72a7af6b0ce6d43b",
  "/assets/ElapsedTimeInput.js?v=72a7af6b0ce6d43b",
  "/assets/ExpandOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/GroupSelect.js?v=72a7af6b0ce6d43b",
  "/assets/HardwareTab.js?v=72a7af6b0ce6d43b",
  "/assets/MediaLibraryTab.js?v=72a7af6b0ce6d43b",
  "/assets/MelodySelect.js?v=72a7af6b0ce6d43b",
  "/assets/PauseOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/PlayCircleOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/PlayerInfoTab.js?v=72a7af6b0ce6d43b",
  "/assets/PlusOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/PreferencesTab.js?v=72a7af6b0ce6d43b",
  "/assets/PushpinOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/RemoteControlTab.js?v=72a7af6b0ce6d43b",
  "/assets/SafetyCertificateOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/ServerFilePicker.js?v=72a7af6b0ce6d43b",
  "/assets/SettingOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/SoundEffectFields.js?v=72a7af6b0ce6d43b",
  "/assets/StopOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/StudioTab.js?v=72a7af6b0ce6d43b",
  "/assets/WifiOutlined.js?v=72a7af6b0ce6d43b",
  "/assets/app.css?v=72a7af6b0ce6d43b",
  "/assets/app.js?v=72a7af6b0ce6d43b",
  "/assets/card.js?v=72a7af6b0ce6d43b",
  "/assets/dropdown.js?v=72a7af6b0ce6d43b",
  "/assets/input-number.js?v=72a7af6b0ce6d43b",
  "/assets/jsx-runtime.js?v=72a7af6b0ce6d43b",
  "/assets/popconfirm.js?v=72a7af6b0ce6d43b",
  "/assets/popover.js?v=72a7af6b0ce6d43b",
  "/assets/progress.js?v=72a7af6b0ce6d43b",
  "/assets/rolldown-runtime.js?v=72a7af6b0ce6d43b",
  "/assets/row.js?v=72a7af6b0ce6d43b",
  "/assets/slider.js?v=72a7af6b0ce6d43b",
  "/assets/style.js?v=72a7af6b0ce6d43b",
  "/assets/table.js?v=72a7af6b0ce6d43b",
  "/assets/typography.js?v=72a7af6b0ce6d43b",
  "/assets/useBreakpoint.js?v=72a7af6b0ce6d43b",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '72a7af6b0ce6d43b' })),
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
