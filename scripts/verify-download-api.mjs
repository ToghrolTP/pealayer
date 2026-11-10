// Run only against an isolated acceptance instance and synthetic fixture.
// Usage: node scripts/verify-download-api.mjs <backend-url> <fixture-url>
import assert from 'node:assert/strict';
const backend = new URL(process.argv[2]);
const fixture = new URL(process.argv[3]);
assert.equal(backend.hostname, '127.0.0.1', 'Acceptance backend must be loopback');
assert.equal(fixture.hostname, '127.0.0.1', 'Use a synthetic loopback fixture');
async function rpc(method, params = {}) {
  const response = await fetch(new URL('/api/rpc', backend), {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: method, method, params }),
    signal: AbortSignal.timeout(15000),
  });
  const body = await response.json();
  assert.equal(response.status, 200);
  if (body.error) throw new Error(body.error.message);
  return body.result;
}
async function jobUntil(id, predicate) {
  const deadline = Date.now() + 45000;
  while (Date.now() < deadline) {
    const job = (await rpc('pealayer.downloads.list')).jobs.find(job => job.id === id);
    assert.ok(job, 'Job remains in authoritative queue');
    if (predicate(job)) return job;
    if (job.state === 'failed') throw new Error(job.error);
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error('Acceptance job timed out');
}
const settings = await rpc('pealayer.downloads.list');
assert.equal(settings.jobs.length, 0, 'Use a fresh isolated download root');
const original = Buffer.from(await (await fetch(fixture)).arrayBuffer());
const { id } = await rpc('pealayer.downloads.add', { url: fixture.href, use_proxy: false });
await jobUntil(id, job => job.downloaded > 0 && job.state === 'downloading');
await rpc('pealayer.downloads.action', { id, action: 'pause' });
const paused = await jobUntil(id, job => job.state === 'paused' && job.actions.includes('resume'));
const media = new URL(`/api/downloads/media?id=${id}`, backend);
const start = Math.max(0, paused.downloaded - 64);
const end = Math.min(original.length - 1, paused.downloaded + 4095);
const gap = await fetch(media, { headers: { Range: `bytes=${start}-${end}` } });
assert.equal(gap.status, 206);
assert.equal(gap.headers.get('content-range'), `bytes ${start}-${end}/${original.length}`);
assert.deepEqual(Buffer.from(await gap.arrayBuffer()), original.subarray(start, end + 1));
console.log('PASS partial cached prefix + origin gap are exact through HTTP');
await rpc('pealayer.downloads.action', { id, action: 'resume' });
const completed = await jobUntil(id, job => job.state === 'complete');
assert.equal(completed.downloaded, original.length);
const head = await fetch(media, { method: 'HEAD' });
assert.equal(head.status, 200);
assert.equal(Number(head.headers.get('content-length')), original.length);
assert.equal((await head.arrayBuffer()).byteLength, 0);
const full = await fetch(media);
assert.deepEqual(Buffer.from(await full.arrayBuffer()), original);
const invalid = await fetch(media, { headers: { Range: `bytes=${original.length}-` } });
assert.equal(invalid.status, 416);
console.log('PASS completed GET/HEAD/range bounds and resumed payload');
const cancelled = await rpc('pealayer.downloads.add', { url: fixture.href });
await rpc('pealayer.downloads.action', { id: cancelled.id, action: 'cancel' });
await jobUntil(cancelled.id, job => job.state === 'cancelled');
console.log('PASS cancellation acknowledged by shared RPC');
console.log(JSON.stringify({ id, paused_bytes: paused.downloaded, complete_bytes: original.length }));
