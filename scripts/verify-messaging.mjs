// Run against a running canonical instance; never operates hardware outputs.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

const origin = process.env.PEALAYER_VERIFY_ORIGIN || 'http://127.0.0.1:8080';
const exe = process.env.PEALAYER_VERIFY_EXE || join(process.env.LOCALAPPDATA, 'Programs', 'Pealayer', 'bin', 'pealayer.exe');
const id = `verify.${Date.now()}`;
const rpc = (method, params = {}) => ({ jsonrpc: '2.0', id: 1, method, params });
const read = async () => (await fetch(`${origin}/api/messages`)).json();
async function waitFor(predicate) {
  for (let i = 0; i < 50; i++) {
    const state = await read();
    if (predicate(state)) return state;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error('Message snapshot did not reach expected state');
}
function native(payload) {
  const result = spawnSync(exe, ['--remote', JSON.stringify(payload)], { encoding: 'utf8', windowsHide: true, timeout: 10000 });
  assert.equal(result.status, 0, result.stderr || String(result.error));
  return JSON.parse(result.stdout.trim());
}
const socket = new WebSocket(origin.replace(/^http/, 'ws') + '/ws');
await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
const received = [];
socket.addEventListener('message', event => received.push(JSON.parse(String(event.data))));
try {
  native({ command: 'publish_toast', id, message: 'IPC verification', severity: 'success', timeout_ms: 0 });
  let state = await waitFor(s => s.toasts.some(t => t.id === id));
  assert.equal(state.toasts.find(t => t.id === id).message, 'IPC verification');
  const ipcState = native(rpc('pealayer.messages.state'));
  assert(ipcState.result.messages.toasts.some(t => t.id === id));
  let response = await fetch(`${origin}/api/messages`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ id, message: 'HTTP update', timeout_ms: 0 }) });
  assert(response.ok);
  state = await waitFor(s => s.toasts.some(t => t.id === id && t.message === 'HTTP update'));
  assert.equal(state.toasts.filter(t => t.id === id).length, 1);
  socket.send(JSON.stringify(rpc('pealayer.toast.show', { id, message: 'WebSocket update', timeout_ms: 0 })));
  await waitFor(s => s.toasts.some(t => t.id === id && t.message === 'WebSocket update'));
  assert(received.some(frame => (frame.state || frame).messages?.toasts.some(t => t.id === id)));
  response = await fetch(`${origin}/api/rpc`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(rpc('pealayer.toast.dismiss', { id })) });
  assert(response.ok);
  await waitFor(s => !s.toasts.some(t => t.id === id));
  socket.send(JSON.stringify({ command: 'publish_toast', message: '', timeout_ms: 1 }));
  await new Promise(resolve => setTimeout(resolve, 200));
  assert(received.some(frame => frame.error?.code === -32602));
  response = await fetch(`${origin}/api/messages`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ message: '', timeout_ms: 1 }) });
  assert.equal(response.status, 400);
  native({ command: 'publish_toast', id, message: 'Expiry verification', timeout_ms: 500 });
  await waitFor(s => s.toasts.some(t => t.id === id));
  await waitFor(s => !s.toasts.some(t => t.id === id));
  console.log('Live messaging passed: native IPC publish/read, HTTP upsert, WS publish/subscription, RPC dismissal, HTTP/WS rejection, expiry.');
} finally {
  socket.close();
  native({ command: 'dismiss_toast', id });
}
