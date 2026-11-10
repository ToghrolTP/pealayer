// One bounded client for both owners. No replacement, rollback or board code
// lives here: those operations remain with the destination application's updater.
import {createHash,randomUUID} from 'node:crypto';
import {open,readFile,link,unlink} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';

const MAX_BYTES=512*1024*1024, CHUNK_BYTES=384*1024;
export class SoftwarePeer {
  constructor(via,gateway,target='self',revision=null){
    this.origin=new URL(via);
    if(!['http:','https:'].includes(this.origin.protocol)||this.origin.username||this.origin.password||this.origin.search||this.origin.hash||this.origin.pathname!=='/')throw Error('Use an HTTP(S) peer origin without credentials, path or query');
    if(!['pealayer','pccontroller'].includes(gateway))throw Error('Choose pealayer or pccontroller gateway');
    Object.assign(this,{gateway,target,revision});
  }
  async rpc(method,params={}){
    const id=randomUUID();
    const response=await fetch(new URL('/api/rpc',this.origin),{method:'POST',redirect:'error',signal:AbortSignal.timeout(15000),headers:{'Content-Type':'application/json','User-Agent':'Pealayer-PCController software-updater'},body:JSON.stringify({jsonrpc:'2.0',id,method,params})});
    if(!response.ok)throw Error(`Gateway rejected update request (HTTP ${response.status})`);
    const reader=response.body.getReader();let size=0;const chunks=[];
    try{for(;;){const {done,value}=await reader.read();if(done)break;size+=value.length;if(size>1024*1024)throw Error('Gateway reply exceeds 1 MiB');chunks.push(value);}}
    finally{await reader.cancel();}
    const envelope=JSON.parse(Buffer.concat(chunks).toString('utf8'));
    if(envelope.id!==id||envelope.jsonrpc!=='2.0')throw Error('Gateway acknowledgement identity mismatch');
    if(envelope.error)throw Error(envelope.error.message||'Gateway RPC failed');
    if(!Object.hasOwn(envelope,'result'))throw Error('Gateway omitted result');
    return envelope.result;
  }
  async controller(method,params={}){
    return this.gateway==='pccontroller'?this.rpc(method,params):this.rpc('pealayer.update.controller.call',{target:'controller',method,params});
  }
  async targets(){
    this.targetErrors=[];
    const bundled=(assets,prefix='')=>assets.map(value=>({id:`${prefix}bundle:${value.id}`,product:value.manifest.service,manifest:value.manifest,asset_id:value.id,bundle_controller:prefix==='controller:'}));
    const result=this.gateway==='pccontroller'?(await this.controller('controller.software.targets')).targets:(await this.rpc('pealayer.update.targets')).targets;
    try{const bundle=await this.rpc(this.gateway==='pealayer'?'pealayer.update.bundle.list':'controller.software.bundle.list');result.push(...bundled(bundle.assets));}catch(error){this.targetErrors.push({scope:'bundle',error:error.message});}
    if(this.gateway==='pealayer'){
      try{result.push(...(await this.controller('controller.software.targets')).targets.map(value=>({...value,id:`controller:${value.id}`})));}catch(error){this.targetErrors.push({scope:'controller',error:error.message});}
      try{result.push(...bundled((await this.controller('controller.software.bundle.list')).assets,'controller:'));}catch(error){this.targetErrors.push({scope:'controller-bundle',error:error.message});}
    }
    return result;
  }
  async select(){
    if(this.target==='self'){this.product=this.gateway;return this;}
    // "connected-controller" is an explicit bootstrap route through the
    // already-existing artifact API, not a silent version-based fallback.
    if(this.gateway==='pealayer'&&this.target==='connected-controller'){this.product='pccontroller';return this;}
    const catalog=this.gateway==='pealayer'&&this.target.startsWith('authority:')?(await this.rpc('pealayer.update.targets')).targets:await this.targets();
    const target=catalog.find(value=>value.id===this.target);
    if(!target)throw Error('Target unavailable; list connected update targets first');
    if(target.origin)this.selectedOrigin=target.origin;
    if(target.asset_id){this.bundleAssetId=target.asset_id;this.bundleManifest=target.manifest;this.bundleViaController=target.bundle_controller;}
    Object.assign(this,{product:target.product,revision:target.revision});return this;
  }
  async call(method,params={}){
    if(this.gateway==='pealayer'&&this.target==='self')return this.rpc(method,params);
    if(this.gateway==='pccontroller'&&this.target==='self')return this.controller(method,params);
    if(this.gateway==='pealayer'&&this.target.startsWith('authority:'))return this.rpc('pealayer.update.peer.call',{target:this.target,method,params});
    if(this.gateway==='pealayer'&&this.target==='connected-controller')return this.controller(method,params);
    const target=this.gateway==='pealayer'?this.target.replace(/^controller:/,''):this.target;
    return this.controller('controller.software.call',{target,revision:this.revision,authorized:true,request:{jsonrpc:'2.0',id:randomUUID(),method,params}});
  }
  async manifest(){
    if(this.bundleAssetId)return this.bundleManifest;
    const value=await this.call(this.product==='pealayer'?'pealayer.update.manifest':'controller.artifact.manifest');
    if(this.product==='pealayer'&&(value.service!=='pealayer'||value.format!=='pealayer-update'))throw Error('Selected peer is not a Pealayer update owner');
    const manifest=this.product==='pealayer'?value:value.current?.host;
    if(!manifest)throw Error('Peer has no current host artifact');
    return {service:this.product,sha256:manifest.sha256,size:manifest.size??manifest.bytes,platform:manifest.platform,arch:manifest.arch,version:manifest.version,libmpv_runtime:manifest.libmpv_runtime,health_state:manifest.health_state,raw:manifest};
  }
  async bundleRead(sha256,offset,length){
    const params={asset_id:this.bundleAssetId,sha256,offset,length};
    return this.bundleViaController?this.controller('controller.software.bundle.read',params):this.rpc(this.gateway==='pealayer'?'pealayer.update.bundle.read':'controller.software.bundle.read',params);
  }
}

