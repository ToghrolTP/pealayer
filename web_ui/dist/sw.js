const PRECACHE = 'pealayer-precache-7ce616e4f63b30c7';
const RUNTIME = 'pealayer-runtime-7ce616e4f63b30c7';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-hes-32ZO.js",
  "/assets/DesktopOutlined-JqUUk4Aw.js",
  "/assets/EditOutlined-Bc_uYgDG.js",
  "/assets/EffectRecorder-e68foAAk.js",
  "/assets/EffectsTab-BPwurard.js",
  "/assets/ExperimentOutlined-Bhx4VUAW.js",
  "/assets/GroupSelect-CkXxHIPU.js",
  "/assets/HardwareTab-CUV_8p_D.js",
  "/assets/MediaLibraryTab-ClHZ9N2X.js",
  "/assets/PlayCircleOutlined-DLpmfwO5.js",
  "/assets/PlayerInfoTab-CvbVKmuC.js",
  "/assets/PlusOutlined-qivEQ7rR.js",
  "/assets/PreferencesTab-DbfXBfgR.js",
  "/assets/PurePanel-BUe2aQHl.js",
  "/assets/ReloadOutlined-jM48Znes.js",
  "/assets/RemoteControlTab-DwzABWMb.js",
  "/assets/SafetyCertificateOutlined-B6m7wgx7.js",
  "/assets/SeekThumbnailPreview-L_z1EQ89.js",
  "/assets/StudioTab-p9Lzcmn4.js",
  "/assets/VideoCameraOutlined-DAXNkNk7.js",
  "/assets/WifiOutlined-D_uvCV7e.js",
  "/assets/card-CoLxnLyB.js",
  "/assets/color-picker-Djkg4QJR.js",
  "/assets/index-Ch8ngeQl.js",
  "/assets/index-CmJ0UNqQ.css",
  "/assets/input-number-pBX-X8NN.js",
  "/assets/jsx-runtime-tAlSFi2n.js",
  "/assets/popconfirm-D5t_2uk7.js",
  "/assets/popover-BJ0kDAjU.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-DCN4sUDw.js",
  "/assets/slider-D99XS3KV.js",
  "/assets/tag-Cn_DjsOF.js",
  "/assets/useBreakpoint-tb7ugNKZ.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: '7ce616e4f63b30c7' })),
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
