// Real-board acceptance, never a physical edge/frame timing certification.
// Only explicitly authorized R5–R7; all mutations use existing app contracts.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

export async function verifyLiveCue({ request, relay = 5, preflight = false, sleep = ms => new Promise(resolve => setTimeout(resolve, ms)), interrupted = () => false }) {
  assert([5, 6, 7].includes(relay), 'Only authorized Relay 5–7');
  let serial = 0;
  const session = () => request('/api/peer/session');
  const same = isDeepStrictEqual;
  const rpc = async (method, params = {}) => {
    const response = await request('/api/rpc', { jsonrpc: '2.0', id: ++serial, method, params });
    assert(!response.error, JSON.stringify(response.error));
    return response.result;
  };
  async function until(check, label, cleanup = false) {
    const deadline = Date.now() + 15_000;
    for (let attempt = 0; attempt < 400 && Date.now() < deadline; attempt++) {
      assert(cleanup || !interrupted(), 'Interrupted; restoring session');
      const value = await check();
      if (value) return value;
      await sleep(40);
    }
    throw Error('Timeout: ' + label);
  }
  const initial = await session();
  assert(initial.paused && initial.media && initial.hardware?.board_connected, 'Paused loaded media/connected board required');
  assert(initial.hardware.active_relays?.length === 0, 'Refusing active or unsampled relays');
  assert(initial.hardware.motion?.left.applied === 'stop' && initial.hardware.motion?.right.applied === 'stop', 'Refusing active or unsampled motion');
  const pwm = initial.hardware.telemetry?.pwm_values;
  assert(pwm?.length && pwm.every(value => value === 0), 'Refusing active or unsampled PWM');
  const output = initial.hardware.relays.find(value => value.id === relay && value.control === 'relay');
  assert(output?.key, 'Authorized relay not advertised as user output');
  const sync = initial.status.hardware_sync;
  assert(sync?.authority_client_id && sync.authority?.owner_id === sync.authority_client_id && !sync.error, 'Authority mismatch or existing sync fault');
  assert(sync.revision === sync.prepared_revision && sync.revision === sync.clock_ack_revision && sync.timeline?.state === 'paused', 'Original timeline must be armed');
  const original = structuredClone(initial.timeline);
  assert(original?.timeline, 'Original timeline unavailable');
  const test = structuredClone(original);
  const cueId = randomUUID(), effectId = randomUUID();
  test.timeline.instances = [{ id: cueId, effect_id: effectId, start_time_ms: Math.round(initial.position * 1000) + 2_000 }];
  assert(initial.status.duration * 1000 > test.timeline.instances[0].start_time_ms + 1_000, 'Insufficient media duration');
  test.timeline.templates = [{ id: effectId, name: 'Temporary relay acceptance', icon: 'plug', duration_ms: 1_000,
    target: { Relay: relay }, actions: [], duration_policy: 'resizable',
    direct_control: { control_key: output.key, value_basis_points: 10_000, behavior: 'hold', end_value_basis_points: 0 } }];
  test.timeline.analog_tracks = [];
  test.timeline.track_states = { ['hardware:' + output.key]: { linked: true, visible: true } };
  test.muted = []; test.soloed = [];
  if (preflight) return { preflight: true, mutations: 0, relay, control_key: output.key, prepared_revision: sync.revision };
  const checkIdentity = current => assert(current.instance_id === initial.instance_id && current.media === initial.media, 'Application/media changed; manual recovery required');
  let attempted = false;
  let playAttempted = false;
  let result;
  try {
    assert(!interrupted(), 'Interrupted before test');
    attempted = true; // A lost mutation ACK may still mean it applied. Never replay it.
    await request('/api/peer/timeline', { expected: original, state: test });
    const armed = await until(async () => {
      const current = await session(); checkIdentity(current);
      assert(current.paused && same(current.timeline, test), 'Concurrent session change');
      const status = current.status.hardware_sync;
      assert(!status?.error, status?.error);
      return status?.timeline?.step_count === 2 && status.revision === status.prepared_revision
        && status.revision === status.clock_ack_revision && status.timeline.state === 'paused' && status;
    }, 'two-edge plan/paused clock arming');
    playAttempted = true;
    await rpc('pealayer.play');
    const complete = await until(async () => {
      const current = await session(); checkIdentity(current);
      assert(same(current.timeline, test), 'Concurrent timeline change');
      const status = current.status.hardware_sync;
      assert(!status?.error, status?.error);
      return status?.timeline?.acknowledged === 2 && status;
    }, 'both real-board ACKs');
    const ledger = complete.timeline;
    assert(ledger.max_ack_lateness_ms <= 50, '50 ms ACK budget exceeded');
    result = { scope: 'real-board device-ACK, not physical output-edge timing', relay, prepared_revision: armed.revision,
      acknowledged: ledger.acknowledged, max_dispatch_lateness_ms: ledger.max_dispatch_lateness_ms,
      max_ack_round_trip_ms: ledger.max_ack_round_trip_ms, max_ack_lateness_ms: ledger.max_ack_lateness_ms,
      device_ack_us: ledger.device_ack_us };
  } finally {
    if (attempted) {
      let current = await session(); checkIdentity(current);
      if (playAttempted || !same(current.timeline, original)) {
        // Stop even on drift, but never overwrite another user's timeline edits.
        await rpc('pealayer.pause');
        await until(async () => (await session()).paused, 'pause', true);
        current = await session(); checkIdentity(current);
        assert(same(current.timeline, test), 'Concurrent timeline change: paused, refusing overwrite; manual recovery required');
        await request('/api/peer/timeline', { expected: test, state: original });
        await rpc('pealayer.seek_to', { seconds: initial.position });
        await until(async () => {
          const restored = await session(); checkIdentity(restored);
          assert(same(restored.timeline, original), 'Timeline changed during restoration');
          const status = restored.status.hardware_sync;
          return restored.paused && Math.abs(restored.position - initial.position) < 0.02
            && restored.hardware?.board_connected && !restored.hardware.active_relays.includes(relay)
            && !status?.error && status.revision === status.prepared_revision
            && status.revision === status.clock_ack_revision && status.timeline?.state === 'paused';
        }, 'original timeline/paused position re-armed and relay OFF', true);
      }
    }
  }
  return { ...result, restored: true, original_cue_count: original.timeline.instances.length, relay_off: true };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [base, relayText, option, ...extra] = process.argv.slice(2);
  assert(base && relayText, 'Usage: node verify-live-cue.mjs http://authority:port 5|6|7');
  assert(!extra.length && (!option || option === '--preflight'), 'Optional --preflight validates without mutation');
  const origin = new URL(base);
  assert(['http:', 'https:'].includes(origin.protocol) && origin.pathname === '/' && !origin.search && !origin.hash && !origin.username && !origin.password, 'Use the authority origin, not a file URL');
  let interrupted = false;
  process.once('SIGINT', () => { interrupted = true; });
  process.once('SIGTERM', () => { interrupted = true; });
  const request = async (path, body) => {
    const response = await fetch(new URL(path, origin), { signal: AbortSignal.timeout(7_000),
      ...(body ? { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) } : {}) });
    const text = await response.text();
    assert(response.ok, path + ': ' + text);
    return JSON.parse(text);
  };
  console.log(JSON.stringify(await verifyLiveCue({ request, relay: Number(relayText), preflight: option === '--preflight', interrupted }), null, 2));
}
