// Installed Edge only. Controls operate on the isolated memory-only fixture.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
const profile=await mkdtemp(path.join(tmpdir(),'pealayer-assets-browser-'));
const child=spawn(process.env.EDGE_EXECUTABLE||'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{windowsHide:true,stdio:'ignore'});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));let socket;
try{
  let port;for(let n=0;n<150;n++){try{port=Number((await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await sleep(100);}}
  assert.ok(port);const target=await(await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
  socket=new WebSocket(target.webSocketDebuggerUrl);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let seq=0;const pending=new Map(),errors=[];
  socket.addEventListener('message',event=>{const v=JSON.parse(event.data);if(v.id){const p=pending.get(v.id);pending.delete(v.id);if(v.error)p?.reject(Error(v.error.message));else p?.resolve(v.result);}if(v.method==='Runtime.exceptionThrown')errors.push(v.params.exceptionDetails.text);});
  const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++seq;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const v=await send('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(v.exceptionDetails)throw Error(v.exceptionDetails.text);return v.result.value;};
  await send('Page.enable');await send('Runtime.enable');
  for(const light of [false,true])for(const width of [390,768,1440]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:850,deviceScaleFactor:1,mobile:false});
    for(const mode of ['drop','review','unsaved']){
      const review=mode==='review';
      await send('Page.navigate',{url:`http://127.0.0.1:18191/scripts/asset-import-fixture.html?${light?'light&':''}${mode}`});
      let ready=false;for(let n=0;n<100;n++){if(await evaluate(`Boolean(document.querySelector('#fixture-state'))`)){ready=true;break;}await sleep(100);}assert.ok(ready);
      await sleep(150);
      assert.ok(await evaluate('document.documentElement.scrollWidth<=innerWidth+1'),`overflow ${width}`);
      assert.deepEqual(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).commands`),[],'rendering never imports or actuates');
      if(mode==='unsaved') {
        const bounds=await evaluate(`(()=>{const r=document.querySelector('.ant-modal').getBoundingClientRect();return {left:r.left,right:r.right};})()`);assert.ok(bounds.left>=0 && bounds.right<=width+1);
        await evaluate(`Array.from(document.querySelectorAll('.ant-modal button')).find(b=>b.textContent.trim()==='Cancel').click()`);
        await sleep(20);assert.deepEqual(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).commands`),[['resolve_timeline_close',{decision:'cancel'}]]);
      }else if(review){
        assert.equal(await evaluate(`document.querySelector('.asset-review-items').textContent.includes('pwm.0 · 35.000%')`),true);
        const bounds=await evaluate(`(()=>{const r=document.querySelector('.ant-modal').getBoundingClientRect();return {left:r.left,right:r.right};})()`);assert.ok(bounds.left>=0 && bounds.right<=width+1);
        if(width===390 && process.env.PEALAYER_VERIFICATION_DIRECTORY){const shot=await send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(process.env.PEALAYER_VERIFICATION_DIRECTORY,`asset-review-phone${light?'-light':''}.png`),Buffer.from(shot.data,'base64'));}
      }else{
        await evaluate(`(()=>{const lane=document.querySelector('[data-asset-lane]'),r=lane.getBoundingClientRect(),data=new DataTransfer();data.items.add(new File([new Uint8Array(600000)],'sound.wav'));lane.dispatchEvent(new DragEvent('dragover',{bubbles:true,cancelable:true,dataTransfer:data,clientX:r.left+r.width*.52145}));lane.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data,clientX:r.left+r.width*.52145}));})()`);
        for(let n=0;n<60;n++){if(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).commands.length===1`))break;await sleep(50);}
        const result=await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent)`);
        assert.equal(result.commands.length,1);assert.equal(result.commands[0][0],'import_assets');
        // DragEvent coordinates are integer CSS pixels; test the same measured
        // integer position, not an unattainable fractional browser event.
        const time=await evaluate(`(()=>{const r=document.querySelector('[data-asset-lane]').getBoundingClientRect();return Math.round((Math.floor(r.left+r.width*.52145)-r.left)/r.width*20000);})()`);
        assert.equal(result.commands[0][1].request.destination.start_time_ms,time);assert.equal(result.commands[0][1].request.destination.track_key,'controller-effect:audio');
        await evaluate(`(()=>{const input=document.querySelector('input.ant-input'),data=new DataTransfer();data.items.add(new File([new Uint8Array(200)],'source.wav'));input.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));})()`);
        for(let n=0;n<60;n++){if(await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent).source.endsWith('source.wav')`))break;await sleep(50);}
        const source=await evaluate(`JSON.parse(document.querySelector('#fixture-state').textContent)`);assert.equal(source.source,'/fixture/assets/source.wav');assert.equal(source.commands.length,1,'source field must not import a second asset or play media');
      }
    }
  }
  assert.deepEqual(errors,[]);console.log('Asset import browser: file drops, exact placement, source reuse, non-actuating review and phone/tablet/desktop light/dark checks passed');
  await send('Browser.close').catch(()=>{});
}finally{socket?.close();if(child.exitCode===null){await Promise.race([new Promise(resolve=>child.once('exit',resolve)),sleep(2000)]);if(child.exitCode===null)child.kill();}assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir())+path.sep));await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});}
