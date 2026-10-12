const PRECACHE = 'pealayer-precache-d15cb33d5c99e074';
const RUNTIME = 'pealayer-runtime-d15cb33d5c99e074';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowDownOutlined.js?v=d15cb33d5c99e074",
  "/assets/ColorDropdown.js?v=d15cb33d5c99e074",
  "/assets/ColorPicker.js?v=d15cb33d5c99e074",
  "/assets/DeleteOutlined.js?v=d15cb33d5c99e074",
  "/assets/DesktopOutlined.js?v=d15cb33d5c99e074",
  "/assets/DownloadsTab.js?v=d15cb33d5c99e074",
  "/assets/EffectsTab.js?v=d15cb33d5c99e074",
  "/assets/ElapsedTimeInput.js?v=d15cb33d5c99e074",
  "/assets/ExpandOutlined.js?v=d15cb33d5c99e074",
  "/assets/GroupSelect.js?v=d15cb33d5c99e074",
  "/assets/HardwareTab.js?v=d15cb33d5c99e074",
  "/assets/MediaLibraryTab.js?v=d15cb33d5c99e074",
  "/assets/MelodySelect.js?v=d15cb33d5c99e074",
  "/assets/PauseOutlined.js?v=d15cb33d5c99e074",
  "/assets/PlayerInfoTab.js?v=d15cb33d5c99e074",
  "/assets/PlusOutlined.js?v=d15cb33d5c99e074",
  "/assets/PreferencesTab.js?v=d15cb33d5c99e074",
  "/assets/PushpinOutlined.js?v=d15cb33d5c99e074",
  "/assets/RemoteControlTab.js?v=d15cb33d5c99e074",
  "/assets/SafetyCertificateOutlined.js?v=d15cb33d5c99e074",
  "/assets/StepRepeatFields.js?v=d15cb33d5c99e074",
  "/assets/StopOutlined.js?v=d15cb33d5c99e074",
  "/assets/StudioTab.js?v=d15cb33d5c99e074",
  "/assets/app.css?v=d15cb33d5c99e074",
  "/assets/app.js?v=d15cb33d5c99e074",
  "/assets/index.js?v=d15cb33d5c99e074",
  "/assets/index2.js?v=d15cb33d5c99e074",
  "/assets/index3.js?v=d15cb33d5c99e074",
  "/assets/index4.js?v=d15cb33d5c99e074",
  "/assets/index5.js?v=d15cb33d5c99e074",
  "/assets/progress.js?v=d15cb33d5c99e074",
  "/assets/row.js?v=d15cb33d5c99e074",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'd15cb33d5c99e074' })),
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
