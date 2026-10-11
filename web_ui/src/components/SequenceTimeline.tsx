import { useEffect, useRef, useState } from 'react';
import { Button, Dropdown, Space, Tooltip } from 'antd';
import { CopyOutlined, DeleteOutlined, SelectOutlined } from '@ant-design/icons';
import { deleteSequenceCues, duplicateSequenceCues, formatTimeMs, moveSequenceStep, selectCueIndices, sequenceKindLabels, SequenceStep } from '../cueAuthoring';
import { sequenceDurationMs } from '../melodyCatalog';
import { tr, UiLocale } from '../i18n';

/** Pure authoring gestures: never seek, preview or send hardware commands. */
export function SequenceTimeline<T extends SequenceStep>({ steps, onChange, locale, controls = [], disabled = false, onSelect, human = true }: {
  steps: T[]; onChange: (steps: T[]) => void; locale: UiLocale;
  controls?: Array<{ key: string; name: string; kind: string; locked?: boolean }>;
  disabled?: boolean; onSelect?: (index: number) => void; human?: boolean;
}) {
  const [selected, setSelected] = useState(() => new Set(steps.length ? [0] : []));
  const primary = useRef(0);
  const anchor = useRef(0);
  const previousSteps = useRef(steps);
  const ownUpdate = useRef<T[] | null>(null);
  const [preview, setPreview] = useState<{ index: number; step: T; copy: boolean } | null>(null);
  const drag = useRef<null | { index: number; origin: number; at: number; scale: number; step: T; copy: boolean; moved: boolean }>(null);
  const container = useRef<HTMLDivElement>(null);
  const section = useRef<HTMLElement>(null);
  const suppressClick = useRef<number | null>(null);
  useEffect(() => {
    if (steps === previousSteps.current) return;
    // External draft replacement/reorder must not retain stale destructive
    // selections. Our own atomic edits keep their selected result.
    if (steps !== ownUpdate.current) {
      primary.current = Math.min(primary.current, Math.max(0, steps.length - 1));
      anchor.current = primary.current;
      setSelected(new Set(steps.length ? [primary.current] : []));
    }
    previousSteps.current = steps; ownUpdate.current = null;
  }, [steps]);
  const commit = (next: T[]) => { ownUpdate.current = next; onChange(next); };
  const laneFor = (step: T) => step.kind === 'motion' ? (step.target === 1 ? 'seat.b' : 'seat.a') : `${step.kind}.${step.target ?? 0}`;
  const lanes = [...new Set([...controls.filter(c => c.kind === 'motion' && !c.locked).map(c => c.key), ...steps.map(laneFor)])];
  const span = Math.max(1000, sequenceDurationMs(steps));
  const width = Math.max(600, span * .08 + 160);
  const scale = .08;
  const choose = (index: number, toggle = false, range = false) => {
    const selectionAnchor = anchor.current;
    setSelected(current => new Set(selectCueIndices(current, index, selectionAnchor, toggle, range)));
    if (!range) anchor.current = index;
    primary.current = index; onSelect?.(index);
  };
  const selectAll = () => setSelected(new Set(steps.map((_, index) => index)));
  const reveal = (index: number) => requestAnimationFrame(() => {
    const element = container.current?.querySelector<HTMLElement>(`[data-step-index="${index}"]`);
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
  return <section className="sequence-authoring" ref={section} tabIndex={0} aria-label={tr(locale, 'Effect timeline')}
    onKeyDown={event => {
      if (disabled || event.target instanceof HTMLElement && event.target.closest('input,textarea,select,[contenteditable="true"]')) return;
      const command = event.ctrlKey || event.metaKey;
      if (command && event.key.toLowerCase() === 'a') { event.preventDefault(); event.stopPropagation(); selectAll(); }
      else if (command && event.key.toLowerCase() === 'd') { event.preventDefault(); event.stopPropagation(); duplicate(); }
      else if (event.key === 'Delete' && !event.altKey && !command) { event.preventDefault(); event.stopPropagation(); remove(); }
    }}>
    <Space wrap><strong>{tr(locale, 'Effect timeline')}</strong><span className="muted">{tr(locale, 'Ctrl-drag to duplicate')}</span>
      <Tooltip title="Ctrl+A"><Button icon={<SelectOutlined />} disabled={disabled || !steps.length} onClick={selectAll}>{tr(locale, 'Select all cues')}</Button></Tooltip>
      <Button disabled={disabled || !selected.size} onClick={() => setSelected(new Set())}>{tr(locale, 'Clear cue selection')}</Button>
      <Tooltip title="Ctrl+D"><Button icon={<CopyOutlined />} disabled={disabled || !selected.size} onClick={() => duplicate()}>{tr(locale, 'Duplicate selected cues')}</Button></Tooltip>
      <Tooltip title={tr(locale, 'Delete')}><Button danger icon={<DeleteOutlined />} disabled={disabled || !selected.size} onClick={() => remove()}>{tr(locale, 'Delete selected cues')}</Button></Tooltip>
      <span aria-live="polite">{tr(locale, 'Selected cues')}: {selected.size}</span>
    </Space>
    <div className="sequence-authoring__scroll" ref={container}>
      <div style={{ minWidth: width + 140 }}>
        {lanes.map(lane => <div className="sequence-authoring__row" key={lane}>
          <div className="sequence-authoring__label">{controls.find(c => c.key === lane)?.name ?? (lane === 'seat.a' ? tr(locale, 'Seat A') : lane === 'seat.b' ? tr(locale, 'Seat B') : lane)}</div>
          <div className="sequence-authoring__lane" data-sequence-lane={lane} style={{ width }} onPointerDown={event => {
            if (event.target === event.currentTarget) section.current?.focus({ preventScroll: true });
          }}>
            {steps.map((step, index) => {
              // Keep the captured DOM node in its original lane until release.
              const shown = preview?.index === index && !preview.copy ? { ...step, at_us: preview.step.at_us } : step;
              if (laneFor(shown) !== lane) return null;
              const length = Math.max(38, Number(shown.duration_ms ?? 0) * scale);
              const label = step.kind === 'motion' ? tr(locale, step.value === 1 ? 'Up' : step.value === 2 ? 'Down' : 'Stop') : tr(locale, sequenceKindLabels[step.kind] ?? step.kind);
              return <Dropdown key={index} trigger={['contextMenu']} menu={{ items: [
                { key: 'duplicate', label: tr(locale, 'Duplicate selected cues'), icon: <CopyOutlined />, disabled },
                { key: 'delete', label: tr(locale, 'Delete selected cues'), icon: <DeleteOutlined />, disabled, danger: true },
              ], onClick: ({ key }) => { if (key === 'duplicate') duplicate(contextSelection(index)); else remove(contextSelection(index)); } }}>
                <button type="button" className={`sequence-authoring__cue ${selected.has(index) ? 'is-selected' : ''}`} aria-pressed={selected.has(index)}
                  data-step-index={index} style={{ left: shown.at_us / 1000 * scale, width: length }} disabled={disabled}
                  title={`${label} · ${formatTimeMs(shown.at_us / 1000, human)} · ${formatTimeMs(Number(shown.duration_ms ?? 0), human)}\n${tr(locale, 'Ctrl-click to toggle; Shift-click to select a range')}`}
                  onContextMenu={() => { if (!selected.has(index)) choose(index); }}
                  onClick={event => { if (suppressClick.current === index) { suppressClick.current = null; return; } choose(index, event.ctrlKey || event.metaKey, event.shiftKey); }} onPointerDown={event => {
                    if (event.button !== 0 || disabled) return;
                    event.preventDefault(); event.currentTarget.focus({ preventScroll: true }); event.currentTarget.setPointerCapture(event.pointerId);
                    drag.current = { index, origin: event.clientX, at: step.at_us, scale, step: structuredClone(step), copy: event.ctrlKey || event.metaKey, moved: false };
                  }} onPointerMove={event => {
                    const active = drag.current; if (!active || active.index !== index) return;
                    const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-sequence-lane]')?.dataset.sequenceLane;
                    const at = active.at + (event.clientX - active.origin) / active.scale * 1000;
                    active.moved ||= Math.abs(event.clientX - active.origin) > 2 || (target != null && target !== laneFor(active.step));
                    setPreview({ index, step: moveSequenceStep(active.step, at, target) as T, copy: active.copy });
                  }} onPointerUp={event => {
                    const active = drag.current; if (!active || active.index !== index) return;
                    const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-sequence-lane]')?.dataset.sequenceLane;
                    const moved = moveSequenceStep(active.step, active.at + (event.clientX - active.origin) / active.scale * 1000, target) as T;
                    if (active.moved) {
                      suppressClick.current = index;
                      window.setTimeout(() => { suppressClick.current = null; }, 0);
                      const next = [...steps]; const destination = active.copy ? next.length : index;
                      if (active.copy) next.push(moved); else next[index] = moved;
                      commit(next); choose(destination); if (active.copy) reveal(destination);
                    }
                    drag.current = null; setPreview(null);
                  }} onPointerCancel={() => { drag.current = null; setPreview(null); }}><span aria-hidden>⋮</span> {label}</button>
              </Dropdown>;
            })}
            {preview && laneFor(preview.step) === lane && <span className="sequence-authoring__ghost" style={{ left: preview.step.at_us / 1000 * scale, width: Math.max(38, Number(preview.step.duration_ms ?? 0) * scale) }}>{preview.copy ? tr(locale, 'Copy') : ''}</span>}
          </div>
        </div>)}
      </div>
    </div>
  </section>;
}
