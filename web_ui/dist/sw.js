const PRECACHE = 'pealayer-precache-5e2f501e894ec06c';
const RUNTIME = 'pealayer-runtime-5e2f501e894ec06c';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-_kmqwLjf.js",
  "/assets/DesktopOutlined-8QncQUbm.js",
  "/assets/EditOutlined-BztVqcXr.js",
  "/assets/EffectRecorder-CxJnkJ4U.js",
  "/assets/EffectsTab-55v6qjdK.js",
  "/assets/ExperimentOutlined-CMFP06nF.js",
  "/assets/GroupSelect-BwanugAl.js",
  "/assets/HardwareTab-CaNhQCp9.js",
  "/assets/MediaLibraryTab-Blrr4zYG.js",
  "/assets/PlayCircleOutlined-DIDFKx_B.js",
  "/assets/PlayerInfoTab-BgWqutkg.js",
  "/assets/PlusOutlined-CzuUAIpV.js",
  "/assets/PreferencesTab-kjo14Y-T.js",
  "/assets/PurePanel-B3cH8JOC.js",
  "/assets/ReloadOutlined-C002nwlE.js",
  "/assets/RemoteControlTab-Bku-jx6V.js",
  "/assets/SafetyCertificateOutlined-BqUSHxp-.js",
  "/assets/SeekThumbnailPreview-BuDUz3pm.js",
  "/assets/StudioTab-DX8ofR71.js",
  "/assets/VideoCameraOutlined-C6SRxCk7.js",
  "/assets/WifiOutlined-DQqOpw_-.js",
  "/assets/card-iXeKmpfq.js",
  "/assets/color-picker-B9zDmUPr.js",
  "/assets/index-CsebwcQS.css",
  "/assets/index-DX7FmwgC.js",
  "/assets/input-number-WRpzs9c4.js",
  "/assets/jsx-runtime-dP4oqQ0J.js",
  "/assets/popconfirm-Bk48klCt.js",
  "/assets/popover-CxjVW4Wd.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-BUmL34XO.js",
  "/assets/slider-B6HUMPJr.js",
  "/assets/tag-BOu0NZT4.js",
  "/assets/useBreakpoint-CnrTiq59.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '5e2f501e894ec06c' })),
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
