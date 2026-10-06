const PRECACHE = 'pealayer-precache-95b9e14cc0c3d172';
const RUNTIME = 'pealayer-runtime-95b9e14cc0c3d172';
const PRECACHE_URLS = [
  "/",
  "/assets/DeleteOutlined-4ZN86VK3.js",
  "/assets/DesktopOutlined-D8K15JLE.js",
  "/assets/EditOutlined-znOOb3uB.js",
  "/assets/EffectRecorder-CYj49Oi_.js",
  "/assets/EffectsTab-BLwkUD3Y.js",
  "/assets/ExperimentOutlined-DqFd4C5N.js",
  "/assets/GroupSelect-DPb3ipY2.js",
  "/assets/HardwareTab-DTJqTYGq.js",
  "/assets/MediaLibraryTab-Ce59piYM.js",
  "/assets/PlayerInfoTab-C774jN2W.js",
  "/assets/PlusOutlined-DjPg-FwS.js",
  "/assets/PoweroffOutlined-Bdp236F-.js",
  "/assets/PreferencesTab-BZh1HBIZ.js",
  "/assets/RemoteControlTab-CMJeJRno.js",
  "/assets/SafetyCertificateOutlined-Cm0RLRZf.js",
  "/assets/SaveOutlined-DVGeYuIC.js",
  "/assets/SeekThumbnailPreview-D6OWnhUJ.js",
  "/assets/StudioTab-UX49_3vA.js",
  "/assets/VideoCameraOutlined-F99UUR77.js",
  "/assets/WifiOutlined-wlggCDI1.js",
  "/assets/card-C3ZhEKOt.js",
  "/assets/color-picker-Bsf2GdjK.js",
  "/assets/index-B8EuuzGy.js",
  "/assets/index-BwFv2342.css",
  "/assets/jsx-runtime-AFcmMaSu.js",
  "/assets/message-DRwuKd7R.js",
  "/assets/popconfirm-Du8cC4DP.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/row-r3lmpsX1.js",
  "/assets/select-BcNaH51l.js",
  "/assets/slider-CWUzJxLE.js",
  "/assets/typography-DPo35_hF.js",
  "/assets/useBreakpoint-CLES_WsJ.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '95b9e14cc0c3d172' })),
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
