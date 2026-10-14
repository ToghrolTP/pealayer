import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { hardwareControlIdentity } from '../src/hardwareIdentity.ts';
import { defaultPwmCard, guardHardwarePointerStart, observedControlActive, powerActionSelected, pwmActionVisible, pwmIndicatorAction, pwmRowActions } from '../src/pwmCard.ts';
assert.equal(observedControlActive({active:true}),true);
assert.equal(observedControlActive({active:false}),false);
for(const active of [undefined,null,'true',1]) assert.equal(observedControlActive({active}),undefined);
assert(powerActionSelected('on',true));
assert(powerActionSelected('off',false));
assert(!powerActionSelected('off',undefined),'Unknown must never select confirmed Off');
assert(!powerActionSelected('on',undefined));
assert(!powerActionSelected('off',true));
for (const button of [1,2,3,4]) {
  let stopped = false; const commands = [];
  const event = {button,stopPropagation(){stopped=true;}};
  guardHardwarePointerStart(event);
  // Capture must isolate both rc-slider's rail and handle start handlers.
  if (!stopped) commands.push({percent:72.2});
  assert.deepEqual(commands,[],`Button ${button} must never reach a slider mutation`);
}
let stopped = false;
guardHardwarePointerStart({button:0,stopPropagation(){stopped=true;}});
assert.equal(stopped,false,'Primary drag remains available; keyboard is not intercepted');
const hardwareSource=readFileSync(new URL('../src/components/HardwareTab.tsx',import.meta.url),'utf8');
assert.doesNotMatch(hardwareSource,/Boolean\(control.active\)|control.percent > 0/,'One authoritative nullable power state must drive all surfaces');
assert.match(hardwareSource,/!\/pwm\|mosfet\/i.test\(control.kind\) && \['on', 'off', 'toggle'\]/,'Queued PWM intent must not replace observed power with optimistic status');
assert.match(hardwareSource, /className="hardware-control__pwm"\s+onMouseDownCapture=\{guardHardwarePointerStart\} onPointerDownCapture=\{guardHardwarePointerStart\}/,
  'Both card/detail share the capture barrier before the third-party slider');
const guardSource=readFileSync(new URL('../src/pwmCard.ts',import.meta.url),'utf8');
assert.doesNotMatch(guardSource,/preventDefault|onContextMenu/,'The independent card contextmenu must remain usable');
const actions = ['on', 'off', 'increase', 'decrease', 'toggle'].map(verb => ({ verb, id: `pwm.1.${verb}` }));
assert.equal(hardwareControlIdentity({key:'pwm.1',kind:'mosfet',channel:1}),'CH2');
assert.equal(hardwareControlIdentity({key:'relay.5',kind:'relay',channel:5}),'R5');
assert.equal(hardwareControlIdentity({key:'seat.a',kind:'motion'}),'A');
const rf = readFileSync(new URL('../src/components/RfManager.tsx',import.meta.url),'utf8');
assert.match(rf, /<Modal key=\{assignment\?\.nonce \?\? 'rf-general'\} zIndex=\{1200\}/,
  'A reused RF portal must be above channel detail, with a fresh body for each explicit assignment.');
assert.deepEqual(pwmRowActions(defaultPwmCard, actions).map(a => a.verb), ['on','off','toggle']);
assert.deepEqual(pwmRowActions({...defaultPwmCard, show_on:false, show_other:false}, actions).map(a => a.verb), ['off']);
assert.equal(pwmActionVisible({...defaultPwmCard,show_decrease:false}, 'decrease'),false);
assert.equal(pwmActionVisible({...defaultPwmCard,show_decrease:false}, 'increase'),true);
assert.equal(pwmIndicatorAction(defaultPwmCard,actions)?.id,'pwm.1.toggle');
assert.equal(pwmIndicatorAction({...defaultPwmCard,indicator_action:'on'},actions,true)?.id,'pwm.1.on');
assert.equal(pwmIndicatorAction({...defaultPwmCard,indicator_action:'off'},actions,false)?.id,'pwm.1.off');
assert.equal(pwmIndicatorAction(defaultPwmCard,actions.slice(0,4)),undefined);
assert.equal(pwmIndicatorAction(defaultPwmCard,actions.slice(0,4),true)?.id,'pwm.1.off');
const css = readFileSync(new URL('../src/styles.css', import.meta.url),'utf8');
assert.match(css, /\.hardware-control__pwm \{[^}]*grid-column: 1 \/ -1;[^}]*display: flex;/,
  'The indicator owns the fourth header column; the flanked slider must use its own full-width row on desktop too.');
assert.match(css, /\.hardware-control__pwm \+ \.hardware-control__actions \{ grid-column: 1 \/ -1; \}/);
console.log('PWM cards: shared visibility, inline step actions and semantic indicator policy passed.');
