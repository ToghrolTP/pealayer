import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { sessionActive, sessionActionEnabled, sessionPosition, sessionSeekOffset, shouldPublishSessionPosition } from '../src/mediaSession.ts';
const state = {
  current_video: 'movie.mkv', playing: true, media_keys_enabled: true, seekable: true,
  duration: 60, playback_time: 12.345, playback_rate: 1.5, quick_seek_seconds: 7.5,
  media_session: { title: 'Embedded title', artist: 'Artist', album: 'Album', video: true },
  media_session_capabilities: { can_play: false, can_pause: true, can_stop: true,
    can_seek: true, can_next: true, can_previous: false },
};
assert.equal(sessionActive(state), true);
assert.equal(sessionActive({ ...state, current_video: null }), false);
assert.equal(sessionActive({ ...state, media_keys_enabled: false }), false);
assert.equal(sessionActionEnabled(state, 'pause'), true);
assert.equal(sessionActionEnabled(state, 'play'), false);
assert.equal(sessionActionEnabled(state, 'nexttrack'), true);
assert.equal(sessionActionEnabled(state, 'previoustrack'), false);
assert.equal(sessionActionEnabled({ ...state, media_session_capabilities: undefined }, 'seekto'), false);
assert.deepEqual(sessionPosition(state), { duration: 60, playbackRate: 1.5, position: 12.345 });
assert.equal(sessionPosition({ ...state, duration: 0 }), undefined, 'live item clears finite position');
assert.equal(sessionPosition({ ...state, seekable: false }), undefined);
assert.equal(sessionPosition({ ...state, media_keys_enabled: false }), undefined);
assert.equal(sessionPosition({ ...state, playback_rate: NaN }), undefined);
assert.equal(sessionPosition({ ...state, playback_time: Infinity }), undefined);
assert.equal(sessionPosition({ ...state, playback_time: 90 }).position, 60);
assert.equal(sessionPosition({ ...state, playback_time: -1 }).position, 0);
assert.equal(sessionSeekOffset(state,undefined),7.5);
assert.equal(sessionSeekOffset(state,1.25),1.25);
assert.equal(sessionSeekOffset(state,0),0);
assert.equal(sessionSeekOffset(state,Infinity),undefined);
assert.equal(sessionSeekOffset(state,-1),undefined);
const previous = sessionPosition({ ...state, playback_time: 10 });
assert.equal(shouldPublishSessionPosition(previous, sessionPosition({ ...state, playback_time: 10.1 }), true, true), false);
assert.equal(shouldPublishSessionPosition(previous, sessionPosition({ ...state, playback_time: 10.25 }), true, true), true);
assert.equal(shouldPublishSessionPosition(previous, sessionPosition({ ...state, playback_time: 9.9 }), true, true), true);
assert.equal(shouldPublishSessionPosition(previous, sessionPosition({ ...state, playback_time: 10.1 }), false, true), true);
assert.equal(shouldPublishSessionPosition(previous, undefined, true, true), true);
const source = readFileSync(new URL('../src/webPlatform.ts', import.meta.url), 'utf8');
assert(source.includes('setPositionState(position)'), 'undefined position must clear old timeline');
assert(source.includes('mediaStateRef.current'), 'retained callbacks must revalidate current capabilities');
assert(source.includes("artist: state.media_session!.artist ?? ''"), 'do not fabricate an artist from app branding');
assert(source.includes("navigator.mediaSession.playbackState = 'none'"), 'Stop/disable must clear browser session');
console.log('OS media session: factual metadata, contextual actions, exact bounded position, live/Stop clearing and callback scope passed.');
