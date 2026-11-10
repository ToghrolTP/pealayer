import assert from 'node:assert/strict';
import { reconcilePWMDrafts } from '../src/pwmDrafts.ts';
const controls=[{key:'pwm.0',percent:60}];
assert.deepEqual(reconcilePWMDrafts({'pwm.0':20},new Set(['pwm.0']),{'pwm.0':10},11,controls),{'pwm.0':20});
assert.deepEqual(reconcilePWMDrafts({'pwm.0':20},new Set(),{'pwm.0':10},11,controls),{});
assert.deepEqual(reconcilePWMDrafts({'pwm.0':20},new Set(),{'pwm.0':10},9,controls),{'pwm.0':20});
assert.deepEqual(reconcilePWMDrafts({'pwm.0':20},new Set(),{'pwm.0':10},11,[{key:'pwm.0',percent:null}]),{'pwm.0':20});
assert.deepEqual(reconcilePWMDrafts({'pwm.0':0},new Set(),{'pwm.0':10},10,[{key:'pwm.0',percent:0}]),{});
console.log('PWM push: live peer values, active drafts, old readback, unknown and actual zero passed.');
