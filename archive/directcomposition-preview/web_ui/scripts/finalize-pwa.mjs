import { createHash } from 'node:crypto';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(root, 'dist');

async function filesBelow(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(entries.map(async (entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? filesBelow(path) : [path];
  }));
  return files.flat();
}

const files = (await filesBelow(dist))
  .filter((path) => relative(dist, path).replaceAll(sep, '/') !== 'sw.js')
  .sort();
const digest = createHash('sha256');
// Stamping rules are part of the build's bytes too: changing this recipe must
// invalidate caches even when Vite's raw inputs/output are unchanged.
digest.update(await readFile(fileURLToPath(import.meta.url)));
for (const path of files) {
  digest.update(relative(dist, path).replaceAll(sep, '/'));
  digest.update(await readFile(path));
}
const serviceWorkerPath = join(dist, 'sw.js');
const template = await readFile(serviceWorkerPath, 'utf8');
if (!template.includes('__CACHE_VERSION__') || !template.includes('__PRECACHE_MANIFEST__')) {
  throw new Error('Service-worker template placeholders are missing');
}
digest.update('sw.js');
digest.update(template);
const version = digest.digest('hex').slice(0, 16);
// Keep the content revision in URLs, not filenames. Stamp only references to
// assets actually emitted by this build, including lazy/preloaded chunks.
const assets = new Set(files.filter(path => relative(dist, path).replaceAll(sep, '/').startsWith('assets/')));
function versionedReference(reference, from) {
  const target = reference.startsWith('/') ? join(dist, reference.slice(1)) : resolve(dirname(from), reference);
  return assets.has(target) ? `${reference}?v=${version}` : reference;
}
for (const path of files) {
  if (path.endsWith('.html') || path.endsWith('.js')) {
    const source = await readFile(path, 'utf8');
    const stamped = source.replace(/(["'`])(\.\.?\/[^"'`\\]+|\/assets\/[^"'`\\]+)\1/g,
      (match, quote, reference) => `${quote}${versionedReference(reference, path)}${quote}`);
    await writeFile(path, stamped, 'utf8');
  }
}
const precache = [
  '/',
  ...files.map((path) => {
    const url = `/${relative(dist, path).replaceAll(sep, '/')}`;
    return assets.has(path) ? `${url}?v=${version}` : url;
  }),
  '/manifest.webmanifest',
  '/api/runtime/app-icon',
  '/api/runtime/app-icon-192.png',
  '/api/runtime/app-icon-512.png',
].filter((value, index, values) => values.indexOf(value) === index);

await writeFile(
  serviceWorkerPath,
  template
    .replaceAll('__CACHE_VERSION__', version)
    .replace('__PRECACHE_MANIFEST__', JSON.stringify(precache, null, 2)),
  'utf8',
);
await writeFile(join(dist, 'pwa-build.json'), `${JSON.stringify({ version, precache }, null, 2)}\n`, 'utf8');
console.log(`PWA ${version}: precached ${precache.length} resources`);
