import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {createHash} from 'node:crypto';
import {SoftwarePeer,push,source,verifyReplacement} from './software-update.mjs';

const bytes=Buffer.from('verified isolated software fixture'),sha256=createHash('sha256').update(bytes).digest('hex');
const metadata={service:'pealayer',sha256,size:bytes.length,platform:'windows',arch:'x86_64'};

async function fixture(product,handler){
  const requests=[];
  const server=createServer(async(req,res)=>{
    const chunks=[];for await(const chunk of req)chunks.push(chunk);
    const request=JSON.parse(Buffer.concat(chunks));requests.push(request);
    try{const result=await handler(request);res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify({jsonrpc:'2.0',id:request.id,result}));}
    catch(error){res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify({jsonrpc:'2.0',id:request.id,error:{code:-32000,message:error.message}}));}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const peer=await new SoftwarePeer(`http://127.0.0.1:${server.address().port}`,product).select();
  return {peer,requests,async close(){await new Promise(resolve=>server.close(resolve));}};
}

let received=[];
const player=await fixture('pealayer',({method,params})=>{
  if(method==='pealayer.update.manifest')return {...metadata,format:'pealayer-update',health_state:'healthy'};
  if(method==='pealayer.update.artifact.read'){const data=bytes.subarray(params.offset,params.offset+params.length);return {sha256,size:bytes.length,offset:params.offset,next_offset:params.offset+data.length,data:data.toString('base64')};}
  if(method==='pealayer.update.upload.begin')return {operation_id:'upload-fixture',sha256,bytes_total:bytes.length,bytes_done:0,state:'receiving'};
  if(method==='pealayer.update.upload.chunk'){received.push(Buffer.from(params.data,'base64'));return {operation_id:'upload-fixture',sha256,bytes_total:bytes.length,bytes_done:params.offset+received.at(-1).length,state:'receiving'};}
  if(method==='pealayer.update.upload.finish')return {operation_id:'upload-fixture',sha256,state:'current'};
  throw Error(`Unexpected operation ${method}`);
});
try{
  const input=await source(player.peer);
  const result=await push(player.peer,input.manifest,input.read,{verify:false});await input.close();
  assert.equal(result.state,'accepted');assert.equal(result.terminal_verified,false);assert.deepEqual(Buffer.concat(received),bytes);
  assert.equal(player.requests.filter(value=>value.method==='pealayer.update.upload.finish').length,1);
  await verifyReplacement(player.peer,metadata,{stableWindow:0,timeout:1000,interval:1});
}finally{await player.close();}

let installs=0;
const controller=await fixture('pccontroller',({method})=>{
  if(method==='controller.artifact.upload.begin')return {transfer_id:'transfer-fixture',next_offset:0,chunk_bytes:384*1024};
  if(method==='controller.artifact.upload.chunk')return {transfer_id:'transfer-fixture',next_offset:bytes.length,bytes_total:bytes.length};
  if(method==='controller.artifact.upload.finish')return {artifact:{sha256,bytes:bytes.length}};
  if(method==='controller.update.host'){installs++;return {operation:{id:'operation-fixture',artifact_sha256:sha256}};}
  if(method==='controller.artifact.manifest')return {current:{host:{sha256,bytes:bytes.length}}};
  if(method==='controller.update.status')return {id:'operation-fixture',state:'completed',artifact_sha256:sha256};
  throw Error(`Unexpected controller operation ${method}`);
});
try{
  await push(controller.peer,{...metadata,service:'pccontroller',platform:'windows/amd64'},async()=>bytes,{verify:false});
  assert.equal(installs,1);
  await verifyReplacement(controller.peer,{...metadata,service:'pccontroller'},{stableWindow:0,timeout:1000,interval:1});
  assert.equal(controller.requests.at(-1).method,'controller.update.status','exact terminal operation must be verified, not just manifest');
}finally{await controller.close();}

let aborted=0,finished=0;
const wrong=await fixture('pealayer',({method})=>{
  if(method==='pealayer.update.upload.begin')return {operation_id:'upload-fixture',sha256,bytes_total:bytes.length,bytes_done:0,state:'receiving'};
  if(method==='pealayer.update.upload.chunk')return {operation_id:'another-owner',sha256,bytes_total:bytes.length,bytes_done:bytes.length,state:'receiving'};
  if(method==='pealayer.update.upload.abort'){aborted++;return {};}
  if(method==='pealayer.update.upload.finish'){finished++;return {};}
  throw Error(method);
});
try{
  await assert.rejects(push(wrong.peer,metadata,async()=>bytes,{verify:false}),/acknowledgement mismatch/);
  assert.equal(aborted,1);assert.equal(finished,0);
  assert.equal(wrong.requests.filter(value=>value.method==='pealayer.update.upload.chunk').length,1,'mutations must not be retried');
  await assert.rejects(push(wrong.peer,{...metadata,service:'pccontroller'},async()=>bytes),/different products/);
}finally{await wrong.close();}

assert.throws(()=>new SoftwarePeer('http://user:secret@example/','pealayer'),/without credentials/);

const bundled=await fixture('pealayer',({method,params})=>{
  if(method==='pealayer.update.targets')return {targets:[{id:'self',product:'pealayer'}]};
  if(method==='pealayer.update.bundle.list')return {assets:[{id:'controller-package',manifest:{...metadata,service:'pccontroller'}}]};
  if(method==='pealayer.update.bundle.read'){assert.equal(params.asset_id,'controller-package');const data=bytes.subarray(params.offset,params.offset+params.length);return {sha256,size:bytes.length,offset:params.offset,next_offset:params.offset+data.length,data:data.toString('base64')};}
  throw Error('No controller connected');
});
try{
  bundled.peer.target='bundle:controller-package';await bundled.peer.select();
  assert.equal(bundled.peer.product,'pccontroller');
  const input=await source(bundled.peer);assert.deepEqual(await input.read(0,bytes.length),bytes);await input.close();
  assert.ok(bundled.peer.targetErrors.length,'disconnected catalog must remain explicit');
  await assert.rejects(push(bundled.peer,input.manifest,async()=>bytes),/source, not an installation target/);
}finally{await bundled.close();}

let bootstrapInstalls=0;
const bootstrap=await fixture('pealayer',({method,params})=>{
  assert.equal(method,'pealayer.update.controller.call');assert.equal(params.target,'controller');
  switch(params.method){
    case 'controller.artifact.upload.begin':return {transfer_id:'old-host-transfer',next_offset:0,chunk_bytes:384*1024};
    case 'controller.artifact.upload.chunk':return {transfer_id:'old-host-transfer',next_offset:bytes.length,bytes_total:bytes.length};
    case 'controller.artifact.upload.finish':return {artifact:{sha256,bytes:bytes.length}};
    case 'controller.update.host':bootstrapInstalls++;return {operation:{id:'bootstrap-operation',artifact_sha256:sha256}};
    default:throw Error('Only the established artifact API exists on this host');
  }
});
try{
  bootstrap.peer.target='connected-controller';await bootstrap.peer.select();
  await push(bootstrap.peer,{...metadata,service:'pccontroller'},async()=>bytes,{verify:false});
  assert.equal(bootstrapInstalls,1);
}finally{await bootstrap.close();}
console.log('Shared software client: both owners, bounded streaming copy, exact ACK/hash/terminal checks, mismatch abort, no mutation replay and product fences passed.');
