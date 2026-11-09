// Real import/editor controls; memory-only transport, never a live controller.
import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ConfigProvider,theme} from 'antd';
import {AssetDropTarget} from '../src/components/AssetDropTarget';
import {AssetImportReview,PendingTimelineChange} from '../src/components/AssetImportReview';
import {SoundEffectFields} from '../src/components/SoundEffectFields';
import {assetDropDestination} from '../src/assetImports';
import type {AssetImportState} from '../src/assetImports';
import type {PlayerState} from '../src/components/RemoteControlTab';
import '../src/styles.css';
import '../src/asset-imports.css';
const owner='00000000-0000-4000-8000-000000000001';
const transfers=new Map<string,{name:string;offset:number}>();
window.fetch=async (input,init)=>{
  const url=new URL(String(input),location.origin);
  if(url.pathname.endsWith('/begin')){const file=JSON.parse(String(init?.body));const id=crypto.randomUUID();transfers.set(id,{name:file.name,offset:0});return Response.json({id,chunk_bytes:512*1024,owner_id:owner});}
  if(url.pathname.endsWith('/chunk')){const transfer=transfers.get(url.searchParams.get('id')!)!;transfer.offset+=(init!.body as Blob).size;return Response.json({offset:transfer.offset});}
  if(url.pathname.endsWith('/finish'))return Response.json({owner_id:owner,path:`/fixture/assets/${transfers.get(url.searchParams.get('id')!)!.name}`});
  if(url.pathname.endsWith('/retain') || init?.method==='DELETE')return Response.json({retained:true});
  throw Error(`Unexpected fixture network request: ${url.pathname}`);
};
function Fixture(){
  const params=new URLSearchParams(location.search);const light=params.has('light');
  document.documentElement.dataset.theme=light?'light':'dark';
  const [commands,setCommands]=useState<unknown[]>([]);const [source,setSource]=useState('');
  const review:AssetImportState={busy:false,reviews:params.has('review')?[{id:owner,name:'Cinema scene.json',kind:'scene',destination:{surface:'timeline',start_time_ms:10429},summary:['relay.5 · On','pwm.0 · 35.000%','relay.6 · Unsampled (unchanged)']}]:[]};
  const send=async (...command:unknown[])=>{setCommands(current=>[...current,command]);return true;};
  return <ConfigProvider theme={{algorithm:light?theme.defaultAlgorithm:theme.darkAlgorithm,token:{colorPrimary:'#8c83df'}}}>
    <main style={{padding:16,minWidth:0,width:'100%',boxSizing:'border-box'}}>
      <AssetDropTarget apiBaseUrl="" sendCmd={send} destination={{surface:'library'}} destinationForDrop={event=>assetDropDestination(event.target,event.clientX,{surface:'library'})}>
        <h2>External assets</h2><div data-asset-lane="controller-effect:audio" data-asset-duration="20000" style={{height:90,border:'1px solid var(--line)',borderRadius:8,padding:8}}>Audio effect lane</div>
      </AssetDropTarget>
      <section style={{marginTop:20}}><SoundEffectFields program={{source}} onChange={patch=>{if(patch.source!==undefined)setSource(patch.source);}} state={{audio_devices:[]} as PlayerState} reference="" apiBaseUrl="" locale="en" sendCmd={send}/></section>
      <AssetImportReview state={review} sendCmd={send} openEffect={()=>{throw Error('Unexpected effect editor');}}/>
      <PendingTimelineChange open={params.has('unsaved')} sendCmd={send} locale="en"/>
      <output id="fixture-state" hidden>{JSON.stringify({commands,source})}</output>
    </main>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
