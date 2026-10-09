import React, { useEffect, useRef, useState } from 'react';
import type { PlayerState } from './RemoteControlTab';
import { formatTimelineTime } from '../timelineTime';

type Chapter = NonNullable<PlayerState['chapters']>[number];
export const chapterLabel = (chapter: Chapter) => `Chapter ${chapter.index + 1}${chapter.title.trim() ? ` · ${chapter.title}` : ''}`;
export function chapterAt(chapters: Chapter[], seconds: number) {
  return chapters.filter((chapter) => Number.isFinite(chapter.time_seconds) && chapter.time_seconds >= 0 && chapter.time_seconds <= seconds)
    .sort((a, b) => b.time_seconds - a.time_seconds)[0];
}

/** Preview and final commit share one ordered gesture; retain the target until
 * the authoritative decoder acknowledges it, rather than flashing old state. */
export function useSeekbar(state: PlayerState, send: (command: string, payload?: Record<string, any>) => Promise<boolean>) {
  const [draft, setDraft] = useState<number | null>(null);
  const [target, setTarget] = useState<number | null>(null);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const lastSent = useRef(0);
  const previewRevision = useRef(0);
  const baselineRevision = useRef(0);
  const mediaGeneration = useRef(0);
  const gestureTarget = useRef<number | null>(null);
  const finishGesture = useRef(() => {});
  const latest = useRef<number | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const clearTimer = () => { if (timer.current) clearTimeout(timer.current); timer.current = null; };
  useEffect(() => {
    setDraft(null); setTarget(null); latest.current = null; gestureTarget.current = null;
    previewRevision.current++; mediaGeneration.current++; clearTimer();
    return clearTimer;
  }, [state.current_video]);
  useEffect(() => {
    const finish = () => finishGesture.current();
    window.addEventListener('pointercancel', finish);
    window.addEventListener('blur', finish);
    return () => {
      window.removeEventListener('pointercancel', finish);
      window.removeEventListener('blur', finish);
    };
  }, []);
  useEffect(() => {
    if (target != null && !state.seek_pending && (state.settled_seek_revision ?? 0) > baselineRevision.current
      && state.settled_seek_target != null && Math.abs(state.settled_seek_target - target) < 0.001) {
      setDraft(null); setTarget(null);
    }
  }, [state.settled_seek_target, state.settled_seek_revision, state.seek_pending, target]);
  const flush = () => {
    clearTimer();
    const seconds = latest.current; latest.current = null;
    if (seconds == null) return;
    lastSent.current = performance.now();
    const revision = ++previewRevision.current;
    queue.current = queue.current.then(() => revision === previewRevision.current ? send('scrub_to', { seconds }) : undefined).catch(() => false);
  };
  const change = (percentage: number) => {
    setDraft(percentage); setTarget(null);
    latest.current = (state.duration ?? 0) * percentage / 100;
    gestureTarget.current = latest.current;
    const remaining = 34 - (performance.now() - lastSent.current);
    if (remaining <= 0) flush();
    else if (!timer.current) timer.current = setTimeout(flush, remaining);
  };
  const commitSeconds = (seconds: number) => {
    clearTimer(); latest.current = null; previewRevision.current++;
    gestureTarget.current = null;
    const generation = mediaGeneration.current;
    baselineRevision.current = state.settled_seek_revision ?? 0;
    setDraft(state.duration ? seconds / state.duration * 100 : 0); setTarget(seconds);
    queue.current = queue.current.then(() => generation === mediaGeneration.current ? send('finish_scrub', { seconds }) : false)
      .catch(() => false).then((ok) => {
        if (!ok && generation === mediaGeneration.current) { setDraft(null); setTarget(null); }
      });
  };
  finishGesture.current = () => { if (gestureTarget.current != null) commitSeconds(gestureTarget.current); };
  return { value: draft ?? (state.duration ? (state.playback_time ?? 0) / state.duration * 100 : 0), change,
    commit: (percentage: number) => commitSeconds((state.duration ?? 0) * percentage / 100), commitSeconds };
}

export const SeekbarMarkers: React.FC<{ state: PlayerState; seconds: number; onChapter: (seconds: number) => void }> = ({ state, seconds, onChapter }) => {
  const duration = state.duration ?? 0;
  if (!(duration > 0)) return null;
  const chapters = (state.chapters ?? []).filter((chapter) => Number.isFinite(chapter.time_seconds) && chapter.time_seconds >= 0 && chapter.time_seconds <= duration);
  const active = chapterAt(chapters, seconds);
  const end = active ? Math.min(duration, ...chapters.filter((chapter) => chapter.time_seconds > active.time_seconds).map((chapter) => chapter.time_seconds)) : 0;
  const colors = state.seekbar_markers;
  return <div className="seekbar-markers">
    {active && <span className="seekbar-markers__active" style={{ left: `${active.time_seconds / duration * 100}%`, width: `${(end - active.time_seconds) / duration * 100}%`, background: colors?.active_chapter_color ?? '#B0B0B0' }} />}
    {chapters.map((chapter) => <button key={chapter.index} type="button" className="seekbar-markers__chapter"
      disabled={!state.seekable} title={`${chapterLabel(chapter)} · ${formatTimelineTime(chapter.time_seconds)}`}
      aria-label={`${chapterLabel(chapter)} · ${formatTimelineTime(chapter.time_seconds)}`}
      style={{ left: `${chapter.time_seconds / duration * 100}%`, color: colors?.chapter_color ?? '#969696' }}
      onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); onChapter(chapter.time_seconds); }} />)}
    {(state.timeline_keyframes ?? []).filter((marker) => marker.time_ms / 1000 <= duration).map((marker) => <span key={marker.id}
      className="seekbar-markers__keyframe" title={`Keyframe${marker.label ? ` · ${marker.label}` : ''} · ${formatTimelineTime(marker.time_ms / 1000)}`}
      style={{ left: `${marker.time_ms / 1000 / duration * 100}%`, background: colors?.keyframe_color ?? '#EF4444' }} />)}
  </div>;
};
