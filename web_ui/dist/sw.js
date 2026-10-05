const PRECACHE = 'pealayer-precache-9ac12a4f610f419a';
const RUNTIME = 'pealayer-runtime-9ac12a4f610f419a';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-DJxotlWD.js",
  "/assets/DesktopOutlined-Bwt1MAKv.js",
  "/assets/EditOutlined-B44UXSHl.js",
  "/assets/EffectRecorder-DeMk5B02.js",
  "/assets/EffectsTab-CA1EoYsD.js",
  "/assets/ExperimentOutlined-Co_vjfwS.js",
  "/assets/HardwareTab-InInRg6H.js",
  "/assets/MediaLibraryTab-KqQf0n6v.js",
  "/assets/PlayCircleOutlined-olgrtN9V.js",
  "/assets/PlayerInfoTab-BKE3WSAV.js",
  "/assets/PlusOutlined-DxSd7o5K.js",
  "/assets/PoweroffOutlined-D3dDDA8k.js",
  "/assets/PreferencesTab-DS5PvPbr.js",
  "/assets/PurePanel-6A2B1xI5.js",
  "/assets/ReloadOutlined-C0uaAqNg.js",
  "/assets/RemoteControlTab-BZpZ8roL.js",
  "/assets/SaveOutlined-DsxFnZRs.js",
  "/assets/StudioTab-BRd3vDV6.js",
  "/assets/VideoCameraOutlined-B5_BYbPh.js",
  "/assets/card-DwPzKmps.js",
  "/assets/color-picker-BCOUnaZY.js",
  "/assets/index-CRuMgLnb.js",
  "/assets/index-CdyoV5Uj.css",
  "/assets/input-number-Bbxp168P.js",
  "/assets/jsx-runtime-B-31jcj5.js",
  "/assets/mediaLabel-UKWxP5V9.js",
  "/assets/popconfirm-DjMQROpF.js",
  "/assets/popover-CUop7NR1.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-Dn496RGN.js",
  "/assets/slider-DDkzKduI.js",
  "/assets/tag-Fjj5fDLD.js",
  "/assets/useBreakpoint-eCZ2ZRgZ.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '9ac12a4f610f419a' })),
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
