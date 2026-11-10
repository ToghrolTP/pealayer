import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyLiveCue } from './verify-live-cue.mjs';

function fixture(options = {}) {
  const original = { timeline: { instances: [{ id: 'original-motion-cue' }], templates: [], analog_tracks: [{ id: 'original-curve' }], keyframes: [], track_states: {} }, muted: [], soloed: [] };
  const state = { instance_id: 'app', media: 'private', paused: true, position: 10, timeline: structuredClone(original),
    hardware: { board_connected: true, active_relays: [], motion: { left: { applied: 'stop' }, right: { applied: 'stop' } },
      telemetry: { pwm_values: [0] }, relays: [{ id: 5, key: 'advertised-key', control: 'relay' }] },
    status: { duration: 120 } };
  let revision = 0, lost = false;
  const mutations = [];
  function arm() {
    revision++;
    state.status.hardware_sync = { revision, prepared_revision: revision, clock_ack_revision: revision,
      authority_client_id: 'owner', authority: { owner_id: 'owner' }, timeline: { state: 'paused', step_count: 2,
        acknowledged: 0, max_dispatch_lateness_ms: 3, max_ack_round_trip_ms: 7, max_ack_lateness_ms: 10 } };
  }
  arm();
  async function request(path, body) {
    if (!body) return structuredClone(state);
    mutations.push(structuredClone(body));
    if (path === '/api/peer/timeline') {
      assert.deepEqual(body.expected, state.timeline);
      state.timeline = structuredClone(body.state); arm();
      if (options.lost && !lost) { lost = true; throw Error('lost mutation ACK'); }
      return { applied: true };
    }
    switch (body.method) {
      case 'pealayer.play':
        assert.equal(state.timeline.timeline.instances.length, 1);
        assert.equal(state.timeline.timeline.analog_tracks.length, 0);
        assert.equal(state.timeline.timeline.templates[0].direct_control.control_key, 'advertised-key');
        state.paused = false; state.status.hardware_sync.timeline.acknowledged = 2;
        if (options.late) state.status.hardware_sync.timeline.max_ack_lateness_ms = 58.1;
        if (options.drift) state.timeline.timeline.keyframes.push({ id: 'user-edit' });
        if (options.reverted) state.timeline = structuredClone(original);
        if (options.reorder) state.timeline.timeline.templates[0] = Object.fromEntries(Object.entries(state.timeline.timeline.templates[0]).reverse());
        break;
      case 'pealayer.pause': state.paused = true; break;
      case 'pealayer.seek_to': state.position = body.params.seconds; arm(); break;
      default: throw Error('Unexpected mutation');
    }
    return { result: { accepted: true } };
  }
  return { state, original, mutations, request };
}

test('advertised relay isolation and exact original paused session restoration', async () => {
  const f = fixture(); const result = await verifyLiveCue(f);
  assert.equal(result.acknowledged, 2); assert.deepEqual(f.state.timeline, f.original);
  assert(f.state.paused); assert.equal(f.state.position, 10);
});
test('JSON property order does not produce false concurrent-edit failure', async () => {
  assert((await verifyLiveCue(fixture({ reorder: true }))).restored);
});
test('lost mutation ACK is inspected, never replayed, and safely restored', async () => {
  const f = fixture({ lost: true }); await assert.rejects(verifyLiveCue(f), /lost mutation ACK/);
  assert.deepEqual(f.state.timeline, f.original);
  assert.equal(f.mutations.filter(value => value.expected).length, 2);
  assert(!f.mutations.some(value => value.method === 'pealayer.play'));
});
test('late ACK retains strict budget and restores session', async () => {
  const f = fixture({ late: true }); await assert.rejects(verifyLiveCue(f), /50 ms/);
  assert.deepEqual(f.state.timeline, f.original); assert(f.state.paused);
});
test('concurrent edit is paused, retained, and never overwritten', async () => {
  const f = fixture({ drift: true }); await assert.rejects(verifyLiveCue(f), /refusing overwrite/);
  assert(f.state.paused); assert.equal(f.state.timeline.timeline.keyframes[0].id, 'user-edit');
  assert.equal(f.mutations.filter(value => value.expected).length, 1);
});
test('concurrent restoration of original timeline still pauses a possibly applied Play', async () => {
  const f = fixture({ reverted: true }); await assert.rejects(verifyLiveCue(f), /refusing overwrite/);
  assert(f.state.paused); assert.deepEqual(f.state.timeline, f.original);
});
test('unauthorized relays are rejected before mutation', async () => {
  for (const relay of [1, 4, 8, NaN]) { const f = fixture(); await assert.rejects(verifyLiveCue({ ...f, relay })); assert.equal(f.mutations.length, 0); }
});
test('unsafe, unsampled, conflicting or insufficient-media baselines reject mutation', async () => {
  for (const alter of [s => { s.hardware.active_relays = [5]; }, s => { s.hardware.motion.left.applied = 'up'; },
    s => { s.hardware.telemetry.pwm_values = [null]; }, s => { s.status.hardware_sync.authority.owner_id = 'other'; }, s => { s.status.duration = 11; }]) {
    const f = fixture(); alter(f.state); await assert.rejects(verifyLiveCue(f)); assert.equal(f.mutations.length, 0);
  }
});
test('interruption before test performs no mutation', async () => {
  const f = fixture(); await assert.rejects(verifyLiveCue({ ...f, interrupted: () => true }), /Interrupted/); assert.equal(f.mutations.length, 0);
});
test('preflight validates the advertised output and arming without mutation', async () => {
  const f = fixture(); const result = await verifyLiveCue({ ...f, preflight: true });
  assert(result.preflight); assert.equal(result.control_key, 'advertised-key'); assert.equal(f.mutations.length, 0);
});
