import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

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
console.log(`Verified installable/offline PWA ${build.version} (${build.precache.length} resources)`);
