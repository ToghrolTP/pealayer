/** Authoritative media-session projection; never retain a previous finite
 * timeline when the current item is live, absent or disabled. */
export interface SessionCapabilities {
  can_play: boolean; can_pause: boolean; can_stop: boolean;
  can_seek: boolean; can_next: boolean; can_previous: boolean;
}
export interface SessionState {
  current_video?: string | null; playing?: boolean; seekable?: boolean;
  duration?: number; playback_time?: number; playback_rate?: number;
  quick_seek_seconds?: number;
  media_keys_enabled?: boolean;
  media_session?: { title: string | null; artist: string | null; album: string | null; video: boolean };
  media_session_capabilities?: SessionCapabilities;
}
export function sessionActive(state: SessionState): boolean {
  return state.media_keys_enabled === true && Boolean(state.current_video) && Boolean(state.media_session);
}
export function sessionActionEnabled(state: SessionState, action: MediaSessionAction): boolean {
  if (!sessionActive(state)) return false;
  const capabilities = state.media_session_capabilities;
  if (!capabilities) return false;
  switch (action) {
    case 'play': return capabilities.can_play;
    case 'pause': return capabilities.can_pause;
    case 'stop': return capabilities.can_stop;
    case 'seekbackward': case 'seekforward': case 'seekto': return capabilities.can_seek;
    case 'nexttrack': return capabilities.can_next;
    case 'previoustrack': return capabilities.can_previous;
    default: return false;
  }
}
export function sessionPosition(state: SessionState): MediaPositionState | undefined {
  if (!sessionActive(state) || !state.seekable || !Number.isFinite(state.duration)
    || (state.duration ?? 0) <= 0 || !Number.isFinite(state.playback_time)
    || !Number.isFinite(state.playback_rate) || (state.playback_rate ?? 0) <= 0) return undefined;
  return { duration: state.duration!, playbackRate: state.playback_rate!,
    position: Math.min(Math.max(0, state.playback_time!), state.duration!) };
}
export function sessionSeekOffset(state: SessionState, requested: number | undefined): number | undefined {
  const offset = requested ?? state.quick_seek_seconds;
  return Number.isFinite(offset) && offset! >= 0 ? offset : undefined;
}
export function shouldPublishSessionPosition(previous: MediaPositionState | undefined,
  next: MediaPositionState | undefined, playing: boolean, previousPlaying: boolean): boolean {
  if (!previous || !next) return previous !== next;
  if (playing !== previousPlaying || next.duration !== previous.duration
    || next.playbackRate !== previous.playbackRate) return true;
  const delta = next.position! - previous.position!;
  return playing ? delta < 0 || delta >= 0.25 : delta !== 0;
}
