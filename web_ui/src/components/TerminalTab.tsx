import React, {useEffect, useRef, useState} from 'react';
import {Alert, Button, Dropdown, Input, Space, Tag, Typography} from 'antd';
import {CloseOutlined, CodeOutlined, CopyOutlined, LinkOutlined, PlusOutlined, ReloadOutlined, DisconnectOutlined} from '@ant-design/icons';
import './terminal.css';
import {terminalKey} from '../terminalKeys';
type Session={id:number;target:string;phase:string;rows:number;cols:number};
type Run={text:string;fg:string|null;fg_role:'accent'|null;bg:string|null;bold:boolean;inverse:boolean;underline:boolean};
type Snapshot={owner_id:string;sessions:Session[];profiles:{id:string;name:string;ready:boolean}[];screen:{id:number;lines:Run[][];cursor:[number,number];hide_cursor:boolean;alternate:boolean;bracketed_paste:boolean;mouse:boolean;application_cursor:boolean}|null;cursor_style:string;cursor_blink:boolean;font_size:number;accent_commands:boolean;system_shell_available:boolean};
/** Frontend only. The native host owns sessions, profiles, parser and history. */
export function TerminalTab({apiBaseUrl}:{apiBaseUrl:string}) {
  const [draft,setDraft]=useState(''),[key,setKey]=useState(''),[state,setState]=useState<Snapshot|null>(null),[error,setError]=useState(''),[line,setLine]=useState('');
  const screen=useRef<HTMLDivElement>(null),owner=useRef('');
  const queue=useRef<Promise<void>>(Promise.resolve());
  const generation=useRef(0);
  const base=apiBaseUrl.replace(/\/$/,'');
  async function command(operation:object) {
    const response=await fetch(`${base}/api/terminal/control`,{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${key}`},body:JSON.stringify(operation),cache:'no-store'});
    const result=await response.json();if(!response.ok)throw new Error(result.error||'Terminal request failed');
    if(owner.current && owner.current!==result.owner_id)throw new Error('Terminal owner changed. Pair with the host again.');
    owner.current=result.owner_id;setError('');
  }
  function send(operation:object){const issued=generation.current;queue.current=queue.current.then(async()=>{if(issued===generation.current)await command(operation);}).catch(e=>setError(String(e.message||e)));}
  useEffect(()=>{
    generation.current++;
    if(!key)return;
    let alive=true;const controller=new AbortController();let revision=0;
    void (async()=>{while(alive){
      try {
        const response=await fetch(`${base}/api/terminal/events?after=${revision}`,{headers:{Authorization:`Bearer ${key}`},cache:'no-store',signal:controller.signal});
        const result=await response.json();if(!response.ok)throw new Error(result.error||'Terminal access denied');
        if(!alive)break;
        if(result.state?.owner_id){if(owner.current && owner.current!==result.state.owner_id)throw new Error('Terminal host changed; pair again');owner.current=result.state.owner_id;setState(result.state);}
        revision=result.revision;
      }catch(e){if(alive)setError(String((e as Error).message));break;}
    }})();
    return()=>{alive=false;generation.current++;controller.abort();};
  },[base,key]);
  const selected=state?.sessions.find(s=>s.id===state.screen?.id);
  const core=selected?.target==='portable_shell';
  const role=(color:string|null)=>core?({'#e6e9ef':'var(--text)','#97a0af':'var(--muted)','#181a1f':'var(--canvas)','#242932':'var(--surface-1)','#303743':'var(--surface-2)','#d3dae6':'var(--accent)','#7ee0a3':'var(--green)','#a6da95':'var(--green)','#ffcc66':'var(--amber)','#ff6b7a':'var(--red)'}[color||'']||color||undefined):(color||undefined);
  const foreground=(run:Run)=>core&&run.bg==='#d3dae6'?'var(--accent-text)':state?.accent_commands&&run.fg_role==='accent'?'var(--accent)':role(run.fg);
  const ink=(run:Run)=>run.inverse?role(run.bg):foreground(run);
  function paste(value:string){
    if(new TextEncoder().encode(value).length>16384){setError('Paste is limited to 16 KiB; nothing was sent.');return;}
    if(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/.test(value)){setError('Paste contains control codes; nothing was sent.');return;}
    if(/[\r\n]/.test(value)&&!state?.screen?.bracketed_paste){setError('Multiline paste requires bracketed-paste support; nothing was sent.');return;}
    input(state?.screen?.bracketed_paste?`\x1b[200~${value}\x1b[201~`:value);
  }
  function input(data:string){if(selected && data)send({operation:'input',id:selected.id,data});}
  function mouse(x:number,y:number,code:number,released=false){
    const element=screen.current;if(!element||!selected)return;
    const rect=element.getBoundingClientRect(),style=getComputedStyle(element);
    const probe=element.querySelector('.terminal-cell-probe')!.getBoundingClientRect().width;
    const col=Math.max(1,Math.min(selected.cols,Math.floor((x-rect.left+element.scrollLeft-12)/probe)+1));
    const row=Math.max(1,Math.min(selected.rows,Math.floor((y-rect.top+element.scrollTop-12)/parseFloat(style.lineHeight))+1));
    input(`\x1b[<${code};${col};${row}${released?'m':'M'}`);
  }
  function keydown(e:React.KeyboardEvent){
    if(!selected||selected.phase!=='ready'||selected.target==='pc_controller')return;
    if(e.ctrlKey&&e.key.toLowerCase()==='c'&&window.getSelection()?.toString())return;
    if(e.ctrlKey&&e.shiftKey&&(e.key.toLowerCase()==='c'||e.key.toLowerCase()==='v'))return;
    const data=terminalKey(e,state?.screen?.application_cursor);
    if(data!==null){e.preventDefault();e.stopPropagation();input(data);}
  }
  if(!key)return <div className="terminal-pair"><CodeOutlined/><Typography.Title level={3}>Host terminal</Typography.Title><Typography.Paragraph type="secondary">View and control the same sessions as the desktop. Enable paired access in the host’s Terminal preferences, then copy an access key from its terminal menu. Keys expire after eight hours and are never saved by this page.</Typography.Paragraph><Space.Compact><Input.Password value={draft} onChange={e=>setDraft(e.target.value)} autoComplete="off" placeholder="Access key" aria-label="Terminal access key"/><Button icon={<LinkOutlined/>} type="primary" disabled={draft.length!==64} onClick={()=>{owner.current='';setKey(draft);setDraft('');}}>Connect</Button></Space.Compact></div>;
  return <section className="host-terminal">
    <div className="terminal-session-bar" role="tablist" aria-label="Host terminal sessions">
      {state?.sessions.map(s=><div className={`terminal-session ${selected?.id===s.id?'is-active':''}`} key={s.id}>
        <button role="tab" aria-selected={selected?.id===s.id} onClick={()=>send({operation:'select',id:s.id})}><CodeOutlined/><span>{s.target==='portable_shell'?'Pealayer Shell':s.target==='system_shell'?'System shell':'PCController'}</span><Tag bordered={false}>{s.phase}</Tag></button>
        <Button type="text" icon={<CloseOutlined/>} aria-label={`Close session ${s.id}`} onClick={()=>send({operation:'close',id:s.id})}/></div>)}
      <Dropdown trigger={['click']} menu={{items:[{key:'portable_shell',icon:<CodeOutlined/>,label:'Pealayer Shell'},{key:'pc_controller',icon:<LinkOutlined/>,label:'PCController'},
        ...(state?.system_shell_available?[{key:'system_shell',icon:<CodeOutlined/>,label:'Host system shell'},...(state.profiles||[]).map(p=>({key:`profile:${p.id}`,icon:<CodeOutlined/>,label:p.name,disabled:!p.ready}))]:[])],
        onClick:({key})=>send(key.startsWith('profile:')?{operation:'create_profile',profile_id:key.slice(8)}:{operation:'create',target:key})}} disabled={(state?.sessions.length||0)>=4}>
        <Button icon={<PlusOutlined/>}>Session</Button></Dropdown>
      <Space className="terminal-actions"><Button icon={<CopyOutlined/>} onClick={()=>void navigator.clipboard.writeText(state?.screen?.lines.map(row=>row.map(r=>r.text).join('')).join('\n')||'')}>Copy screen</Button><Button icon={<ReloadOutlined/>} onClick={()=>send({operation:'refresh_profiles'})} title="Refresh host profiles"/><Button icon={<DisconnectOutlined/>} onClick={()=>{setKey('');setState(null);owner.current='';}}>Disconnect viewer</Button></Space>
    </div>
    {error&&<Alert type="error" message={error} showIcon closable onClose={()=>setError('')}/>}
    <div ref={screen} className="terminal-grid" tabIndex={0} role="textbox" aria-label="Host terminal" aria-multiline="true" onKeyDown={keydown}
      onPointerDown={e=>{screen.current?.focus();if(state?.screen?.mouse&&state.screen.alternate&&!e.shiftKey){e.preventDefault();e.currentTarget.setPointerCapture(e.pointerId);mouse(e.clientX,e.clientY,e.button===2?2:0);}}}
      onPointerUp={e=>{if(state?.screen?.mouse&&state.screen.alternate&&!e.shiftKey)mouse(e.clientX,e.clientY,e.button===2?2:0,true);}}
      onPointerMove={e=>{if(e.buttons===1&&state?.screen?.mouse&&state.screen.alternate&&!e.shiftKey)mouse(e.clientX,e.clientY,32);}}
      onContextMenu={e=>{if(state?.screen?.mouse&&state.screen.alternate&&!e.shiftKey)e.preventDefault();}}
      onWheel={e=>{if(state?.screen?.mouse&&state.screen.alternate&&!e.shiftKey){mouse(e.clientX,e.clientY,e.deltaY<0?64:65);}else if(selected&&!state?.screen?.alternate){send({operation:'scroll',id:selected.id,lines:e.deltaY<0?3:-3});}}}
      onPaste={e=>{e.preventDefault();paste(e.clipboardData.getData('text'));}}
      style={{fontSize:state?.font_size||13}}>
      <span className="terminal-cell-probe" aria-hidden>0</span>
      {state?.screen?.lines.map((row,i)=><div className="terminal-line" key={i}>{row.map((run,j)=><span key={j} style={{color:ink(run),backgroundColor:run.inverse?foreground(run):role(run.bg),fontWeight:run.bold?600:400,textDecoration:run.underline?'underline':undefined}}>{run.text}</span>)}</div>)}
      {selected?.phase==='ready'&&state?.screen&&!state.screen.hide_cursor&&<span aria-hidden className={`terminal-caret ${state.cursor_style} ${state.cursor_blink?'blink':''}`} style={{top:`calc(12px + ${state.screen.cursor[0]} * 1.5em)`,left:`calc(12px + ${state.screen.cursor[1]}ch)`}}/>}
      {!state?.sessions.length&&<div className="terminal-empty"><CodeOutlined/><p>Open a session to begin</p><Button icon={<PlusOutlined/>} onClick={()=>send({operation:'create',target:'portable_shell'})}>Open Pealayer Shell</Button></div>}
    </div>
    {selected?.target==='pc_controller'&&<Space.Compact><Input value={line} onChange={e=>setLine(e.target.value)} onPressEnter={()=>{input(line);setLine('');}} placeholder="PCController command"/><Button onClick={()=>{input(line);setLine('');}}>Send</Button></Space.Compact>}
    <footer><CodeOutlined/> Shared host sessions <span>{state?.profiles.length||0} host profiles · {selected?`${selected.cols} × ${selected.rows}`:'No session selected'}</span></footer>
  </section>;
}
