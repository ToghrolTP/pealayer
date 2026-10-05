const PRECACHE = 'pealayer-precache-4010653929ee8c77';
const RUNTIME = 'pealayer-runtime-4010653929ee8c77';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-SHxqCr-T.js",
  "/assets/DesktopOutlined-C0UExJev.js",
  "/assets/EditOutlined-DMwmCJGO.js",
  "/assets/EffectRecorder-BmddXdLs.js",
  "/assets/EffectsTab-ChoCPR-k.js",
  "/assets/ExperimentOutlined-DyF3_tzl.js",
  "/assets/HardwareTab-Bwec1rFY.js",
  "/assets/MediaLibraryTab-CzP_j2uc.js",
  "/assets/PlayCircleOutlined-0QaiL77P.js",
  "/assets/PlayerInfoTab-DmX4IhA_.js",
  "/assets/PlusOutlined-BG_aT52j.js",
  "/assets/PoweroffOutlined-AvBs5h9y.js",
  "/assets/PreferencesTab-s49_oe-b.js",
  "/assets/PurePanel-DZepFf9f.js",
  "/assets/ReloadOutlined-CY65vbqz.js",
  "/assets/RemoteControlTab-AG-JKaHr.js",
  "/assets/SaveOutlined-Byr_peut.js",
  "/assets/StudioTab-CI01HsUO.js",
  "/assets/VideoCameraOutlined-BEuEe_m8.js",
  "/assets/card-ClVN66nK.js",
  "/assets/color-picker-BSIL1DQo.js",
  "/assets/index-Bf0t9ckl.js",
  "/assets/index-D7ZmPguO.css",
  "/assets/input-number-BKNczKTw.js",
  "/assets/jsx-runtime-Cpbjta6n.js",
  "/assets/mediaLabel-Bq8_juWt.js",
  "/assets/popconfirm-8I5MlDKY.js",
  "/assets/popover-Cf65V9_G.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-h75GkNaZ.js",
  "/assets/slider-CgOl5QKy.js",
  "/assets/tag-XmJeZ4Ss.js",
  "/assets/useBreakpoint-Q8BvB26C.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '4010653929ee8c77' })),
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
