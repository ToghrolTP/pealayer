import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function waitFor(read) {
  for (let attempt = 0; attempt < 120; attempt++) {
    const value = await read();
    if (value) return value;
    await sleep(100);
  }
  throw Error('Timed out waiting for the recording retention fixture');
}

const profile = await mkdtemp(path.join(tmpdir(), 'pealayer-recording-retention-'));
const edge = process.env.EDGE_EXECUTABLE || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const child = spawn(edge, ['--headless=new', '--no-first-run', '--disable-extensions',
  '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'],
{ windowsHide: true, stdio: 'ignore' });
let socket;
try {
  const port = await waitFor(async () => {
    try {
      return Number((await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]);
    } catch {
      return undefined;
    }
  });
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  const pending = new Map();
  const errors = [];
  socket.addEventListener('message', ({ data }) => {
    const event = JSON.parse(data);
    if (event.method === 'Runtime.exceptionThrown') errors.push(event.params.exceptionDetails.text);
    const request = pending.get(event.id);
    if (!request) return;
    pending.delete(event.id);
    if (event.error) request.reject(Error(event.error.message));
    else request.resolve(event.result);
  });
  let id = 0;
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    pending.set(++id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async expression => {
    const result = await send('Runtime.evaluate', { expression, returnByValue: true });
    if (result.exceptionDetails) throw Error(result.exceptionDetails.text);
    return result.result.value;
  };
  await send('Page.enable');
  await send('Runtime.enable');
  for (const [query, width, expected] of [
    ['', 1180, 'Earlier actions may be missing.'],
    ['fa', 430, 'ممکن است برخی فرمان‌های قبلی در ضبط نباشند.'],
    ['within-capacity', 1180, ''],
    ['host', 1180, ''],
    ['inactive', 1180, ''],
  ]) {
    const baseUrl = process.env.PEALAYER_FIXTURE_URL || 'http://127.0.0.1:18289';
    await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
    await send('Page.navigate', { url: `${baseUrl}/scripts/recording-retention-fixture.html?${query}` });
    await waitFor(() => evaluate(`location.search.slice(1) === ${JSON.stringify(query)} && !!document.querySelector('.effect-capture-controls')`));
    const warnings = await evaluate(`Array.from(document.querySelectorAll('.effect-capture-controls .ant-alert-warning'), element => ({
      text: element.textContent, left: element.getBoundingClientRect().left, right: element.getBoundingClientRect().right,
      icon: !!element.querySelector('svg')
    }))`);
    assert.equal(warnings.length, expected ? 1 : 0, query);
    if (!expected) continue;
    assert.ok(warnings[0].text.endsWith(expected), query);
    assert.ok(warnings[0].icon && warnings[0].left >= 0 && warnings[0].right <= width, query);
    const directory = process.env.PEALAYER_VERIFICATION_DIRECTORY;
    if (directory) {
      await evaluate(`document.querySelector('.effect-capture-controls .ant-alert-warning').scrollIntoView({ block: 'center' })`);
      await sleep(250);
      assert.ok(await evaluate(`(() => {
        const warning = document.querySelector('.effect-capture-controls .ant-alert-warning').getBoundingClientRect();
        const body = document.querySelector('.effect-editor-modal .ant-modal-body').getBoundingClientRect();
        return warning.top >= body.top && warning.bottom <= body.bottom;
      })()`), query);
      const clip = await evaluate(`(() => {
        const rect = document.querySelector('.effect-editor-modal').getBoundingClientRect();
        return { x: Math.max(0, Math.floor(rect.left - 8)), y: Math.max(0, Math.floor(rect.top - 8)),
          width: Math.min(innerWidth, Math.ceil(rect.width + 16)), height: Math.min(innerHeight - rect.top, Math.ceil(rect.height + 16)), scale: 1 };
      })()`);
      const { data } = await send('Page.captureScreenshot', { format: 'png', clip });
      await writeFile(path.join(directory, `recording-retention-${query || 'en'}.png`), Buffer.from(data, 'base64'));
    }
  }
  assert.deepEqual(errors, []);
  console.log('PASS: the Effects Designer shows the localized, in-bounds warning only for active board capture after overflow.');
  await send('Browser.close').catch(() => {});
} finally {
  socket?.close();
  if (child.exitCode === null) {
    await Promise.race([new Promise(resolve => child.once('exit', resolve)), sleep(2000)]);
    if (child.exitCode === null) child.kill();
  }
  assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir()) + path.sep));
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
