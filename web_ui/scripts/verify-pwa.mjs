import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readdir } from 'node:fs/promises';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(root, 'dist');
const [index, manifestText, worker, buildText] = await Promise.all([
  readFile(join(dist, 'index.html'), 'utf8'),
  readFile(join(dist, 'manifest.webmanifest'), 'utf8'),
  readFile(join(dist, 'sw.js'), 'utf8'),
  readFile(join(dist, 'pwa-build.json'), 'utf8'),
]);
const manifest = JSON.parse(manifestText);
const build = JSON.parse(buildText);
const requiredManifestFields = ['id', 'name', 'short_name', 'start_url', 'scope', 'display', 'icons'];
for (const field of requiredManifestFields) {
  if (!(field in manifest)) throw new Error(`PWA manifest is missing ${field}`);
}
if (!Array.isArray(manifest.icons) || !manifest.icons.some((icon) => icon.sizes === '192x192') || !manifest.icons.some((icon) => icon.sizes === '512x512')) {
  throw new Error('PWA manifest must include 192px and 512px icons');
}
if (worker.includes('__CACHE_VERSION__') || worker.includes('__PRECACHE_MANIFEST__')) {
  throw new Error('Service worker was not finalized');
}
for (const match of index.matchAll(/(?:src|href)="\.?(\/assets\/[^"]+)"/g)) {
  if (!worker.includes(JSON.stringify(match[1]))) {
    throw new Error(`Service worker does not precache ${match[1]}`);
  }
}
if (!Array.isArray(build.precache) || build.precache.length < 5) {
  throw new Error('PWA precache manifest is unexpectedly empty');
}
const assets = await readdir(join(dist, 'assets'));
if (!assets.includes('app.js') || !assets.includes('app.css')) throw new Error('Readable app.js/app.css entry names are missing');
for (const name of assets) {
  if (/-[A-Za-z0-9_-]{8}\.(js|css)$/.test(name)) throw new Error(`Hashed asset suffix returned: ${name}`);
  const url = `/assets/${name}?v=${build.version}`;
  if (!build.precache.includes(url)) throw new Error(`Versioned asset missing from precache: ${url}`);
  if (!name.endsWith('.js')) continue;
  const script = await readFile(join(dist, 'assets', name), 'utf8');
  for (const [, , reference] of script.matchAll(/(["'])(\.\.?\/[^"'\\]+|\/assets\/[^"'\\]+)\1/g)) {
    const target = new URL(reference, `https://local.invalid/assets/${name}`);
    if (assets.includes(target.pathname.split('/').at(-1)) && target.searchParams.get('v') !== build.version) {
      throw new Error(`Unversioned or wrong-version import/preload in ${name}: ${reference}`);
    }
  }
}
for (const name of ['fuji-loader.css', 'fuji-loader.svg']) {
  if (!build.precache.includes(`/${name}`)) throw new Error(`Loader not available offline: ${name}`);
}
console.log(`Verified installable/offline PWA ${build.version} (${build.precache.length} resources)`);
