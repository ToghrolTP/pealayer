import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import vm from 'node:vm';

const dist = new URL('../dist/', import.meta.url);
const build = JSON.parse(await readFile(new URL('pwa-build.json', dist), 'utf8'));
const stylesheets = (await readdir(new URL('assets/', dist))).filter(name => name.endsWith('.css'));
assert.deepEqual(stylesheets, ['app.css'], 'Keep shared CSS in the initial stylesheet; versioned lazy CSS breaks the preload suffix check');
const appCss = await readFile(new URL('assets/app.css', dist), 'utf8');
assert.match(appCss, /\.download-center/, 'Downloads must ship in the initial application stylesheet');
const worker = await readFile(new URL('sw.js', dist), 'utf8');
const events = new Map();
const storage = new Map();
const requests = [];
let offline = false;
const normalize = request => new URL(typeof request === 'string' ? request : request.url, 'https://local.invalid').href;
const caches = {
  async open(name) {
    if (!storage.has(name)) storage.set(name, new Map());
    const cache = storage.get(name);
    return {
      async match(request) { return cache.get(normalize(request))?.clone(); },
      async put(request, response) { cache.set(normalize(request), response); },
      async addAll(urls) { for (const url of urls) cache.set(normalize(url), new Response(url)); },
    };
  },
  async match() { throw new Error('Cross-generation cache lookup is forbidden'); },
};
const fetch = async (request, options) => {
  requests.push({ url: normalize(request), options });
  if (offline) throw new Error('offline');
  return new Response('network:current');
};
vm.runInNewContext(worker, {
  URL, Request, Response, caches, fetch,
  self: { location: new URL('https://local.invalid'), registration: {},
    addEventListener: (name, handler) => events.set(name, handler) },
});
const currentCache = await caches.open(`pealayer-precache-${build.version}`);
const asset = `/assets/app.js?v=${build.version}`;
await currentCache.put(asset, new Response('cached:current'));
await (await caches.open('pealayer-precache-previous')).put(asset, new Response('cached:wrong-generation'));
async function responseFor(url, mode = 'cors') {
  let response;
  events.get('fetch')({ request: { method: 'GET', url: normalize(url), mode },
    respondWith: promise => { response = promise; } });
  return response ? await response : undefined;
}
assert.equal(await (await responseFor(asset)).text(), 'cached:current');
assert.equal(requests.length, 0, 'Exact-version precache serves offline assets without requests');
assert.equal(await (await responseFor('/assets/app.js?v=other')).text(), 'network:current');
assert.equal(await (await responseFor('/assets/app.js')).text(), 'network:current');
assert.equal(requests.at(-1).options.cache, 'no-cache');
await currentCache.put('/index.html', new Response('current offline shell'));
await currentCache.put('/fuji-loader.svg', new Response('offline mountain'));
offline = true;
assert.equal(await (await responseFor(asset)).text(), 'cached:current');
assert.equal(await (await responseFor('/fuji-loader.svg')).text(), 'offline mountain');
assert.equal(await (await responseFor('/', 'navigate')).text(), 'current offline shell');
await assert.rejects(() => responseFor('/assets/app.js?v=missing'), /offline/,
  'A missing generation must fail, never substitute current bytes');
assert.equal(await responseFor('/api/player/status'), undefined, 'Never cache live/API state');
const css = await readFile(new URL('../public/fuji-loader.css', import.meta.url), 'utf8');
const svg = await readFile(new URL('../public/fuji-loader.svg', import.meta.url), 'utf8');
assert.match(css, /prefers-reduced-motion/);
assert.match(svg, /prefers-reduced-motion/);
assert.match(css, /height: 131px/, 'Stable loader footprint');
assert.match(css, /box-sizing: border-box/, 'Bootstrap must fit before the main stylesheet arrives');
console.log('Readable/versioned asset cache and Fuji loader guardrails passed.');
