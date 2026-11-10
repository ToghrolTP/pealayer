// Real browser fixture; no production configuration or clipboard is accessed.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
const profile = await mkdtemp(path.join(tmpdir(), 'pealayer-shortcut-input-'));
const child = spawn(process.env.EDGE_EXECUTABLE || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  ['--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'],
  { windowsHide: true, stdio: 'ignore' });
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let socket;
try {
  let port;
  for (let n = 0; n < 150; n++) { try { port = Number((await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); break; } catch { await sleep(100); } }
  assert.ok(port);
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise(resolve => socket.addEventListener('open', resolve, { once: true }));
  let sequence = 0;
  const pending = new Map(), errors = [];
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    if (message.id) { const request = pending.get(message.id); pending.delete(message.id); message.error ? request?.reject(Error(message.error.message)) : request?.resolve(message.result); }
    if (message.method === 'Runtime.exceptionThrown') errors.push(message.params.exceptionDetails.text);
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => { const id = ++sequence; pending.set(id, { resolve, reject }); socket.send(JSON.stringify({ id, method, params })); });
  const evaluate = async expression => {
    const reply = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (reply.exceptionDetails) throw Error(reply.exceptionDetails.text);
    return reply.result.value;
  };
  await send('Page.enable'); await send('Runtime.enable');
  for (const width of [390, 768, 1440]) for (const light of [false, true]) for (const rtl of [false, true]) for (const neutral of [false, true]) {
    await send('Emulation.setDeviceMetricsOverride', { width, height: 700, deviceScaleFactor: 1.25, mobile: false });
    await send('Page.navigate', { url: `http://127.0.0.1:18191/scripts/shortcut-input-fixture.html?${light ? 'light&' : ''}${rtl ? 'rtl&' : ''}${neutral ? 'neutral' : ''}` });
    let ready = false;
    for (let n = 0; n < 100; n++) { if (await evaluate(`Boolean(document.querySelector('.shortcut-input input'))`)) { ready = true; break; } await sleep(50); }
    assert.ok(ready);
    const resting = await evaluate(`(()=>{const input=document.querySelector('.shortcut-input input'),r=input.getBoundingClientRect(),caps=[...document.querySelectorAll('kbd')];return {value:input.value,labels:caps.map(k=>k.textContent),center:r.y+r.height/2,ys:caps.map(k=>{const b=k.getBoundingClientRect();return b.y+b.height/2}),left:r.left,right:r.right,width:r.width,height:r.height,color:getComputedStyle(input).color};})()`);
    assert.equal(resting.value, 'Ctrl+Shift+F10'); assert.deepEqual(resting.labels, ['Ctrl', 'Shift', 'F10']);
    assert.ok(resting.ys.every(y => Math.abs(y - resting.center) < .5)); assert.ok(resting.left >= 0 && resting.right <= width);
    await evaluate(`document.querySelector('.shortcut-input input').focus()`);
    assert.equal(await evaluate(`getComputedStyle(document.querySelector('.shortcut-input__caps')).visibility`), 'hidden');
    await evaluate(`document.querySelector('#outside').focus()`);
    assert.equal(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).commits`), 0);
    await evaluate(`document.querySelector('.shortcut-input input').focus();document.querySelector('.shortcut-input input').select()`);
    await send('Input.insertText', { text: 'Alt+F12' });
    await evaluate(`document.querySelector('#outside').focus()`);
    const result = await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent)`);
    assert.deepEqual(result, { value: 'Alt+F12', commits: 1 });
    assert.deepEqual(await evaluate(`Array.from(document.querySelectorAll('kbd')).map(k=>k.textContent)`), ['Alt', 'F12']);
    const final = await evaluate(`(()=>{const r=document.querySelector('.shortcut-input input').getBoundingClientRect();return {width:r.width,height:r.height};})()`);
    assert.equal(final.width, resting.width); assert.equal(final.height, resting.height);
  }
  assert.deepEqual(errors, []);
  console.log('Shortcut keycaps: 24 palette/theme/RTL/viewport cases; centered caps, focus editing, exact commit and stable field geometry passed');
  await send('Browser.close').catch(() => {});
} finally {
  socket?.close();
  if (child.exitCode === null) { await Promise.race([new Promise(resolve => child.once('exit', resolve)), sleep(2000)]); if (child.exitCode === null) child.kill(); }
  assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir()) + path.sep));
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