function identity(value){
  if(!['pealayer','pccontroller'].includes(value.service)||!/^[a-f0-9]{64}$/.test(value.sha256)||!Number.isSafeInteger(value.size)||value.size<1||value.size>MAX_BYTES)throw Error('Supply a software manifest with product, SHA-256 and bounded size');
}
function ack(value,id,hash,size,offset,product){
  if(product==='pealayer'){
    if(value.operation_id!==id||value.sha256!==hash||value.bytes_total!==size||value.bytes_done!==offset||value.state!=='receiving')throw Error('Peer upload acknowledgement mismatch');
  }else if(value.transfer_id!==id||value.next_offset!==offset||(offset>0&&value.bytes_total!==size))throw Error('Controller upload acknowledgement mismatch');
}

export async function push(peer,manifest,readChunk,{verify=true}={}){
  if(peer.bundleAssetId)throw Error('A deployment bundle is a source, not an installation target');
  identity(manifest);if(manifest.service!==peer.product)throw Error('Source and selected target are different products');
  const player=peer.product==='pealayer';
  const begin=await peer.call(player?'pealayer.update.upload.begin':'controller.artifact.upload.begin',player?
    {sha256:manifest.sha256,size:manifest.size,version:manifest.version,platform:manifest.platform,arch:manifest.arch,libmpv_runtime:manifest.libmpv_runtime}:
    {kind:'host-executable',name:manifest.name,sha256:manifest.sha256,bytes:manifest.size,platform:manifest.platform?.includes('/')?manifest.platform:(manifest.platform&&manifest.arch?`${manifest.platform}/${({x86_64:'amd64',aarch64:'arm64',x86:'386'})[manifest.arch]??manifest.arch}`:manifest.platform)});
  const id=player?begin.operation_id:begin.transfer_id;
  let finishing=false,offset=0;const hash=createHash('sha256');
  const chunkSize=player?CHUNK_BYTES:begin.chunk_bytes;
  try{
    if(typeof id!=='string'||!/^[a-zA-Z0-9_-]{1,128}$/.test(id)||!Number.isSafeInteger(chunkSize)||chunkSize<1||chunkSize>CHUNK_BYTES)throw Error('Peer returned an invalid transfer identity or chunk size');
    ack(begin,id,manifest.sha256,manifest.size,0,peer.product);
    while(offset<manifest.size){
      const bytes=await readChunk(offset,Math.min(chunkSize,manifest.size-offset));
      if(!Buffer.isBuffer(bytes)||bytes.length!==Math.min(chunkSize,manifest.size-offset))throw Error('Source artifact changed or ended unexpectedly');
      hash.update(bytes);
      const value=await peer.call(player?'pealayer.update.upload.chunk':'controller.artifact.upload.chunk',{
        [player?'operation_id':'transfer_id']:id,offset,data:bytes.toString('base64')});
      offset+=bytes.length;ack(value,id,manifest.sha256,manifest.size,offset,peer.product);
    }
    if(hash.digest('hex')!==manifest.sha256)throw Error('Source SHA-256 changed; refusing to finish update');
    if(player){
      finishing=true;
      const finish=await peer.call('pealayer.update.upload.finish',{operation_id:id});
      if(finish.operation_id!==id||finish.sha256!==manifest.sha256||!['current','restarting'].includes(finish.state))throw Error('Finish acknowledgement mismatch; do not replay update');
    }else{
      const uploaded=await peer.call('controller.artifact.upload.finish',{transfer_id:id});
      if(uploaded.artifact?.sha256!==manifest.sha256||uploaded.artifact?.bytes!==manifest.size)throw Error('Controller did not verify the exact uploaded artifact');
      finishing=true;
      const result=await peer.call('controller.update.host',{artifact_sha256:manifest.sha256,authorized:true,idempotency_key:`software:${randomUUID()}`});
      if(result.operation?.artifact_sha256!==manifest.sha256)throw Error('Controller update acknowledgement mismatch; do not replay');
      if(typeof result.operation?.id!=='string'||!result.operation.id)throw Error('Controller omitted tracked update operation');
      peer.pendingOperation=result.operation.id;
    }
  }catch(error){
    if(!finishing&&typeof id==='string')try{await peer.call(player?'pealayer.update.upload.abort':'controller.artifact.upload.abort',{[player?'operation_id':'transfer_id']:id});}catch{}
    if(finishing)throw Error(`Update outcome not verified; do not replay the mutation: ${error.message}`);
    throw error;
  }
  if(verify)await verifyReplacement(peer,manifest);
  return {state:verify?'completed':'accepted',sha256:manifest.sha256,terminal_verified:verify};
}

