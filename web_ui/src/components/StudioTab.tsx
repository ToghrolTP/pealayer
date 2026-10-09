import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  AimOutlined,
  AppstoreOutlined,
  ArrowDownOutlined,
  ArrowUpOutlined,
  BookOutlined,
  CaretRightFilled,
  ClockCircleOutlined,
  DeleteOutlined,
  DesktopOutlined,
  DisconnectOutlined,
  EditOutlined,
  FastBackwardOutlined,
  FastForwardOutlined,
  FileTextOutlined,
  EyeInvisibleOutlined,
  EyeOutlined,
  LinkOutlined,
  LockOutlined,
  MoreOutlined,
  PauseOutlined,
  PlusOutlined,
  RadarChartOutlined,
  SaveOutlined,
  SettingOutlined,
  SoundOutlined,
  StopOutlined,
  UnlockOutlined,
  VideoCameraOutlined,
} from '@ant-design/icons';
import { Button, ConfigProvider, Divider, Dropdown, Empty, Input, InputNumber, message, Modal, Popconfirm, Select, Slider, Space, Tooltip } from 'antd';
import type { PlayerState } from './RemoteControlTab';
import { tr, UiLocale } from '../i18n';
import { EffectIconPicker, effectGlyph as configuredEffectGlyph } from '../effectIcons';
import { EffectRecorder } from './EffectRecorder';
import recordingColors from '../../../assets/themes/recording-colors.json';
import { mediaBasename } from '../mediaLabel';
import { formatTimelineTime } from '../timelineTime';
import { SeekThumbnailPreview } from './SeekThumbnailPreview';
import { SeekbarMarkers, useSeekbar } from './seekbar';
import { MediaSurface } from './MediaSurface';
import type { MediaGesturePreferences } from './MediaSurface';
import { defaultTimelineWheelPreferences, timelineWheelAction, timelineZoomAtPointer } from '../timelineWheel';
import type { TimelineWheelPreferences } from '../timelineWheel';
import { appendMelodySteps, sequenceDurationMs } from '../melodyCatalog';
import { MediaTrackSelectors } from './MediaTrackSelectors';

interface StudioTabProps {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  locale: UiLocale;
  appName: string;
  quickSeekSeconds: number;
  apiBaseUrl: string;
  seekbarHoverThumbnails: boolean;
  surface?: 'studio' | 'timeline';
  timelineWheelPreferences?: TimelineWheelPreferences;
  mediaGestures: MediaGesturePreferences;
}

const formatTime = (seconds = 0, showMilliseconds = true) => {
  if (!Number.isFinite(seconds) || seconds < 0) return '--:--';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const wholeSeconds = Math.floor(seconds % 60);
  const milliseconds = Math.floor((seconds % 1) * 1000);
  const prefix = hours > 0 ? `${String(hours).padStart(2, '0')}:` : '';
  return `${prefix}${String(minutes).padStart(2, '0')}:${String(wholeSeconds).padStart(2, '0')}${showMilliseconds ? `.${String(milliseconds).padStart(3, '0')}` : ''}`;
};

const effectGlyph = (target: string) => {
  if (target.startsWith('relay:')) return <RadarChartOutlined />;
  if (target === 'controller') return <AppstoreOutlined />;
  return <ClockCircleOutlined />;
};

const workspaceIconOptions = [
  { value: 'monitor', label: 'Monitor' },
  { value: 'timeline', label: 'Timeline' },
  { value: 'tabs', label: 'Tabs' },
  { value: 'window', label: 'Window' },
  { value: 'video', label: 'Video' },
  { value: 'hardware', label: 'Hardware' },
  { value: 'effects', label: 'Effects' },
  { value: 'layout', label: 'Layout' },
];

const workspaceGlyph = (icon?: string) => {
  switch (icon) {
    case 'timeline': return <EditOutlined />;
    case 'hardware': return <RadarChartOutlined />;
    case 'effects': return <AppstoreOutlined />;
    case 'tabs': return <AppstoreOutlined />;
    default: return <DesktopOutlined />;
  }
};

