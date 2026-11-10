import assert from 'node:assert/strict';
import { editTimecode, formatEditTime, parseEditTime, shouldCommitTime } from '../src/timecodeEditor.ts';

assert.equal(formatEditTime(3.125), '00:00:03.125');
assert.equal(parseEditTime('۰۰:۰۱:۰۲.۰۰۳'), 62.003);
for (const value of ['00:-1', '1e2', 'NaN', '1.', '.5', '00:60:00', '100:00:00']) {
  assert.equal(parseEditTime(value), null, value);
}
const original = '00:01:02.003';
assert.equal(shouldCommitTime(original, original, false, true, false), false);
assert.equal(shouldCommitTime(original, original, true, false, false), true);
assert.equal(shouldCommitTime(original, '00:01:02.004', false, true, false), true);
assert.equal(shouldCommitTime(original, '00:01:02.004', true, true, true), false);
let draft = editTimecode({ value: '00:00:00.000', group: 0, digits: 0 }, '01:23:45.678');
assert.equal(draft.value, '01:23:45.678');
draft = editTimecode(draft, 'Backspace');
assert.equal(draft.value, '01:23:45.067');
draft = editTimecode(draft, 'ArrowLeft');
assert.equal(draft.group, 2);
draft = editTimecode(draft, 'Delete');
assert.equal(draft.value, '01:23:00.067');
assert.equal(editTimecode(draft, 'x').value, draft.value);
console.log('Elapsed-time fixed mask and commit guards passed');
