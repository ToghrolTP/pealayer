import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { duplicateStep, moveSequenceStep, parseTimeMs, formatTimeMs, repeatedSequenceDurationMs, sequenceKindLabels, selectCueIndices, duplicateSequenceCues, deleteSequenceCues } from '../src/cueAuthoring.ts';
const original = { at_us: 404000, kind: 'motion', target: 0, value: 2, duration_ms: 500, action_ids: ['seat.a.down'] };
const copy = duplicateStep(original);
assert.equal(copy.at_us, 1404000);
copy.action_ids.push('custom');
assert.deepEqual(original.action_ids, ['seat.a.down']);
const moved = moveSequenceStep(original, 154000, 'seat.b');
assert.equal(moved.target, 1);
assert.deepEqual(moved.action_ids, ['seat.b.down']);
assert.equal(moved.duration_ms, 500);
assert.equal(original.target, 0);
assert.equal(moveSequenceStep(original, -1, 'pwm.1').target, 0);
assert.deepEqual(selectCueIndices([1], 3, 1, true), [1, 3]);
assert.deepEqual(selectCueIndices([1, 3], 1, 3, true), [3]);
assert.deepEqual(selectCueIndices([3], 4, 1, false, true), [1, 2, 3, 4]);
assert.deepEqual(selectCueIndices([0], 4, 2, true, true), [0, 2, 3, 4]);
const group = [original, { ...original, at_us: 2404000, duration_ms: 200 }, { at_us: 10000000, kind: 'beep' }];
const bulk = duplicateSequenceCues(group, [0, 1, 1, -1, 99]);
assert.deepEqual(bulk.indices, [3, 4]);
assert.deepEqual(bulk.steps.slice(0, 3), group);
assert.equal(bulk.steps[3].at_us, 2604000);
assert.equal(bulk.steps[4].at_us - bulk.steps[3].at_us, 2000000);
bulk.steps[3].action_ids.push('copy-only');
assert.deepEqual(original.action_ids, ['seat.a.down']);
assert.deepEqual(deleteSequenceCues(bulk.steps, [0, 1, 3, 4]).steps, [group[2]]);
assert.deepEqual(deleteSequenceCues(group, [0, 1, 2]), { steps: [], primary: 0 });
assert.deepEqual(deleteSequenceCues(group, [-1, 99]).steps, group);
for (const [input, expected] of [['1s', 1000], ['500ms', 500], ['1.5min', 90000], ['250us', .25], ['0s', 0]]) assert.equal(parseTimeMs(input), expected);
for (const invalid of ['-1s', 'abc', 'Infinity', '1s junk']) assert.equal(parseTimeMs(invalid), null);
assert.equal(formatTimeMs(0), '0s'); assert.equal(formatTimeMs(0, false), '0s');
assert.equal(formatTimeMs(1000), '1s'); assert.equal(formatTimeMs(1000, false), '1000ms');
const fractional = [{ at_us: 100500, kind: 'relay', duration_ms: 100 }];
assert.equal(repeatedSequenceDurationMs(fractional, 3, 0), 602);
assert.equal(repeatedSequenceDurationMs(fractional, 3, 200), null);
assert.equal(repeatedSequenceDurationMs(fractional, 0, 0), null);
assert.equal(repeatedSequenceDurationMs([], 2, 0), null);
assert.equal(repeatedSequenceDurationMs(fractional, 1000, 3600000), null);
const timeline = readFileSync(new URL('../src/components/SequenceTimeline.tsx', import.meta.url), 'utf8');
assert.ok(timeline.includes('setPointerCapture') && timeline.includes('onPointerCancel'));
assert.ok(timeline.includes('active.copy') && timeline.includes('reveal(primary.current)') && timeline.includes('reveal(destination)'));
assert.ok(timeline.includes("event.key.toLowerCase() === 'a'") && timeline.includes("event.key === 'Delete'"));
assert.ok(timeline.includes('event.stopPropagation()') && timeline.includes('input,textarea,select,[contenteditable="true"]'));
assert.ok(timeline.includes('ownUpdate.current') && timeline.includes('aria-pressed={selected.has(index)}'));
assert.ok(timeline.includes('suppressClick.current = index') && timeline.includes('suppressClick.current === index'), 'release click must not deselect the new copy');
assert.ok(!timeline.includes('sendCmd') && !timeline.includes('seek_to'), 'designer gestures must not actuate or seek');
for (const file of ['EffectsTab', 'StudioTab']) {
  const source = readFileSync(new URL(`../src/components/${file}.tsx`, import.meta.url), 'utf8');
  assert.ok(source.includes('<SequenceTimeline') && source.includes("tr(locale, 'Effects Designer')"));
  assert.ok(!source.includes('step.repeat_count') && !source.includes('step.repeat_interval_ms'));
}
// Scope translation parity to the surfaces changed in this pass. Learned
// peripheral names, media metadata and user-authored text remain untranslated.
const nativeDictionary = readFileSync(new URL('../../src/ui/i18n.rs', import.meta.url), 'utf8');
const nativeKeys = new Set([...nativeDictionary.matchAll(/"([^"\n]+)"\s*=>/g)].map(match => match[1]));
const webDictionary = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
const webKeys = new Set([...webDictionary.matchAll(/(?:'([^'\n]+)'|"([^"\n]+)")\s*:/g)].map(match => match[1] ?? match[2]));
for (const file of ['effects_library', 'audio']) {
  const source = readFileSync(new URL(`../../src/ui/${file}.rs`, import.meta.url), 'utf8');
  for (const match of source.matchAll(/(?:designer_tr\(ui,|app\.tr\()\s*"([^"\n]+)"/g)) {
    assert.ok(nativeKeys.has(match[1]), `Missing native designer Persian translation: ${match[1]}`);
  }
}
for (const file of ['EffectsTab', 'StudioTab', 'EffectRecorder', 'SequenceTimeline', 'SoundEffectFields']) {
  const source = readFileSync(new URL(`../src/components/${file}.tsx`, import.meta.url), 'utf8');
  for (const match of source.matchAll(/tr\(locale, '([^'\n]+)'/g)) {
    assert.ok(webKeys.has(match[1]), `Missing Web designer Persian translation: ${match[1]}`);
  }
}
for (const key of ['Effects Designer', 'Sequence step', 'Repeat effect', 'Interval', 'Length', 'Finish', 'Capture selection', 'Publish & Run', 'Rest', 'All relays off', 'All PWM outputs off', 'Status RGB', 'Raw opcode', 'Choose…']) {
  assert.ok(nativeKeys.has(key) && webKeys.has(key), `Designer translation parity missing: ${key}`);
}
for (const label of Object.values(sequenceKindLabels)) assert.ok(webKeys.has(label), `Missing command caption: ${label}`);
console.log('Cue copy/retarget, exact repeated timing, units and shared authoring surface checks passed.');
