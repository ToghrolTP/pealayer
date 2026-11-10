import React, { useEffect, useState } from 'react';
import { Button, Dropdown, InputNumber, Modal, Slider, message } from 'antd';
import { CopyOutlined, MinusOutlined, PlusOutlined, ReloadOutlined, SettingOutlined, SnippetsOutlined } from '@ant-design/icons';

export interface NumericSteps { normal: number; fine: number; coarse: number }
interface Props {
  value: number; minimum: number; maximum: number; defaultValue: number; defaultStep: number;
  integer?: boolean; steps?: NumericSteps; label: string;
  tr: (label: string) => string;
  onChange: (value: number) => void;
  onStepsChange: (steps: NumericSteps) => void;
}

/** Shared bounded input: value actions use the same persistent step contract as egui. */
export const NumericValueControl: React.FC<Props> = (props) => {
  const { value, minimum, maximum, defaultValue, defaultStep, integer, label, tr, onChange, onStepsChange } = props;
  const minStep = integer ? 1 : 1e-9;
  const defaults = { normal: Math.max(minStep, defaultStep), fine: Math.max(minStep, defaultStep / 10), coarse: Math.max(minStep, defaultStep * 10) };
  const steps = props.steps ?? defaults;
  const [stepDialog, setStepDialog] = useState(false);
  const [draftSteps, setDraftSteps] = useState(steps);
  const [draftValue, setDraftValue] = useState(value);
  useEffect(() => setDraftValue(value), [value]);
  const bounded = (next: number) => Math.min(maximum, Math.max(minimum, integer ? Math.round(next) : next));
  const apply = (next: number) => { if (Number.isFinite(next)) { setDraftValue(bounded(next)); onChange(bounded(next)); } };
  const stepFor = (event: { ctrlKey?: boolean; metaKey?: boolean; shiftKey?: boolean }) =>
    Math.max(minStep, event.ctrlKey || event.metaKey ? steps.fine : event.shiftKey ? steps.coarse : steps.normal);
  const validSteps = Object.values(draftSteps).every((step) => Number.isFinite(step) && step >= minStep && step <= 1e12);
  return <>
    <Dropdown trigger={['contextMenu']} menu={{ items: [
      { key: 'reset', icon: <ReloadOutlined />, label: tr('Reset') },
      { key: 'copy', icon: <CopyOutlined />, label: tr('Copy') },
      { key: 'paste', icon: <SnippetsOutlined />, label: tr('Paste') },
      { type: 'divider' },
      { key: 'increase', icon: <PlusOutlined />, label: tr('Increase'), disabled: draftValue >= maximum },
      { key: 'decrease', icon: <MinusOutlined />, label: tr('Decrease'), disabled: draftValue <= minimum },
      { key: 'steps', icon: <SettingOutlined />, label: tr('Adjustment steps') },
    ], onClick: ({ key, domEvent }) => {
      if (key === 'reset') apply(defaultValue);
      if (key === 'increase') apply(draftValue + stepFor(domEvent));
      if (key === 'decrease') apply(draftValue - stepFor(domEvent));
      if (key === 'steps') { setDraftSteps(steps); setStepDialog(true); }
      if ((key === 'copy' || key === 'paste') && !navigator.clipboard) { void message.error(tr('Clipboard requires a secure connection')); return; }
      if (key === 'copy') void navigator.clipboard.writeText(String(draftValue)).catch(() => message.error(tr('Clipboard access denied')));
      if (key === 'paste') void navigator.clipboard.readText().then((text) => {
        const next = Number(text.trim());
        if (!text.trim() || !Number.isFinite(next)) { void message.error(tr('Enter a valid number')); return; }
        apply(next);
      }).catch(() => message.error(tr('Clipboard access denied')));
    } }}>
      <div className="numeric-value-control">
        <Slider min={minimum} max={maximum} step={integer ? 1 : defaultStep} value={draftValue}
          onChange={setDraftValue} onChangeComplete={apply} aria-label={label} />
        <div className="numeric-value-control__input">
          <Button size="small" icon={<MinusOutlined />} disabled={draftValue <= minimum}
            aria-label={tr('Decrease')} title={tr('Ctrl: fine; Shift: coarse')} onClick={(event) => apply(draftValue - stepFor(event))} />
          <InputNumber controls={false} min={minimum} max={maximum} step={steps.normal} value={draftValue}
            aria-label={label} onChange={(next) => next !== null && setDraftValue(Number(next))}
            onBlur={() => draftValue !== value && apply(draftValue)} onPressEnter={() => apply(draftValue)} />
          <Button size="small" icon={<PlusOutlined />} disabled={draftValue >= maximum}
            aria-label={tr('Increase')} title={tr('Ctrl: fine; Shift: coarse')} onClick={(event) => apply(draftValue + stepFor(event))} />
        </div>
      </div>
    </Dropdown>
    <Modal title={`${tr('Adjustment steps')} — ${label}`} open={stepDialog} onCancel={() => setStepDialog(false)}
      onOk={() => { onStepsChange(draftSteps); setStepDialog(false); }} okText={tr('Save')} cancelText={tr('Cancel')}
      okButtonProps={{ disabled: !validSteps }} width={390}>
      {(['normal', 'fine', 'coarse'] as const).map((key, index) => <label className="numeric-step-row" key={key}>
        <span>{tr(['Normal', 'Fine (Ctrl)', 'Coarse (Shift)'][index])}</span>
        <InputNumber min={minStep} max={1e12} value={draftSteps[key]} step={Math.max(minStep, defaultStep / 10)}
          onChange={(next) => setDraftSteps({ ...draftSteps, [key]: next === null ? NaN : Number(next) })} />
      </label>)}
      <Button icon={<ReloadOutlined />} onClick={() => setDraftSteps(defaults)}>{tr('Reset steps')}</Button>
    </Modal>
  </>;
};
