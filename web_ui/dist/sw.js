const PRECACHE = 'pealayer-precache-4031bfe7e40aa864';
const RUNTIME = 'pealayer-runtime-4031bfe7e40aa864';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-BAlLv-fE.js",
  "/assets/DesktopOutlined-7WgZ6K7i.js",
  "/assets/EditOutlined-CJP0Hha4.js",
  "/assets/EffectRecorder-BuRkFG8h.js",
  "/assets/EffectsTab-D3iUAMUw.js",
  "/assets/ExperimentOutlined-jJd3yYQX.js",
  "/assets/GroupSelect-BH5qBUHW.js",
  "/assets/HardwareTab-CHNoWpAT.js",
  "/assets/MediaLibraryTab-Y6vO234k.js",
  "/assets/PlayCircleOutlined-DkU5vG56.js",
  "/assets/PlayerInfoTab-DYgCVz1m.js",
  "/assets/PlusOutlined-3y78twXu.js",
  "/assets/PreferencesTab-Yz-b7J75.js",
  "/assets/PurePanel-CmAbvnXM.js",
  "/assets/ReloadOutlined-DrYbNdIG.js",
  "/assets/RemoteControlTab-KHnQdhpp.js",
  "/assets/SafetyCertificateOutlined-DIbLm714.js",
  "/assets/SeekThumbnailPreview-_sbtjBe2.js",
  "/assets/StudioTab-DJK9pqJP.js",
  "/assets/VideoCameraOutlined-C8TbqC0K.js",
  "/assets/WifiOutlined-Ck5PNDt5.js",
  "/assets/card-BNMURnyy.js",
  "/assets/color-picker-CEtTziGC.js",
  "/assets/index-DX5jK1IH.js",
  "/assets/index-QqMlIa4n.css",
  "/assets/input-number-BBPhxzMi.js",
  "/assets/jsx-runtime-UWGBd04a.js",
  "/assets/popconfirm-B-iZkMAi.js",
  "/assets/popover-YJfIngdA.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-DIEwPBcW.js",
  "/assets/slider-DYmP1Ih6.js",
  "/assets/useBreakpoint-VH5G0sbX.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '4031bfe7e40aa864' })),
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
