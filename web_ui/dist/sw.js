const PRECACHE = 'pealayer-precache-0c0e630e41aa9929';
const RUNTIME = 'pealayer-runtime-0c0e630e41aa9929';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-_kmqwLjf.js",
  "/assets/DesktopOutlined-8QncQUbm.js",
  "/assets/EditOutlined-C-KRu3WA.js",
  "/assets/EffectRecorder-C7mMVXLJ.js",
  "/assets/EffectsTab-TuKilKoI.js",
  "/assets/ExperimentOutlined-CMFP06nF.js",
  "/assets/GroupSelect-DHbAPWR8.js",
  "/assets/HardwareTab-CrrA3LTM.js",
  "/assets/MediaLibraryTab-DAt-d_lu.js",
  "/assets/PlayCircleOutlined-CMCvusps.js",
  "/assets/PlayerInfoTab-Br9oME-q.js",
  "/assets/PlusOutlined-CzuUAIpV.js",
  "/assets/PreferencesTab-BrimGYOU.js",
  "/assets/PurePanel-B3cH8JOC.js",
  "/assets/ReloadOutlined-C002nwlE.js",
  "/assets/RemoteControlTab-Ba7cDgQq.js",
  "/assets/SafetyCertificateOutlined-BqUSHxp-.js",
  "/assets/SeekThumbnailPreview-BuDUz3pm.js",
  "/assets/StudioTab-BdJpJLxX.js",
  "/assets/VideoCameraOutlined-C6SRxCk7.js",
  "/assets/WifiOutlined-DQqOpw_-.js",
  "/assets/card-DMNBwyLw.js",
  "/assets/color-picker-BKwgeQOq.js",
  "/assets/index-BjJmqBaL.css",
  "/assets/index-LgjW3V3L.js",
  "/assets/input-number-BRQQKLlc.js",
  "/assets/jsx-runtime-dP4oqQ0J.js",
  "/assets/popconfirm-BMIXZ7lr.js",
  "/assets/popover-CxjVW4Wd.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-BUmL34XO.js",
  "/assets/slider-B6HUMPJr.js",
  "/assets/useBreakpoint-fnRYBo5P.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '0c0e630e41aa9929' })),
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
