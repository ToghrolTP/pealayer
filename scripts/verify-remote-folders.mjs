// Live app verification. Private test URLs are arguments only, never fixtures or output.
import assert from 'node:assert/strict';
import http from 'node:http';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2);
const base = process.env.PEALAYER_TEST_BASE || 'http://127.0.0.1:8080';
const exe = process.env.PEALAYER_TEST_EXE;
const urls = args.filter(arg => /^https?:\/\//.test(arg));
const hold = args.includes('--hold');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function api(path, body) {
  const response = await fetch(`${base}${path}`, body ? { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) } : {});
  const json = await response.json(); assert.ok(response.ok, `API failed: ${response.status} ${json.error || ''}`); return json;
}
const command = body => api('/api/ipc', body);
const rpc = (method, params) => api('/api/rpc', { jsonrpc: '2.0', id: 'remote-test', method, params });
async function until(read, predicate, timeout = 35000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) { const value = await read(); if (predicate(value)) return value; await delay(100); }
  throw Error('Timed out waiting for the expected app state');
}
async function browse(target, transport = 'rpc') {
  const before = await api('/api/remote/state');
  if (transport === 'rpc') await rpc('pealayer.remote.browse', { target, use_proxy: false });
  else await command({ command: 'browse_remote', target, use_proxy: false });
  const state = await until(() => api('/api/remote/state'), s => s.request_id > before.request_id && s.target === target && !s.loading);
  assert.equal(state.use_proxy, false); return state;
}
async function thumbnail(url) {
  const deadline = Date.now() + 90000;
  while (Date.now() < deadline) {
    const response = await fetch(`${base}/api/remote/thumbnail?url=${encodeURIComponent(url)}`);
    if (response.status === 202) { await delay(500); continue; }
    if (!response.ok) throw Error((await response.json()).error || 'Thumbnail failed');
    const data = new Uint8Array(await response.arrayBuffer());
    assert.ok(data.length > 500); assert.equal(data[0], 0xff); assert.equal(data[1], 0xd8); return;
  }
  throw Error('Thumbnail deadline exceeded');
}

