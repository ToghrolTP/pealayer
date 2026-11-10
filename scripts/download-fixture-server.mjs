// Local UI/HTTP acceptance fixture, not a production media server or downloader.
// Usage: node scripts/download-fixture-server.mjs <synthetic-media-file>
import { createServer } from 'node:http';
import { createReadStream, statSync } from 'node:fs';
import { basename, resolve } from 'node:path';
const file = resolve(process.argv[2] || '');
const size = statSync(file).size;
const server = createServer((request, response) => {
  if (!['GET', 'HEAD'].includes(request.method)) { response.writeHead(405).end(); return; }
  const range = request.headers.range?.match(/^bytes=(\d+)-(\d*)$/);
  const start = range ? Number(range[1]) : 0;
  const end = Math.min(size - 1, range?.[2] ? Number(range[2]) : size - 1);
  if (start > end || start >= size) { response.writeHead(416, { 'Content-Range': `bytes */${size}` }).end(); return; }
  const headers = { 'Content-Type': 'video/mp4', 'Content-Length': end - start + 1, 'ETag': '"synthetic-media-fixture"', 'Accept-Ranges': 'bytes' };
  if (range) headers['Content-Range'] = `bytes ${start}-${end}/${size}`;
  response.writeHead(range ? 206 : 200, headers);
  if (request.method === 'HEAD') { response.end(); return; }
  const source = createReadStream(file, { start, end, highWaterMark: 16384 });
  let timer;
  source.on('data', bytes => {
    source.pause(); response.write(bytes);
    timer = setTimeout(() => source.resume(), 40);
  });
  source.on('end', () => response.end());
  source.on('error', () => response.destroy());
  response.on('close', () => { clearTimeout(timer); source.destroy(); });
});
server.listen(0, '127.0.0.1', () => {
  console.log(JSON.stringify({ pid: process.pid, url: `http://127.0.0.1:${server.address().port}/${encodeURIComponent(basename(file))}` }));
});
setTimeout(() => { server.close(); server.closeAllConnections(); }, 15 * 60 * 1000).unref();
