const PRECACHE = 'pealayer-precache-fd187b84c0ff7b55';
const RUNTIME = 'pealayer-runtime-fd187b84c0ff7b55';
const PRECACHE_URLS = [
  "/",
  "/assets/ArrowUpOutlined-SD3p3fMs.js",
  "/assets/DesktopOutlined-rdx9E6uG.js",
  "/assets/EditOutlined-CEx4jkL1.js",
  "/assets/EffectRecorder-Ck43afTl.js",
  "/assets/EffectsTab-jtDaALII.js",
  "/assets/ExperimentOutlined-DaMNI6pf.js",
  "/assets/HardwareTab-CXaw7Mlu.js",
  "/assets/MediaLibraryTab-C9-QBvr2.js",
  "/assets/PlayCircleOutlined-Ciz16_mj.js",
  "/assets/PlayerInfoTab-71sBC-PY.js",
  "/assets/PlusOutlined-Txz64LYM.js",
  "/assets/PoweroffOutlined-CL0yxdSn.js",
  "/assets/PreferencesTab-CYOoLwAT.js",
  "/assets/PurePanel-BmkIWIqd.js",
  "/assets/ReloadOutlined-K2CyP5HI.js",
  "/assets/RemoteControlTab-Cg8zLZsj.js",
  "/assets/SafetyCertificateOutlined-DlQz7AmJ.js",
  "/assets/SeekThumbnailPreview-n8fnsNiJ.js",
  "/assets/StudioTab-BOKYvUnw.js",
  "/assets/VideoCameraOutlined-DpYksmjw.js",
  "/assets/WifiOutlined-CcHtcAPB.js",
  "/assets/card-Di76FJMQ.js",
  "/assets/color-picker-BO6oTBFv.js",
  "/assets/index-DcC9_MpE.js",
  "/assets/index-fRorUOSV.css",
  "/assets/input-number-BPtkIO4q.js",
  "/assets/jsx-runtime-CeFq0KU_.js",
  "/assets/popconfirm-DxxDFlxP.js",
  "/assets/popover-DWIrCK8G.js",
  "/assets/rolldown-runtime-CbXtAM7H.js",
  "/assets/select-CpJVPwFs.js",
  "/assets/slider-cwKTlQqJ.js",
  "/assets/tag-vRz2exhp.js",
  "/assets/useBreakpoint-B6dO6j2x.js",
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
      .then(() => notifyClients({ type: 'PEALAYER_SW_READY', version: 'fd187b84c0ff7b55' })),
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
