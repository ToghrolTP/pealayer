import { useEffect, useRef, useState } from 'react';
import { Button, Dropdown, InputNumber, Select, Space, Tooltip } from 'antd';
import { CopyOutlined, DeleteOutlined, SelectOutlined, ScissorOutlined } from '@ant-design/icons';
import { deleteSequenceCues, duplicateSequenceCues, editStepInstance, formatTimeMs, moveSequenceStep, selectCueIndices, sequenceKindLabels, sequenceLaneName, sequenceLaneKey, sequenceLaneChoices, sequenceStepTemplate, sequenceNeedsQuantize, quantizeSequence, stepInstances, stepRepetition, SequenceStep } from '../cueAuthoring';
import { sequenceDurationMs } from '../melodyCatalog';
import { tr, UiLocale } from '../i18n';

/** Pure authoring gestures: never seek, preview or send hardware commands. */
export interface SequenceTimelineActions { addStep: () => void }
export function SequenceTimeline<T extends SequenceStep>({ steps, onChange, locale, controls = [], disabled = false, onSelect, onEdit, human = true, extraKinds = [], actionsRef }: {
  steps: T[]; onChange: (steps: T[]) => void; locale: UiLocale;
  controls?: Array<{ key: string; name: string; default_name?: string; kind: string; locked?: boolean }>;
  disabled?: boolean; onSelect?: (index: number) => void; human?: boolean;
  onEdit?: (index: number) => void;
  extraKinds?: string[]; actionsRef?: { current: SequenceTimelineActions | null };
}) {
  const [selected, setSelected] = useState(() => new Set(steps.length ? [0] : []));
  const primary = useRef(0);
  const anchor = useRef(0);
  const previousSteps = useRef(steps);
  const ownUpdate = useRef<T[] | null>(null);
  const [preview, setPreview] = useState<{ index: number; step: T; copy: boolean } | null>(null);
  const drag = useRef<null | { index: number; iteration: number; mode: 'move' | 'length'; origin: number; at: number; scale: number; step: T; copy: boolean; moved: boolean }>(null);
  const container = useRef<HTMLDivElement>(null);
  const section = useRef<HTMLElement>(null);
  const suppressClick = useRef<number | null>(null);
  const [viewport, setViewport] = useState({ left: 0, width: 1000 });
  const [selectedLane, setSelectedLane] = useState('');
  const insertionTime = useRef<number | undefined>(undefined);
  const [quantum, setQuantum] = useState(50);
  const laneChoices = sequenceLaneChoices(controls, extraKinds);
  useEffect(() => {
    const element = container.current; if (!element) return;
    const refresh = () => setViewport({ left: element.scrollLeft, width: element.clientWidth });
    const observer = new ResizeObserver(refresh); observer.observe(element); refresh();
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (steps === previousSteps.current) return;
    // External draft replacement/reorder must not retain stale destructive
    // selections. Our own atomic edits keep their selected result.
    if (steps !== ownUpdate.current) {
      drag.current = null; setPreview(null);
      primary.current = Math.min(primary.current, Math.max(0, steps.length - 1));
      anchor.current = primary.current;
      setSelected(new Set(steps.length ? [primary.current] : []));
    }
    previousSteps.current = steps; ownUpdate.current = null;
  }, [steps]);
  useEffect(() => { if (disabled) { drag.current = null; setPreview(null); } }, [disabled]);
  const commit = (next: T[]) => { ownUpdate.current = next; onChange(next); };
  const laneFor = sequenceLaneKey;
  const lanes = [...new Set([...controls.filter(c => ['motion', 'seat'].includes(c.kind) && !c.locked).map(c => c.key), ...steps.map(laneFor)])];
  const span = Math.max(1000, sequenceDurationMs(steps));
  const width = Math.max(600, span * .08 + 160);
  const scale = .08;
  const choose = (index: number, toggle = false, range = false) => {
    if (steps[index]) setSelectedLane(laneFor(steps[index])); insertionTime.current = undefined;
    const selectionAnchor = anchor.current;
    setSelected(current => new Set(selectCueIndices(current, index, selectionAnchor, toggle, range)));
    if (!range) anchor.current = index;
    primary.current = index; onSelect?.(index);
  };
  const selectAll = () => setSelected(new Set(steps.map((_, index) => index)));
  const reveal = (index: number) => requestAnimationFrame(() => {
    const element = container.current?.querySelector<HTMLElement>(`[data-step-index="${index}"][data-repeat-index="0"]`);
    element?.scrollIntoView({ block: 'nearest', inline: 'center', behavior: 'smooth' }); element?.focus({ preventScroll: true });
  });
  const duplicate = (indices: Iterable<number> = selected) => {
    if (disabled) return;
    const next = duplicateSequenceCues(steps, indices); if (!next.indices.length) return;
    commit(next.steps); setSelected(new Set(next.indices)); primary.current = anchor.current = next.indices[0];
    onSelect?.(primary.current); reveal(primary.current);
  };
  const remove = (indices: Iterable<number> = selected) => {
    if (disabled) return;
    const targets = [...indices]; if (!targets.length) return;
    const next = deleteSequenceCues(steps, targets);
    commit(next.steps); primary.current = anchor.current = next.primary;
    setSelected(new Set(next.steps.length ? [next.primary] : [])); onSelect?.(next.primary);
    section.current?.focus({ preventScroll: true });
  };
  const contextSelection = (index: number) => selected.has(index) ? selected : [index];
  const addStep = (key = selectedLane || lanes[0] || laneChoices[0]?.key, at = insertionTime.current) => {
    if (disabled || !key) return;
    const existing = steps.find(step => laneFor(step) === key);
    const template = laneChoices.find(choice => choice.key === key)?.template
      ?? (existing ? sequenceStepTemplate(existing.kind, existing.target, String(existing.destination ?? 'segments')) : undefined);
    if (!template) return;
    const end = steps.filter(step => laneFor(step) === key).reduce((end, step) => Math.max(end, stepRepetition(step)?.endUs ?? step.at_us), 0);
    const step = { ...structuredClone(template), at_us: Math.min(2147483647, Math.max(0, Math.round(at ?? end))) } as T;
    const index = steps.length; commit([...steps, step]); setSelected(new Set([index]));
    primary.current = anchor.current = index; setSelectedLane(key); insertionTime.current = stepRepetition(step)?.endUs;
    onSelect?.(index); reveal(index);
  };
  useEffect(() => {
    if (!actionsRef) return;
    actionsRef.current = { addStep: () => addStep() };
    return () => { actionsRef.current = null; };
  });
  const laneCaption = (key: string) => {
    const choice = laneChoices.find(choice => choice.key === key);
    return choice ? (choice.translated ? tr(locale, choice.name) : choice.name) : sequenceLaneName(key, controls);
  };
  return <section className="sequence-authoring" ref={section} tabIndex={0} aria-label={tr(locale, 'Effect timeline')}
    onCopy={event => {
      if (disabled || !selected.size || (event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
      event.preventDefault(); event.stopPropagation();
      event.clipboardData.setData('text/plain', JSON.stringify({ format: 'pealayer.sequence-cues', steps: [...selected].map(index => steps[index]) }));
    }} onPaste={event => {
      if (disabled || (event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
      try {
        const clipboard = JSON.parse(event.clipboardData.getData('text/plain'));
        if (clipboard.format !== 'pealayer.sequence-cues' || !Array.isArray(clipboard.steps) || !clipboard.steps.length
          || clipboard.steps.length + steps.length > 65535 || clipboard.steps.some((step: SequenceStep) => !stepRepetition(step))) return;
        event.preventDefault(); event.stopPropagation();
        const first = clipboard.steps.reduce((start: number, step: SequenceStep) => Math.min(start, step.at_us), Infinity);
        const at = insertionTime.current ?? sequenceDurationMs(steps) * 1000;
        const copied = clipboard.steps.map((step: SequenceStep) => moveSequenceStep(step, step.at_us - first + at, selectedLane)) as T[];
        if (copied.some(step => !stepRepetition(step))) return;
        const primaryIndex = steps.length; commit([...steps, ...copied]); setSelected(new Set(copied.map((_, offset) => primaryIndex + offset)));
        primary.current = anchor.current = primaryIndex; onSelect?.(primaryIndex); reveal(primaryIndex);
      } catch { /* An unrelated clipboard payload is not a step insertion. */ }
    }}
    onKeyDown={event => {
      if (disabled || event.target instanceof HTMLElement && event.target.closest('input,textarea,select,[contenteditable="true"]')) return;
      const command = event.ctrlKey || event.metaKey;
      if (command && event.key.toLowerCase() === 'a') { event.preventDefault(); event.stopPropagation(); selectAll(); }
      else if (command && event.key.toLowerCase() === 'd') { event.preventDefault(); event.stopPropagation(); duplicate(); }
      else if (event.key === 'Delete' && !event.altKey && !command) { event.preventDefault(); event.stopPropagation(); remove(); }
      else if (event.key === 'Insert' && !command) { event.preventDefault(); event.stopPropagation(); addStep(); }
      else if (event.key === 'Enter' && selected.size) { event.preventDefault(); event.stopPropagation(); (onEdit ?? onSelect)?.(primary.current); }
      else if (['ArrowLeft', 'ArrowRight'].includes(event.key) && selected.size && !command) {
        event.preventDefault(); event.stopPropagation();
        const delta = (event.key === 'ArrowLeft' ? -1 : 1) * quantum * 1000;
        const earliest = Math.min(...[...selected].map(index => steps[index]?.at_us ?? 0));
        commit(steps.map((step, index) => selected.has(index) ? { ...step, at_us: Math.min(2147483647, step.at_us + Math.max(delta, -earliest)) } : step));
      }
    }}>
    <Space wrap><strong>{tr(locale, 'Effect timeline')}</strong><span className="muted">{tr(locale, 'Ctrl-drag to duplicate')}</span>
      <Tooltip title="Ctrl+A"><Button icon={<SelectOutlined />} disabled={disabled || !steps.length} onClick={selectAll}>{tr(locale, 'Select all cues')}</Button></Tooltip>
      <Button disabled={disabled || !selected.size} onClick={() => setSelected(new Set())}>{tr(locale, 'Clear cue selection')}</Button>
      <Tooltip title="Ctrl+D"><Button icon={<CopyOutlined />} disabled={disabled || !selected.size} onClick={() => duplicate()}>{tr(locale, 'Duplicate selected cues')}</Button></Tooltip>
      <Tooltip title={tr(locale, 'Delete')}><Button danger icon={<DeleteOutlined />} disabled={disabled || !selected.size} onClick={() => remove()}>{tr(locale, 'Delete selected cues')}</Button></Tooltip>
      <span aria-live="polite">{tr(locale, 'Selected cues')}: {selected.size}</span>
      <InputNumber aria-label={tr(locale, 'Grid')} min={1} max={1000} value={quantum} onChange={value => setQuantum(value ?? 50)} />
      <Button icon={<SelectOutlined />} disabled={disabled || !sequenceNeedsQuantize(steps, quantum)} onClick={() => commit(quantizeSequence(steps, quantum))}>{tr(locale, 'Quantize')}</Button>
      <Button icon={<ScissorOutlined />} disabled={disabled || !steps.length || Math.min(...steps.map(step => step.at_us)) === 0} onClick={() => {
        const first = Math.min(...steps.map(step => step.at_us)); commit(steps.map(step => ({ ...step, at_us: step.at_us - first })));
      }}>{tr(locale, 'Remove delay')}</Button>
    </Space>
    <div className="sequence-authoring__scroll" ref={container} onScroll={event => setViewport({ left: event.currentTarget.scrollLeft, width: event.currentTarget.clientWidth })}>
      <div style={{ minWidth: width + 140 }}>
        {lanes.map(lane => <div className={`sequence-authoring__row ${selectedLane === lane ? 'is-selected' : ''}`} key={lane}>
          <button type="button" className="sequence-authoring__label" aria-pressed={selectedLane === lane} disabled={disabled}
            onClick={() => { setSelectedLane(lane); insertionTime.current = undefined; setSelected(new Set()); }}>{laneCaption(lane)}</button>
          <div className="sequence-authoring__lane" data-sequence-lane={lane} style={{ width }} onPointerDown={event => {
            if (event.target === event.currentTarget && !disabled) {
              section.current?.focus({ preventScroll: true }); setSelectedLane(lane); setSelected(new Set());
              insertionTime.current = Math.round(Math.max(0, event.clientX - event.currentTarget.getBoundingClientRect().left) / scale) * 1000;
            }
          }} onDoubleClick={event => {
            if (event.target === event.currentTarget && !disabled) addStep(lane, Math.round(Math.max(0, event.clientX - event.currentTarget.getBoundingClientRect().left) / scale) * 1000);
          }}>
            {steps.flatMap((step, index) => stepInstances(preview?.index === index && !preview.copy ? preview.step : step, index,
              Math.max(0, (viewport.left - 140 - 40) / scale * 1000), (viewport.left + viewport.width) / scale * 1000, drag.current?.index === index ? drag.current.iteration : undefined))
              .sort((a, b) => Number(a.iteration === 0) - Number(b.iteration === 0)).map(({ source: index, iteration, at_us }) => {
              const step = steps[index], ghost = iteration > 0;
              // Keep the captured DOM node in its original lane until release.
              const shown = { ...step, at_us, duration_ms: preview?.index === index && !preview.copy ? preview.step.duration_ms : step.duration_ms };
              if (laneFor(shown) !== lane) return null;
              const length = Math.max(38, Number(shown.duration_ms ?? 0) * scale);
              const label = step.kind === 'motion' ? tr(locale, step.value === 1 ? 'Up' : step.value === 2 ? 'Down' : 'Stop') : tr(locale, sequenceKindLabels[step.kind] ?? step.kind);
              return <Dropdown key={`${index}:${iteration}`} trigger={['contextMenu']} menu={{ items: [
                { key: 'duplicate', label: tr(locale, 'Duplicate selected cues'), icon: <CopyOutlined />, disabled },
                { key: 'delete', label: tr(locale, 'Delete selected cues'), icon: <DeleteOutlined />, disabled, danger: true },
              ], onClick: ({ key }) => { if (key === 'duplicate') duplicate(contextSelection(index)); else remove(contextSelection(index)); } }}>
                <button type="button" className={`sequence-authoring__cue ${ghost ? 'is-repeat' : ''} ${selected.has(index) ? 'is-selected' : ''}`} aria-pressed={selected.has(index)}
                  data-step-index={index} data-repeat-index={iteration} aria-label={`${label}${ghost ? ` · ${tr(locale, 'Repeat step')} ${iteration + 1}` : ''}`} style={{ left: shown.at_us / 1000 * scale, width: length }} disabled={disabled}
                  title={`${label} · ${formatTimeMs(shown.at_us / 1000, human)} · ${formatTimeMs(Number(shown.duration_ms ?? 0), human)}\n${tr(locale, 'Ctrl-click to toggle; Shift-click to select a range')}`}
                  onContextMenu={() => { if (!selected.has(index)) choose(index); }}
                  onDoubleClick={event => { event.stopPropagation(); choose(index); (onEdit ?? onSelect)?.(index); }}
                  onClick={event => { if (suppressClick.current === index) { suppressClick.current = null; return; } choose(index, event.ctrlKey || event.metaKey, event.shiftKey); }} onPointerDown={event => {
                    if (event.button !== 0 || disabled) return;
                    event.preventDefault(); event.currentTarget.focus({ preventScroll: true }); event.currentTarget.setPointerCapture(event.pointerId);
                    const rect = event.currentTarget.getBoundingClientRect();
                    const mode = step.duration_ms != null && event.clientX >= rect.right - 8 ? 'length' : 'move';
                    drag.current = { index, iteration, mode, origin: event.clientX, at: step.at_us, scale, step: structuredClone(step), copy: !ghost && mode === 'move' && (event.ctrlKey || event.metaKey), moved: false };
                    if (!selected.has(index) && !event.ctrlKey && !event.metaKey && !event.shiftKey) choose(index);
                  }} onPointerMove={event => {
                    const active = drag.current; if (!active || active.index !== index || active.iteration !== iteration) return;
                    const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-sequence-lane]')?.dataset.sequenceLane;
                    const at = active.at + (event.clientX - active.origin) / active.scale * 1000;
                    active.moved ||= Math.abs(event.clientX - active.origin) > 2 || (!ghost && active.mode === 'move' && target != null && target !== laneFor(active.step));
                    const edited = ghost || active.mode === 'length' ? editStepInstance(active.step, iteration, (event.clientX - active.origin) / active.scale, active.mode) : moveSequenceStep(active.step, at, target) as T;
                    setPreview({ index, step: edited, copy: active.copy });
                  }} onPointerUp={event => {
                    const active = drag.current; if (!active || active.index !== index || active.iteration !== iteration) return;
                    const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-sequence-lane]')?.dataset.sequenceLane;
                    const moved = ghost || active.mode === 'length' ? editStepInstance(active.step, iteration, (event.clientX - active.origin) / active.scale, active.mode) : moveSequenceStep(active.step, active.at + (event.clientX - active.origin) / active.scale * 1000, target) as T;
                    if (active.moved) {
                      suppressClick.current = index;
                      window.setTimeout(() => { suppressClick.current = null; }, 0);
                      const next = [...steps]; const destination = active.copy ? next.length : index;
                      if (active.copy) next.push(moved); else next[index] = moved;
                      commit(next); choose(destination); setSelectedLane(laneFor(moved)); if (active.copy) reveal(destination);
                    }
                    drag.current = null; setPreview(null);
                  }} onPointerCancel={() => { drag.current = null; setPreview(null); }} onLostPointerCapture={() => { drag.current = null; setPreview(null); }}><span aria-hidden>{ghost ? `↻${iteration + 1}` : '⋮'}</span> {label}
                  {step.duration_ms != null && <span className="sequence-authoring__resize" aria-hidden>⋮</span>}</button>
              </Dropdown>;
            })}
            {preview && laneFor(preview.step) === lane && <span className="sequence-authoring__ghost" style={{ left: preview.step.at_us / 1000 * scale, width: Math.max(38, Number(preview.step.duration_ms ?? 0) * scale) }}>{preview.copy ? tr(locale, 'Copy') : ''}</span>}
          </div>
        </div>)}
        <div className="sequence-authoring__row sequence-authoring__add-track">
          <Select<string | null> aria-label={tr(locale, 'Add track')} placeholder={tr(locale, 'Add track')} value={null}
            disabled={disabled || !laneChoices.some(choice => !lanes.includes(choice.key))}
            options={laneChoices.filter(choice => !lanes.includes(choice.key)).map(choice => ({ value: choice.key, label: laneCaption(choice.key) }))}
            onChange={key => { if (key) addStep(key, insertionTime.current ?? 0); }} />
        </div>
      </div>
    </div>
  </section>;
}
