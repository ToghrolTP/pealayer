const PRECACHE = 'pealayer-precache-d537950f175ead49';
const RUNTIME = 'pealayer-runtime-d537950f175ead49';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-DcoVeA3V.js",
  "/assets/DesktopOutlined-Bajsyvjo.js",
  "/assets/EditOutlined-BYZ6JYHV.js",
  "/assets/EffectRecorder-C_gWbiUi.js",
  "/assets/EffectsTab-nWvN7oCR.js",
  "/assets/ExperimentOutlined-B2Wn7bNB.js",
  "/assets/HardwareTab-aHaQCTA8.js",
  "/assets/MediaLibraryTab-6culGiDm.js",
  "/assets/PlayCircleOutlined-W0VUyr-m.js",
  "/assets/PlayerInfoTab-B2UyNQNV.js",
  "/assets/PlusOutlined-D2ank2eV.js",
  "/assets/PoweroffOutlined-B5PCVPBK.js",
  "/assets/PreferencesTab-QFuQ5bC_.js",
  "/assets/PurePanel-D7FIvLfW.js",
  "/assets/ReloadOutlined-W1AeqZS_.js",
  "/assets/RemoteControlTab-COBAtMye.js",
  "/assets/SafetyCertificateOutlined-BO_K00mv.js",
  "/assets/SeekThumbnailPreview-CLR4_pBn.js",
  "/assets/StudioTab-zVpgEHQ9.js",
  "/assets/VideoCameraOutlined-Qdc00shw.js",
  "/assets/WifiOutlined-l5S4B6pe.js",
  "/assets/card-B8iOG9Bj.js",
  "/assets/color-picker-C_ZH1H_m.js",
  "/assets/index-BVOMCPhc.js",
  "/assets/index-DAJzYKSk.css",
  "/assets/input-number-AKDadGj0.js",
  "/assets/jsx-runtime-CMD6hquH.js",
  "/assets/popconfirm-C8ab8yOd.js",
  "/assets/popover-BZxs_oFC.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-zQMfE-8U.js",
  "/assets/slider-B9v2sD5V.js",
  "/assets/tag-D9n-sW6V.js",
  "/assets/useBreakpoint-CMrOadtt.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'd537950f175ead49' })),
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
