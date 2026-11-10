// Run against an isolated Vite fixture at :18191, using the installed browser.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
const profile=await mkdtemp(path.join(tmpdir(),'pealayer-choice-browser-'));
const child=spawn(process.env.EDGE_EXECUTABLE || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  ['--headless=new','--no-first-run','--disable-extensions','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],
  {windowsHide:true,stdio:'ignore'});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let socket;
try {
  let port;
  for(let n=0;n<150;n++){try{port=Number((await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await sleep(100);}}
  assert.ok(port,'isolated browser did not start');
  const target=await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
  socket=new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let sequence=0;const pending=new Map();const errors=[];
  socket.addEventListener('message',event=>{const value=JSON.parse(event.data);if(value.id){const entry=pending.get(value.id);pending.delete(value.id);if(value.error)entry?.reject(Error(value.error.message));else entry?.resolve(value.result);}if(value.method==='Runtime.exceptionThrown')errors.push(value.params.exceptionDetails.text);});
  const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const value=await send('Runtime.evaluate',{expression,returnByValue:true});if(value.exceptionDetails)throw Error(value.exceptionDetails.text);return value.result.value;};
  const state=()=>evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent)`);
  await send('Page.enable');await send('Runtime.enable');
  for(const light of [false,true])for(const width of [390,768,1440]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:500,deviceScaleFactor:1,mobile:false});
    await send('Page.navigate',{url:`http://127.0.0.1:18191/scripts/choice-slider-fixture.html${light?'?light':''}`});
    let ready=false;for(let n=0;n<100;n++){if(await evaluate(`!!document.querySelector('.preference-choice-slider__value')`)){ready=true;break;}await sleep(100);}
    assert.ok(ready,'choice fixture did not load');await sleep(100);
    const initial=await evaluate(`(()=>{const r=document.querySelector('.preference-choice-slider__value').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};})()`);
    for(const [index,expected] of ['regular','comfortable','spacious','extra_compact','compact'].entries()){
      await evaluate(`document.querySelector('.preference-choice-slider__value').click()`);
      assert.equal((await state()).busy,true);
      // Repeated input while persistence is pending must not double-submit.
      await evaluate(`document.querySelector('.preference-choice-slider__value').click()`);
      await sleep(230);const saved=await state();assert.equal(saved.value,expected);assert.equal(saved.commits.length,index+1);
      const geometry=await evaluate(`(()=>{const r=document.querySelector('.preference-choice-slider__value').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};})()`);
      assert.deepEqual(geometry,initial,'caption changes moved the capsule');
    }
    await evaluate(`document.querySelector('#reject').click()`);await sleep(30);
    await evaluate(`document.querySelector('.preference-choice-slider__value').click()`);
    await sleep(230);assert.equal((await state()).value,'compact');
    assert.equal(await evaluate(`document.querySelector('.preference-choice-slider__caption').textContent`),'Compact','failed save must restore the authoritative value');
    assert.ok(await evaluate(`document.documentElement.scrollWidth<=innerWidth+1`),'choice overflowed');
    if(process.env.PEALAYER_VERIFICATION_DIRECTORY && [390,1440].includes(width)){
      const capture=await send('Page.captureScreenshot',{format:'png'});
      await writeFile(path.join(process.env.PEALAYER_VERIFICATION_DIRECTORY,`choice-slider-${width}${light?'-light':''}.png`),Buffer.from(capture.data,'base64'));
    }
  }
  assert.deepEqual(errors,[]);console.log('Density capsule: cycle/wrap, pending/failed saves and stable geometry pass at phone/tablet/desktop widths in both themes.');
} finally {
  socket?.close();child.kill();await sleep(300);
  await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});
}
