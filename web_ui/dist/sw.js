const PRECACHE = 'pealayer-precache-a5d064d591d168a0';
const RUNTIME = 'pealayer-runtime-a5d064d591d168a0';
const PRECACHE_URLS = [
  "/",
  "/assets/AimOutlined.js?v=a5d064d591d168a0",
  "/assets/ArrowUpOutlined.js?v=a5d064d591d168a0",
  "/assets/ColorPicker.js?v=a5d064d591d168a0",
  "/assets/DeleteOutlined.js?v=a5d064d591d168a0",
  "/assets/DesktopOutlined.js?v=a5d064d591d168a0",
  "/assets/EditOutlined.js?v=a5d064d591d168a0",
  "/assets/EffectsTab.js?v=a5d064d591d168a0",
  "/assets/ElapsedTimeInput.js?v=a5d064d591d168a0",
  "/assets/ExpandOutlined.js?v=a5d064d591d168a0",
  "/assets/HardwareTab.js?v=a5d064d591d168a0",
  "/assets/MediaLibraryTab.js?v=a5d064d591d168a0",
  "/assets/MelodySelect.js?v=a5d064d591d168a0",
  "/assets/PlayCircleOutlined.js?v=a5d064d591d168a0",
  "/assets/PlayerInfoTab.js?v=a5d064d591d168a0",
  "/assets/PlusOutlined.js?v=a5d064d591d168a0",
  "/assets/PreferencesTab.js?v=a5d064d591d168a0",
  "/assets/PushpinOutlined.js?v=a5d064d591d168a0",
  "/assets/RemoteControlTab.js?v=a5d064d591d168a0",
  "/assets/SafetyCertificateOutlined.js?v=a5d064d591d168a0",
  "/assets/ServerFilePicker.js?v=a5d064d591d168a0",
  "/assets/SettingOutlined.js?v=a5d064d591d168a0",
  "/assets/StopOutlined.js?v=a5d064d591d168a0",
  "/assets/StudioTab.js?v=a5d064d591d168a0",
  "/assets/WifiOutlined.js?v=a5d064d591d168a0",
  "/assets/app.css?v=a5d064d591d168a0",
  "/assets/app.js?v=a5d064d591d168a0",
  "/assets/card.js?v=a5d064d591d168a0",
  "/assets/effectIcons.js?v=a5d064d591d168a0",
  "/assets/jsx-runtime.js?v=a5d064d591d168a0",
  "/assets/melodyCatalog.js?v=a5d064d591d168a0",
  "/assets/popconfirm.js?v=a5d064d591d168a0",
  "/assets/rolldown-runtime.js?v=a5d064d591d168a0",
  "/assets/row.js?v=a5d064d591d168a0",
  "/assets/style.js?v=a5d064d591d168a0",
  "/assets/typography.js?v=a5d064d591d168a0",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'a5d064d591d168a0' })),
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
