const PRECACHE = 'pealayer-precache-de2cc20cb9005730';
const RUNTIME = 'pealayer-runtime-de2cc20cb9005730';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-DRwsF4Qg.js",
  "/assets/DesktopOutlined-DRmRIzUP.js",
  "/assets/EditOutlined-CaC9-jmg.js",
  "/assets/EffectRecorder-CQ5n6K9O.js",
  "/assets/EffectsTab-B9ejA1e7.js",
  "/assets/ExperimentOutlined-ciTlWVco.js",
  "/assets/GroupSelect-vaiwaDIC.js",
  "/assets/HardwareTab-DubPuuPZ.js",
  "/assets/MediaLibraryTab-PSvgc3so.js",
  "/assets/PlayCircleOutlined-DIfRAzp7.js",
  "/assets/PlayerInfoTab-CNPx7iN-.js",
  "/assets/PlusOutlined-DLqzeTmD.js",
  "/assets/PreferencesTab-lbQry3k_.js",
  "/assets/PurePanel-DgKSN6v0.js",
  "/assets/ReloadOutlined-DQm0WaHI.js",
  "/assets/RemoteControlTab-Bpfuca41.js",
  "/assets/SafetyCertificateOutlined-BC7A_IKm.js",
  "/assets/SeekThumbnailPreview-CuaqBnqm.js",
  "/assets/StudioTab-1BYSzkt4.js",
  "/assets/VideoCameraOutlined-B7ozPsPa.js",
  "/assets/WifiOutlined-Dj0LgeQU.js",
  "/assets/card-DBsfQTSJ.js",
  "/assets/color-picker-BwLIfDph.js",
  "/assets/index-CJ662kxW.js",
  "/assets/index-CmJ0UNqQ.css",
  "/assets/input-number-CEqGqSJm.js",
  "/assets/jsx-runtime-Cd3lxc-q.js",
  "/assets/popconfirm-D_311LhX.js",
  "/assets/popover-C-1N7kAQ.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-BcvTvTSC.js",
  "/assets/slider-k5PRYBn4.js",
  "/assets/tag-C6mcpO-Q.js",
  "/assets/useBreakpoint-6jeVsX0-.js",
  "/index.html",
  "/manifest.webmanifest",
  "/api/runtime/app-icon",
  "/api/runtime/app-icon-192.png",
  "/api/runtime/app-icon-512.png"
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'de2cc20cb9005730' })),
  );
});

self.addEventListener('message', (event) => {
  if (event.data?.type === 'SKIP_WAITING') {
    event.waitUntil(self.skipWaiting());
    return;
  }
  if (event.data?.type === 'CLEAR_RUNTIME_CACHE') {
    event.waitUntil(caches.delete(RUNTIME));
  }
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
        .catch(async () => (await caches.match('/')) || (await caches.match('/index.html')) || Response.error()),
    );
    return;
  }

  const immutableAsset = url.pathname.startsWith('/assets/');
  if (immutableAsset || precached) {
    event.respondWith(caches.match(request).then((cached) => cached || fetch(request)));
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
