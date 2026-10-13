import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { hardwareControlIdentity } from '../src/hardwareIdentity.ts';
import { defaultPwmCard, pwmActionVisible, pwmIndicatorAction, pwmRowActions } from '../src/pwmCard.ts';
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
