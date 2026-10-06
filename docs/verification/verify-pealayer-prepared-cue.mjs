// Paired application acceptance, VirtualBoard only. Requires an existing
// PCController-owned R8 one-second effect. Only a temporary cue is added/removed;
// the effect library is never changed and paused media/rate are restored.
import net from 'node:net';
import assert from 'node:assert/strict';
const base = process.argv[2] || 'http://127.0.0.1:8080';
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const socket = net.connect(Number(process.argv[3] || 8787), '127.0.0.1');
await new Promise((resolve, reject) => { socket.once('connect', resolve); socket.once('error', reject); });
let serial = 0, input = '';
const pending = new Map();
socket.on('data', data => {
  input += data;
  let newline;
  while ((newline = input.indexOf('\n')) >= 0) {
    const value = JSON.parse(input.slice(0, newline)); input = input.slice(newline + 1);
    const request = pending.get(value.id);
    if (request) { pending.delete(value.id); clearTimeout(request.timer); value.error ? request.reject(Error(JSON.stringify(value.error))) : request.resolve(value.result); }
  }
});
const rpc = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++serial;
  const timer = setTimeout(() => { pending.delete(id); reject(Error(method + ' timeout')); }, 5000);
  pending.set(id, { resolve, reject, timer });
  socket.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
});
async function http(path, body) {
  const response = await fetch(base + path, { signal: AbortSignal.timeout(5000), ...(body ? { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) } : {}) });
  assert(response.ok, path + ': ' + response.status); return response.json();
}
async function pea(method, params = {}) {
  const result = await http('/api/rpc', { jsonrpc: '2.0', id: ++serial, method, params });
  if (result.error) throw Error(JSON.stringify(result.error)); return result.result;
}
async function until(check, description) {
  for (let i = 0; i < 600; i++) { const value = await check(); if (value) return value; await delay(50); }
  throw Error('Timed out: ' + description);
}
const command = command => rpc('controller.command.execute', { command });
let initial, instance;
try {
  initial = await http('/api/player/status');
  const board = await rpc('controller.snapshot');
  assert(board.connected && board.port?.product === 'PCController Virtual Board', 'Refusing physical board actuation');
  assert(!initial.playing && initial.current_video && initial.duration > 0, 'Load and pause media first');
  assert(!(initial.cues || []).length && !initial.hardware_sync, 'Refusing to replace an existing hardware timeline');
  const catalog = JSON.parse((await command('effect list')).output);
  const effect = catalog.find(effect => effect.kind === 'sequence' && effect.engine !== 'mcu'
    && effect.steps?.length === 2 && effect.steps.every(step => step.kind === 'relay' && step.target === 7)
    && !effect.steps[0].at_us && effect.steps[0].value === 1
    && effect.steps[1].at_us === 1_000_000 && !effect.steps[1].value);
  assert(effect, 'Create a PCController-owned R8 ON / one second / OFF effect before this check');
  const reference = effect.reference;
  await until(async () => (await http('/api/player/status')).controller_effects?.some(effect => effect.reference === reference), 'effect discovery');
  const start = Math.round(initial.playback_time * 1000) + 2000;
  await pea('pealayer.controller_effect_cue.add', { reference, start_time_ms: start });
  instance = await until(async () => (await http('/api/player/status')).cues?.[0]?.id, 'cue creation');
  const armed = await until(async () => { const s = await http('/api/player/status'); const t = s.hardware_sync; return t && t.revision === t.prepared_revision && t.clock_ack_revision === t.revision && t.timeline?.state === 'paused' && !t.error && t; }, 'prepared and paused clock ACK');
  assert.equal(armed.timeline.step_count, 2);
  await pea('pealayer.play');
  const complete = await until(async () => {
    const s = await http('/api/player/status'); const t = s.hardware_sync;
    assert(!t?.error, t?.error || 'timing fault');
    return t?.timeline?.acknowledged === 2 && t;
  }, 'both native cue edges acknowledged');
  console.log(JSON.stringify({ virtual_board: true, actual_pealayer_cue: true, prepared_before_play: true,
    revision: complete.revision, hash: complete.timeline.hash, acknowledged: complete.timeline.acknowledged,
    max_host_clock_ack_lateness_ms: complete.timeline.max_ack_lateness_ms }, null, 2));
} finally {
  const failures = [];
  const clean = async fn => { try { await fn(); } catch (error) { failures.push(String(error)); } };
  if (initial) {
    await clean(() => pea('pealayer.pause'));
    if (!instance) await clean(async () => { instance = (await http('/api/player/status')).cues?.[0]?.id; });
    if (instance) await clean(() => pea('pealayer.effect_cue.remove', { instance_id: instance }));
    await clean(() => pea('pealayer.rate.set', { rate: initial.playback_rate || 1 }));
    await clean(() => pea('pealayer.seek_to', { seconds: initial.playback_time }));
    await clean(() => until(async () => { const s = await http('/api/player/status'); return !s.playing && !(s.cues || []).length && Math.abs(s.playback_time - initial.playback_time) < .15; }, 'original paused media restored'));
  }
  socket.end();
  if (failures.length) throw Error('Cleanup needs attention: ' + failures.join('; '));
}
