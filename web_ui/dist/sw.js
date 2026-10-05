const PRECACHE = 'pealayer-precache-c2e763e3fce816d4';
const RUNTIME = 'pealayer-runtime-c2e763e3fce816d4';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-DJxotlWD.js",
  "/assets/DesktopOutlined-Bwt1MAKv.js",
  "/assets/EditOutlined-B44UXSHl.js",
  "/assets/EffectRecorder-Bh0VNejV.js",
  "/assets/EffectsTab-CrJuMM_p.js",
  "/assets/ExperimentOutlined-Co_vjfwS.js",
  "/assets/HardwareTab-CMtx0ItD.js",
  "/assets/MediaLibraryTab-Ta7qSPQS.js",
  "/assets/PlayCircleOutlined-olgrtN9V.js",
  "/assets/PlayerInfoTab-CQmPFgqp.js",
  "/assets/PlusOutlined-DxSd7o5K.js",
  "/assets/PoweroffOutlined-D3dDDA8k.js",
  "/assets/PreferencesTab-Ct5NaN7Q.js",
  "/assets/PurePanel-6A2B1xI5.js",
  "/assets/ReloadOutlined-C0uaAqNg.js",
  "/assets/RemoteControlTab-BdFZ79gU.js",
  "/assets/SafetyCertificateOutlined-7dZdlCrT.js",
  "/assets/SeekThumbnailPreview-BWNunlg4.js",
  "/assets/StudioTab-BA64VDd_.js",
  "/assets/VideoCameraOutlined-B5_BYbPh.js",
  "/assets/WifiOutlined-BijiM7sv.js",
  "/assets/card-DwPzKmps.js",
  "/assets/color-picker-BCOUnaZY.js",
  "/assets/index-1mh2vuc4.js",
  "/assets/index-BwFr6dhT.css",
  "/assets/input-number-Bbxp168P.js",
  "/assets/jsx-runtime-B-31jcj5.js",
  "/assets/popconfirm-DArZZqEr.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'c2e763e3fce816d4' })),
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
