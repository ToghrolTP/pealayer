import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {hardwareAvailability,hardwareOutputsAvailable,hardwareLabels,hardwareMonitorReady,hardwareMonitorRfAvailable,hardwareMonitorEmpty} from '../src/hardwareAvailability.ts';
const read = (path) => readFileSync(new URL(path,import.meta.url),'utf8');
const state={controller_connected:true,hardware_connected:true,hardware_details:{outputs_available:true,programming:{latch_known:true,programming_latch:true,active:false,restoration_pending:true,intentional_uart_release:false}}};
assert.equal(hardwareAvailability(state),'programming_latched');
assert.equal(hardwareOutputsAvailable(state),false);
state.hardware_details.programming.active=true;
assert.equal(hardwareAvailability(state),'programming');
state.hardware_connected=false; state.hardware_details.programming.intentional_uart_release=true;
assert.equal(hardwareAvailability(state),'programming');
assert.equal(hardwareAvailability(state,false),'disconnected','cached latch must not hide lost peer transport');
state.controller_connected=false;
assert.equal(hardwareAvailability(state),'disconnected','cached latch must not hide lost coordinator');
state.controller_connected=true; state.hardware_details.programming.active=false; state.hardware_details.programming.latch_known=false;
assert.equal(hardwareAvailability(state),'programming_recovery','unknown is not cleared, and pending restore is not an active programmer');
state.hardware_details.programming.intentional_uart_release=false;
assert.equal(hardwareAvailability(state),'hardware_unavailable','unexpected board loss is not intentional handoff');
state.hardware_connected=true;
assert.equal(hardwareAvailability(state),'ok','historical latch value cannot override explicit unknown knowledge');
state.hardware_details.outputs_available=false;
assert.equal(hardwareOutputsAvailable(state),false);
assert.equal(hardwareLabels.programming_latched,'Programming latch active');
for(const component of ['ApplicationStatusBar','HardwareTab','StudioTab']) assert(read(`../src/components/${component}.tsx`).includes('hardwareAvailability'),`${component} uses the same state distinction`);
for(const component of ['HardwareTab','EffectsTab','StudioTab','EffectRecorder']) assert(read(`../src/components/${component}.tsx`).includes('hardwareOutputsAvailable'),`${component} respects output safety availability`);
console.log('Hardware availability: latch, intentional handoff, recovery, unknown and genuine transport loss passed.');

const stale={controller_connected:false,hardware_connected:true,hardware:{supports_rf_transmit:true},hardware_details:{outputs_available:true}};
assert.equal(hardwareMonitorReady(stale),false,'cached board cannot restore a disconnected monitor');
assert.equal(hardwareMonitorRfAvailable(stale),false);
assert.equal(hardwareMonitorEmpty(stale).title,'Hardware Disconnected','not every offline state is connecting');
stale.hardware_connection_requested=true;
assert.equal(hardwareMonitorEmpty(stale).title,'Connecting to PCController…');
assert.equal(hardwareMonitorEmpty(stale).primary,null,'rendering must not offer duplicate reconnect requests');
stale.controller_connected=true;stale.hardware_connected=false;
assert.equal(hardwareMonitorRfAvailable(stale),false);
assert.equal(hardwareMonitorEmpty(stale).title,'No board connected');
stale.hardware_connected=true;
assert.equal(hardwareMonitorReady(stale),true);assert.equal(hardwareMonitorRfAvailable(stale),true);
stale.hardware.supports_rf_transmit=false;assert.equal(hardwareMonitorRfAvailable(stale),false,'no invented RF capability');
stale.hardware.supports_rf_transmit=true;stale.hardware_details.outputs_available=false;assert.equal(hardwareMonitorRfAvailable(stale),false,'programming/latch disables RF entry');
assert.equal(hardwareMonitorEmpty({controller_connected:true}).title,'Loading hardware…');
assert.equal(hardwareMonitorEmpty({controller_connected:true,hardware_error:'Catalog read failed'}).title,'Hardware catalog unavailable');
const hardwareSource=read('../src/components/HardwareTab.tsx');
assert.ok(hardwareSource.includes('if (!details || !hardwareMonitorReady(state))'));
assert.ok(hardwareSource.includes('{hardwareMonitorRfAvailable(state) && <RfManager'));
console.log('Hardware Monitor: disconnected, connecting, missing catalog, missing board, stale state and advertised RF gates passed.');
const diagnostic='embedded PCController unavailable (PEALAYER_PCCONTROLLER_LIBRARY); external coordinator unavailable (connection timed out)';
for (const input of [
  {controller_connected:false},
  {controller_connected:true},
  {controller_connected:true,hardware_details:{outputs_available:false}},
]) {
  const empty=hardwareMonitorEmpty({...input,hardware_error:diagnostic});
  assert.equal(empty.diagnostics,diagnostic,'full diagnostic is preserved for the info hint');
  assert.ok(!empty.detail.includes('PEALAYER_')&&!empty.detail.includes('embedded'),'default copy must be plain English, not diagnostic parsing');
}
assert.ok(hardwareMonitorEmpty({controller_connected:false,hardware_error:diagnostic}).detail.startsWith("Couldn't connect"));
assert.ok(hardwareMonitorEmpty({controller_connected:true,hardware_error:diagnostic}).detail.startsWith("Couldn't load"));
assert.equal(hardwareMonitorEmpty({controller_connected:false,hardware_error:'  '}).diagnostics,null);
assert.ok(hardwareSource.includes('<InfoCircleOutlined />')&&hardwareSource.includes("trigger={['hover', 'click']}"));
assert.ok(hardwareSource.includes('{empty.diagnostics}</Typography.Paragraph>'),'diagnostics belong only in the opt-in popover');
console.log('Connection summaries: plain default copy, retained diagnostics and hover/click info passed.');
