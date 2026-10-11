import { Checkbox, InputNumber } from 'antd';
import { tr, UiLocale } from '../i18n';
import { TimeValueField } from './TimeValueField';

export function StepRepeatFields({ step, onChange, locale, human }: {
  step: { repeat_count?: number; repeat_interval_ms?: number };
  onChange: (patch: { repeat_count?: number; repeat_interval_ms?: number }) => void;
  locale: UiLocale; human: boolean;
}) {
  const enabled = (step.repeat_count ?? 1) > 1;
  return <>
    <label><span>{tr(locale, 'Repeat step')}</span><Checkbox checked={enabled} onChange={event => onChange({ repeat_count: event.target.checked ? 2 : undefined, repeat_interval_ms: undefined })}>{tr(locale, 'Enabled')}</Checkbox>
      {enabled && <InputNumber aria-label={tr(locale, 'Repeat count')} min={2} max={1000} value={step.repeat_count} onChange={value => onChange({ repeat_count: value ?? 2 })} />}</label>
    {enabled && <label><span>{tr(locale, 'Interval')}</span><TimeValueField max={3600000} value={step.repeat_interval_ms ?? 0} human={human} onChange={value => onChange({ repeat_interval_ms: Math.round(value) })} /></label>}
  </>;
}
