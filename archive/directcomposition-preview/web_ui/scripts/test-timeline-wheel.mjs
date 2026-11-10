import assert from 'node:assert/strict';
import { defaultTimelineWheelPreferences as defaults, timelineWheelAction, timelineZoomAtPointer } from '../src/timelineWheel.ts';
const wheel = (values = {}) => ({ deltaX: 0, deltaY: 120, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...values });
assert.deepEqual(timelineWheelAction(wheel(), defaults), { action: 'zoom', delta: 120 });
assert.equal(timelineWheelAction(wheel({ ctrlKey: true }), defaults).action, 'vertical_scroll');
assert.equal(timelineWheelAction(wheel({ metaKey: true }), defaults).action, 'vertical_scroll');
assert.equal(timelineWheelAction(wheel({ shiftKey: true }), defaults).action, 'horizontal_scroll');
assert.equal(timelineWheelAction(wheel({ altKey: true }), defaults).action, 'zoom');
for (const modifier of [{}, { ctrlKey: true }, { shiftKey: true }, { altKey: true }]) {
  assert.deepEqual(timelineWheelAction(wheel({ ...modifier, deltaX: 40 }), defaults), { action: 'horizontal_scroll', delta: 40 });
}
const swapped = { plain: 'vertical_scroll', ctrl: 'zoom', shift: 'none', alt: 'horizontal_scroll' };
for (const [keys, action] of [[{}, 'vertical_scroll'], [{ctrlKey:true}, 'zoom'], [{shiftKey:true}, 'none'], [{altKey:true}, 'horizontal_scroll']]) {
  assert.equal(timelineWheelAction(wheel(keys), swapped).action, action);
}
assert.equal(timelineWheelAction(wheel({ ctrlKey: true, shiftKey: true }), defaults).action, 'horizontal_scroll');
const next = timelineZoomAtPointer(2, -120, 300, 150);
assert.ok(Math.abs((next.scrollLeft + 150) / next.zoom - 225) < 1e-8);
assert.equal(timelineZoomAtPointer(1, 10000, 0, 150).zoom, 1);
assert.equal(timelineZoomAtPointer(25, -10000, 0, 150).zoom, 25);
console.log('Timeline wheel routing, modifier swap, horizontal priority, and cursor-anchored zoom checks passed.');
