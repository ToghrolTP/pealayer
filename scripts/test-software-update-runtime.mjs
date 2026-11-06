// Opt-in real-process acceptance, not a production updater or automatic test.
// Start an isolated Pealayer profile connected to an isolated --no-auto host.
import {SoftwarePeer,push} from './software-update.mjs';
import {open,readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';

export async function verifyControllerReplacement({via,candidate,manifestPath}){
  const origin=new URL(via);
  if(origin.protocol!=='http:'||origin.hostname!=='127.0.0.1'||!origin.port||['8080','8787'].includes(origin.port)||origin.pathname!=='/'||origin.username||origin.password||origin.search||origin.hash)throw Error('Choose an explicit isolated loopback fixture port');
  const config=await fetch(new URL('/api/config',origin),{redirect:'error',signal:AbortSignal.timeout(10000)}).then(r=>{if(!r.ok)throw Error('Fixture configuration unavailable');return r.json();});
  if(config.app_name!=='Pealayer mutual-update fixture'||config.auto_connect_hardware!==false)throw Error('Not the isolated player fixture');
  const endpoint=new URL(config.hardware_endpoint.replace(/^pccontroller:/,'http:'));
  if(endpoint.hostname!=='127.0.0.1'||!endpoint.port||['8080','8787'].includes(endpoint.port))throw Error('Fixture must bind only an isolated controller');
  const peer=await new SoftwarePeer(via,'pealayer','controller:self').select();
  // Process identity is an existing read-only controller API, deliberately
  // outside the software-only mutation relay's allowlist.
  const inspectOwner=()=>fetch(new URL('/api/peer/controller',origin),{method:'POST',redirect:'error',signal:AbortSignal.timeout(10000),headers:{'Content-Type':'application/json'},body:JSON.stringify({method:'controller.app.bridge',params:{}})}).then(r=>{if(!r.ok)throw Error('Controller identity unavailable');return r.json();});
  const before=await inspectOwner();
  const beforeManifest=await peer.call('controller.artifact.manifest',{});
  if(beforeManifest.board?.connected!==false||!before.self?.pid)throw Error('Controller is not a disconnected process fixture');
  const manifest=JSON.parse(await readFile(manifestPath,'utf8'));
  const phases=[],call=peer.call.bind(peer);
  peer.call=async(method,params)=>{
    const value=await call(method,params);
    if(method==='controller.update.status'){
      if(value.state==='completed'&&!['completed','current'].includes(value.stage))throw Error('Staging was incorrectly reported as terminal completion');
      const phase=`${value.state}:${value.stage}`;
      if(!phases.includes(phase)){phases.push(phase);console.log(JSON.stringify({observed_state:value.state,stage:value.stage}));}
    }
    return value;
  };
  const file=await open(candidate,'r');
  let result;
  try{
    if((await file.stat()).size!==manifest.size)throw Error('Candidate size mismatch');
    result=await push(peer,manifest,async(offset,length)=>{const data=Buffer.alloc(length);const read=await file.read(data,0,length,offset);return data.subarray(0,read.bytesRead);});
  }finally{await file.close();}
  const after=await inspectOwner();
  const afterManifest=await peer.call('controller.artifact.manifest',{});
  if(afterManifest.board?.connected!==false)throw Error('Hardware isolation lost');
  const identical=beforeManifest.current.host.sha256===manifest.sha256;
  if(identical){
    if(after.self.pid!==before.self.pid||!phases.includes('completed:current'))throw Error('Identical image restarted or was not acknowledged as current');
  }else if(after.self.pid===before.self.pid||!phases.some(value=>value.startsWith('awaiting-'))){
    throw Error('A distinct replacement did not demonstrate pending activation and a changed process');
  }
  return {...result,through_player_port:true,observed_phases:phases,before_pid:before.self.pid,after_pid:after.self.pid,identical};
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  const args=process.argv.slice(2);
  if(args.length!==3)throw Error('Usage: node scripts/test-software-update-runtime.mjs FIXTURE_ORIGIN CANDIDATE_EXE CANDIDATE_MANIFEST');
  verifyControllerReplacement({via:args[0],candidate:args[1],manifestPath:args[2]}).then(result=>console.log(JSON.stringify(result))).catch(error=>{console.error(`Runtime acceptance failed; no mutation retried: ${error.message}`);process.exitCode=1;});
}
