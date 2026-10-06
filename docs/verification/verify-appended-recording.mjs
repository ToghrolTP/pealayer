// Temporary catalog entry, VirtualBoard only; existing catalog/media are kept.
import net from 'node:net';
import assert from 'node:assert/strict';
const base = process.argv[2] || 'http://127.0.0.1:8080';
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
  for (let i = 0; i < 300; i++) { const value = await check(); if (value) return value; await new Promise(resolve => setTimeout(resolve, 50)); }
  throw Error('Timed out: ' + description);
}
const command = command => rpc('controller.command.execute', { command });
let id, initialConfig, initial, changed = false;
try {
  const board = await rpc('controller.snapshot');
  assert(board.connected && board.port?.product === 'PCController Virtual Board', 'Refusing physical board actuation');
  initial = await http('/api/player/status');
  assert(!initial.playing && !initial.effect_recording?.active && !(initial.cues || []).length, 'Requires paused media, no capture/cues');
  initialConfig = await http('/api/config');
  const catalog = JSON.parse((await command('effect list')).output);
  id = Array.from({ length: 256 }, (_, id) => id).find(id => !catalog.some(effect => effect.id === String(id)));
  assert(id !== undefined, 'No free temporary sequence ID');
  let effect = { id: String(id), reference: `effect:${id}`, name: `append-check-${Date.now()}`, icon: 'record', category: catalog[0]?.category || 'Effects', color: 'blue', kind: 'sequence', duration_ms: 1000, is_new: true,
    program: { steps: [{ at_us: 0, kind: 'pwm', target: 0, value: 32, duration_ms: 1000 }], properties: { mode: 'host', timing_tolerance_us: 1234, color: 'blue' } } };
  const start = async () => {
    changed = true;
    await pea('pealayer.controller_effect.record.start', { name: effect.name, category: effect.category, color: effect.color, mode: 'automatic', effect });
    return until(async () => { const state = await http('/api/player/status'); return state.effect_recording?.active && state.effect_recording.id === id && !state.effect_recording.pending && state; }, 'capture after publish acknowledgement');
  };
  const edges = async count => {
    await command('relay 8 on');
    await new Promise(resolve => setTimeout(resolve, 100));
    await command('relay 8 off');
    return until(async () => { const state = await http('/api/player/status'); return state.effect_recording?.preview.length === count && state; }, 'live applied relay edges');
  };
  const finish = async count => {
    await pea('pealayer.controller_effect.record.save');
    return until(async () => { const state = await http('/api/player/status'); const saved = state.controller_effects?.find(item => item.reference === effect.reference); return !state.effect_recording?.active && !state.effect_recording?.pending && saved?.program?.steps?.length === count && saved; }, 'same effect saved and refreshed');
  };
  await start();
  const live = await edges(3);
  assert.equal(live.effect_recording.preview[1].at_us, 1_000_000);
  const saved = await finish(3);
  assert.equal(saved.name, effect.name);
  assert.equal(saved.program.properties.mode, 'host');
  assert.equal(saved.program.properties.timing_tolerance_us, 1234);
  effect = { ...effect, is_new: false, program: saved.program };
  await start(); await edges(5);
  await pea('pealayer.controller_effect.record.discard');
  const discarded = await until(async () => { const state = await http('/api/player/status'); return !state.effect_recording?.active && !state.effect_recording?.pending && state.controller_effects?.find(item => item.reference === effect.reference); }, 'discard acknowledged');
  assert.deepEqual(discarded.program.steps, saved.program.steps);
  effect = { ...effect, program: { ...effect.program, steps: [] } };
  await start(); const replacement = await edges(2); await finish(2);
  assert.equal(replacement.effect_recording.preview[0].at_us, 0);
  assert.equal((await http('/api/player/status')).cues?.length || 0, initial.cues?.length || 0);
  console.log(JSON.stringify({ virtual_board: true, pealayer_publish_before_capture: true, same_id: id, append_end_us: 1000000, original_steps_kept: true, discard_preserves_sequence: true, cleared_sequence_replaced: true, no_extra_cue: true }, null, 2));
} finally {
  const failures = [];
  const clean = async fn => { try { await fn(); } catch (error) { failures.push(String(error)); } };
  if (changed) {
    await clean(async () => { const state = await http('/api/player/status'); if (state.effect_recording?.active) await pea('pealayer.controller_effect.record.discard'); });
    await clean(() => until(async () => !(await http('/api/player/status')).effect_recording?.pending, 'cleanup ready'));
    await clean(() => command('relay 8 off'));
    await clean(() => command(`effect delete effect:${id}`));
    if (initialConfig) await clean(() => http('/api/config', { effect_working_draft: initialConfig.effect_working_draft ?? null }));
    await clean(() => until(async () => !(await http('/api/player/status')).controller_effects?.some(item => item.reference === `effect:${id}`), 'temporary effect removed'));
  }
  socket.end();
  if (failures.length) throw Error('Cleanup needs attention: ' + failures.join('; '));
}
