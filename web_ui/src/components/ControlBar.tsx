import React, { useEffect, useRef, useState } from 'react';
import { Button, Dropdown, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import { CheckOutlined, DeleteOutlined, EyeInvisibleOutlined, ExpandOutlined, FastBackwardOutlined,
  FastForwardOutlined, MoreOutlined, MutedOutlined, SettingOutlined, SoundOutlined,
  StepBackwardOutlined, StepForwardOutlined, StopOutlined } from '@ant-design/icons';
import { actionIndex, actionLabels, browserActions, defaultLayout, moveAction, orderedActions, patchAction } from '../controlBars';
import type { ControlAction, ControlLayout, ControlSurface } from '../controlBars';
import type { PlayerState } from './RemoteControlTab';
import { PlaybackButton } from './PlaybackButton';
import { MediaTrackSelectors } from './MediaTrackSelectors';
import { VolumeControl } from './VolumeControl';
import { tr, UiLocale } from '../i18n';

const MIME = 'application/x-pealayer-control-action';
export interface ControlBarProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  surface: ControlSurface;
  locale: UiLocale;
  quickSeekSeconds: number;
  onLayoutChange: (surface: ControlSurface, layout: ControlLayout) => Promise<boolean>;
  onPreferences: () => void;
  onFullscreen: () => void;
  middle?: React.ReactNode;
}

/** One customization model for both workspaces. The wrapper owns drag/context
 * interaction even when the inner transport/selector is unavailable. */
