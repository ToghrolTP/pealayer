import { useEffect, useState } from 'react';
import { Alert, Button, ConfigProvider, InputNumber, Select, Slider, Space, Tooltip } from 'antd';
import { DeleteOutlined, PauseOutlined, PlayCircleOutlined, PlusOutlined } from '@ant-design/icons';
import { ColorPicker } from './ColorPicker';
import { TimeValueField } from './TimeValueField';
import { tr, type UiLocale } from '../i18n';
import { stripFrame, stripHex, stripPreset, stripProgramError, stripLayersEditable, newStripDesign, type StripProgram } from '../stripProgram';
import { StripLayers } from './StripLayers';
import './stripDesigner.css';

interface Props { value: string; pixels: number; onChange: (json: string)=>void; locale: UiLocale; disabled?: boolean }
// Shared by Effects Library and Studio: no duplicate editor or transport state.
export function StripDesigner({value,pixels,onChange,locale,disabled=false}:Props) {
  const [playing,setPlaying]=useState(false), [phase,setPhase]=useState(0);
  let program: StripProgram | null = null;
  try { program=JSON.parse(value); } catch { /* Keep the user's technical draft intact. */ }
  const error=stripProgramError(program,pixels);
  const layered=program?.primitive==='layers';
  const editableLayers=stripLayersEditable(program);
  const period=editableLayers?Math.max(50,...program!.layers!.map(l=>l.period_ms)):(!error?program?.period_ms:3000) || 3000;
  useEffect(()=>{
    if (!playing || disabled || error) return;
    const start=performance.now()-phase; let frame=0,last=0;
    const tick=(now:number)=>{ if(now-last>=33) { setPhase((now-start)%period); last=now; } frame=requestAnimationFrame(tick); };
    frame=requestAnimationFrame(tick); return ()=>cancelAnimationFrame(frame);
    // phase is an initial playback offset, not a reason to restart the clock.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  },[playing,disabled,error,period]);
  const update=(patch:Partial<StripProgram>)=>onChange(JSON.stringify({...program,...patch},null,2));
  const colors=program&&!error?stripFrame(program,pixels,phase):[];
  return <ConfigProvider componentDisabled={disabled}><section className="strip-designer" aria-label={tr(locale,'Strip designer')}>
    <Space wrap>
      {!program?.primitive&&<Button icon={<PlusOutlined/>} onClick={()=>onChange(JSON.stringify(newStripDesign(pixels),null,2))}>{tr(locale,'Segmented design')}</Button>}
      <Select<string> value={undefined} disabled={disabled} placeholder={tr(locale,'Examples…')} aria-label={tr(locale,'Examples…')} style={{minWidth:180}}
        options={[['fade','Fade'],['pulse','Pulse'],['alternating-zones','Police red / blue'],['thunder','White thunder'],['converging-points','Converging red dots']].map(([value,label])=>({value,label:tr(locale,label)}))}
        onChange={kind=>{setPlaying(false);setPhase(0);onChange(JSON.stringify(stripPreset(kind),null,2));}} />
    </Space>
    {error&&<Alert type="warning" title={tr(locale,error)} />}
    {editableLayers&&program&&<StripLayers program={program} pixels={pixels} locale={locale} disabled={disabled} onChange={p=>onChange(JSON.stringify(p,null,2))}/>}
    {!error && program && <>
      {!layered&&<div className="strip-designer__fields">
        <label>{tr(locale,'Cycle')}<TimeValueField min={50} max={3_600_000} value={program.period_ms} onChange={period_ms=>update({period_ms:Math.round(period_ms)})} /></label>
        {(['primary',...(program.primitive==='alternating-zones'?['secondary']:[])] as const).map(key=><label key={key}>{tr(locale,key==='primary'?'Color':'Secondary color')}<ColorPicker locale={locale} disabledAlpha value={stripHex(program![key as 'primary'|'secondary']??{red:0,green:0,blue:0})} onChangeComplete={c=>{const rgb=c.toRgb();update({[key]:{red:Math.round(rgb.r),green:Math.round(rgb.g),blue:Math.round(rgb.b)}});}} /></label>)}
        {program.primitive==='converging-points'&&<label>{tr(locale,'Tail pixels')}<InputNumber min={0} max={100} value={program.tail_pixels??0} onChange={v=>update({tail_pixels:v??0})} /></label>}
        {program.primitive==='alternating-zones'&&<>
          <label>{tr(locale,'Flash interval')}<TimeValueField min={10} max={program.period_ms} value={program.step_ms!} onChange={step_ms=>update({step_ms:Math.round(step_ms)})}/></label>
          <label>{tr(locale,'Swap after flashes')}<InputNumber min={1} max={1024} value={program.swap_after_steps} onChange={v=>update({swap_after_steps:v??1})}/></label>
          <label>{tr(locale,'Dim brightness')}<Slider min={0} max={255} value={program.dim_intensity??0} onChange={dim_intensity=>update({dim_intensity})}/></label>
        </>}
      </div>}
      {program.primitive==='envelope'&&<div className="strip-designer__envelope">
        <header><strong>{tr(locale,'Brightness envelope')}</strong><Button icon={<PlusOutlined/>} disabled={disabled||program.envelope!.length>=256||program.envelope![program.envelope!.length-1].at_ms>=period-1} onClick={()=>update({envelope:[...program!.envelope!,{at_ms:period-1,intensity:0}]})}>{tr(locale,'Add point')}</Button></header>
        <svg viewBox="0 0 600 100" preserveAspectRatio="none" role="img" aria-label={tr(locale,'Brightness envelope')}><polyline points={program.envelope!.map(p=>`${p.at_ms/period*600},${98-p.intensity/255*96}`).join(' ')} /></svg>
        {program.envelope!.map((p,i)=><div className="strip-designer__point" key={i}>
          <TimeValueField min={i?program!.envelope![i-1].at_ms+1:0} max={i+1<program!.envelope!.length?program!.envelope![i+1].at_ms-1:period-1} value={p.at_ms} onChange={at_ms=>update({envelope:program!.envelope!.map((v,j)=>i===j?{...v,at_ms:Math.round(at_ms)}:v)})}/>
          <Slider min={0} max={255} value={p.intensity} onChange={intensity=>update({envelope:program!.envelope!.map((v,j)=>i===j?{...v,intensity}:v)})}/>
          <Button aria-label={tr(locale,'Delete point')} icon={<DeleteOutlined/>} disabled={disabled||program!.envelope!.length<=2} onClick={()=>update({envelope:program!.envelope!.filter((_,j)=>i!==j)})}/>
        </div>)}
      </div>}
      <header><strong>{tr(locale,'Offline preview')}</strong><Tooltip title={tr(locale,'Animate preview without sending to hardware')}><Button disabled={disabled} icon={playing?<PauseOutlined/>:<PlayCircleOutlined/>} onClick={()=>setPlaying(!playing)}>{tr(locale,playing?'Pause':'Preview')}</Button></Tooltip></header>
      <Slider min={0} max={period-1} value={Math.floor(phase)} onChange={v=>{setPlaying(false);setPhase(v);}} tooltip={{formatter:v=>`${v} ms`}}/>
      <div className="strip-designer__pixels">{colors.map((color,i)=><span key={i} style={{background:stripHex(color)}} title={`${i+1} · ${stripHex(color)}`}/>)}</div>
    </>}
  </section></ConfigProvider>;
}
