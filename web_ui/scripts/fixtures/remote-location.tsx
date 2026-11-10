// Actual component and shared palette; fixture data only, never a live authority.
import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ConfigProvider, theme} from 'antd';
import {RemoteLocationDialog, type RemoteBrowser} from '../../src/components/RemoteLocationDialog';
import palettes from '../../../assets/themes/palettes.json';
import '../../src/styles.css';
import '../../src/remote-location.css';

const query=new URLSearchParams(location.search);
const light=query.get('theme')==='light';
const palette=palettes[query.get('palette')==='native'?'native':'studio'][light?'light':'dark'];
const accent='#8278df';
for(const [key,value] of Object.entries(palette)) document.documentElement.style.setProperty(`--${key}`,value);
document.documentElement.style.setProperty('--accent',accent);
document.documentElement.dataset.theme=light?'light':'dark';
function Fixture(){
  const [state,setState]=useState<RemoteBrowser>({
    visible:true,request_id:1,revision:1,loading:false,target:'https://files.invalid/Films/',use_proxy:false,
    history_back_url:null,history_forward_url:null,auto_next:false,thumbnails:true,sort:'name',descending:false,
    selected:'https://files.invalid/ready.mkv',view_mode:query.get('mode')==='grid'?'grid':'list',inspected:null,
    error:null,route_errors:[],previous_file:null,next_file:null,
    listing:{url:'https://files.invalid/Films/',host:'files.invalid',server:'Review fixture · no live connection',content_type:'text/html',parent_url:'https://files.invalid/',warning:null,file:null,
      entries:[
        {name:'Short films',url:'https://files.invalid/Short/',is_dir:true,playable:false,size_bytes:null,modified:null},
        {name:'A quiet morning — feature film.mkv',url:'https://files.invalid/ready.mkv',is_dir:false,playable:true,size_bytes:1476395008,modified:'2026-01-02'},
        {name:'Fetching a remote preview.mkv',url:'https://files.invalid/pending.mkv',is_dir:false,playable:true,size_bytes:2048000,modified:'2026-01-03'},
        {name:'Unavailable thumbnail — retry is available.mkv',url:'https://files.invalid/unavailable.mkv',is_dir:false,playable:true,size_bytes:4096000,modified:null},
        {name:`${'A very long filename '.repeat(15)}.mkv`,url:'https://files.invalid/long.mkv',is_dir:false,playable:true,size_bytes:1024000,modified:'2026-01-04'},
      ]},
  });
  const command=async(name:string,payload:Record<string,unknown>={})=>{
    setState(previous=>{
      if(name==='pealayer.config.update') return {...previous,view_mode:(payload.remote_folder_view as 'list'|'grid')??previous.view_mode,thumbnails:(payload.remote_folder_thumbnails as boolean)??previous.thumbnails};
      if(name==='pealayer.remote.inspect') return {...previous,inspected:previous.listing?.entries.find(entry=>entry.url===payload.target)??null};
      if(name==='pealayer.remote.select' && !payload.play) return {...previous,selected:String(payload.target)};
      if(name==='pealayer.remote.sort') return {...previous,sort:payload.by as RemoteBrowser['sort'],descending:!!payload.descending};
      return previous;
    });
    return true;
  };
  return <ConfigProvider theme={{algorithm:light?theme.defaultAlgorithm:theme.darkAlgorithm,token:{colorPrimary:accent,colorBgContainer:palette['surface-1'],colorBgElevated:palette['surface-1'],colorBgBase:palette.canvas,colorText:palette.text,colorTextSecondary:palette.muted,colorBorder:palette.line,borderRadius:9,controlHeight:28},components:{Modal:{contentBg:palette['surface-1'],headerBg:'transparent',footerBg:'transparent'}}}}>
    <RemoteLocationDialog state={state} connected base="" sendCmd={command} autoInspect={false}/>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
