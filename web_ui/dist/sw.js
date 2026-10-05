const PRECACHE = 'pealayer-precache-7b5038a8d0061195';
const RUNTIME = 'pealayer-runtime-7b5038a8d0061195';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-D0mMD2E9.js",
  "/assets/DesktopOutlined-Bksiu6Vw.js",
  "/assets/EditOutlined-nhUtv0du.js",
  "/assets/EffectRecorder-gzGH0Ibv.js",
  "/assets/EffectsTab-BYtO-lqk.js",
  "/assets/ExperimentOutlined-BGyxqksx.js",
  "/assets/HardwareTab-CQlwiSxz.js",
  "/assets/MediaLibraryTab-DN1ojCGw.js",
  "/assets/PlayCircleOutlined-D69oIG1O.js",
  "/assets/PlayerInfoTab-TJmVYUcB.js",
  "/assets/PlusOutlined-jELm7Pdb.js",
  "/assets/PoweroffOutlined-CKwkcrL3.js",
  "/assets/PreferencesTab-3eOk95Ng.js",
  "/assets/PurePanel-BY8bUteB.js",
  "/assets/ReloadOutlined-DUNyhQdp.js",
  "/assets/RemoteControlTab-W3x7ymLX.js",
  "/assets/SafetyCertificateOutlined-3WQOHumX.js",
  "/assets/SeekThumbnailPreview-BkovJVzu.js",
  "/assets/StudioTab-BkWMr9MO.js",
  "/assets/VideoCameraOutlined-DIack_63.js",
  "/assets/WifiOutlined-DLEfzLi2.js",
  "/assets/card-DHw6zC5m.js",
  "/assets/color-picker-BZzHCZvV.js",
  "/assets/index-CmJ0UNqQ.css",
  "/assets/index-b_yjpaSe.js",
  "/assets/input-number-CaV1I03H.js",
  "/assets/jsx-runtime-CN-so136.js",
  "/assets/popconfirm-LzH-CIc4.js",
  "/assets/popover-DbNDQ74-.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-3pxX9uOk.js",
  "/assets/slider-BkGswul_.js",
  "/assets/tag-CI2FjkKX.js",
  "/assets/useBreakpoint-DLeCFfgu.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '7b5038a8d0061195' })),
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
