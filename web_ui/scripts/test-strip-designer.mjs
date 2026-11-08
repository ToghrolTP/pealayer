import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {newStripDesign,newStripLayer,stripFrame,stripPreset,stripProgramError,stripLayersEditable,stripDraftError} from '../src/stripProgram.ts';
for(const kind of ['fade','pulse','alternating-zones','thunder','converging-points']){const p=stripPreset(kind);assert.equal(stripProgramError(p),null);assert.equal(stripFrame(p,3,1000).length,3);}
const p=newStripDesign(5);p.segments[0].pixels=[4,0,2];p.layers[0].primitive='gradient';p.layers[0].primary={red:255,green:0,blue:0};p.layers[0].secondary={red:0,green:0,blue:255};
assert.equal(stripProgramError(p,5),null);let frame=stripFrame(p,5,0);assert.equal(frame[4].red,255);assert.equal(frame[2].blue,255);assert.equal(frame[0].red,128);assert.deepEqual(frame[1],{red:0,green:0,blue:0});
assert.ok(stripProgramError(p,3));
const overlay=newStripLayer('second','all');overlay.enabled=false;overlay.primary={red:0,green:255,blue:0};p.layers.push(overlay);assert.equal(stripFrame(p,5,0)[4].green,0);p.layers[1].enabled=true;assert.equal(stripFrame(p,5,0)[4].green,255);
p.layers.pop();p.segments[0].pixels=[0,0];assert.ok(stripProgramError(p,5));
for(const primitive of ['solid','gradient','ramp','blink','breathe'])for(const easing of ['linear','ease-in','ease-out','ease-in-out']){const p=newStripDesign(100);Object.assign(p.layers[0],{primitive,easing});assert.equal(stripProgramError(p,100),null);assert.equal(stripFrame(p,100,1234).length,100);}
const blink=newStripDesign(1);blink.layers[0].primitive='blink';assert.equal(stripFrame(blink,1,1500)[0].blue,0);assert.deepEqual(stripFrame(blink,1,0),stripFrame(blink,1,3000));
delete blink.layers[0].enabled;assert.ok(stripProgramError(blink,1));
for(const malformed of [null,{primitive:'layers',layers:{}},{primitive:'layers',segments:[null],layers:[null]}])assert.equal(stripLayersEditable(malformed),false);
const emptySelection=newStripDesign(5);emptySelection.segments[0].pixels=[];assert.equal(stripLayersEditable(emptySelection),true);assert.ok(stripProgramError(emptySelection,5));
const draft={programText:JSON.stringify(newStripDesign(5)),default_pixels:5,default_fps:20,duration_ms:1000};assert.equal(stripDraftError(draft),null);assert.ok(stripDraftError({...draft,default_pixels:3}));assert.ok(stripDraftError({...draft,duration_ms:1}));
for(const file of ['EffectsTab.tsx','StudioTab.tsx'])assert.match(readFileSync(new URL(`../src/components/${file}`,import.meta.url),'utf8'),/<StripDesigner /);
const css=readFileSync(new URL('../src/components/stripDesigner.css',import.meta.url),'utf8');assert.match(css,/minmax\(min\(100%,180px\),1fr\)/);assert.match(css,/align-items:center/);
console.log('Strip designer: ordered pixels, layers, curves, bounds, previews and both editor integrations passed.');
