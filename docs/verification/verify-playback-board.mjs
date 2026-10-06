// Live, no-output-actuation acceptance check. Requires loaded media, no cues,
// and an explicitly attached updated VirtualBoard. Restores playback in finally.
import net from 'node:net';
import assert from 'node:assert/strict';
const base = process.argv[2] || 'http://127.0.0.1:8080';
const controllerPort = Number(process.argv[3] || 8787);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
let nextID = 0;
const socket = net.connect(controllerPort, '127.0.0.1');
await new Promise((resolve, reject) => { socket.once('connect', resolve); socket.once('error', reject); });
let input = '';
const waiting = new Map();
socket.on('data', data => {
  input += data;
  let newline;
  while ((newline = input.indexOf('\n')) >= 0) {
    const value = JSON.parse(input.slice(0, newline)); input = input.slice(newline + 1);
    const request = waiting.get(value.id);
    if (request) { waiting.delete(value.id); clearTimeout(request.timer); value.error ? request.reject(Error(JSON.stringify(value.error))) : request.resolve(value.result); }
  }
});
const rpc = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++nextID;
  const timer = setTimeout(() => { waiting.delete(id); reject(Error(`${method} timeout`)); }, 5000);
  waiting.set(id, { resolve, reject, timer });
  socket.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
});
async function http(path, value) {
  const response = await fetch(base + path, { signal: AbortSignal.timeout(5000), ...(value ? { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(value) } : {}) });
  assert(response.ok, `${path}: ${response.status}`);
  return response.json();
}
const command = (command, params = {}) => http('/api/ipc', { command, ...params });
async function until(check) {
  for (let i = 0; i < 60; i++) { const value = await rpc('controller.media.playback.get'); if (await check(value)) return value; await delay(100); }
  throw Error('media clock did not reach expected state');
}
const initial = await http('/api/player/status');
const board = await rpc('controller.snapshot');
assert(board.port?.product === 'PCController Virtual Board', 'Refusing live acceptance gestures on a physical board');
assert(initial.duration > 0 && initial.current_video, 'Load media before running this check');
assert((initial.cues || []).length === 0, 'Remove/disable cues before testing playback gestures');
const events = [];
const ws = new WebSocket(`ws://127.0.0.1:${controllerPort}/ipc`);
await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, { once: true }); ws.addEventListener('error', reject, { once: true }); });
ws.addEventListener('message', message => {
  const value = JSON.parse(message.data);
  function visit(v) { if (!v || typeof v !== 'object') return; if (v.kind === 'media.playback') events.push(v); for (const child of Object.values(v)) { if (typeof child === 'object') visit(child); } }
  visit(value);
});
ws.send(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'controller.subscribe', params: { topics: ['events'], interval_ms: 100, after_id: 0 } }));
const evidence = { generated_at: new Date().toISOString(), virtual_board: true, checks: [] };
try {
  await command('pause'); await command('set_rate', { rate: 1 }); await command('seek_to', { seconds: 65 });
  const paused = await until(v => !v.playing && Math.abs(v.position_ms - 65000) < 150 && v.board_synced && v.board_sequence === v.sequence);
  const panel = await rpc('controller.front_panel');
  assert.deepEqual(panel.raw_segments, [0x3f, 0x86, 0x3f, 0x6d]);
  assert.equal(paused.duration_ms, Math.round(initial.duration * 1000));
  evidence.checks.push({ name: 'paused decoded time / duration / 01:05 display', pass: true, position_ms: paused.position_ms, duration_ms: paused.duration_ms, raw_segments: panel.raw_segments });
  await command('play'); const playing = await until(v => v.playing && v.board_synced);
  await delay(1400); const advancing = await rpc('controller.media.playback.get');
  assert(advancing.position_ms - playing.position_ms > 1100);
  assert(advancing.position_ms - playing.position_ms < 1800);
  assert.equal((await rpc('controller.program_state.get')).mode, 'Running');
  evidence.checks.push({ name: 'play and program-state advancement', pass: true, advance_ms: advancing.position_ms - playing.position_ms, board_round_trip_ms: advancing.board_round_trip_ms });
  await command('pause'); const frozen = await until(v => !v.playing && v.board_synced);
  await delay(1200); const still = await rpc('controller.media.playback.get');
  assert(Math.abs(still.position_ms - frozen.position_ms) < 100);
  assert.equal((await rpc('controller.program_state.get')).mode, 'Idle');
  evidence.checks.push({ name: 'pause freezes display and releases program claim', pass: true, delta_ms: still.position_ms - frozen.position_ms });
  await command('seek_to', { seconds: 6000 });
  await until(v => !v.playing && Math.abs(v.position_ms - 6000000) < 150 && v.board_synced && v.board_sequence === v.sequence);
  assert.deepEqual((await rpc('controller.front_panel')).raw_segments, [0x3f, 0x86, 0x66, 0x3f]);
  evidence.checks.push({ name: 'seek switches four digits to 01:40 hours/minutes', pass: true });
  await command('seek_to', { seconds: 65 }); await command('set_rate', { rate: 2 }); await command('play');
  const fast = await until(v => v.playing && v.rate === 2 && v.board_synced); await delay(1000);
  const fastEnd = await rpc('controller.media.playback.get');
  assert(fastEnd.position_ms - fast.position_ms > 1650);
  assert(fastEnd.position_ms - fast.position_ms < 2700);
  evidence.checks.push({ name: 'actual 2x MPV rate', pass: true, advance_ms: fastEnd.position_ms - fast.position_ms });
  const identities = (await rpc('controller.app.instances')).filter(v => v.id === fastEnd.client_id);
  assert.equal(identities.length, 1); assert.equal(identities[0].values.application, 'Pealayer');
  assert(identities[0].values.app_actions.includes('pealayer.pause'));
  assert(events.length > 10); assert(events.some(v => v.metadata?.playing === 'true')); assert(events.some(v => v.metadata?.playing === 'false'));
  evidence.checks.push({ name: 'registered identity and subscribed playback events', pass: true, identity: { id: identities[0].id, values: identities[0].values }, event_count: events.length });
} finally {
  await command('pause'); await command('set_rate', { rate: initial.playback_rate || 1 });
  await command('seek_to', { seconds: initial.playback_time });
  if (initial.playing) await command('play');
  ws.close(); socket.end();
}
console.log(JSON.stringify(evidence, null, 2));
