import assert from 'node:assert/strict';
import { importAssetFiles } from '../src/assetImports.ts';
const originalFetch = globalThis.fetch;
const calls = [], commands = [], progress = [];
let failChunk = false;
globalThis.fetch = async (url, init) => {
  calls.push([url, init]);
  if (url.includes('/begin')) {
    const value = JSON.parse(init.body);
    assert.match(value.batch_id, /^[0-9a-f-]{36}$/);
    assert.equal(value.name, 'sound.wav');
    return Response.json({ id: 'transfer', chunk_bytes: 512 * 1024, owner_id:'selected-authority' });
  }
  if (url.includes('/chunk')) {
    if (failChunk) return Response.json({ error: 'Test transfer rejection' }, { status: 400 });
    const offset = Number(new URL(url).searchParams.get('offset'));
    assert.equal(init.headers['Content-Type'], 'application/octet-stream');
    assert.ok(init.body.size <= 512 * 1024);
    return Response.json({ offset: offset + init.body.size });
  }
  if (url.includes('/finish')) return Response.json({ path: '/authority/assets/sound.wav', owner_id:'selected-authority' });
  if(url.includes('/retain'))return Response.json({retained:true});
  if (init.method === 'DELETE') return Response.json({ cancelled: true });
  throw Error(`Unexpected route ${url}`);
};
try {
  const destination = { surface: 'timeline', start_time_ms: 10429, track_key: 'controller-effect:audio' };
  const file = new File([new Uint8Array(512 * 1024 + 17)], 'sound.wav');
  const send = async (...command) => { commands.push(command); return true; };
  await importAssetFiles([file], destination, 'http://example.invalid', send, value => progress.push(value));
  assert.equal(calls.filter(([url]) => url.includes('/chunk')).length, 2);
  assert.deepEqual(progress, [100, 100]);
  assert.deepEqual(commands, [['import_assets', { request: { paths: ['/authority/assets/sound.wav'], destination,owner_id:'selected-authority' } }]]);
  assert.ok(!JSON.stringify(commands).includes('fakepath'));
  failChunk = true; commands.length = 0; calls.length = 0;
  await assert.rejects(importAssetFiles([file], destination, 'http://example.invalid', send), /Test transfer rejection/);
  assert.equal(commands.length, 0, 'a failed transfer must never import a partial file');
  assert.ok(calls.some(([,init]) => init.method === 'DELETE'), 'failed transfers are cancelled');
  assert.ok(calls.some(([url,init])=>url.includes('/batch?') && init.method==='DELETE'),'the whole partially completed batch is cancelled');
  failChunk=false;calls.length=0;
  await assert.rejects(importAssetFiles([file],destination,'http://example.invalid',async()=>{throw Error('Command queue disconnected');}),/queue disconnected/);
  assert.ok(calls.some(([url,init])=>url.includes('/batch?') && init.method==='DELETE'));
  await assert.rejects(importAssetFiles([file,file],destination,'',send),/distinct names/);
  await assert.rejects(importAssetFiles([], destination, '', send), /1–64/);
  console.log('Asset imports: ordered bounded chunks, authority paths, exact drop time, cancellation and failure isolation passed');
} finally { globalThis.fetch = originalFetch; }
