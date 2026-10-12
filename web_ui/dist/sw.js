const PRECACHE = 'pealayer-precache-9823f6686b810340';
const RUNTIME = 'pealayer-runtime-9823f6686b810340';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowDownOutlined.js?v=9823f6686b810340",
  "/assets/ArrowUpOutlined.js?v=9823f6686b810340",
  "/assets/ColorPicker.js?v=9823f6686b810340",
  "/assets/DeleteOutlined.js?v=9823f6686b810340",
  "/assets/DownloadsTab.js?v=9823f6686b810340",
  "/assets/EffectsTab.js?v=9823f6686b810340",
  "/assets/ElapsedTimeInput.js?v=9823f6686b810340",
  "/assets/ExpandOutlined.js?v=9823f6686b810340",
  "/assets/GroupSelect.js?v=9823f6686b810340",
  "/assets/Hack-Regular.ttf?v=9823f6686b810340",
  "/assets/HardwareTab.js?v=9823f6686b810340",
  "/assets/MediaLibraryTab.js?v=9823f6686b810340",
  "/assets/MelodySelect.js?v=9823f6686b810340",
  "/assets/PauseOutlined.js?v=9823f6686b810340",
  "/assets/PlayCircleOutlined.js?v=9823f6686b810340",
  "/assets/PlayerInfoTab.js?v=9823f6686b810340",
  "/assets/PreferencesTab.js?v=9823f6686b810340",
  "/assets/PushpinOutlined.js?v=9823f6686b810340",
  "/assets/RemoteControlTab.js?v=9823f6686b810340",
  "/assets/SafetyCertificateOutlined.js?v=9823f6686b810340",
  "/assets/ServerFilePicker.js?v=9823f6686b810340",
  "/assets/StepRepeatFields.js?v=9823f6686b810340",
  "/assets/StopOutlined.js?v=9823f6686b810340",
  "/assets/StudioTab.js?v=9823f6686b810340",
  "/assets/Ubuntu-Light.ttf?v=9823f6686b810340",
  "/assets/Vazirmatn-Regular.ttf?v=9823f6686b810340",
  "/assets/alert.js?v=9823f6686b810340",
  "/assets/app.css?v=9823f6686b810340",
  "/assets/app.js?v=9823f6686b810340",
  "/assets/card.js?v=9823f6686b810340",
  "/assets/dropdown.js?v=9823f6686b810340",
  "/assets/jsx-runtime.js?v=9823f6686b810340",
  "/assets/modal.js?v=9823f6686b810340",
  "/assets/popconfirm.js?v=9823f6686b810340",
  "/assets/popover.js?v=9823f6686b810340",
  "/assets/progress.js?v=9823f6686b810340",
  "/assets/rolldown-runtime.js?v=9823f6686b810340",
  "/assets/row.js?v=9823f6686b810340",
  "/assets/slider.js?v=9823f6686b810340",
  "/assets/style.js?v=9823f6686b810340",
  "/assets/table.js?v=9823f6686b810340",
  "/assets/typography.js?v=9823f6686b810340",
  "/assets/useBreakpoint.js?v=9823f6686b810340",
  "/font-licenses/Hack-LICENSE.txt",
  "/font-licenses/Ubuntu-LICENSE.txt",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '9823f6686b810340' })),
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
