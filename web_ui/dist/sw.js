const PRECACHE = 'pealayer-precache-6063d42bcdabd775';
const RUNTIME = 'pealayer-runtime-6063d42bcdabd775';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-qqGmZHCX.js",
  "/assets/DesktopOutlined-DO9CdVDh.js",
  "/assets/EditOutlined-DiuPm6ii.js",
  "/assets/EffectRecorder-DJvS0fQB.js",
  "/assets/EffectsTab--Q8pJa0F.js",
  "/assets/ExperimentOutlined-mWzy_BVM.js",
  "/assets/HardwareTab-BefpMAq5.js",
  "/assets/MediaLibraryTab-BBAsSZ8l.js",
  "/assets/PlayCircleOutlined-9Gxc7cTN.js",
  "/assets/PlayerInfoTab-BsZbtNsJ.js",
  "/assets/PlusOutlined-S5Bd90wi.js",
  "/assets/PoweroffOutlined-B-Namu2b.js",
  "/assets/PreferencesTab-DbDhLj1V.js",
  "/assets/PurePanel-Cxj56a1R.js",
  "/assets/ReloadOutlined-uU1_tJi6.js",
  "/assets/RemoteControlTab-B1B8xiNp.js",
  "/assets/SafetyCertificateOutlined-CO42S9DA.js",
  "/assets/SeekThumbnailPreview-CzVAriB0.js",
  "/assets/StudioTab-CMM3R8PH.js",
  "/assets/VideoCameraOutlined-ClHXM_Do.js",
  "/assets/WifiOutlined-Bd-ykqHa.js",
  "/assets/card-4Ho8Iwsr.js",
  "/assets/color-picker-JgFFpjcv.js",
  "/assets/index-BgjOrAmG.js",
  "/assets/index-DAq0e4qm.css",
  "/assets/input-number-DZ_9MOsF.js",
  "/assets/jsx-runtime-CnMSMgvj.js",
  "/assets/popconfirm-zYPtzHYY.js",
  "/assets/popover-2LxV5Q0d.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-DstQIq0p.js",
  "/assets/slider-CmCCAdct.js",
  "/assets/tag-BYuro5_x.js",
  "/assets/useBreakpoint-CMC9TXn1.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '6063d42bcdabd775' })),
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