export async function verifyReplacement(peer,expected,{timeout=120000,interval=500,stableWindow=12000}={}){
  const deadline=Date.now()+timeout;let stable=0;
  while(Date.now()<deadline){
    try{
      // A restarted registered process has a new target revision. Refresh only
      // for read-only verification; never replay transfer or install commands.
      if(peer.target.startsWith('authority:')){
        const targets=(await peer.rpc('pealayer.update.targets')).targets;
        const target=targets.find(value=>value.id.startsWith('authority:')&&value.origin===peer.selectedOrigin);
        if(!target)throw Error('Update authority endpoint changed');
        peer.target=target.id;
      }else await peer.select();
      const value=await peer.manifest();
      let committed=peer.product==='pealayer'&&value.health_state==='healthy';
      if(peer.product==='pccontroller'&&peer.pendingOperation){const status=await peer.call('controller.update.status',{id:peer.pendingOperation});committed=status.id===peer.pendingOperation&&status.state==='completed'&&status.artifact_sha256===expected.sha256;}
      if(value.sha256===expected.sha256&&value.size===expected.size&&committed){
        stable ||= Date.now();if(Date.now()-stable>=stableWindow)return;
      }else stable=0;
    }catch{stable=0;}
    await new Promise(resolve=>setTimeout(resolve,interval));
  }
  throw Error('Expected healthy replacement was not verified; outcome unknown, no mutation retried');
}

