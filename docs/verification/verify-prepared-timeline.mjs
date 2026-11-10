// Explicit VirtualBoard-only acceptance. Never actuate a physical board here.
import net from 'node:net';
import assert from 'node:assert/strict';
const base = process.argv[2] || 'http://127.0.0.1:8080';
const port = Number(process.argv[3] || 8787);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const socket = net.connect(port, '127.0.0.1');
await new Promise((resolve, reject) => {socket.once('connect', resolve);socket.once('error', reject)});
let serial=0, input='';const pending=new Map();
socket.on('data', data=>{input+=data;let newline;while((newline=input.indexOf('\n'))>=0){const value=JSON.parse(input.slice(0,newline));input=input.slice(newline+1);const request=pending.get(value.id);if(request){pending.delete(value.id);clearTimeout(request.timer);value.error?request.reject(Error(JSON.stringify(value.error))):request.resolve(value.result)}}});
const rpc=(method,params={})=>new Promise((resolve,reject)=>{const id=++serial;const timer=setTimeout(()=>{pending.delete(id);reject(Error(method+' timeout'))},5000);pending.set(id,{resolve,reject,timer});socket.write(JSON.stringify({jsonrpc:'2.0',id,method,params})+'\n')});
async function http(path,body){const result=await fetch(base+path,{signal:AbortSignal.timeout(5000),...(body?{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)}:{})});assert(result.ok,path);return result.json()}
async function until(fn){for(let i=0;i<100;i++){const value=await fn();if(value)return value;await delay(25)}throw Error('acceptance condition timed out')}
const initial=await http('/api/player/status');const config=await http('/api/config');const board=await rpc('controller.snapshot');
assert(board.connected && board.port?.product==='PCController Virtual Board','Refusing non-VirtualBoard actuation');
assert(!initial.playing && !(initial.cues||[]).length,'Pause media and remove cues before this acceptance check');
assert(config.auto_connect_hardware && initial.hardware_connection_requested,'Need automatic connection enabled to restore its state safely');
const id='verification:prepared-'+Date.now();let sequence=0;let registered=false,changed=false;
const clock=(revision,playing,position_ms)=>rpc('controller.media.playback.update',{client_id:id,sequence:++sequence,position_ms,loaded:true,playing,rate:1,epoch:1,plan_revision:revision});
try{
 changed=true;await http('/api/config',{auto_connect_hardware:false});
 await until(async()=>!(await rpc('controller.media.playback.get')).loaded);
 await rpc('controller.app.instance.report',{id,surface:'verification',lease_seconds:30,values:{application:'Prepared timeline acceptance'}});registered=true;
 const plan={client_id:id,revision:1,max_lateness_ms:100,cues:[],actions:[
  {id:'relay8-on',time_ms:200,step:{kind:'relay',target:7,value:1}},
  {id:'relay8-off',time_ms:400,step:{kind:'relay',target:7,value:0}}]};
 const prepared=await rpc('controller.media.timeline.prepare',plan);assert.equal(prepared.step_count,2);assert(prepared.hash);
 await clock(1,false,0);await until(async()=>{const value=await rpc('controller.media.timeline.get');return value.armed_epoch===1 && value.clock_sequence>0 && value});
 const start=performance.now();await clock(1,true,0);
 for(let i=0;i<30;i++){await delay(20);await clock(1,true,Math.round(performance.now()-start))}
 const complete=await rpc('controller.media.timeline.get');assert.equal(complete.acknowledged,2);assert.equal(complete.state,'playing');assert(complete.max_ack_lateness_ms<=100);
 await clock(1,false,Math.round(performance.now()-start));
 const late={...plan,revision:2};await rpc('controller.media.timeline.prepare',late);
 await clock(2,false,0);await until(async()=>{const value=await rpc('controller.media.timeline.get');return value.revision===2 && value.armed_epoch===1 && value.clock_sequence>0 && value});
 await clock(2,true,1000);const fault=await until(async()=>{const value=await rpc('controller.media.timeline.get');return value.state==='faulted' && value});
 assert.equal(fault.acknowledged,0);assert.match(fault.error,/missed its deadline/);
 console.log(JSON.stringify({virtual_board:true,prepared_hash:prepared.hash,ordered_native_acks:complete.acknowledged,max_ack_lateness_ms:complete.max_ack_lateness_ms,late_action_not_executed:fault.acknowledged===0,fault:fault.error},null,2));
}finally{
 if(registered){await clock(2,false,0).catch(()=>{});await rpc('controller.media.timeline.prepare',{client_id:id,revision:3,cues:[],actions:[]}).catch(()=>{});await rpc('controller.media.playback.update',{client_id:id,sequence:++sequence,position_ms:0,loaded:false,playing:false,rate:1}).catch(()=>{});await rpc('controller.app.instance.remove',{id}).catch(()=>{})}
 if(changed){await http('/api/config',{auto_connect_hardware:config.auto_connect_hardware});await until(async()=>{const status=await http('/api/player/status');return status.hardware_connected && status})}
 socket.end();
}