export function ControlBar({state,sendCmd,surface,locale,quickSeekSeconds,onLayoutChange,onPreferences,onFullscreen,middle}: ControlBarProps) {
  const authoritative = state.control_bars?.[surface];
  const [draft,setDraft] = useState<ControlLayout>();
  const [saving,setSaving] = useState(false);
  const [dragged,setDragged] = useState<ControlAction>();
  const [removing,setRemoving] = useState(false);
  const [target,setTarget] = useState<{action:ControlAction;after:boolean}>();
  const drag = useRef<ControlAction | undefined>(undefined);
  const suppressClick = useRef(false);
  const layout = draft ?? authoritative ?? defaultLayout(surface);
  const order = orderedActions(layout.order).filter((a) => browserActions.includes(a));
  useEffect(() => {
    if (draft && JSON.stringify(authoritative) === JSON.stringify(draft)) setDraft(undefined);
  },[authoritative,draft]);
  const finishDrag = () => { drag.current=undefined;setDragged(undefined);setRemoving(false);setTarget(undefined); };
  useEffect(() => {
    const escape = (event:KeyboardEvent) => { if(event.key==='Escape') finishDrag(); };
    window.addEventListener('keydown',escape);return()=>window.removeEventListener('keydown',escape);
  },[]);
  const change = async (next:ControlLayout) => {
    if(saving || !authoritative) return;
    setDraft(next);setSaving(true);
    try { if(!await onLayoutChange(surface,next)) setDraft(undefined); }
    finally {setSaving(false);}
  };
  const loaded = Boolean(state.current_video);
  const enabled = (a:ControlAction) => {
    if(a==='fullscreen'||a==='preferences') return true;
    if(a==='previous_chapter'||a==='next_chapter') return loaded&&Boolean(state.seekable)&&Boolean(state.chapters?.length);
    if(a==='back'||a==='forward') return loaded&&(state.playing===false||Boolean(state.seekable));
    if(a==='play_pause') return loaded&&state.playing!==undefined;
    if(a==='mute') return loaded&&state.muted!==undefined;
    return loaded;
  };
  const invoke = (a:ControlAction) => {
    if(!enabled(a)||drag.current||suppressClick.current) return;
    switch(a) {
      case 'play_pause': void sendCmd('toggle_pause');break;
      case 'stop': void sendCmd('stop');break;
      case 'back': case 'forward': void sendCmd('pealayer.transport.nudge',{direction:a==='back'?-1:1});break;
      case 'previous_chapter': void sendCmd('chapter_previous');break;
      case 'next_chapter': void sendCmd('chapter_next');break;
      case 'mute': void sendCmd('set_mute',{muted:!state.muted});break;
      case 'fullscreen': onFullscreen();break;
      case 'preferences': onPreferences();break;
    }
  };
  const icon = (a:ControlAction) => ({
    stop:<StopOutlined/>,back:state.playing===false?<StepBackwardOutlined/>:<FastBackwardOutlined/>,
    forward:state.playing===false?<StepForwardOutlined/>:<FastForwardOutlined/>,
    previous_chapter:<StepBackwardOutlined/>,next_chapter:<StepForwardOutlined/>,
    fullscreen:<ExpandOutlined/>,mute:state.muted?<MutedOutlined/>:<SoundOutlined/>,preferences:<SettingOutlined/>,
  } as Partial<Record<ControlAction,React.ReactNode>>)[a];
  const label = (a:ControlAction) => tr(locale,a==='play_pause'?(state.playing?'Pause':'Play'):a==='mute'&&state.muted?'Unmute':actionLabels[a]);
  const menu = (action?:ControlAction):MenuProps => {
    const peers=action?order.filter((a)=>!layout.hidden[actionIndex(a)]&&layout.end[actionIndex(a)]===layout.end[actionIndex(action)]):[];
    const index=action?peers.indexOf(action):-1;
    return {items:[
      ...(action?[
        {key:'caption',label:label(action),icon:icon(action),disabled:true},
        ...(!action.endsWith('_track')?[{key:'invoke',label:label(action),icon:icon(action),disabled:!enabled(action)}]:[]),
        {type:'divider' as const},
        {key:'start',label:tr(locale,'Place at start'),icon:!layout.end[actionIndex(action)]?<CheckOutlined/>:undefined,disabled:saving},
        {key:'end',label:tr(locale,'Place at end'),icon:layout.end[actionIndex(action)]?<CheckOutlined/>:undefined,disabled:saving},
        {key:'earlier',label:tr(locale,'Move earlier'),disabled:saving||index<=0},
        {key:'later',label:tr(locale,'Move later'),disabled:saving||index>=peers.length-1},
        {type:'divider' as const},
        {key:'hide',label:<>{tr(locale,'Hide')} {icon(action)} {label(action)}</>,icon:<EyeInvisibleOutlined/>,disabled:saving},
        {type:'divider' as const},
      ]:[]),
      {key:'visibility',label:tr(locale,'Show / hide actions'),children:order.map((a)=>({
        key:a,label:label(a),icon:layout.hidden[actionIndex(a)]?icon(a):<CheckOutlined/>,disabled:saving||!authoritative
      }))},
      {key:'reset',label:tr(locale,'Reset toolbar'),disabled:saving||!authoritative},
    ],onClick:({key})=>{
      if(key==='invoke'&&action){invoke(action);return;}
      if(key==='reset')void change(defaultLayout(surface));
      else if(action&&key==='hide')void change(patchAction(layout,action,'hidden',true));
      else if(action&&(key==='start'||key==='end'))void change(patchAction(layout,action,'end',key==='end'));
      else if(action&&(key==='earlier'||key==='later')){
        const neighbor=peers[index+(key==='earlier'?-1:1)];
        if(neighbor)void change(moveAction(layout,action,neighbor,key==='later'));
      }else if(browserActions.includes(key as ControlAction)){
        const a=key as ControlAction;void change(patchAction(layout,a,'hidden',!layout.hidden[actionIndex(a)]));
      }
    }};
  };
  const ownsDrag = (event:React.DragEvent) => Boolean(drag.current)&&event.dataTransfer.types.includes(MIME);
  const item = (a:ControlAction) => <Dropdown key={a} trigger={['contextMenu']} menu={menu(a)}>
    <span className={`control-bar__action${dragged===a?' is-dragging':''}${target?.action===a?` is-drop-${target.after?'after':'before'}`:''}`}
      draggable={!saving&&Boolean(authoritative)}
      onPointerDown={(event)=>{if(event.button===0)suppressClick.current=false;}}
      onClickCapture={(event)=>{if(suppressClick.current||drag.current){event.preventDefault();event.stopPropagation();}}}
      onDragStart={(event)=>{
        const rect=event.currentTarget.getBoundingClientRect();
        event.dataTransfer.setData(MIME,JSON.stringify({surface,action:a}));event.dataTransfer.effectAllowed='move';
        event.dataTransfer.setDragImage(event.currentTarget,event.clientX-rect.left,event.clientY-rect.top);
        drag.current=a;suppressClick.current=true;setDragged(a);
      }}
      onDragEnd={finishDrag}
      onDragOver={(event)=>{
        if(!ownsDrag(event)||drag.current===a)return;
        event.preventDefault();event.stopPropagation();
        const rect=event.currentTarget.getBoundingClientRect(),rtl=getComputedStyle(event.currentTarget).direction==='rtl';
        setTarget({action:a,after:(event.clientX>rect.left+rect.width/2)!==rtl});setRemoving(false);
      }}
      onDragLeave={()=>setTarget(undefined)}
      onDrop={(event)=>{
        if(!ownsDrag(event)||!drag.current)return;
        event.preventDefault();event.stopPropagation();
        const rect=event.currentTarget.getBoundingClientRect(),rtl=getComputedStyle(event.currentTarget).direction==='rtl';
        void change(moveAction(layout,drag.current,a,(event.clientX>rect.left+rect.width/2)!==rtl));finishDrag();
      }}>
      {a==='play_pause'?<PlaybackButton className={`control-bar__play${state.playing?' transport-action--active':''}`} playing={state.playing} loaded={loaded} locale={locale} onClick={()=>invoke(a)}/>
        :a.endsWith('_track')?<MediaTrackSelectors state={state} sendCmd={sendCmd} locale={locale} compact only={a.replace('_track','') as 'video'|'audio'|'subtitle'}/>
        :<Tooltip title={a==='back'||a==='forward'?`${label(a)} · ${state.playing===false?tr(locale,'frames'):`${quickSeekSeconds}s`}`:label(a)}>
          <Button className={`transport-action${a==='stop'?' transport-action--stop':a==='mute'&&state.muted?' transport-action--pause transport-action--active':''}`}
            aria-label={label(a)} disabled={!enabled(a)} icon={icon(a)} onClick={()=>invoke(a)}/>
        </Tooltip>}
    </span>
  </Dropdown>;
  const group=(end:boolean)=><div className={`control-bar__group control-bar__group--${end?'end':'start'}`}
    onDragOver={(event)=>{if(ownsDrag(event)){event.preventDefault();setTarget(undefined);setRemoving(false);}}}
    onDrop={(event)=>{if(ownsDrag(event)&&drag.current){event.preventDefault();void change(patchAction(layout,drag.current,'end',end));finishDrag();}}}>
    <div className="control-bar__strip">{order.filter((a)=>!layout.hidden[actionIndex(a)]&&layout.end[actionIndex(a)]===end).map(item)}</div>
    {end&&<Dropdown trigger={['click','contextMenu']} menu={menu()}><span className={`control-bar__overflow${removing?' is-removing':''}`}
      onDragOver={(event)=>{if(ownsDrag(event)){event.preventDefault();event.stopPropagation();setRemoving(true);setTarget(undefined);}}}
      onDragLeave={()=>setRemoving(false)}
      onDrop={(event)=>{if(ownsDrag(event)&&drag.current){event.preventDefault();event.stopPropagation();void change(patchAction(layout,drag.current,'hidden',true));finishDrag();}}}>
      <Button aria-label={tr(locale,dragged?'Hide':'Control bar')} icon={dragged?<DeleteOutlined/>:<MoreOutlined/>}/>
    </span></Dropdown>}
  </div>;
  return <div className="control-bar" aria-label={tr(locale,'Control bar')}>
    {group(false)}{middle}{group(true)}
    <VolumeControl className="control-bar__volume" state={state} sendCmd={sendCmd} locale={locale} showMute={false}/>
  </div>;
}
