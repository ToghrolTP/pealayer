// Installed Edge. Real footer/tooltip against memory-only fixture preferences.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
const profile=await mkdtemp(path.join(tmpdir(),'pealayer-fps-tooltip-'));
const child=spawn(process.env.EDGE_EXECUTABLE||'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{windowsHide:true,stdio:'ignore'});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));let socket;
try{
  let port;for(let n=0;n<150;n++){try{port=Number((await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await sleep(100);}}assert.ok(port);
  const target=await(await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
  socket=new WebSocket(target.webSocketDebuggerUrl);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let seq=0;const pending=new Map(),errors=[];
  socket.addEventListener('message',event=>{const v=JSON.parse(event.data);if(v.id){const p=pending.get(v.id);pending.delete(v.id);if(v.error)p?.reject(Error(v.error.message));else p?.resolve(v.result);}if(v.method==='Runtime.exceptionThrown')errors.push(v.params.exceptionDetails.text);});
  const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++seq;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const v=await send('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(v.exceptionDetails)throw Error(v.exceptionDetails.text);return v.result.value;};
  await send('Page.enable');await send('Runtime.enable');
  await send('Page.bringToFront');
  for(const width of [390,768,1440])for(const light of [false,true])for(const rtl of [false,true])for(const neutral of [false,true]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:850,deviceScaleFactor:1,mobile:false});
    await send('Page.navigate',{url:`http://127.0.0.1:18191/scripts/frame-rate-tooltip-fixture.html?${light?'light&':''}${rtl?'rtl&':''}${neutral?'neutral':''}`});
    let ready=false;for(let n=0;n<100;n++){if(await evaluate(`Boolean(document.querySelector('.app-statusbar__fps'))`)){ready=true;break;}await sleep(100);}assert.ok(ready);
    const point=await evaluate(`(()=>{const r=document.querySelector('.app-statusbar__fps').getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',...point});
    let shown=false;for(let n=0;n<60;n++){if(await evaluate(`Boolean(document.querySelector('.ant-tooltip:not(.ant-tooltip-hidden) .frame-rate-details'))`)){shown=true;break;}await sleep(50);}assert.ok(shown);
    await sleep(250);
    const detail=await evaluate(`(()=>{const root=document.querySelector('.frame-rate-details'),r=root.getBoundingClientRect(),values=[...root.querySelectorAll('.frame-rate-details__value')];return {left:r.left,right:r.right,text:root.textContent,values:values.map(v=>v.textContent),ends:values.map(v=>v.getBoundingClientRect().right),dividers:root.querySelectorAll('hr').length,weight:values.map(v=>getComputedStyle(v).fontWeight),titles:[...document.querySelectorAll('[data-status-widget="fps"],.app-statusbar__fps')].map(v=>v.getAttribute('title'))};})()`);
    assert.ok(detail.left>=0&&detail.right<=width+1);assert.deepEqual(detail.values,['23.98','30.62']);assert.ok(Math.abs(detail.ends[0]-detail.ends[1])<.2);assert.equal(detail.dividers,2);assert.deepEqual(detail.weight,['400','400']);assert.deepEqual(detail.titles,[null,null]);assert.ok(!/Active mode|NaN|Infinity/.test(detail.text));
    if(!neutral&&(width===390||width===1440)&&process.env.PEALAYER_VERIFICATION_DIRECTORY){const shot=await send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(process.env.PEALAYER_VERIFICATION_DIRECTORY,`fps-tooltip-${width}${light?'-light':''}${rtl?'-rtl':''}.png`),Buffer.from(shot.data,'base64'));}
    await evaluate(`document.querySelector('.app-statusbar__fps').click()`);await sleep(50);
    assert.equal(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).mode`),'ui');
    assert.equal(await evaluate(`document.querySelector('.frame-rate-details__row.is-active .frame-rate-details__value').textContent`),'30.62');
  }
  await send('Page.navigate',{url:'http://127.0.0.1:18191/scripts/frame-rate-tooltip-fixture.html?unknown'});
  for(let n=0;n<100;n++){if(await evaluate(`Boolean(document.querySelector('.app-statusbar__fps'))`))break;await sleep(50);}
  await evaluate(`document.querySelector('.app-statusbar__fps').focus()`);
  for(let n=0;n<60;n++){if(await evaluate(`Boolean(document.querySelector('.ant-tooltip:not(.ant-tooltip-hidden) .frame-rate-details'))`))break;await sleep(50);}
  assert.deepEqual(await evaluate(`Array.from(document.querySelectorAll('.frame-rate-details__value')).map(v=>v.textContent)`),['—','—']);
  assert.deepEqual(errors,[]);console.log('FPS tooltip: real hover/focus, aligned values, palette/phone/tablet/desktop/RTL bounds, source cycling and unknown data checks passed');
  await send('Browser.close').catch(()=>{});
}finally{socket?.close();if(child.exitCode===null){await Promise.race([new Promise(resolve=>child.once('exit',resolve)),sleep(2000)]);if(child.exitCode===null)child.kill();}assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir())+path.sep));await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});}
