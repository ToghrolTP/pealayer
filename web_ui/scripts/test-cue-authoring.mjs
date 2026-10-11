import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { duplicateStep, moveSequenceStep, parseTimeMs, formatTimeMs, repeatedSequenceDurationMs, sequenceKindLabels, sequenceLaneName, selectCueIndices, duplicateSequenceCues, deleteSequenceCues, stepRepetition, stepInstances, editStepInstance } from '../src/cueAuthoring.ts';
const controls = [
  { key: 'seat.a', name: 'Seat Left', default_name: 'Seat A' },
  { key: 'seat.b', name: 'صندلی راست', default_name: 'Seat B' },
];
const { sequenceLaneChoices, sequenceStepTemplate, sequenceNeedsQuantize, quantizeSequence, sequenceExtraKinds } = await import('../src/cueAuthoring.ts');
const choices = sequenceLaneChoices([
  { key:'seat.b', name:'VIP Right', kind:'seat' }, { key:'relay.5', name:'Screen', kind:'relay' },
  { key:'pwm.3', name:'Orange spots', kind:'mosfet' }, { key:'relay.1', name:'Internal', kind:'relay', locked:true },
], sequenceExtraKinds({ supports:{ lcd_display:true, segment_display:true }, buzzer:{} }));
assert.equal(choices.find(choice => choice.key === 'seat.b').template.target, 1);
assert.equal(choices.find(choice => choice.key === 'seat.b').name, 'VIP Right');
assert.equal(choices.find(choice => choice.key === 'relay.5').template.target, 4);
assert.equal(choices.find(choice => choice.key === 'pwm.3').template.target, 3);
assert.ok(!choices.some(choice => choice.key === 'relay.1'));
assert.equal(choices.find(choice => choice.key === 'display.lcd').template.destination, 'lcd');
assert.ok(choices.some(choice => choice.key === 'display.segments'));
const template = sequenceStepTemplate('motion',1); assert.deepEqual(template.action_ids,['seat.b.up']);
assert.equal(sequenceNeedsQuantize([template],50),false);
assert.equal(sequenceNeedsQuantize([{...template,at_us:125001,duration_ms:503}],50),true);
assert.deepEqual(quantizeSequence([{...template,at_us:125001,duration_ms:503}],50)[0],{...template,at_us:150000,duration_ms:500});
assert.equal(sequenceLaneName('seat.a', controls), 'Seat Left');
assert.equal(sequenceLaneName('seat.b', controls), 'صندلی راست');
assert.equal(sequenceLaneName('seat.a', [{ ...controls[0], name: 'VIP Left' }]), 'VIP Left');
assert.equal(sequenceLaneName('seat.b', [{ ...controls[1], name: '  ' }]), 'Seat B');
assert.equal(sequenceLaneName('seat.a', []), 'seat.a');
assert.equal(sequenceLaneName('seat.b', []), 'seat.b');
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
assert.ok(timeline.includes('sequenceLaneName(key, controls)') && !timeline.includes("tr(locale, 'Seat A')") && !timeline.includes("tr(locale, 'Seat B')"));
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
  assert.ok(source.includes('<StepRepeatFields'), 'both designers must expose the same step-repeat controls');
}
const repeatFields = readFileSync(new URL('../src/components/StepRepeatFields.tsx', import.meta.url), 'utf8');
assert.ok(repeatFields.includes('className="step-repeat-controls"'), 'repeat toggle and count share one compact row');
assert.ok(repeatFields.includes('<div className="step-repeat-field">'), 'the checkbox must not be nested inside another label');
// Scope translation parity to the surfaces changed in this pass. Learned
// peripheral names, media metadata and user-authored text remain untranslated.
const nativeDictionary = readFileSync(new URL('../../src/ui/i18n.rs', import.meta.url), 'utf8');
const nativeKeys = new Set([...nativeDictionary.matchAll(/"([^"\n]+)"\s*=>/g)].map(match => match[1]));
const webDictionary = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
const webKeys = new Set([...webDictionary.matchAll(/(?:'([^'\n]+)'|"([^"\n]+)")\s*:/g)].map(match => match[1] ?? match[2]));
for (const file of ['effects_library', 'audio']) {
  const source = readFileSync(new URL(`../../src/ui/${file}.rs`, import.meta.url), 'utf8');
  for (const match of source.matchAll(/(?:designer_(?:tr|label)\(ui,|app\.tr\()\s*"([^"\n]+)"/g)) {
    assert.ok(nativeKeys.has(match[1]), `Missing native designer Persian translation: ${match[1]}`);
  }
}
for (const file of ['EffectsTab', 'StudioTab', 'EffectRecorder', 'SequenceTimeline', 'SoundEffectFields', 'ColorDropdown', 'DesignerFieldLabel']) {
  const source = readFileSync(new URL(`../src/components/${file}.tsx`, import.meta.url), 'utf8');
  for (const match of source.matchAll(/tr\(locale, '([^'\n]+)'/g)) {
    assert.ok(webKeys.has(match[1]), `Missing Web designer Persian translation: ${match[1]}`);
  }
  for (const match of source.matchAll(/DesignerFieldLabel label="([^"]+)"/g)) assert.ok(webKeys.has(match[1]), `Missing field caption: ${match[1]}`);
}
for (const key of ['Effects Designer', 'Sequence step', 'Repeat effect', 'Repeat step', 'Interval', 'Length', 'Finish', 'Capture selection', 'Publish & Run', 'Rest', 'All relays off', 'All PWM outputs off', 'Status RGB', 'Raw opcode', 'Choose…']) {
  assert.ok(nativeKeys.has(key) && webKeys.has(key), `Designer translation parity missing: ${key}`);
}
for (const label of Object.values(sequenceKindLabels)) assert.ok(webKeys.has(label), `Missing command caption: ${label}`);
console.log('Cue copy/retarget, exact repeated timing, units and shared authoring surface checks passed.');

const repeating = { ...original, repeat_count: 3, repeat_interval_ms: 750 };
assert.deepEqual(stepRepetition(repeating), { count: 3, intervalUs: 750000, endUs: 2404000 });
assert.deepEqual(stepInstances(repeating, 7), [
  { source: 7, iteration: 2, at_us: 1904000 },
  { source: 7, iteration: 1, at_us: 1154000 },
  { source: 7, iteration: 0, at_us: 404000 },
]);
const spacing = editStepInstance(repeating, 2, 200, 'move');
assert.equal(spacing.repeat_interval_ms, 850);
assert.equal(spacing.at_us, original.at_us);
assert.equal(spacing.target, original.target);
assert.equal(spacing.duration_ms, original.duration_ms);
assert.equal(repeating.repeat_interval_ms, 750, 'a cancelled preview cannot change the saved draft');
const sized = editStepInstance(repeating, 1, 100, 'length');
assert.equal(sized.duration_ms, 600);
assert.equal(sized.repeat_interval_ms, 750);
assert.equal(stepRepetition({ at_us: 500, kind: 'opcode', repeat_count: 3 })?.endUs, 2500);
assert.equal(repeatedSequenceDurationMs([repeating], 2, 2500), 4904);
assert.equal(repeatedSequenceDurationMs([repeating], 2, 2400), null);
for (const patch of [{ repeat_count: 0 }, { repeat_count: 1001 }, { repeat_interval_ms: -1 }, { repeat_interval_ms: 3600001 }, { repeat_interval_ms: 3600000 }]) assert.equal(stepRepetition({ ...repeating, ...patch }), null);
assert.equal(stepInstances({ ...repeating, repeat_count: 1000, repeat_interval_ms: 1000 }, 0, 900000000, 901500000).length, 3, 'only visible repeated instances plus source are materialized');
assert.ok(timeline.includes('data-repeat-index={iteration}') && timeline.includes('editStepInstance(active.step, iteration'));
console.log('Step repeat ghost binding, interval/length gestures, cancellation, bounds and nested timing passed.');
