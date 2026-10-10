// Live default-policy acceptance. Uses synthetic loopback pages and changes no
// preferences, media, cues or hardware. Run only while the browser is closed.
// node scripts/verify-copied-link-api.mjs http://127.0.0.1:8080
import assert from 'node:assert/strict';
import { createServer } from 'node:http';

const backend = new URL(process.argv[2] || 'http://127.0.0.1:8080');
assert.equal(backend.hostname, '127.0.0.1', 'Run on the host being verified');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
let completed = 0;
const fixture = createServer((request, response) => {
  const path = new URL(request.url, 'http://127.0.0.1').pathname;
  const body = path === '/unsupported' ? '<h1>A normal website</h1>'
    : path === '/file.txt' ? 'ordinary text' : '<h1>Index of /</h1><pre><a href="clip.mp4">clip.mp4</a></pre>';
  const send = () => {
    response.writeHead(200, {'Content-Type': path === '/file.txt' ? 'text/plain' : 'text/html', 'Content-Length': Buffer.byteLength(body)});
    response.end(request.method === 'HEAD' ? '' : body);
    completed++;
  };
  if (path === '/slow/') setTimeout(send, 500); else send();
});
async function api(path, value) {
  const response = await fetch(new URL(path, backend), {method: value ? 'POST' : 'GET',
    headers: value ? {'Content-Type': 'application/json'} : {}, body: value ? JSON.stringify(value) : undefined,
    signal: AbortSignal.timeout(10000)});
  const body = await response.json();
  assert.equal(response.status, 200, JSON.stringify(body));
  assert.ok(!body.error, JSON.stringify(body));
  return body;
}
const state = () => api('/api/remote/state');
const command = value => api('/api/player/command', value);
async function until(predicate) {
  const deadline = Date.now() + 7000;
  while (Date.now() < deadline) { const value = await state(); if (predicate(value)) return value; await delay(40); }
  throw new Error('Remote browser did not reach the expected state');
}
const config = await api('/api/config');
assert.equal(config.clipboard_url_detection, true);
assert.equal(config.clipboard_link_behavior, 'verify_before_dialog');
assert.equal((await state()).visible, false, 'Preserve any existing user dialog');
const before = await api('/api/player/status');
assert.equal(before.playing, false, 'Run while playback is paused');
await new Promise(resolve => fixture.listen(0, '127.0.0.1', resolve));
const source = `http://127.0.0.1:${fixture.address().port}`;
try {
  const contract = await api('/api/preferences');
  assert.equal(contract.controls.find(control => control.key === 'clipboard_link_behavior').options.length, 3);
  await command({command: 'browse_remote', target: `${source}/unsupported`, clipboard: true});
  await delay(500); assert.ok(completed > 0); assert.equal((await state()).visible, false);
  await command({command: 'browse_remote', target: `${source}/file.txt`, clipboard: true});
  await delay(400); assert.equal((await state()).visible, false);
  await command({command: 'browse_remote', target: `${source}/directory/`, clipboard: true});
  const supported = await until(value => value.visible && value.listing?.entries.length === 1 && !value.loading);
  assert.equal(supported.listing.entries[0].name, 'clip.mp4');
  await command({command: 'browse_remote', target: `${source}/unsupported`, clipboard: true});
  await delay(400); assert.equal((await state()).target, supported.target);
  await command({command: 'close_remote_browser'}); await until(value => !value.visible);
  await command({command: 'browse_remote', target: `${source}/slow/`, clipboard: true});
  await delay(100); await command({command: 'close_remote_browser'}); await delay(1200);
  assert.equal((await state()).visible, false, 'Dismissed verification cannot reopen the dialog');
  const after = await api('/api/player/status');
  for (const key of ['playing', 'current_file', 'hardware_connected', 'estop_active']) assert.deepEqual(after[key], before[key]);
  assert.deepEqual(after.timeline?.cues, before.timeline?.cues);
  console.log('PASS live copied-link preference, supported/unsupported/non-media links, preserved dialog and dismissed verification');
  console.log('PASS unchanged playback, cue and hardware state');
} finally {
  await command({command: 'close_remote_browser'}).catch(() => {});
  fixture.close(); fixture.closeAllConnections();
}
