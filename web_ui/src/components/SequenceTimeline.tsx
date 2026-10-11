import React, { useRef, useState } from 'react';
import { Button, Dropdown, Space, Tooltip } from 'antd';
import { CopyOutlined, DeleteOutlined } from '@ant-design/icons';
import { duplicateStep, formatTimeMs, moveSequenceStep, SequenceStep } from '../cueAuthoring';
import { sequenceDurationMs } from '../melodyCatalog';
import { tr, UiLocale } from '../i18n';

/** Pure authoring gestures: never seek, preview or send hardware commands. */
export function SequenceTimeline<T extends SequenceStep>({ steps, onChange, locale, controls = [], disabled = false, onSelect, human = true }: {
  steps: T[]; onChange: (steps: T[]) => void; locale: UiLocale;
  controls?: Array<{ key: string; name: string; kind: string; locked?: boolean }>;
  disabled?: boolean; onSelect?: (index: number) => void; human?: boolean;
}) {
  const [selected, setSelected] = useState(0);
  const [preview, setPreview] = useState<{ index: number; step: T; copy: boolean } | null>(null);
  const drag = useRef<null | { index: number; origin: number; at: number; scale: number; step: T; copy: boolean; moved: boolean }>(null);
  const container = useRef<HTMLDivElement>(null);
  const laneFor = (step: T) => step.kind === 'motion' ? (step.target === 1 ? 'seat.b' : 'seat.a') : `${step.kind}.${step.target ?? 0}`;
  const lanes = [...new Set([...controls.filter(c => c.kind === 'motion' && !c.locked).map(c => c.key), ...steps.map(laneFor)])];
  const span = Math.max(1000, sequenceDurationMs(steps));
  const width = Math.max(600, span * .08 + 160);
  const scale = .08;
  const choose = (index: number) => { setSelected(index); onSelect?.(index); };
  const reveal = (index: number) => requestAnimationFrame(() => {
    const element = container.current?.querySelector<HTMLElement>(`[data-step-index="${index}"]`);
    element?.scrollIntoView({ block: 'nearest', inline: 'center', behavior: 'smooth' }); element?.focus({ preventScroll: true });
  });
  const duplicate = (index: number) => {
    if (disabled || !steps[index]) return;
    const copy = duplicateStep(steps[index]) as T;
    const next = [...steps]; next.splice(index + 1, 0, copy); onChange(next); choose(index + 1); reveal(index + 1);
  };
  return <section className="sequence-authoring" aria-label={tr(locale, 'Effect timeline')}>
    <Space wrap><strong>{tr(locale, 'Effect timeline')}</strong><span className="muted">{tr(locale, 'Ctrl-drag to duplicate')}</span>
      <Tooltip title={tr(locale, 'Duplicate cue')}><Button icon={<CopyOutlined />} disabled={disabled || !steps.length} onClick={() => duplicate(Math.min(selected, steps.length - 1))}>{tr(locale, 'Duplicate')}</Button></Tooltip>
    </Space>
    <div className="sequence-authoring__scroll" ref={container}>
      <div style={{ minWidth: width + 140 }}>
        {lanes.map(lane => <div className="sequence-authoring__row" key={lane}>
          <div className="sequence-authoring__label">{controls.find(c => c.key === lane)?.name ?? (lane === 'seat.a' ? tr(locale, 'Seat A') : lane === 'seat.b' ? tr(locale, 'Seat B') : lane)}</div>
          <div className="sequence-authoring__lane" data-sequence-lane={lane} style={{ width }}>
            {steps.map((step, index) => {
              // Keep the captured DOM node in its original lane until release.
              const shown = preview?.index === index && !preview.copy ? { ...step, at_us: preview.step.at_us } : step;
              if (laneFor(shown) !== lane) return null;
              const length = Math.max(38, Number(shown.duration_ms ?? 0) * scale);
              const label = step.kind === 'motion' ? tr(locale, step.value === 1 ? 'Up' : step.value === 2 ? 'Down' : 'Stop') : step.kind;
              return <Dropdown key={index} trigger={['contextMenu']} menu={{ items: [
                { key: 'duplicate', label: tr(locale, 'Duplicate cue'), icon: <CopyOutlined />, disabled },
                { key: 'delete', label: tr(locale, 'Delete cue'), icon: <DeleteOutlined />, disabled, danger: true },
              ], onClick: ({ key }) => { if (key === 'duplicate') duplicate(index); else { onChange(steps.filter((_, i) => i !== index)); choose(Math.max(0, index - 1)); } } }}>
                <button type="button" className={`sequence-authoring__cue ${selected === index ? 'is-selected' : ''}`}
                  data-step-index={index} style={{ left: shown.at_us / 1000 * scale, width: length }} disabled={disabled}
                  title={`${label} · ${formatTimeMs(shown.at_us / 1000, human)} · ${formatTimeMs(Number(shown.duration_ms ?? 0), human)}`}
                  onClick={() => choose(index)} onKeyDown={event => {
                    if (event.key === 'Delete' && !disabled) { event.preventDefault(); onChange(steps.filter((_, i) => i !== index)); choose(Math.max(0, index - 1)); }
                  }} onPointerDown={event => {
                    if (event.button !== 0 || disabled) return;
                    event.preventDefault(); event.currentTarget.setPointerCapture(event.pointerId); choose(index);
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
                      const next = [...steps]; if (active.copy) { next.splice(index + 1, 0, moved); choose(index + 1); reveal(index + 1); } else next[index] = moved;
                      onChange(next);
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