const config = await api('/api/config');
const original = await api('/api/player/status');
const temporary = await mkdtemp(join(tmpdir(), 'pealayer-folder-verify-'));
let server;
let finishHold;
try {
  const clip = join(temporary, 'test.mp4');
  const generated = spawnSync('ffmpeg', ['-hide_banner','-loglevel','error','-f','lavfi','-i','testsrc2=size=160x90:rate=12','-t','3','-an','-c:v','libx264','-preset','ultrafast','-pix_fmt','yuv420p','-movflags','+faststart','-y',clip], { windowsHide: true });
  assert.equal(generated.status, 0, 'FFmpeg fixture generation failed');
  const media = await readFile(clip);
  const names = ['Clip.S01E10.mp4','Clip.S01E02.mp4','Clip.S01E01.mp4','فارسی 03 +%20.mp4'];
  const table = `<title>Index of /folder/</title><table summary="Directory Listing"><tr><td></td><td><a href="sub/">sub/</a></td><td>-</td><td>-</td></tr>${names.map((name, i) => `<tr><td>V</td><td><a href="${encodeURIComponent(name)}">${name}</a></td><td>2025-02-${String(i + 1).padStart(2,'0')} 12:00</td><td>${media.length}</td></tr>`).join('')}</table>`;
  let requests = 0; let correctAgent = false;
  server = http.createServer((request, response) => {
    if (request.url === '/__finish' && request.method === 'POST') {
      response.writeHead(200); response.end('Finishing verification'); finishHold?.(); return;
    }
    requests++; correctAgent ||= /^Pealayer\//.test(request.headers['user-agent'] || '');
    const target = new URL(request.url, 'http://fixture.invalid');
    const path = decodeURIComponent(target.pathname);
    if (path === '/folder/' || path === '/folder/sub/') {
      const html = path.endsWith('sub/') ? '<h1>Index of /folder/sub/</h1>' : table;
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'content-length': Buffer.byteLength(html), server: 'Pealayer verification fixture' });
      response.end(request.method === 'HEAD' ? undefined : html); return;
    }
    if (path === '/login') { response.writeHead(200, { 'content-type':'text/html' }); response.end('<title>Login</title><a href="/account">Account</a>'); return; }
    if (path.startsWith('/folder/') && names.includes(path.slice('/folder/'.length))) {
      let start = 0, end = media.length - 1; const range = /bytes=(\d+)-(\d*)/.exec(request.headers.range || '');
      if (range) { start = Number(range[1]); if (range[2]) end = Math.min(end,Number(range[2])); }
      if (start > end) { response.writeHead(416); response.end(); return; }
      response.writeHead(range ? 206 : 200, { 'content-type':'video/mp4', 'content-length':end-start+1, 'accept-ranges':'bytes', 'last-modified':'Mon, 03 Feb 2025 12:00:00 GMT', ...(range ? {'content-range':`bytes ${start}-${end}/${media.length}`} : {}) });
      response.end(request.method === 'HEAD' ? undefined : media.subarray(start,end+1)); return;
    }
    response.writeHead(404); response.end();
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const folder = `http://127.0.0.1:${server.address().port}/folder/`;
  await command({ command:'update_config', values:{ remote_folder_auto_next:false, remote_folder_thumbnails:true } });
  let state = await browse(folder, 'ipc'); assert.equal(state.error,null); assert.equal(state.listing.entries.length,5);
  assert.equal(state.listing.entries[0].is_dir,true);
  assert.deepEqual(state.listing.entries.slice(1,4).map(e=>e.name), ['Clip.S01E01.mp4','Clip.S01E02.mp4','Clip.S01E10.mp4']);
  assert.equal(state.auto_next,false); assert.ok(correctAgent);
  await rpc('pealayer.remote.sort', { by:'date', descending:true });
  state = await until(() => api('/api/remote/state'), s => s.sort === 'date' && s.descending); assert.equal(state.listing.entries[1].name,names[3]);
  await rpc('pealayer.remote.sort', { by:'name', descending:false });
  await until(() => api('/api/remote/state'), s => s.sort === 'name' && !s.descending);
  const file1 = `${folder}${names[2]}`, file2 = `${folder}${names[1]}`;
  await browse(file1); state = await api('/api/remote/state'); assert.equal(state.listing.file.url,file1); assert.equal(state.listing.file.size_bytes,media.length); assert.equal(state.listing.parent_url,folder);
  await thumbnail(file1);
  await thumbnail(`${folder}${encodeURIComponent(names[3])}`);
  state = await browse(`${folder}sub/`); assert.equal(state.listing.entries.length,0); assert.equal(state.listing.parent_url,folder);
  state = await browse(`${folder.replace(/folder\/$/,'')}login`); assert.ok(state.error); assert.equal(state.listing,null);
  await browse(folder);
  await rpc('pealayer.remote.select',{target:file1,play:true});
  await until(()=>api('/api/player/status'),s=>s.current_video===file1 && s.duration>0);
  await command({command:'next'}); await until(()=>api('/api/player/status'),s=>s.current_video===file2);
  await command({command:'previous'}); await until(()=>api('/api/player/status'),s=>s.current_video===file1);
  await command({command:'update_config',values:{remote_folder_auto_next:true}});
  await command({command:'seek_to',seconds:2.8});
  await until(()=>api('/api/player/status'),s=>s.current_video===file2,12000);
  await command({command:'update_config',values:{remote_folder_auto_next:false}});
  await command({command:'pause'});
  if (exe) { const result=spawnSync(exe,['--browse',folder,'--no-proxy'],{windowsHide:true,timeout:10000}); assert.equal(result.status,0); await until(()=>api('/api/remote/state'),s=>s.visible&&s.target===folder&&!s.loading); }
  console.log(JSON.stringify({fixture:'passed',http_requests:requests,user_agent:correctAgent,cli:exe?'passed':'not requested',checks:['table metadata','Unicode normalization','sorting','parent navigation','unsupported HTML','thumbnail JPEG','manual previous/next','EOF auto-next']},null,2));
  const internal = [];
  for (let index=0;index<urls.length;index++) {
    state=await browse(urls[index]); assert.equal(state.error,null); assert.ok(state.listing.entries.length>0);
    const files=state.listing.entries.filter(e=>e.playable);
    assert.ok(files.every(e=>e.modified&&e.size_bytes>0));
    internal.push(files.map(e=>e.name));
    console.log(JSON.stringify({test:index+1,files:files.length,metadata:true,proxy:false}));
  }
  if(internal.length===3) assert.deepEqual(internal[1],internal[2],'Query and path listings must discover the same files in the same order');
  if(hold) {
    await browse(folder); await thumbnail(file1);
    console.log(`Ready for browser verification. Send input or POST http://127.0.0.1:${server.address().port}/__finish to restore playback/settings.`);
    await new Promise(resolve => {
      const timeout = setTimeout(done, 15 * 60 * 1000);
      function done() {
        clearTimeout(timeout); process.stdin.off('data',done);
        process.stdin.pause(); process.stdin.unref?.(); resolve();
      }
      finishHold = done; process.stdin.once('data',done);
    });
  }
} finally {
  await command({command:'close_remote_browser'}).catch(()=>{});
  await command({command:'update_config',values:{remote_folder_auto_next:config.remote_folder_auto_next,remote_folder_thumbnails:config.remote_folder_thumbnails}}).catch(()=>{});
  if(original.current_video) {
    await command({command:'open',target:original.current_video});
    await until(()=>api('/api/player/status'),s=>s.current_video===original.current_video&&s.duration>0&&(!original.seekable||s.seekable)).catch(()=>{});
    await command({command:'pause'});
    if(original.seekable) await command({command:'seek_to',seconds:original.playback_time||0});
    await command({command:original.playing?'play':'pause'});
  } else await command({command:'stop'}).catch(()=>{});
  await command({command:'update_config', values:{recent_media:config.recent_media,playback_positions:config.playback_positions,last_media_target:config.last_media_target,last_media_paused:config.last_media_paused}}).catch(()=>{});
  if(server) await new Promise(resolve=>server.close(resolve));
  await rm(temporary,{recursive:true,force:true});
}
