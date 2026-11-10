// PCController owns persisted programs and rendering on the board. This renderer
// is an offline editor preview only; it never opens a transport or publishes.
export type StripColor = { red: number; green: number; blue: number };
export type EnvelopePoint = { at_ms: number; intensity: number };
export interface StripProgram {
  primitive: string;
  primary: StripColor;
  secondary?: StripColor;
  period_ms: number;
  step_ms?: number;
  swap_after_steps?: number;
  dim_intensity?: number;
  tail_pixels?: number;
  envelope?: EnvelopePoint[];
  pixels_per_meter?: number;
  segments?: StripSegment[];
  layers?: StripLayer[];
}
export interface StripSegment { id:string; name:string; pixels:number[] }
export interface StripLayer { id:string; name:string; segment_id:string; enabled:boolean; primitive:string; primary:StripColor; secondary:StripColor; period_ms:number; phase_ms:number; duty_percent:number; easing:string; reverse:boolean }
export const newStripLayer=(id:string,segment_id:string):StripLayer=>({id,name:'Layer',segment_id,enabled:true,primitive:'solid',primary:{red:0,green:96,blue:255},secondary:{red:0,green:0,blue:0},period_ms:3000,phase_ms:0,duty_percent:50,easing:'linear',reverse:false});
export const newStripDesign=(count:number):StripProgram=>({primitive:'layers',primary:{red:0,green:0,blue:0},period_ms:3000,pixels_per_meter:60,segments:[{id:'all',name:'All pixels',pixels:Array.from({length:count},(_,i)=>i)}],layers:[newStripLayer('layer-1','all')]});
const integer = (n: unknown, min: number, max: number): n is number => typeof n === 'number' && Number.isInteger(n) && n >= min && n <= max;
const rgb = (c: unknown): c is StripColor => !!c && typeof c === 'object' && ['red','green','blue'].every(k => integer((c as Record<string,unknown>)[k],0,255));
// Technical JSON may be invalid while editing. Keep semantic mistakes editable,
// but never hand malformed object shapes to the visual controls.
export function stripLayersEditable(p: StripProgram | null): boolean {
  return !!p && p.primitive === 'layers'
    && Array.isArray(p.segments) && p.segments.length > 0 && p.segments.length <= 64
    && p.segments.every(s => s && typeof s.id === 'string' && typeof s.name === 'string' && Array.isArray(s.pixels) && s.pixels.every(Number.isInteger))
    && Array.isArray(p.layers) && p.layers.length > 0 && p.layers.length <= 64
    && p.layers.every(l => l && ['id','name','segment_id','primitive','easing'].every(k => typeof l[k as keyof StripLayer] === 'string')
      && typeof l.enabled === 'boolean' && typeof l.reverse === 'boolean' && rgb(l.primary) && rgb(l.secondary)
      && [l.period_ms,l.phase_ms,l.duty_percent].every(Number.isFinite));
}
export function stripDraftError(draft: Partial<{programText:string;default_pixels:number;default_fps:number;duration_ms:number}>): string | null {
  if (!integer(draft.default_pixels,1,100) || !integer(draft.default_fps,1,120) || !integer(draft.duration_ms,100,3_600_000)) return 'Invalid strip pixel count, frame rate or duration';
  try { return stripProgramError(JSON.parse(draft.programText ?? ''),draft.default_pixels); }
  catch { return 'Program must be valid JSON'; }
}
export function stripProgramError(value: unknown, count=100): string | null {
  if (!value || typeof value !== 'object') return 'Choose a pattern to start designing';
  const p = value as StripProgram;
  if (!integer(p.period_ms,50,3_600_000)) return 'Cycle must be between 50 ms and 1 hour';
  if(p.primitive==='layers') {
    if(!integer(count,1,100)||!integer(p.pixels_per_meter,1,1000)) return 'Invalid pixel count or density';
    if(!Array.isArray(p.segments)||p.segments.length<1||p.segments.length>64||!Array.isArray(p.layers)||p.layers.length<1||p.layers.length>64) return 'Use 1–64 segments and layers';
    const token=(s:unknown)=>typeof s==='string'&&/^[a-z0-9._-]{1,64}$/.test(s);
    const name=(s:unknown)=>typeof s==='string'&&!!s.trim()&&[...s].length<=64&&!/[\u0000-\u001f\u007f-\u009f]/.test(s);
    const ids=new Set<string>();
    for(const s of p.segments){if(!s||!token(s.id)||ids.has(s.id)||!name(s.name))return 'Segments require unique IDs and names';ids.add(s.id);if(!Array.isArray(s.pixels)||!s.pixels.length||s.pixels.length>count||new Set(s.pixels).size!==s.pixels.length||s.pixels.some(v=>!integer(v,0,count-1)))return 'Segment pixels must be unique and inside the strip';}
    const layers=new Set<string>();
    for(const l of p.layers){if(!l||!token(l.id)||layers.has(l.id)||!name(l.name)||!ids.has(l.segment_id))return 'Layer requires a unique ID, name and existing segment';layers.add(l.id);
      if(!integer(l.period_ms,50,3_600_000)||!integer(l.phase_ms,0,l.period_ms-1)||!integer(l.duty_percent,0,100)||typeof l.enabled!=='boolean'||typeof l.reverse!=='boolean'||!rgb(l.primary)||!rgb(l.secondary))return 'Invalid layer cycle, phase, duty or color';
      if(!['solid','gradient','ramp','blink','breathe'].includes(l.primitive)||!['linear','ease-in','ease-out','ease-in-out'].includes(l.easing))return 'Unsupported layer pattern or easing';}
    return null;
  }
  if (!rgb(p.primary) || (p.secondary !== undefined && !rgb(p.secondary))) return 'Colors must contain red, green and blue values from 0 to 255';
  if (p.dim_intensity !== undefined && !integer(p.dim_intensity,0,255)) return 'Invalid dim brightness';
  if (p.primitive === 'alternating-zones') return integer(p.step_ms,10,p.period_ms) && integer(p.swap_after_steps,1,1024) ? null : 'Invalid flash interval or swap count';
  if (p.primitive === 'converging-points') return p.tail_pixels === undefined || integer(p.tail_pixels,0,100) ? null : 'Invalid tail length';
  if (p.primitive === 'envelope') {
    if (!Array.isArray(p.envelope) || p.envelope.length < 2 || p.envelope.length > 256) return 'Add between 2 and 256 brightness points';
    let previous = -1;
    for (const point of p.envelope) {
      if (!point || !integer(point.at_ms,previous+1,p.period_ms-1) || !integer(point.intensity,0,255)) return 'Envelope times must increase and remain inside the cycle';
      previous = point.at_ms;
    }
    return null;
  }
  return 'Unsupported lighting primitive';
}
export function stripPreset(kind: string): StripProgram {
  const white = {red:255,green:255,blue:255};
  if (kind === 'alternating-zones') return {primitive:kind,primary:{red:255,green:0,blue:0},secondary:{red:0,green:0,blue:255},period_ms:800,step_ms:100,swap_after_steps:4,dim_intensity:36};
  if (kind === 'converging-points') return {primitive:kind,primary:{red:255,green:0,blue:0},period_ms:2000,tail_pixels:3};
  if (kind === 'thunder') return {primitive:'envelope',primary:white,period_ms:1600,envelope:[{at_ms:0,intensity:255},{at_ms:45,intensity:24},{at_ms:90,intensity:220},{at_ms:145,intensity:52},{at_ms:220,intensity:150},{at_ms:360,intensity:0}]};
  return {primitive:'envelope',primary:{red:0,green:96,blue:255},period_ms:3000,envelope:[{at_ms:0,intensity:0},{at_ms:kind==='pulse'?500:1400,intensity:255},{at_ms:kind==='pulse'?1000:2800,intensity:0}]};
}
export const stripHex = (c: StripColor): string => `#${[c.red,c.green,c.blue].map(v=>v.toString(16).padStart(2,'0')).join('')}`;
export function stripFrame(p: StripProgram, count: number, at: number): StripColor[] {
  if (stripProgramError(p,count) || !integer(count,1,100) || !Number.isFinite(at) || at < 0) return [];
  at = Math.floor(at);
  if(p.primitive==='layers') {
    const frame=Array.from({length:count},()=>({red:0,green:0,blue:0}));
    for(const l of p.layers!) {if(!l.enabled)continue;const pixels=p.segments!.find(s=>s.id===l.segment_id)!.pixels, phase=(at+l.phase_ms)%l.period_ms,t=phase/(l.period_ms-1);
      pixels.forEach((pixel,i)=>{let mix=l.primitive==='gradient'?(pixels.length>1?i/(pixels.length-1):0):l.primitive==='ramp'?t:l.primitive==='blink'?(phase*100>=l.period_ms*l.duty_percent?1:0):l.primitive==='breathe'?(1-Math.cos(t*2*Math.PI))/2:0;
        if(l.reverse)mix=1-mix;mix=l.easing==='ease-in'?mix*mix:l.easing==='ease-out'?1-(1-mix)*(1-mix):l.easing==='ease-in-out'?mix*mix*(3-2*mix):mix;
        const channel=(k:keyof StripColor)=>Math.round(l.primary[k]+(l.secondary[k]-l.primary[k])*mix);frame[pixel]={red:channel('red'),green:channel('green'),blue:channel('blue')};});
    }
    return frame;
  }
  const time = at % p.period_ms;
  const scale = (c: StripColor, intensity: number): StripColor => ({red:Math.floor(c.red*intensity/255),green:Math.floor(c.green*intensity/255),blue:Math.floor(c.blue*intensity/255)});
  return Array.from({length:count},(_,pixel)=>{
    if (p.primitive === 'alternating-zones') {
      const phase = Math.floor(at/p.step_ms!) % Math.max(2,Math.floor(p.period_ms/p.step_ms!));
      const primary = (pixel < Math.ceil(count/2)) === (phase < p.swap_after_steps!);
      return scale(primary?p.primary:p.secondary??{red:0,green:0,blue:0},phase%2===0?255:p.dim_intensity??0);
    }
    if (p.primitive === 'envelope') {
      let intensity = 0;
      for (let i=0; i<p.envelope!.length; i++) {
        const point=p.envelope![i];
        if (time < point.at_ms) {
          if (i>0) { const previous=p.envelope![i-1]; intensity=previous.intensity+Math.trunc((point.intensity-previous.intensity)*(time-previous.at_ms)/(point.at_ms-previous.at_ms)); }
          break;
        }
        intensity=point.intensity;
      }
      return scale(p.primary,intensity);
    }
    const head=Math.floor(time*(Math.floor((count-1)/2)+1)/p.period_ms);
    const tail=Math.max(3,p.tail_pixels || Math.floor(count/12));
    const intensity=(d:number)=>d<=tail?Math.floor(255*(tail+1-d)/(tail+1)):0;
    return scale(p.primary,Math.max(pixel<=head?intensity(head-pixel):0,pixel>=count-1-head?intensity(pixel-(count-1-head)):0));
  });
}