export async function source(peer){
  const manifest=await peer.manifest();identity(manifest);
  let transfer;
  if(peer.product==='pccontroller'&&!peer.bundleAssetId){
    transfer=await peer.call('controller.artifact.download.begin',{kind:'host-executable',sha256:manifest.sha256});
    if(transfer.sha256!==manifest.sha256||transfer.bytes_total!==manifest.size)throw Error('Source download identity mismatch');
  }
  let closed=false;
  const close=async()=>{if(transfer&&!closed){await peer.call('controller.artifact.download.close',{transfer_id:transfer.transfer_id});closed=true;}};
  return {manifest,async read(offset,length){
    const value=peer.bundleAssetId?await peer.bundleRead(manifest.sha256,offset,length):await peer.call(peer.product==='pealayer'?'pealayer.update.artifact.read':'controller.artifact.download.chunk',peer.product==='pealayer'?{sha256:manifest.sha256,offset,length}:{transfer_id:transfer.transfer_id,offset,length});
    const bytes=Buffer.from(value.data??'','base64');
    if(value.sha256!==manifest.sha256||value.offset!==offset||value.next_offset!==offset+bytes.length||(value.size??value.bytes_total)!==manifest.size||bytes.length!==length)throw Error('Source chunk acknowledgement mismatch');
    if(offset+bytes.length===manifest.size)await close();
    return bytes;
  },close};
}

async function main(args){
  const action=args.shift(),options={};
  for(let i=0;i<args.length;i+=2){if(!args[i].startsWith('--')||!args[i+1]||Object.hasOwn(options,args[i]))throw Error('Options require unique --name value pairs');options[args[i]]=args[i+1];}
  const allowed=new Set(['--via','--gateway','--target','--file','--manifest','--output','--source-via','--source-gateway','--source-target']);
  for(const key of Object.keys(options))if(!allowed.has(key))throw Error(`Unknown update option ${key}`);
  if(action==='push'&&(!options['--file']||!options['--manifest']))throw Error('Push requires --file and --manifest');
  if(action==='pull'&&!options['--output'])throw Error('Pull requires an explicit --output path');
  if(action==='targets'){const peer=new SoftwarePeer(options['--via'],options['--gateway']);const targets=await peer.targets();console.log(JSON.stringify({targets,errors:peer.targetErrors},null,2));return;}
  const peer=await new SoftwarePeer(options['--via'],options['--gateway'],options['--target']).select();
  if(action==='push'){
    const metadata=JSON.parse(await readFile(options['--manifest'],'utf8'));
    const file=await open(options['--file'],'r');
    try{if((await file.stat()).size!==metadata.size)throw Error('Supplied file size does not match its manifest');const result=await push(peer,metadata,async(offset,length)=>{const bytes=Buffer.alloc(length);const read=await file.read(bytes,0,length,offset);return bytes.subarray(0,read.bytesRead);});console.log(JSON.stringify(result,null,2));}finally{await file.close();}
  }else if(action==='copy'){
    const from=await new SoftwarePeer(options['--source-via'],options['--source-gateway'],options['--source-target']).select();
    const input=await source(from);
    try{console.log(JSON.stringify(await push(peer,input.manifest,input.read),null,2));}finally{await input.close();}
  }else if(action==='pull'){
    const input=await source(peer),part=options['--output']+'.partial-'+randomUUID();let file;
    try{file=await open(part,'wx');const hash=createHash('sha256');for(let offset=0;offset<input.manifest.size;){const bytes=await input.read(offset,Math.min(CHUNK_BYTES,input.manifest.size-offset));hash.update(bytes);await file.write(bytes);offset+=bytes.length;}if(hash.digest('hex')!==input.manifest.sha256)throw Error('Downloaded artifact SHA-256 mismatch');await file.sync();await file.close();file=null;await link(part,options['--output']);await unlink(part);console.log(JSON.stringify(input.manifest,null,2));}
    finally{if(file)await file.close();await unlink(part).catch(()=>{});await input.close();}
  }else throw Error('Use targets, push, pull or copy; choose --via URL --gateway pealayer|pccontroller --target ID. Push requires --file and --manifest; pull requires --output; copy requires --source-via, --source-gateway and --source-target.');
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)main(process.argv.slice(2)).catch(error=>{console.error(`Update failed: ${error.message}`);process.exitCode=1;});
