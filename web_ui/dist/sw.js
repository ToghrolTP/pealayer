const PRECACHE = 'pealayer-precache-deaba88c7cca6b3e';
const RUNTIME = 'pealayer-runtime-deaba88c7cca6b3e';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-BKy28yYq.js",
  "/assets/DesktopOutlined-BTNPyqkY.js",
  "/assets/EditOutlined-cWY-2H6E.js",
  "/assets/EffectRecorder-Dr9-lctS.js",
  "/assets/EffectsTab-Dwz4alwd.js",
  "/assets/ExperimentOutlined-3QDmGUTm.js",
  "/assets/HardwareTab-huHSAgXf.js",
  "/assets/MediaLibraryTab-GNKE2CuZ.js",
  "/assets/PlayCircleOutlined-DUExUWBf.js",
  "/assets/PlayerInfoTab-DAOWY4TO.js",
  "/assets/PlusOutlined-Dnvd5TXg.js",
  "/assets/PoweroffOutlined-ZsUOtQJl.js",
  "/assets/PreferencesTab-Mwi3RotG.js",
  "/assets/PurePanel-Cb-nGSld.js",
  "/assets/ReloadOutlined-BiT51hVf.js",
  "/assets/RemoteControlTab-Bek81OL2.js",
  "/assets/SafetyCertificateOutlined-C0W260ai.js",
  "/assets/SeekThumbnailPreview-BIarQ8TI.js",
  "/assets/StudioTab-egSNnmaj.js",
  "/assets/VideoCameraOutlined-9FDKlFT6.js",
  "/assets/WifiOutlined-C4DroNDA.js",
  "/assets/card-DZN5x2Nj.js",
  "/assets/color-picker-CahMNHK1.js",
  "/assets/index-DE7AHmL8.js",
  "/assets/index-fRorUOSV.css",
  "/assets/input-number-BECgPZTS.js",
  "/assets/jsx-runtime-DIGo9crn.js",
  "/assets/popconfirm-DLxOr3vj.js",
  "/assets/popover-q_pHk2eF.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-DNC_Yjoh.js",
  "/assets/slider-BmE8Lp7w.js",
  "/assets/tag-BDGkM9by.js",
  "/assets/useBreakpoint-CGirdC2Y.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'deaba88c7cca6b3e' })),
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