export const StudioTab: React.FC<StudioTabProps> = ({ state, sendCmd, locale, appName, quickSeekSeconds, apiBaseUrl, seekbarHoverThumbnails, surface = 'studio', timelineWheelPreferences = defaultTimelineWheelPreferences, mediaGestures }) => {
  const timelineGridRef = useRef<HTMLDivElement | null>(null);
  const timelinePointersRef = useRef(new Map<number, { x: number; y: number; pointerType: string }>());
  const timelineGestureRef = useRef<null | {
    kind: 'pan' | 'pinch';
    startX: number; startY: number; startScrollLeft: number; startScrollTop: number;
    startZoom: number; startDistance: number; startCenterX: number; startCenterY: number;
    axis: 'both' | 'horizontal' | 'vertical';
  }>(null);
  const [timelineZoom, setTimelineZoom] = useState(1);
  const timelineZoomRef = useRef(1);
  useEffect(() => {
    const grid = timelineGridRef.current;
    if (!grid) return;
    const onWheel = (event: WheelEvent) => {
      // The listener covers labels, cues, ruler, and empty space, not just a child.
      event.preventDefault(); // Suppress browser zoom and duplicate native scrolling.
      const scale = event.deltaMode === 1 ? 20 : event.deltaMode === 2 ? grid.clientHeight : 1;
      // Chromium reports precision-trackpad pinch as a small pixel wheel with
      // Ctrl/Meta synthesized by the browser. Keep it distinct from a real
      // Ctrl+mouse-wheel notch so preference-driven vertical scrolling remains.
      const precisionPinch = event.deltaMode === 0
        && (event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey
        && Math.abs(event.deltaY) < 80 && Math.abs(event.deltaX) < 4;
      const { action, delta } = precisionPinch
        ? { action: 'zoom' as const, delta: event.deltaY }
        : timelineWheelAction(event, timelineWheelPreferences);
      if (action === 'horizontal_scroll') grid.scrollLeft += delta * scale;
      else if (action === 'vertical_scroll') grid.scrollTop += delta * scale;
      else if (action === 'zoom' && delta !== 0) {
        const pointerX = Math.max(0, event.clientX - grid.getBoundingClientRect().left);
        const next = timelineZoomAtPointer(timelineZoomRef.current, delta * scale, grid.scrollLeft, pointerX);
        timelineZoomRef.current = next.zoom;
        // Apply width before scrollLeft so the browser does not clamp to the old extent.
        const content = grid.firstElementChild as HTMLElement | null;
        if (content) content.style.width = `${next.zoom * 100}%`;
        grid.scrollLeft = next.scrollLeft;
        setTimelineZoom(next.zoom);
      }
    };
    grid.addEventListener('wheel', onWheel, { passive: false });
    return () => grid.removeEventListener('wheel', onWheel);
  }, [timelineWheelPreferences]);
  useEffect(() => {
    const grid = timelineGridRef.current;
    if (!grid) return;
    const pointers = timelinePointersRef.current;
    const content = () => grid.firstElementChild as HTMLElement | null;
    const centerOf = (values: Array<{ x: number; y: number }>) => ({
      x: values.reduce((sum, point) => sum + point.x, 0) / values.length,
      y: values.reduce((sum, point) => sum + point.y, 0) / values.length,
    });
    const beginPinch = () => {
      const points = [...pointers.values()].slice(0, 2);
      if (points.length < 2) return;
      const center = centerOf(points);
      timelineGestureRef.current = {
        kind: 'pinch', startX: center.x, startY: center.y,
        startScrollLeft: grid.scrollLeft, startScrollTop: grid.scrollTop,
        startZoom: timelineZoomRef.current,
        startDistance: Math.max(1, Math.hypot(points[1].x - points[0].x, points[1].y - points[0].y)),
        startCenterX: center.x, startCenterY: center.y, axis: 'both',
      };
    };
    const beginPan = (point: { x: number; y: number }, axis: 'both' | 'horizontal' | 'vertical') => {
      timelineGestureRef.current = {
        kind: 'pan', startX: point.x, startY: point.y,
        startScrollLeft: grid.scrollLeft, startScrollTop: grid.scrollTop,
        startZoom: timelineZoomRef.current, startDistance: 1,
        startCenterX: point.x, startCenterY: point.y, axis,
      };
    };
    const onPointerDown = (event: PointerEvent) => {
      const element = event.target as HTMLElement;
      if (element.closest('button,input,textarea,select,.timeline-cue,[role="menuitem"]')) return;
      const isTouch = event.pointerType === 'touch' || event.pointerType === 'pen';
      if (!isTouch && event.button !== 1) return;
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY, pointerType: event.pointerType });
      try { grid.setPointerCapture(event.pointerId); } catch { /* capture can be unavailable during teardown */ }
      if (isTouch && pointers.size >= 2) beginPinch();
      else beginPan({ x: event.clientX, y: event.clientY }, event.shiftKey ? 'horizontal' : event.ctrlKey ? 'vertical' : 'both');
      grid.classList.add('is-panning');
      event.preventDefault();
    };
    const onPointerMove = (event: PointerEvent) => {
      if (!pointers.has(event.pointerId)) return;
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY, pointerType: event.pointerType });
      const gesture = timelineGestureRef.current;
      if (!gesture) return;
      if (pointers.size >= 2) {
        if (gesture.kind !== 'pinch') beginPinch();
        const pinch = timelineGestureRef.current;
        if (!pinch || pinch.kind !== 'pinch') return;
        const points = [...pointers.values()].slice(0, 2);
        const center = centerOf(points);
        const distance = Math.max(1, Math.hypot(points[1].x - points[0].x, points[1].y - points[0].y));
        const nextZoom = Math.max(1, Math.min(25, pinch.startZoom * distance / pinch.startDistance));
        const rect = grid.getBoundingClientRect();
        const anchorX = pinch.startCenterX - rect.left;
        const currentX = center.x - rect.left;
        const worldX = (pinch.startScrollLeft + anchorX) / pinch.startZoom;
        timelineZoomRef.current = nextZoom;
        if (content()) content()!.style.width = `${nextZoom * 100}%`;
        grid.scrollLeft = Math.max(0, worldX * nextZoom - currentX);
        grid.scrollTop = Math.max(0, pinch.startScrollTop + pinch.startCenterY - center.y);
        setTimelineZoom(nextZoom);
      } else if (gesture.kind === 'pan') {
        if (gesture.axis !== 'vertical') grid.scrollLeft = gesture.startScrollLeft - (event.clientX - gesture.startX);
        if (gesture.axis !== 'horizontal') grid.scrollTop = gesture.startScrollTop - (event.clientY - gesture.startY);
      }
      event.preventDefault();
    };
    const finishPointer = (event: PointerEvent) => {
      if (!pointers.has(event.pointerId)) return;
      pointers.delete(event.pointerId);
      try { if (grid.hasPointerCapture(event.pointerId)) grid.releasePointerCapture(event.pointerId); } catch { /* already released */ }
      if (pointers.size >= 2) beginPinch();
      else if (pointers.size === 1) {
        const point = [...pointers.values()][0];
        beginPan(point, 'both');
      } else {
        timelineGestureRef.current = null;
        grid.classList.remove('is-panning');
      }
    };
    grid.addEventListener('pointerdown', onPointerDown);
    grid.addEventListener('pointermove', onPointerMove);
    grid.addEventListener('pointerup', finishPointer);
    grid.addEventListener('pointercancel', finishPointer);
    return () => {
      grid.removeEventListener('pointerdown', onPointerDown);
      grid.removeEventListener('pointermove', onPointerMove);
      grid.removeEventListener('pointerup', finishPointer);
      grid.removeEventListener('pointercancel', finishPointer);
      pointers.clear();
      timelineGestureRef.current = null;
      grid.classList.remove('is-panning');
    };
  }, []);
  const [selectedEffect, setSelectedEffect] = useState<string | null>(null);
  const [effectEditorOpen, setEffectEditorOpen] = useState(false);
  const [savingEffect, setSavingEffect] = useState(false);
  const [effectDraft, setEffectDraft] = useState<Record<string, any> | null>(null);
  const captureBusy = Boolean(state.effect_recording?.active || state.effect_recording?.pending);
  const effectPayload = (draft: Record<string, any>) => {
    const { programText, steps, engine, ...payload } = draft;
    return { ...payload, program: draft.kind === 'sequence' ? {
      steps: steps ?? [], properties: { ...(draft.program?.properties ?? {}), mode: engine ?? 'auto', ...(draft.color ? { color: draft.color } : {}) },
    } : JSON.parse(programText || '{}') };
  };
  const seek = useSeekbar(state, sendCmd);
  const [workspaceManagerOpen, setWorkspaceManagerOpen] = useState(false);
  const [workspaceName, setWorkspaceName] = useState('');
  const [workspaceIcon, setWorkspaceIcon] = useState('window');
  const [cuePreviews, setCuePreviews] = useState<Record<string, { start_time_ms: number; duration_ms: number }>>({});
  const [directCueControl, setDirectCueControl] = useState<string | null>(null);
  const [directCuePercent, setDirectCuePercent] = useState(50);
  const [directCueBehavior, setDirectCueBehavior] = useState<'set-keep' | 'hold' | 'ramp'>('set-keep');
  const [directCueEndPercent, setDirectCueEndPercent] = useState(0);
  const [directCueDuration, setDirectCueDuration] = useState(1);
  const [directCueStart, setDirectCueStart] = useState(0);
  const [editingDirectCue, setEditingDirectCue] = useState<string | null>(null);
  const openDirectCue = (controlKey: string, cue?: NonNullable<PlayerState['cues']>[number]) => {
    setDirectCueControl(controlKey);
    setEditingDirectCue(cue?.id ?? null);
    setDirectCuePercent((cue?.value_basis_points ?? 5000) / 100);
    setDirectCueBehavior(cue?.behavior ?? 'set-keep');
    setDirectCueEndPercent((cue?.end_value_basis_points ?? 0) / 100);
    setDirectCueDuration((cue?.duration_ms ?? 1000) / 1000);
    setDirectCueStart((cue?.start_time_ms ?? currentSeconds * 1000) / 1000);
  };
  const [activeCueDrag, setActiveCueDrag] = useState<null | {
    id: string;
    mode: 'move' | 'resize-left' | 'resize-right';
    originX: number;
    laneWidth: number;
    startTimeMs: number;
    durationMs: number;
    moved: boolean;
  }>(null);
  const suppressCueClick = useRef<string | null>(null);
  const effects = state.effects ?? [];
  const controllerEffects = state.controller_effects ?? [];
  const cues = state.cues ?? [];
  const chapters = state.chapters ?? [];
  const currentSeconds = state.playback_time ?? 0;
  const durationSeconds = state.duration ?? 0;
  const timelineDurationMs = useMemo(() => {
    const cueEnd = cues.reduce((maximum, cue) => Math.max(maximum, cue.start_time_ms + (cue.behavior === 'set-keep' ? 0 : cue.duration_ms)), 0);
    return Math.max(durationSeconds * 1000, cueEnd, 1000);
  }, [cues, durationSeconds]);
  const selected = selectedEffect ? controllerEffects.find((effect) => effect.reference === selectedEffect) : undefined;
  const mediaName = state.current_video
    ? mediaBasename(state.current_video, tr(locale, 'Untitled'))
    : tr(locale, 'No Media Playing');
  const activeSeek = seek.value;
  const directControls = useMemo(
    () => (state.hardware_details?.controls ?? []).filter((control) =>
      !control.hidden && (control.kind === 'relay' || control.kind === 'pwm' || control.kind === 'mosfet')),
    [state.hardware_details?.controls],
  );
  const timelineLanes = useMemo(() => {
    const order = ['motion', 'relay', 'pwm', 'lighting', 'display', 'rf', 'audio', 'sequence', 'composite'];
    const active = new Set(effects.map((effect) => effect.lane || 'sequence'));
    const lanes = order.filter((lane) => active.has(lane));
    directControls.forEach((control) => {
      if (!lanes.includes(control.key)) lanes.push(control.key);
    });
    return lanes;
  }, [directControls, effects]);
  const timelineRows = useMemo(() => {
    const shared = (state.timeline_tracks ?? []).filter((track) => track.linked && track.visible);
    if (shared.length > 0) return shared;
    return timelineLanes.map((lane) => ({
      key: lane,
      name: directControls.find((control) => control.key === lane)?.name ?? (lane.charAt(0).toUpperCase() + lane.slice(1)),
      detail: null,
      kind: directControls.some((control) => control.key === lane) ? 'hardware' as const : 'effect' as const,
      lane: directControls.some((control) => control.key === lane) ? null : lane,
      control_key: directControls.some((control) => control.key === lane) ? lane : null,
      active: true, enabled: true, linked: true, visible: true, dimmed: false,
      selected: false, muted: false, soloed: false, locked: false,
      supports_mute: false, supports_solo: false, supports_lock: false, manageable: false,
    }));
  }, [directControls, state.timeline_tracks, timelineLanes]);
  const anyTimelineTrackSoloed = timelineRows.some((track) => track.supports_solo && track.soloed);
  const workspaceProfiles = useMemo(
    () => [...(state.workspace_profiles ?? [])].sort((left, right) => left.order - right.order || left.name.localeCompare(right.name)),
    [state.workspace_profiles],
  );
  const activeWorkspace = workspaceProfiles.find((profile) => profile.id === state.active_workspace_profile);

  useEffect(() => {
    try {
      const stored = window.localStorage.getItem('pealayer.effect-working-draft');
      if (stored) setEffectDraft(JSON.parse(stored));
    } catch {
      window.localStorage.removeItem('pealayer.effect-working-draft');
    }
  }, []);

  useEffect(() => {
    if (effectDraft) {
      window.localStorage.setItem('pealayer.effect-working-draft', JSON.stringify(effectDraft));
    } else {
      window.localStorage.removeItem('pealayer.effect-working-draft');
    }
  }, [effectDraft]);

  useEffect(() => {
    if (!activeCueDrag) return undefined;
    const onPointerMove = (event: PointerEvent) => {
      const deltaMs = Math.round(((event.clientX - activeCueDrag.originX) / Math.max(1, activeCueDrag.laneWidth)) * timelineDurationMs);
      let startTimeMs = activeCueDrag.startTimeMs;
      let durationMs = activeCueDrag.durationMs;
      if (activeCueDrag.mode === 'move') {
        startTimeMs = Math.max(0, activeCueDrag.startTimeMs + deltaMs);
      } else if (activeCueDrag.mode === 'resize-left') {
        const requestedStart = Math.max(0, activeCueDrag.startTimeMs + deltaMs);
        const fixedEnd = activeCueDrag.startTimeMs + activeCueDrag.durationMs;
        startTimeMs = Math.min(fixedEnd - 1, requestedStart);
        durationMs = Math.max(1, fixedEnd - startTimeMs);
      } else {
        durationMs = Math.max(1, activeCueDrag.durationMs + deltaMs);
      }
      setCuePreviews((current) => ({ ...current, [activeCueDrag.id]: { start_time_ms: startTimeMs, duration_ms: durationMs } }));
      if (Math.abs(event.clientX - activeCueDrag.originX) > 2 && !activeCueDrag.moved) {
        setActiveCueDrag((current) => current ? { ...current, moved: true } : current);
      }
    };
    const onPointerUp = () => {
      const preview = cuePreviews[activeCueDrag.id] ?? {
        start_time_ms: activeCueDrag.startTimeMs,
        duration_ms: activeCueDrag.durationMs,
      };
      if (activeCueDrag.moved) {
        suppressCueClick.current = activeCueDrag.id;
        sendCmd('effect_cue.update', {
          instance_id: activeCueDrag.id,
          start_time_ms: preview.start_time_ms,
          duration_ms: preview.duration_ms,
        });
      }
      setActiveCueDrag(null);
    };
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp, { once: true });
    return () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
    };
  }, [activeCueDrag, cuePreviews, sendCmd, timelineDurationMs]);

  const addCue = (reference: string) => {
    sendCmd('controller_effect_cue.add', {
      reference,
      start_time_ms: Math.max(0, Math.round(currentSeconds * 1000)),
    });
  };

  const editEffect = (effect?: typeof controllerEffects[number]) => {
    const program = effect?.program && typeof effect.program === 'object'
      ? effect.program as Record<string, any>
      : {};
    setEffectDraft(effect ? {
      ...effect,
      programText: JSON.stringify(program, null, 2),
      steps: Array.isArray(program.steps) ? program.steps : [],
      color: program.properties?.color,
      engine: program.properties?.mode ?? 'auto',
      default_fps: effect.default_fps ?? 20,
      default_pixels: effect.default_pixels ?? 100,
      is_new: false,
    } : {
      reference: '', id: String(Array.from({ length: 256 }, (_, id) => id).find((id) => !controllerEffects.some((effect) => effect.id === String(id))) ?? ''), name: '', icon: 'sparkle', category: 'Effects', description: '',
      kind: 'sequence', programText: '{}', steps: [], engine: 'auto',
      color: 'violet', default_fps: 20, duration_ms: 1, default_pixels: 100,
      is_new: true,
    });
    setEffectEditorOpen(true);
  };

  const addSequenceStep = () => {
    if (!effectDraft) return;
    const previous = effectDraft.steps?.[effectDraft.steps.length - 1];
    setEffectDraft({
      ...effectDraft,
      steps: [...(effectDraft.steps ?? []), {
        at_us: previous ? Number(previous.at_us ?? 0) + 250_000 : 0,
        kind: 'relay', target: 4, value: 1,
      }],
    });
  };
  const addMelody = (name: string) => {
    if (!effectDraft) return;
    const melody = state.hardware_details?.melodies?.find((item) => item.name === name);
    if (!melody) return;
    const steps = appendMelodySteps(effectDraft.steps ?? [], melody);
    setEffectDraft({ ...effectDraft, steps, duration_ms: sequenceDurationMs(steps) });
  };

  return (
    <div className={`studio-suite studio-suite--${surface}`}>
      <section className="studio-panel effects-panel">
        <header className="studio-panel__header">
          <div>
            <span className="eyebrow">{tr(locale, 'Live project')}</span>
            <h2>{tr(locale, 'Effects')}</h2>
          </div>
          <div className="effects-heading-actions">
            <Button type="text" icon={<PlusOutlined />} onClick={() => editEffect()} aria-label={tr(locale, 'New effect')} />
            <span className="count-badge">{controllerEffects.length}</span>
          </div>
        </header>

        <div className="effect-list">
          {controllerEffects.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={tr(locale, 'No effects')}
            />
          ) : controllerEffects.map((effect) => {
            const selectedCard = selectedEffect === effect.reference;
            return (
              <article
                key={effect.reference}
                className={`effect-profile ${selectedCard ? 'is-selected' : ''}`}
                onClick={() => setSelectedEffect(effect.reference)}
                draggable
                onDragStart={(event) => {
                  event.dataTransfer.effectAllowed = 'copy';
                  event.dataTransfer.setData('application/x-pealayer-effect', effect.reference);
                  event.dataTransfer.setData('text/plain', effect.name);
                }}
              >
                <span className="effect-profile__icon">{configuredEffectGlyph(effect.icon)}</span>
                <div className="effect-profile__body">
                  <strong>{effect.name}</strong>
                  <span>{effect.category} · {effect.action_count} {tr(locale, 'actions')} · {effect.duration_display}</span>
                </div>
                <Tooltip title={tr(locale, 'Play effect')}>
                  <Button
                    type="text"
                    icon={<CaretRightFilled />}
                    onClick={(event) => {
                      event.stopPropagation();
                      sendCmd('controller_effect.play', { reference: effect.reference });
                    }}
                  />
                </Tooltip>
                <Tooltip title={tr(locale, 'Add cue at playhead')}>
                  <Button
                    type="text"
                    icon={<PlusOutlined />}
                    onClick={(event) => {
                      event.stopPropagation();
                      addCue(effect.reference);
                    }}
                  />
                </Tooltip>
              </article>
            );
          })}
        </div>

        <footer className="effects-panel__footer">
          <span className={`status-light ${state.hardware_connected ? 'is-online' : ''}`} />
          <span>{state.hardware?.board_name || tr(locale, 'No hardware')}</span>
          {state.hardware_sync && <span title={state.hardware_sync.error ||
            `Revision ${state.hardware_sync.revision} · ${state.hardware_sync.timeline?.acknowledged || 0}/${state.hardware_sync.timeline?.step_count || 0} ACK · max ${state.hardware_sync.timeline?.max_ack_lateness_ms || 0} ms`}>
            {state.hardware_sync.error ? tr(locale, 'Hardware timing fault') :
              state.hardware_sync.revision !== state.hardware_sync.prepared_revision ? tr(locale, 'Preparing hardware') : tr(locale, 'Hardware timeline')}
          </span>}
          {selected && (
            <span className="effect-selection-actions">
              {selected.editable && <Button type="text" size="small" icon={<EditOutlined />} onClick={() => editEffect(selected)} />}
              <Popconfirm
                title={tr(locale, 'Delete effect?')}
                onConfirm={() => sendCmd('controller_effect.delete', { reference: selected.reference })}
              >
                <Button type="text" danger size="small" icon={<DeleteOutlined />} />
              </Popconfirm>
              <strong>{selected.name}</strong>
            </span>
          )}
        </footer>
      </section>

      <Modal
        title={effectDraft?.is_new ? tr(locale, 'New effect') : tr(locale, 'Effect properties')}
        open={effectEditorOpen}
        onCancel={() => { if (!captureBusy && !savingEffect) setEffectEditorOpen(false); }}
        closable={!captureBusy && !savingEffect}
        maskClosable={!captureBusy && !savingEffect}
        keyboard={!captureBusy && !savingEffect}
        okButtonProps={{ disabled: captureBusy || savingEffect }}
        cancelButtonProps={{ disabled: captureBusy || savingEffect }}
        confirmLoading={savingEffect}
        okText={tr(locale, 'Save')}
        width={640}
        onOk={async () => {
          if (!effectDraft || savingEffect) return;
          setSavingEffect(true);
          try {
            if (!await sendCmd('controller_effect.save', effectPayload(effectDraft))) return;
            window.localStorage.removeItem('pealayer.effect-working-draft');
            setEffectEditorOpen(false);
          }
          catch { void message.error(tr(locale, 'Program must be valid JSON')); return; }
          finally { setSavingEffect(false); }
        }}
      >
        {effectDraft && (
          <ConfigProvider componentDisabled={captureBusy}><div className="effect-editor-grid">
            <label><span>{tr(locale, 'Type')}</span><Select value={effectDraft.kind} options={[{ value: 'sequence', label: tr(locale, 'Timed sequence') }, { value: 'strip-stream', label: tr(locale, 'Addressable lighting') }]} onChange={(kind) => setEffectDraft({ ...effectDraft, kind })} /></label>
            <label><span>{tr(locale, 'ID')}</span><Input value={effectDraft.id} onChange={(event) => setEffectDraft({ ...effectDraft, id: event.target.value })} /></label>
            <label><span>{tr(locale, 'Name')}</span><Input value={effectDraft.name} onChange={(event) => setEffectDraft({ ...effectDraft, name: event.target.value })} /></label>
            <label><span>{tr(locale, 'Icon')}</span><EffectIconPicker value={effectDraft.icon} searchPlaceholder={tr(locale, 'Search icons...')} presetsLabel={tr(locale, 'Presets')} emptyLabel={tr(locale, 'No matching icons')} onChange={(icon) => setEffectDraft({ ...effectDraft, icon })} /></label>
            <label><span>{tr(locale, 'Category')}</span><Input value={effectDraft.category} onChange={(event) => setEffectDraft({ ...effectDraft, category: event.target.value })} /></label>
            <label className="effect-editor-grid__wide"><span>{tr(locale, 'Description')}</span><Input value={effectDraft.description} onChange={(event) => setEffectDraft({ ...effectDraft, description: event.target.value })} /></label>
            <label><span>{tr(locale, 'Duration (ms)')}</span><InputNumber min={1} value={effectDraft.duration_ms} onChange={(duration_ms) => setEffectDraft({ ...effectDraft, duration_ms: duration_ms ?? 1 })} /></label>
            {effectDraft.kind === 'strip-stream' && <label><span>{tr(locale, 'Frames per second')}</span><InputNumber min={1} max={120} value={effectDraft.default_fps} onChange={(default_fps) => setEffectDraft({ ...effectDraft, default_fps: default_fps ?? 20 })} /></label>}
            {effectDraft.kind === 'strip-stream' && <label><span>{tr(locale, 'Pixels')}</span><InputNumber min={1} value={effectDraft.default_pixels} onChange={(default_pixels) => setEffectDraft({ ...effectDraft, default_pixels: default_pixels ?? 100 })} /></label>}
            {effectDraft.kind === 'sequence' && <label><span>{tr(locale, 'Color')}</span><Select disabled={captureBusy} value={effectDraft.color} onChange={(color) => setEffectDraft({ ...effectDraft, color })} options={recordingColors.map((color) => ({ value: color.id, label: <span className="recording-color-option"><span className="recording-color-swatch" style={{ backgroundColor: color.hex }} />{tr(locale, color.label)}</span> }))} /></label>}
            {effectDraft.kind === 'sequence' && <label><span>{tr(locale, 'Execution')}</span><Select value={effectDraft.engine ?? 'auto'} options={[
              { value: 'auto', label: tr(locale, 'Automatic (recommended)') },
              { value: 'host', label: tr(locale, 'Host clock') },
              { value: 'mcu', label: tr(locale, 'Device clock') },
            ]} onChange={(engine) => setEffectDraft({ ...effectDraft, engine })} /></label>}
            {effectDraft.kind === 'strip-stream' && <label className="effect-editor-grid__wide"><span>{tr(locale, 'Program')}</span><Input.TextArea autoSize={{ minRows: 7, maxRows: 16 }} value={effectDraft.programText} onChange={(event) => setEffectDraft({ ...effectDraft, programText: event.target.value })} /></label>}
            {effectDraft.kind === 'sequence' && (
              <div className="effect-editor-grid__wide sequence-editor-web">
                <Divider titlePlacement="start" plain>{tr(locale, 'Sequence steps')}</Divider>
                <div className="sequence-editor-web__toolbar">
                  <ConfigProvider componentDisabled={false}><EffectRecorder state={state} sendCmd={sendCmd} locale={locale} effect={effectPayload(effectDraft)} onSequenceChange={(steps, id) => setEffectDraft((current) => current ? { ...current, steps, id: String(id), reference: `effect:${id}`, is_new: false } : current)} /></ConfigProvider>
                  <Button icon={<PlusOutlined />} onClick={addSequenceStep}>{tr(locale, 'Add step')}</Button>
                  <Select
                    className="effect-melody-picker"
                    disabled={captureBusy || !state.controller_connected}
                    placeholder={<><SoundOutlined /> {tr(locale, 'Add melody')}</>}
                    value={undefined}
                    options={(state.hardware_details?.melodies ?? []).map((melody) => ({
                      value: melody.name,
                      label: `${melody.name} · ${melody.duration_ms} ms`,
                    }))}
                    notFoundContent={tr(locale, 'No configured melodies')}
                    onOpenChange={(open) => { if (open) void sendCmd('hardware.catalog.refresh'); }}
                    onChange={addMelody}
                  />
                  <span>{(effectDraft.steps ?? []).length} {tr(locale, 'actions')}</span>
                </div>
                <div className="sequence-step-list">
                  {(effectDraft.steps ?? []).length === 0 ? (
                    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={tr(locale, 'No actions yet')} />
                  ) : (effectDraft.steps ?? []).map((step: Record<string, any>, index: number) => (
                    <div className="sequence-step-row" key={`${index}:${step.at_us}:${step.kind}`}>
                      <span className="sequence-step-row__index">{index + 1}</span>
                      <label><span>{tr(locale, 'Time')}</span><InputNumber min={0} step={10} addonAfter="ms" value={Math.round(Number(step.at_us ?? 0) / 1000)} onChange={(value) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, at_us: Math.max(0, Number(value ?? 0)) * 1000 }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      <label><span>{tr(locale, 'Action')}</span><Select value={step.kind} options={[
                        { value: 'relay', label: tr(locale, 'Relay') },
                        { value: 'motion', label: tr(locale, 'Seat motion') },
                        { value: 'pwm', label: tr(locale, 'PWM') },
                        { value: 'status-led', label: tr(locale, 'Status light') },
                        { value: 'addressable', label: tr(locale, 'Strip pixel') },
                        { value: 'display', label: tr(locale, 'Display text') },
                        { value: 'rf', label: tr(locale, 'RF code') },
                        { value: 'beep', label: tr(locale, 'Buzzer') },
                      ]} onChange={(kind) => {
                        const steps = [...effectDraft.steps]; steps[index] = { at_us: step.at_us ?? 0, kind, target: 0, value: kind === 'motion' ? 1 : 0 }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      {['relay', 'motion', 'pwm', 'addressable'].includes(step.kind) && <label><span>{tr(locale, step.kind === 'motion' ? 'Side' : 'Channel')}</span><InputNumber min={0} max={step.kind === 'relay' ? 7 : step.kind === 'motion' ? 1 : 15} value={step.target ?? 0} onChange={(target) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, target: Number(target ?? 0) }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>}
                      {['relay', 'motion', 'pwm'].includes(step.kind) && <label><span>{tr(locale, 'Value')}</span><InputNumber min={0} max={step.kind === 'relay' ? 1 : step.kind === 'motion' ? 2 : 4095} value={step.value ?? 0} onChange={(value) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, value: Number(value ?? 0) }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>}
                      <label><span>{tr(locale, 'Duration')}</span><InputNumber min={0} step={50} addonAfter="ms" value={step.duration_ms ?? 0} onChange={(duration_ms) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, duration_ms: Number(duration_ms ?? 0) || undefined }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      <label><span>{tr(locale, 'Repeat')}</span><InputNumber min={0} max={1000} value={step.repeat_count ?? 0} onChange={(repeat_count) => {
                        const steps = [...effectDraft.steps]; steps[index] = { ...step, repeat_count: Number(repeat_count ?? 0) || undefined }; setEffectDraft({ ...effectDraft, steps });
                      }} /></label>
                      <Space.Compact className="sequence-step-row__actions">
                        <Button icon={<ArrowUpOutlined />} disabled={index === 0} onClick={() => {
                          const steps = [...effectDraft.steps]; [steps[index - 1], steps[index]] = [steps[index], steps[index - 1]]; setEffectDraft({ ...effectDraft, steps });
                        }} />
                        <Button icon={<ArrowDownOutlined />} disabled={index + 1 === effectDraft.steps.length} onClick={() => {
                          const steps = [...effectDraft.steps]; [steps[index + 1], steps[index]] = [steps[index], steps[index + 1]]; setEffectDraft({ ...effectDraft, steps });
                        }} />
                        <Button danger icon={<DeleteOutlined />} onClick={() => setEffectDraft({ ...effectDraft, steps: effectDraft.steps.filter((_: unknown, stepIndex: number) => stepIndex !== index) })} />
                      </Space.Compact>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div></ConfigProvider>
        )}
      </Modal>

      <Modal
        title={tr(locale, 'Workspaces')}
        open={workspaceManagerOpen}
        footer={null}
        width={720}
        onCancel={() => setWorkspaceManagerOpen(false)}
      >
        <div className="workspace-create-row">
          <Input
            value={workspaceName}
            placeholder={tr(locale, 'Workspace name')}
            onChange={(event) => setWorkspaceName(event.target.value)}
          />
          <Select
            value={workspaceIcon}
            options={workspaceIconOptions.map((option) => ({
              ...option,
              label: <span className="workspace-icon-option">{workspaceGlyph(option.value)} {tr(locale, option.label)}</span>,
            }))}
            onChange={setWorkspaceIcon}
          />
          <Button
            type="primary"
            icon={<PlusOutlined />}
            disabled={!workspaceName.trim()}
            onClick={() => {
              sendCmd('workspace.create', { name: workspaceName.trim(), icon: workspaceIcon });
              setWorkspaceName('');
            }}
          >
            {tr(locale, 'Add workspace')}
          </Button>
        </div>
        <div className="workspace-profile-list">
          {workspaceProfiles.map((profile, index) => (
            <div className={`workspace-profile-row ${profile.id === state.active_workspace_profile ? 'is-active' : ''}`} key={profile.id}>
              <span className="workspace-profile-row__state">{workspaceGlyph(profile.icon)}</span>
              <Input
                defaultValue={profile.name}
                key={`${profile.id}:${profile.name}`}
                onBlur={(event) => {
                  const name = event.target.value.trim();
                  if (name && name !== profile.name) {
                    sendCmd('workspace.update', { id: profile.id, name, icon: profile.icon, capture: false });
                  }
                }}
                onPressEnter={(event) => event.currentTarget.blur()}
              />
              <Select
                value={profile.icon}
                options={workspaceIconOptions.map((option) => ({
                  ...option,
                  label: <span className="workspace-icon-option">{workspaceGlyph(option.value)} {tr(locale, option.label)}</span>,
                }))}
                onChange={(icon) => sendCmd('workspace.update', {
                  id: profile.id,
                  name: profile.name,
                  icon,
                  capture: false,
                })}
              />
              <Tooltip title={tr(locale, 'Restore')}>
                <Button icon={workspaceGlyph(profile.icon)} onClick={() => sendCmd('set_workspace', { profile: profile.id })} />
              </Tooltip>
              <Tooltip title={tr(locale, 'Replace with current workspace')}>
                <Button icon={<SaveOutlined />} onClick={() => sendCmd('workspace.update', {
                  id: profile.id,
                  name: profile.name,
                  icon: profile.icon,
                  capture: true,
                })} />
              </Tooltip>
              <Button
                icon={<ArrowUpOutlined />}
                disabled={index === 0}
                onClick={() => sendCmd('workspace.move', { id: profile.id, direction: -1 })}
              />
              <Button
                icon={<ArrowDownOutlined />}
                disabled={index + 1 === workspaceProfiles.length}
                onClick={() => sendCmd('workspace.move', { id: profile.id, direction: 1 })}
              />
              <Popconfirm
                title={tr(locale, 'Delete workspace?')}
                onConfirm={() => sendCmd('workspace.delete', { id: profile.id })}
              >
                <Button danger icon={<DeleteOutlined />} />
              </Popconfirm>
            </div>
          ))}
        </div>
      </Modal>

      <section className="studio-panel player-panel-web">
        <header className="studio-panel__header">
          <div>
            <span className="eyebrow">{tr(locale, 'Program monitor')}</span>
            <h2>{mediaName}</h2>
          </div>
          <span className={`transport-state ${state.playing ? 'is-playing' : ''}`}>
            {state.playing ? tr(locale, 'Playing') : tr(locale, 'Paused')}
          </span>
        </header>

        <div className="program-viewer">
          {state.current_video ? (
            <MediaSurface state={state} apiBaseUrl={apiBaseUrl} emptyLabel={tr(locale, 'Video Preview')} sendCmd={sendCmd} gestures={mediaGestures} locale={locale} />
          ) : (
            <div className="program-viewer__empty">
              <VideoCameraOutlined />
              <strong>{tr(locale, 'No Media Playing')}</strong>
            </div>
          )}
          <div className="program-viewer__chrome">
            <span>{appName}</span>
            <span>{state.workspace === 'nle' ? tr(locale, 'Timeline') : tr(locale, 'Player')}</span>
          </div>
        </div>

        <div className="studio-transport">
          <Tooltip title={`${tr(locale, 'Seek backward')} ${quickSeekSeconds}s`}>
            <Button icon={<FastBackwardOutlined />} onClick={() => sendCmd('seek', { seconds: -quickSeekSeconds })} />
          </Tooltip>
          <Button
            className="studio-transport__play"
            icon={state.playing ? <PauseOutlined /> : <CaretRightFilled />}
            onClick={() => sendCmd('toggle_pause')}
          />
          <Tooltip title={`${tr(locale, 'Seek forward')} ${quickSeekSeconds}s`}>
            <Button icon={<FastForwardOutlined />} onClick={() => sendCmd('seek', { seconds: quickSeekSeconds })} />
          </Tooltip>
          <span className="studio-timecode">{formatTime(currentSeconds)}</span>
          <SeekThumbnailPreview
            enabled={seekbarHoverThumbnails && Boolean(state.seekable) && durationSeconds > 0}
            duration={durationSeconds}
            mediaIdentity={state.current_video}
            apiBaseUrl={apiBaseUrl}
            unavailableLabel={tr(locale, 'Preview unavailable')}
            className="studio-scrubber"
            chapters={state.chapters}
            keyframes={state.timeline_keyframes}
          >
            <Slider
              min={0}
              max={100}
              value={activeSeek}
              disabled={!state.seekable || durationSeconds <= 0}
              onChange={seek.change}
              onChangeComplete={seek.commit}
              tooltip={seekbarHoverThumbnails ? { open: false } : { formatter: (value) => formatTime(((value ?? 0) / 100) * durationSeconds) }}
            />
            <SeekbarMarkers state={state} seconds={activeSeek / 100 * durationSeconds} onChapter={seek.commitSeconds} />
          </SeekThumbnailPreview>
          <span className="studio-timecode studio-timecode--muted">
            {state.live ? tr(locale, 'LIVE') : formatTime(durationSeconds)}
          </span>
          <SoundOutlined className="volume-icon" />
          <Slider
            className="studio-volume"
            min={0}
            max={130}
            value={state.muted ? 0 : (state.volume ?? 0)}
            onChange={(value) => sendCmd('set_volume', { value })}
          />
        </div>
        <MediaTrackSelectors state={state} sendCmd={sendCmd} locale={locale} compact />
      </section>

      <section className="studio-panel timeline-panel-web">
        <header className="timeline-toolbar">
          <div>
            <span className="eyebrow">{tr(locale, 'Effect timeline')}</span>
            <h2>{tr(locale, 'Cues')}</h2>
          </div>
          <div className="timeline-toolbar__actions">
            <Dropdown
              menu={{
                selectedKeys: state.active_workspace_profile ? [state.active_workspace_profile] : [],
                items: workspaceProfiles.map((profile) => ({
                  key: profile.id,
                  icon: workspaceGlyph(profile.icon),
                  label: profile.name,
                })),
                onClick: ({ key }) => sendCmd('set_workspace', { profile: key }),
              }}
            >
              <Button icon={workspaceGlyph(activeWorkspace?.icon)}>
                {activeWorkspace?.name ?? tr(locale, 'Workspaces')}
              </Button>
            </Dropdown>
            <Tooltip title={tr(locale, 'Manage workspaces')}>
              <Button icon={<SettingOutlined />} onClick={() => setWorkspaceManagerOpen(true)} />
            </Tooltip>
            {chapters.length > 0 && (
              <Dropdown
                menu={{
                  selectedKeys: state.current_chapter_index === null || state.current_chapter_index === undefined
                    ? []
                    : [String(state.current_chapter_index)],
                  items: chapters.map((chapter) => ({
                    key: String(chapter.index),
                    label: `${formatTime(chapter.time_seconds, false)} · ${chapter.title}`,
                  })),
                  onClick: ({ key }) => sendCmd('set_chapter', { index: Number(key) }),
                }}
              >
                <Button icon={<BookOutlined />}>{tr(locale, 'Chapters')}</Button>
              </Dropdown>
            )}
            <span className="timeline-meta">{cues.length} {tr(locale, 'cues')}</span>
          </div>
        </header>

        <div className="timeline-grid" ref={timelineGridRef}>
        <div className="timeline-content" style={{ width: `${timelineZoom * 100}%` }}>
        <div className="timeline-ruler">
          <span>{formatTimelineTime(0)}</span>
          <span>{formatTimelineTime((timelineDurationMs / 1000) * .25)}</span>
          <span>{formatTimelineTime((timelineDurationMs / 1000) * .5)}</span>
          <span>{formatTimelineTime((timelineDurationMs / 1000) * .75)}</span>
          <span>{formatTimelineTime(timelineDurationMs / 1000)}</span>
        </div>

        <div
          className="timeline-tracks"
          onDragOver={(event) => {
            if (event.dataTransfer.types.includes('application/x-pealayer-effect')) {
              event.preventDefault();
              event.dataTransfer.dropEffect = 'copy';
            }
          }}
          onDrop={(event) => {
            const reference = event.dataTransfer.getData('application/x-pealayer-effect');
            if (!reference) return;
            event.preventDefault();
            const rect = event.currentTarget.getBoundingClientRect();
            const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / Math.max(1, rect.width)));
            sendCmd('controller_effect_cue.add', { reference, start_time_ms: Math.round(fraction * timelineDurationMs) });
          }}
        >
          <div className="timeline-markers-web">
          <div
            className="timeline-playhead-web"
            style={{ left: `${Math.min(100, (currentSeconds * 1000 / timelineDurationMs) * 100)}%` }}
          />
          {chapters.map((chapter) => (
            <button
              key={chapter.index}
              type="button"
              className={`timeline-chapter-web ${state.current_chapter_index === chapter.index ? 'is-active' : ''}`}
              style={{ left: `${Math.min(100, (chapter.time_seconds * 1000 / timelineDurationMs) * 100)}%` }}
              title={`${chapter.title} · ${formatTime(chapter.time_seconds)}`}
              aria-label={`${tr(locale, 'Chapter')}: ${chapter.title}`}
              onClick={() => sendCmd('set_chapter', { index: chapter.index })}
            />
          ))}
          </div>
          {timelineRows.length === 0 ? (
            <div className="timeline-empty">{tr(locale, 'No effects')}</div>
          ) : timelineRows.map((track) => {
            const lane = track.lane ?? track.control_key ?? track.key;
            const laneEffects = effects.filter((effect) => (effect.lane || 'sequence') === lane);
            const laneEffectIds = new Set(laneEffects.map((effect) => effect.id));
            const effectCues = cues.filter((cue) => laneEffectIds.has(cue.effect_id)
              || Boolean(track.control_key && cue.control_key === track.control_key));
            const directControl = directControls.find((control) => control.key === track.control_key);
            const trackIcon = track.kind === 'video' ? <VideoCameraOutlined />
              : track.kind === 'audio' ? <SoundOutlined />
                : track.kind === 'subtitle' ? <FileTextOutlined />
                  : effectGlyph(lane === 'relay' || lane === 'motion' ? 'relay:lane' : lane === 'sequence' ? 'controller' : lane);
            const updateTrack = (values: Record<string, boolean>) => sendCmd('timeline.track.update', {
              key: track.key,
              ...values,
            });
            const excludedBySolo = anyTimelineTrackSoloed && track.supports_solo && !track.soloed;
            const trackMenu = {
              items: [
                ...(track.manageable ? [{ key: 'manage', icon: <SettingOutlined />, label: tr(locale, 'Manage...') }] : []),
                ...(track.manageable ? [{ type: 'divider' as const }] : []),
                { key: 'linked', icon: track.linked ? <DisconnectOutlined /> : <LinkOutlined />, label: tr(locale, track.linked ? 'Unlink from timeline' : 'Link to timeline') },
                { key: 'visible', icon: track.visible ? <EyeInvisibleOutlined /> : <EyeOutlined />, label: tr(locale, track.visible ? 'Hide timeline track' : 'Show timeline track') },
                ...(track.supports_mute ? [{ key: 'muted', icon: <StopOutlined />, label: tr(locale, track.muted ? 'Unmute' : 'Mute') }] : []),
                ...(track.supports_solo ? [{ key: 'soloed', icon: <AimOutlined />, label: tr(locale, track.soloed ? 'Unsolo' : 'Solo') }] : []),
                ...(track.supports_lock ? [{ key: 'locked', icon: track.locked ? <UnlockOutlined /> : <LockOutlined />, label: tr(locale, track.locked ? 'Unlock' : 'Lock') }] : []),
              ],
              onClick: ({ key }: { key: string }) => {
                if (key === 'manage') void sendCmd('timeline.track.manage', { key: track.key });
                else if (key === 'linked') void updateTrack({ linked: !track.linked });
                else if (key === 'visible') void updateTrack({ visible: !track.visible });
                else if (key === 'muted') void updateTrack({ muted: !track.muted });
                else if (key === 'soloed') void updateTrack({ soloed: !track.soloed });
                else if (key === 'locked') void updateTrack({ locked: !track.locked });
              },
            };
            return (
              <div className={`timeline-row ${track.dimmed ? 'is-dimmed' : ''} ${track.active ? 'is-active' : ''} ${track.selected ? 'is-selected' : ''} ${track.muted ? 'is-muted' : ''} ${track.soloed ? 'is-soloed' : ''} ${excludedBySolo ? 'is-solo-filtered' : ''} ${track.locked ? 'is-locked' : ''}`} key={track.key}>
                <Dropdown trigger={['contextMenu']} menu={trackMenu}>
                <div
                  className="timeline-row__label"
                  onClick={() => updateTrack({ selected: true })}
                  title={track.detail ? `${track.name} — ${track.detail}` : track.name}
                >
                  <span>{trackIcon}</span>
                  <span className="timeline-row__identity"><strong>{track.name}</strong>{track.detail && <small title={track.detail}>{track.detail}</small>}</span>
                  {directControl && (
                    <Dropdown
                      trigger={['click']}
                      menu={{
                        items: directControl.kind === 'relay'
                          ? [
                            { key: '10000', label: tr(locale, 'On') },
                            { key: '0', label: tr(locale, 'Off') },
                            { key: 'custom', label: tr(locale, 'Timed cue...') },
                          ]
                          : [0, 25, 50, 75, 100].map((value) => ({ key: String(value * 100), label: `${value}%` }))
                            .concat([{ key: 'custom', label: tr(locale, 'Custom value...') }]),
                        onClick: ({ key }) => {
                          if (key === 'custom') {
                            openDirectCue(directControl.key);
                          } else {
                            sendCmd('direct_cue.add', {
                              control_key: directControl.key,
                              value_basis_points: Number(key),
                              start_time_ms: Math.max(0, Math.round(currentSeconds * 1000)),
                              duration_ms: 1000,
                              behavior: 'set-keep',
                            });
                          }
                        },
                      }}
                    >
                      <Button type="text" size="small" icon={<PlusOutlined />} aria-label={tr(locale, 'Add cue at playhead')} />
                    </Dropdown>
                  )}
                  <Dropdown trigger={['click']} menu={trackMenu}>
                    <Button
                      className="timeline-row__menu"
                      type="text"
                      size="small"
                      icon={<MoreOutlined />}
                      aria-label={tr(locale, 'Track actions')}
                      onClick={(event) => event.stopPropagation()}
                    />
                  </Dropdown>
                </div>
                </Dropdown>
                <div className="timeline-lane">
                  {effectCues.map((cue) => (
                    (() => {
                      const placement = cuePreviews[cue.id] ?? cue;
                      return (
                    <Dropdown key={cue.id} trigger={['contextMenu']} menu={{ items: [
                      ...(cue.control_key ? [{ key: 'manage', label: tr(locale, 'Manage...'), icon: <EditOutlined /> }] : []),
                      { key: 'jump', label: tr(locale, 'Jump to cue start'), icon: <AimOutlined /> },
                      { key: 'delete', label: tr(locale, 'Delete cue'), icon: <DeleteOutlined />, danger: true },
                    ], onClick: ({ key }) => {
                      if (key === 'manage' && cue.control_key) openDirectCue(cue.control_key, cue);
                      if (key === 'jump') sendCmd('seek_to', { seconds: placement.start_time_ms / 1000 });
                      if (key === 'delete') sendCmd('pealayer.timeline.effect.remove', { instance_id: cue.id });
                    } }}>
                    <div
                      key={cue.id}
                      role="button"
                      tabIndex={0}
                      aria-label={cue.name}
                      className={`timeline-cue ${cue.behavior === 'set-keep' ? 'timeline-cue--state' : ''} ${cue.behavior === 'ramp' ? 'timeline-cue--ramp' : ''}`}
                      style={{
                        left: `${(placement.start_time_ms / timelineDurationMs) * 100}%`,
                        width: cue.behavior === 'set-keep' ? 88 : `${Math.max(1.2, (placement.duration_ms / timelineDurationMs) * 100)}%`,
                        ...(cue.behavior === 'ramp' ? { background: `linear-gradient(90deg, color-mix(in srgb, var(--accent) ${25 + (cue.value_basis_points ?? 0) / 200}%, var(--surface-0)), color-mix(in srgb, var(--accent) ${25 + (cue.end_value_basis_points ?? 0) / 200}%, var(--surface-0)))` } : {}),
                      }}
                      title={`${cue.name} · ${formatTime(placement.start_time_ms / 1000)} · ${tr(locale, cue.behavior === 'set-keep' ? 'Set and keep — until next command; drag to move' : cue.resizable ? 'Drag to move; use the edges to resize' : 'Recorded effect · drag to move')}`}
                      onDoubleClick={() => { if (cue.control_key) openDirectCue(cue.control_key, cue); }}
                      onPointerDown={(event) => {
                        if (event.button !== 0 || (event.target as HTMLElement).closest('.timeline-cue__delete')) return;
                        event.preventDefault();
                        const cueRect = event.currentTarget.getBoundingClientRect();
                        const laneRect = event.currentTarget.parentElement?.getBoundingClientRect();
                        const edge = Math.min(12, cueRect.width * .3);
                        const localX = event.clientX - cueRect.left;
                        const mode = cue.resizable && localX <= edge
                          ? 'resize-left'
                          : cue.resizable && localX >= cueRect.width - edge
                            ? 'resize-right'
                            : 'move';
                        setActiveCueDrag({
                          id: cue.id,
                          mode,
                          originX: event.clientX,
                          laneWidth: laneRect?.width ?? 1,
                          startTimeMs: placement.start_time_ms,
                          durationMs: placement.duration_ms,
                          moved: false,
                        });
                      }}
                      onClick={() => {
                        if (suppressCueClick.current === cue.id) {
                          suppressCueClick.current = null;
                          return;
                        }
                        // Selection/editing must not move the media playhead.
                      }}
                      onKeyDown={(event) => {
                        if (event.target !== event.currentTarget || !['Enter', ' '].includes(event.key)) return;
                        event.preventDefault();
                        if (cue.control_key) openDirectCue(cue.control_key, cue);
                      }}
                    >
                      {cue.resizable && <span className="timeline-cue__resize timeline-cue__resize--left" aria-hidden="true" />}
                      <span>{cue.behavior === 'set-keep' ? `${cue.control_key?.startsWith('relay.') ? (Number(cue.value_basis_points) >= 5000 ? 'On' : 'Off') : `${Number(cue.value_basis_points) / 100}%`} → ∞` : cue.name}</span>
                      <Button
                        className="timeline-cue__delete"
                        type="text"
                        size="small"
                        icon={<DeleteOutlined />}
                        onClick={(event) => {
                          event.stopPropagation();
                          sendCmd('pealayer.timeline.effect.remove', { instance_id: cue.id });
                        }}
                      />
                      {cue.resizable && <span className="timeline-cue__resize timeline-cue__resize--right" aria-hidden="true" />}
                    </div>
                    </Dropdown>
                      );
                    })()
                  ))}
                </div>
                <span className="timeline-row__duration">{effectCues.length}</span>
              </div>
            );
          })}
        </div>
        </div>
        </div>
      </section>
      <Modal
        open={directCueControl !== null}
        title={tr(locale, editingDirectCue ? 'Manage cue' : 'Add hardware cue')}
        okText={tr(locale, editingDirectCue ? 'Save' : 'Add cue')}
        cancelText={tr(locale, 'Cancel')}
        onCancel={() => setDirectCueControl(null)}
        onOk={async () => {
          if (!directCueControl) return;
          const saved = await sendCmd(editingDirectCue ? 'direct_cue.value' : 'direct_cue.add', {
            instance_id: editingDirectCue,
            control_key: directCueControl,
            value_basis_points: Math.round(directCuePercent * 100),
            start_time_ms: Math.max(0, Math.round(directCueStart * 1000)),
            duration_ms: Math.max(100, Math.round(directCueDuration * 1000)),
            behavior: directCueBehavior,
            end_value_basis_points: Math.round(directCueEndPercent * 100),
          });
          if (!saved) return;
          if (editingDirectCue && !await sendCmd('pealayer.timeline.effect.update', {
            instance_id: editingDirectCue, start_time_ms: Math.round(directCueStart * 1000),
            duration_ms: Math.max(100, Math.round(directCueDuration * 1000)),
          })) return;
          setDirectCueControl(null);
        }}
      >
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
        <Select aria-label={tr(locale, 'Cue behavior')} style={{ width: '100%' }} value={directCueBehavior} onChange={setDirectCueBehavior}
          options={[{ value: 'set-keep', label: tr(locale, 'Set and keep') }, { value: 'hold', label: tr(locale, 'Timed hold') },
            ...(directCueControl?.startsWith('pwm.') ? [{ value: 'ramp', label: tr(locale, 'PWM ramp') }] : [])]} />
        <InputNumber aria-label={tr(locale, 'Start time')} addonBefore={tr(locale, 'Starts')} addonAfter="s" min={0} max={86400} step={.001} value={directCueStart} onChange={(value) => setDirectCueStart(Number(value ?? 0))} />
        {directCueControl?.startsWith('relay.') ? <Select aria-label={tr(locale, 'State')} value={directCuePercent}
          onChange={setDirectCuePercent} options={[{ value: 100, label: tr(locale, 'On') }, { value: 0, label: tr(locale, 'Off') }]} /> :
        <Space.Compact block>
          <Slider
            style={{ flex: 1 }}
            min={0}
            max={100}
            step={0.01}
            value={directCuePercent}
            onChange={setDirectCuePercent}
          />
          <InputNumber
            min={0}
            max={100}
            step={0.01}
            precision={2}
            value={directCuePercent}
            addonAfter="%"
            onChange={(value) => setDirectCuePercent(Number(value ?? 0))}
          />
        </Space.Compact>
        }
        {directCueBehavior !== 'set-keep' && <>
          <InputNumber aria-label={tr(locale, 'Duration')} addonBefore={tr(locale, 'Duration')} addonAfter="s" min={.1} max={86400} step={.1} value={directCueDuration} onChange={(value) => setDirectCueDuration(Number(value ?? 1))} />
          {directCueControl?.startsWith('relay.') ? <Select aria-label={tr(locale, 'On exit')} value={directCueEndPercent}
            onChange={setDirectCueEndPercent} options={[{ value: 0, label: tr(locale, 'Exit: Off') }, { value: 100, label: tr(locale, 'Exit: On') }]} /> :
            <InputNumber aria-label={tr(locale, 'End value')} addonBefore={tr(locale, directCueBehavior === 'ramp' ? 'Ramp to' : 'On exit')} addonAfter="%" min={0} max={100} value={directCueEndPercent} onChange={(value) => setDirectCueEndPercent(Number(value ?? 0))} />}
        </>}
        </Space>
      </Modal>
    </div>
  );
};
