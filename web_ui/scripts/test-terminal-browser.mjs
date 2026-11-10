// Real paired host, never a browser-owned shell. Access key is process-only.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
const origin=process.env.PEALAYER_TERMINAL_TEST_ORIGIN||'http://127.0.0.1:18192';
assert.equal(new URL(origin).hostname,'127.0.0.1','use the isolated loopback fixture');
const key=process.env.PEALAYER_TERMINAL_TEST_KEY;
assert.equal(key?.length,64,'supply an ephemeral native-paired access key');
const control=async operation=>{
  const response=await fetch(`${origin}/api/terminal/control`,{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${key}`},body:JSON.stringify(operation)});
  assert.ok(response.ok,`host control failed (${response.status})`);return response.json();
};
const config=await (await fetch(`${origin}/api/config`)).json();
assert.equal(config.app_name,'Pealayer Terminal Review');assert.equal(config.auto_connect_hardware,false);
assert.equal(config.ui.terminal.allow_system_shell,false);assert.equal(config.ui.terminal.allow_remote_system_shell,false);
const profile=await mkdtemp(path.join(tmpdir(),'pealayer-terminal-browser-'));
const child=spawn(process.env.EDGE_EXECUTABLE||'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  ['--headless=new','--no-first-run','--no-default-browser-check','--disable-extensions','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{windowsHide:true,stdio:'ignore'});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let socket,sessionId;
try {
  let port;for(let n=0;n<100;n++){try{port=Number((await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await sleep(100);}}
  assert.ok(port,'isolated browser did not start');
  const target=await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
  socket=new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let sequence=0;const pending=new Map(),errors=[];
  socket.addEventListener('message',event=>{const value=JSON.parse(event.data);if(value.id){const entry=pending.get(value.id);pending.delete(value.id);if(value.error)entry?.reject(Error(value.error.message));else entry?.resolve(value.result);}if(value.method==='Runtime.exceptionThrown')errors.push(value.params.exceptionDetails.text);});
  const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const value=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(value.exceptionDetails)throw Error(value.exceptionDetails.text);return value.result.value;};
  // Pass test data as CDP values, never interpolate it into executable source.
  const call=async(functionDeclaration,value)=>{
    const global=await send('Runtime.evaluate',{expression:'globalThis'});
    const result=await send('Runtime.callFunctionOn',{objectId:global.result.objectId,functionDeclaration,arguments:[{value}],returnByValue:true,awaitPromise:true});
    if(result.exceptionDetails)throw Error(result.exceptionDetails.text);return result.result.value;
  };
  const until=async(predicate,message)=>{for(let n=0;n<100;n++){if(await predicate())return;await sleep(80);}throw Error(message);};
  await send('Page.enable');await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride',{width:1440,height:940,deviceScaleFactor:1,mobile:false});
  await send('Page.navigate',{url:`${origin}/#terminal`});
  await until(()=>evaluate(`!!document.querySelector('input[aria-label="Terminal access key"]')`),'pairing form missing');
  // React-controlled password. Never put credentials in a URL or browser storage.
  await call(`function(value){const element=document.querySelector('input[aria-label="Terminal access key"]');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(element,value);element.dispatchEvent(new Event('input',{bubbles:true}));}`,key);
  await until(()=>evaluate(`Array.from(document.querySelectorAll('button')).some(b=>b.textContent==='Connect'&&!b.disabled)`),'pairing input did not update');
  await evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent==='Connect').click()`);
  await until(()=>evaluate(`!!document.querySelector('.terminal-grid')`),'paired screen missing');
  // Create this test's own session from the actual UI, not another process.
  const previousIds=(await control({operation:'snapshot'})).sessions.map(s=>s.id);
  await evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent==='Session').click()`);
  await until(()=>evaluate(`!!Array.from(document.querySelectorAll('[role="menuitem"]')).find(e=>e.textContent==='Pealayer Shell')`),'session menu missing');
  await evaluate(`Array.from(document.querySelectorAll('[role="menuitem"]')).find(e=>e.textContent==='Pealayer Shell').click()`);
  await until(async()=>{const state=await control({operation:'snapshot'});const created=state.sessions.find(s=>!previousIds.includes(s.id));if(created)sessionId=created.id;return created?.phase==='ready'&&state.screen?.id===sessionId;},'UI-created PortableShell not ready');
  await until(()=>evaluate(`!!document.querySelector('.terminal-session.is-active')`),'active session tab missing');
  const paste=async text=>{
    await call(`function(value){const element=document.querySelector('.terminal-grid');element.focus();const data=new DataTransfer();data.setData('text/plain',value);element.dispatchEvent(new ClipboardEvent('paste',{bubbles:true,cancelable:true,clipboardData:data}));}`,text);
  };
  const enter=async()=>{await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});};
  await paste('set ui.terminal.font_size 14');await enter();
  await until(async()=> (await (await fetch(`${origin}/api/config`)).json()).ui.terminal.font_size===14,'Web shell did not change host config');
  await paste('set ui.terminal.font_size 13');await enter();
  await until(async()=> (await (await fetch(`${origin}/api/config`)).json()).ui.terminal.font_size===13,'Web shell did not restore host config');
  await paste('configure');await enter();
  await until(async()=> (await control({operation:'snapshot'})).screen?.alternate,'Web configure did not open shared table');
  await until(()=>evaluate(`document.querySelector('.terminal-grid').textContent.includes('font')`),'Web table not painted');
  assert.equal(await call(`function(key){return Object.values(localStorage).concat(Object.values(sessionStorage)).some(value=>value.includes(key));}`,key),false,'Terminal pairing must not persist credentials');
  for(const width of [1440,390]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:940,deviceScaleFactor:1,mobile:false});await sleep(200);
    assert.ok(await evaluate(`document.documentElement.scrollWidth<=innerWidth+1`),`page overflows at ${width}`);
    const shared=await control({operation:'snapshot'});
    assert.equal(shared.screen.id,sessionId,'browser viewport must not replace host session');
    if(process.env.PEALAYER_VERIFICATION_DIRECTORY){const image=await send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(process.env.PEALAYER_VERIFICATION_DIRECTORY,`terminal-web-${width}.png`),Buffer.from(image.data,'base64'));}
  }
  assert.deepEqual(errors,[]);
  await evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent==='Disconnect viewer').click()`);
  await until(()=>evaluate(`!!document.querySelector('input[aria-label="Terminal access key"]')`),'viewer disconnect missing');
  assert.ok((await control({operation:'snapshot'})).sessions.some(s=>s.id===sessionId),'disconnect must not close shared host session');
  console.log('PASS: actual Web pairing, session creation, ordered shell config edits/readback, shared table, responsive screen and non-destructive viewer disconnect.');
  await send('Browser.close').catch(()=>{});
} finally {
  if(sessionId)await control({operation:'close',id:sessionId});
  socket?.close();
  if(child.exitCode===null){await Promise.race([new Promise(resolve=>child.once('exit',resolve)),sleep(2000)]);if(child.exitCode===null)child.kill();}
  // Only this invocation's newly-created temporary browser profile is removed.
  assert.ok(path.resolve(profile).startsWith(path.resolve(tmpdir())+path.sep));
  await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});
}
