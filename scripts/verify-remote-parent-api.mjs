// Local index fixtures only. Does not open media, save config or command hardware.
import assert from 'node:assert/strict';
import http from 'node:http';
const base=process.env.PEALAYER_TEST_BASE || 'http://127.0.0.1:8080';
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function api(path,body){
  const response=await fetch(`${base}${path}`,body?{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)}:{});
  const value=await response.json();assert.ok(response.ok,`API status ${response.status}`);return value;
}
const state=()=>api('/api/remote/state');
const rpc=(method,params={})=>api('/api/rpc',{jsonrpc:'2.0',id:'parent-verification',method,params});
async function browse(target){
  const before=await state();const result=await rpc('pealayer.remote.browse',{target,use_proxy:false});assert.ok(!result.error);
  for(let n=0;n<180;n++){const value=await state();if(value.request_id>before.request_id&&!value.loading)return value;await delay(50);}
  throw Error('Remote browser did not finish');
}
assert.equal((await state()).visible,false,'Do not replace an open user remote browser');
let parentAvailable=true;
const server=http.createServer((request,response)=>{
  const route=new URL(request.url,'http://fixture.invalid');
  let body='';let status=200;
  if(route.pathname==='/open/' && !parentAvailable){status=403;body='Denied';}
  else if(['/open/','/open/sub/','/locked/sub/'].includes(route.pathname))body=`<title>Index of ${route.pathname}</title>`;
  else{status=403;body='Denied';}
  response.writeHead(status,{'Content-Type':'text/html','Content-Length':Buffer.byteLength(body)});
  response.end(request.method==='HEAD'?undefined:body);
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const host=`http://127.0.0.1:${server.address().port}`;
try{
  let value=await browse(`${host}/locked/sub/`);assert.equal(value.error,null);assert.equal(value.listing.parent_url,null);
  value=await browse(`${host}/open/sub/`);assert.equal(value.error,null);assert.equal(value.listing.parent_url,`${host}/open/`);
  value=await browse(value.listing.parent_url);assert.equal(value.error,null);assert.equal(value.listing.url,`${host}/open/`);assert.deepEqual(value.listing.entries,[]);
  await browse(`${host}/open/sub/`);parentAvailable=false;
  value=await browse(`${host}/open/`);assert.ok(value.error);assert.equal(value.listing.url,`${host}/open/sub/`);assert.equal(value.listing.parent_url,null);
  console.log('Installed API: forbidden parent hidden, empty parent navigable, changed permissions revoke action and preserve current listing');
}finally{
  await rpc('pealayer.remote.close').catch(()=>{});
  server.closeAllConnections();await new Promise(resolve=>server.close(resolve));
}
