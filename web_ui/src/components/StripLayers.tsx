import {useState} from 'react';
import {Button,Input,InputNumber,Select,Slider,Space,Switch} from 'antd';
import {ArrowUpOutlined,ArrowDownOutlined,CopyOutlined,DeleteOutlined,PlusOutlined} from '@ant-design/icons';
import {ColorPicker} from './ColorPicker';
import {TimeValueField} from './TimeValueField';
import {tr,type UiLocale} from '../i18n';
import {newStripLayer,stripHex,type StripProgram,type StripLayer} from '../stripProgram';

export function StripLayers({program,pixels,onChange,locale,disabled=false}:{program:StripProgram;pixels:number;onChange:(p:StripProgram)=>void;locale:UiLocale;disabled?:boolean}) {
  const [selected,setSelected]=useState(''),[selectedLayer,setSelectedLayer]=useState('');
  const segments=program.segments??[],layers=program.layers??[];
  const segment=segments.find(s=>s.id===selected)??segments[0],layer=layers.find(l=>l.id===selectedLayer)??layers[0];
  if(!segment||!layer)return null;
  const nextID=(prefix:string,ids:string[])=>{let i=1;while(ids.includes(`${prefix}-${i}`))i++;return `${prefix}-${i}`;};
  const updateLayer=(patch:Partial<StripLayer>)=>onChange({...program,layers:layers.map(l=>l.id===layer.id?{...l,...patch}:l)});
  const selection=(indices:number[])=>onChange({...program,segments:segments.map(s=>s.id===segment.id?{...s,pixels:indices}:s)});
  const move=(delta:number)=>{const index=layers.indexOf(layer),to=index+delta;if(to<0||to>=layers.length)return;const copy=[...layers];[copy[index],copy[to]]=[copy[to],copy[index]];onChange({...program,layers:copy});};
  return <>
    <label>{tr(locale,'Density')}<InputNumber min={1} max={1000} suffix="px/m" value={program.pixels_per_meter} onChange={v=>onChange({...program,pixels_per_meter:v??60})}/></label>
    <header><strong>{tr(locale,'Segments')}</strong><Space wrap>
      <Select value={segment.id} options={segments.map(s=>({value:s.id,label:s.name}))} onChange={setSelected}/>
      <Button icon={<PlusOutlined/>} disabled={segments.length>=64} onClick={()=>{const id=nextID('segment',segments.map(s=>s.id));onChange({...program,segments:[...segments,{id,name:tr(locale,'Segment'),pixels:[0]}]});setSelected(id);}}>{tr(locale,'Add segment')}</Button>
      <Button icon={<DeleteOutlined/>} aria-label={tr(locale,'Delete segment')} disabled={segments.length<=1||layers.some(l=>l.segment_id===segment.id)} onClick={()=>onChange({...program,segments:segments.filter(s=>s.id!==segment.id)})}/>
    </Space></header>
    <Input aria-label={tr(locale,'Segment name')} value={segment.name} maxLength={64} onChange={e=>onChange({...program,segments:segments.map(s=>s.id===segment.id?{...s,name:e.target.value}:s)})}/>
    <Space wrap><Button onClick={()=>selection(Array.from({length:pixels},(_,i)=>i))}>{tr(locale,'All pixels')}</Button><Button onClick={()=>selection([])}>{tr(locale,'Clear selection')}</Button><Button onClick={()=>selection(Array.from({length:pixels},(_,i)=>i).filter(i=>!segment.pixels.includes(i)))}>{tr(locale,'Invert selection')}</Button><Button onClick={()=>selection([...segment.pixels].reverse())}>{tr(locale,'Reverse order')}</Button></Space>
    <div className="strip-designer__selection">{Array.from({length:Math.min(100,Math.max(1,pixels))},(_,i)=><button type="button" key={i} disabled={disabled} aria-pressed={segment.pixels.includes(i)} onClick={()=>selection(segment.pixels.includes(i)?segment.pixels.filter(v=>v!==i):[...segment.pixels,i])}>{i+1}</button>)}</div>
    <header><strong>{tr(locale,'Layers')}</strong><Space wrap>
      <Select value={layer.id} options={layers.map(l=>({value:l.id,label:l.name}))} onChange={setSelectedLayer}/>
      <Button icon={<PlusOutlined/>} disabled={layers.length>=64} onClick={()=>{const id=nextID('layer',layers.map(l=>l.id));onChange({...program,layers:[...layers,newStripLayer(id,segment.id)]});setSelectedLayer(id);}}>{tr(locale,'Add layer')}</Button>
      <Button icon={<CopyOutlined/>} aria-label={tr(locale,'Duplicate')} disabled={layers.length>=64} onClick={()=>{const id=nextID('layer',layers.map(l=>l.id));onChange({...program,layers:[...layers,{...layer,id}]});setSelectedLayer(id);}}/>
      <Button icon={<ArrowUpOutlined/>} aria-label={tr(locale,'Move up')} disabled={layers.indexOf(layer)===0} onClick={()=>move(-1)}/><Button icon={<ArrowDownOutlined/>} aria-label={tr(locale,'Move down')} disabled={layers.indexOf(layer)===layers.length-1} onClick={()=>move(1)}/>
      <Button icon={<DeleteOutlined/>} aria-label={tr(locale,'Delete layer')} disabled={layers.length<=1} onClick={()=>onChange({...program,layers:layers.filter(l=>l.id!==layer.id)})}/>
    </Space></header>
    <Space.Compact block><Switch checked={layer.enabled} checkedChildren={tr(locale,'Enabled')} unCheckedChildren={tr(locale,'Disabled')} onChange={enabled=>updateLayer({enabled})}/><Input aria-label={tr(locale,'Layer name')} value={layer.name} maxLength={64} onChange={e=>updateLayer({name:e.target.value})}/></Space.Compact>
    <div className="strip-designer__fields">
      <label>{tr(locale,'Segment')}<Select value={layer.segment_id} options={segments.map(s=>({value:s.id,label:s.name}))} onChange={segment_id=>updateLayer({segment_id})}/></label>
      <label>{tr(locale,'Pattern')}<Select value={layer.primitive} options={[['solid','Solid'],['gradient','Gradient'],['ramp','Fade'],['blink','Blink / Strobe'],['breathe','Breathing']].map(([value,label])=>({value,label:tr(locale,label)}))} onChange={primitive=>updateLayer({primitive})}/></label>
      {(['primary',...(layer.primitive==='solid'?[]:['secondary'])] as Array<'primary'|'secondary'>).map(key=><label key={key}>{tr(locale,key==='primary'?'Color':'Secondary color')}<ColorPicker locale={locale} disabledAlpha value={stripHex(layer[key])} onChangeComplete={c=>{const v=c.toRgb();updateLayer({[key]:{red:Math.round(v.r),green:Math.round(v.g),blue:Math.round(v.b)}});}}/></label>)}
      {!['solid','gradient'].includes(layer.primitive)&&<><label>{tr(locale,'Cycle')}<TimeValueField min={50} max={3600000} value={layer.period_ms} onChange={period_ms=>updateLayer({period_ms})}/></label><label>{tr(locale,'Phase offset')}<TimeValueField max={layer.period_ms-1} value={layer.phase_ms} onChange={phase_ms=>updateLayer({phase_ms})}/></label></>}
      {layer.primitive==='blink'&&<label>{tr(locale,'Duty cycle')}<Slider min={0} max={100} value={layer.duty_percent} onChange={duty_percent=>updateLayer({duty_percent})}/></label>}
      <label>{tr(locale,'Easing')}<Select value={layer.easing} options={['linear','ease-in','ease-out','ease-in-out'].map(value=>({value,label:value}))} onChange={easing=>updateLayer({easing})}/></label>
      <label>{tr(locale,'Reverse')}<Switch checked={layer.reverse} onChange={reverse=>updateLayer({reverse})}/></label>
    </div>
  </>;
}
