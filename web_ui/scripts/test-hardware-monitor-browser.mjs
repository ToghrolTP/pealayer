// Use the installed Edge and its DevTools socket, without downloading a browser
// or touching the user's profile. Drive only the isolated Vite test fixture.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
const profile=await mkdtemp(path.join(tmpdir(),'pealayer-monitor-browser-'));
const edge=process.env.EDGE_EXECUTABLE || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const child=spawn(edge,['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{windowsHide:true,stdio:'ignore'});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let socket;
try {
  let port;
  for(let n=0;n<150;n++){try {port=Number((await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await sleep(100);}}
  assert.ok(port,'isolated browser did not start');
  const target=await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
  socket=new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let sequence=0;const pending=new Map();const errors=[];
  socket.addEventListener('message',event=>{const value=JSON.parse(event.data);if(value.id){const entry=pending.get(value.id);pending.delete(value.id);if(value.error)entry?.reject(Error(value.error.message));else entry?.resolve(value.result);}if(value.method==='Runtime.exceptionThrown')errors.push(value.params.exceptionDetails.text);});
  const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const value=await send('Runtime.evaluate',{expression,returnByValue:true});if(value.exceptionDetails)throw Error(value.exceptionDetails.text);return value.result.value;};
  await send('Page.enable');await send('Runtime.enable');
  for(const light of [false,true]) {for(const width of [390,768,1440]) {
    await send('Emulation.setDeviceMetricsOverride',{width,height:850,deviceScaleFactor:1,mobile:false});
    for(const [mode,title] of [['stale','Hardware Disconnected'],['connecting','Connecting to PCController…'],['catalog','Loading hardware…'],['board','No board connected']]) {
      await send('Page.navigate',{url:`http://127.0.0.1:18191/scripts/hardware-monitor-fixture.html?state=${mode}${light?'&light':''}`});
      let ready=false;
      for(let n=0;n<100;n++){if(await evaluate(`document.querySelector('.hardware-empty-state h4')?.textContent === ${JSON.stringify(title)}`)){ready=true;break;}await sleep(100);}
      assert.ok(ready,`missing ${title} at ${width}: ${await evaluate('document.body.innerText.slice(0,1500)')} / ${errors.join('; ')}`);
      await sleep(120);
      const result=await evaluate(`({cards:document.querySelectorAll('.ant-card').length,width:document.documentElement.scrollWidth,viewport:innerWidth,commands:JSON.parse(document.querySelector('#fixture-state').textContent),rf:document.body.textContent.includes('RF remote')})`);
      assert.equal(result.cards,1,'unavailable monitor must not render cached board or RF cards');
      assert.ok(result.width<=result.viewport+1,`${mode} overflowed at ${width}`);assert.deepEqual(result.commands,[],'rendering must not connect, refresh or actuate');assert.equal(result.rf,false);
      if(mode==='stale') {
        if(process.env.PEALAYER_VERIFICATION_DIRECTORY){const image=await send('Page.captureScreenshot',{format:'png',clip:{x:0,y:0,width,height:340,scale:1}});await writeFile(path.join(process.env.PEALAYER_VERIFICATION_DIRECTORY,`hardware-monitor-web-${width}${light?'-light':''}.png`),Buffer.from(image.data,'base64'));}
        await evaluate(`Array.from(document.querySelectorAll('button')).find(button=>button.textContent.trim()==='Connect').click()`);
        await sleep(30);assert.deepEqual(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent)`),[['pealayer.hardware.connection',{connected:true}]]);
      }
    }
  }}
  assert.deepEqual(errors,[]);console.log('Hardware Monitor browser: four unavailable states, stale RF exclusion, user-only Connect and 390/768/1440 widths passed');
  await send('Browser.close').catch(()=>{});
} finally {
  socket?.close();
  if(child.exitCode===null){await Promise.race([new Promise(resolve=>child.once('exit',resolve)),sleep(2000)]);if(child.exitCode===null)child.kill();}
  // The only deletion target is this test's newly created isolated profile.
  assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir())+path.sep));
  await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});
}
