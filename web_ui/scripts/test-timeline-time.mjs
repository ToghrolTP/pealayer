import assert from 'node:assert/strict';
import { formatTimelineTime } from '../src/timelineTime.ts';
for (const [seconds, expected] of [[0,'0'],[5,'5'],[59,'59'],[60,'1:00'],[65,'1:05'],[3600,'1:00:00'],[3661,'1:01:01'],[360000,'100:00:00']]) {
  assert.equal(formatTimelineTime(seconds),expected);
  assert.equal(formatTimelineTime(seconds,true),expected);
}
assert.equal(formatTimelineTime(.5,true),'0.500');
assert.equal(formatTimelineTime(65.125,true),'1:05.125');
assert.equal(formatTimelineTime(65.999),'1:05');
assert.equal(formatTimelineTime(59.9996,true),'1:00');
assert.equal(formatTimelineTime(NaN),'—');
assert.equal(formatTimelineTime(-1),'—');
console.log('Compact timeline clock formatting checks passed.');
