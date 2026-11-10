// Installed Edge, private temporary profile, memory-only fixture command sink.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp, readFile, rm, writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
const profile = await mkdtemp(path.join(tmpdir(), 'pealayer-remote-browser-'));
const child = spawn(process.env.EDGE_EXECUTABLE || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', ['--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], {windowsHide: true, stdio: 'ignore'});
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let socket;
try {
  let port;
  for (let n=0; n<150; n++) { try { port=Number((await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); break; } catch { await sleep(100); } }
  assert.ok(port);
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, {method:'PUT'})).json();
  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve,reject) => { socket.addEventListener('open',resolve,{once:true}); socket.addEventListener('error',reject,{once:true}); });
  let seq=0; const pending=new Map(), errors=[];
  socket.addEventListener('message',event => { const v=JSON.parse(event.data); if(v.id) { const p=pending.get(v.id); pending.delete(v.id); if(v.error)p?.reject(Error(v.error.message)); else p?.resolve(v.result); } if(v.method==='Runtime.exceptionThrown')errors.push(v.params.exceptionDetails.text); });
  const send=(method,params={}) => new Promise((resolve,reject) => { const id=++seq; pending.set(id,{resolve,reject}); socket.send(JSON.stringify({id,method,params})); });
  const evaluate=async expression => { const v=await send('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true}); if(v.exceptionDetails)throw Error(v.exceptionDetails.text); return v.result.value; };
  await send('Page.enable'); await send('Runtime.enable');
  for (const light of [false,true]) for (const width of [390,768,1440]) for (const parent of [false,true]) {
    await send('Emulation.setDeviceMetricsOverride',{width,height:850,deviceScaleFactor:1,mobile:false});
    await send('Page.navigate',{url:`http://127.0.0.1:18191/scripts/remote-navigation-fixture.html?${light?'light&':''}${parent?'parent':''}`});
    let ready=false; for(let n=0;n<100;n++){ if(await evaluate(`Boolean(document.querySelector('.remote-location-modal') && document.querySelector('#fixture-state'))`)){ready=true;break;} await sleep(100); } assert.ok(ready);
    await sleep(300);
    assert.deepEqual(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).commands`),[]);
    assert.ok(await evaluate(`document.documentElement.scrollWidth<=innerWidth+1`),`document overflow at ${width}`);
    const bounds=await evaluate(`(()=>{const r=document.querySelector('.ant-modal').getBoundingClientRect();return {left:r.left,right:r.right};})()`);
    assert.ok(bounds.left>=0 && bounds.right<=width+1);
    assert.equal(await evaluate(`document.querySelectorAll('tr[data-row-key="https://files.invalid/"]').length`), parent?1:0);
    assert.equal(await evaluate(`Array.from(document.querySelectorAll('button')).some(b=>b.textContent.includes('Parent folder'))`), parent);
    assert.equal(await evaluate(`/Date unavailable|Size unavailable|Back ·/.test(document.querySelector('.ant-modal').textContent)`),false);
    if(parent){
      assert.deepEqual(await evaluate(`Array.from(document.querySelector('tr[data-row-key="https://files.invalid/"]').cells).slice(1,3).map(c=>c.textContent)`),['','']);
      if(process.env.PEALAYER_VERIFICATION_DIRECTORY){ const shot=await send('Page.captureScreenshot',{format:'png'}); await writeFile(path.join(process.env.PEALAYER_VERIFICATION_DIRECTORY,`remote-parent-${width}${light?'-light':''}.png`),Buffer.from(shot.data,'base64')); }
      await evaluate(`document.querySelector('tr[data-row-key="https://files.invalid/"]').click()`);
    }else{
      await evaluate(`document.querySelector('tr[data-row-key="https://files.invalid/folder/child/"]').click()`);
    }
    await sleep(30);
    const commands=await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).commands`);
    assert.equal(commands.length,1); assert.equal(commands[0][0],'pealayer.remote.browse');
    assert.equal(commands[0][1].target,parent?'https://files.invalid/':'https://files.invalid/folder/child/');
  }
  assert.deepEqual(errors,[]);
  console.log('Remote navigation: verified-parent visibility, metadata, single-click navigation, phone/tablet/desktop and light/dark checks passed');
  await send('Browser.close').catch(()=>{});
} finally {
  socket?.close();
  if(child.exitCode===null){await Promise.race([new Promise(resolve=>child.once('exit',resolve)),sleep(2000)]);if(child.exitCode===null)child.kill();}
  assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir())+path.sep));
  await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});
}
