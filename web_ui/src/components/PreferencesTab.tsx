import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Input,
  InputNumber,
  Select,
  Slider,
  Spin,
  Switch,
  Typography,
} from 'antd';
import {
  BgColorsOutlined,
  CheckOutlined,
  ControlOutlined,
  ExperimentOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  SaveOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { tr } from '../i18n';

type JsonObject = Record<string, any>;

interface PreferenceSection {
  id: string;
  label: string;
  icon: string;
}

interface PreferenceOption {
  value: string | number | boolean;
  label: string;
}

interface PreferenceControl {
  key: string;
  section: string;
  group: string;
  label: string;
  kind: 'boolean' | 'number' | 'select' | 'text';
  description?: string;
  options?: PreferenceOption[];
  minimum?: number;
  maximum?: number;
  step?: number;
  logarithmic?: boolean;
  inverted?: boolean;
  placeholder?: string;
}

interface PreferencesContract {
  format: 'pealayer-preferences';
  sections: PreferenceSection[];
  controls: PreferenceControl[];
  values: JsonObject;
}

interface PreferencesTabProps {
  apiBaseUrl: string;
  locale: 'en' | 'fa';
  onConfigChange?: (values: JsonObject) => void;
}

const sectionIcons: Record<string, React.ReactNode> = {
  appearance: <BgColorsOutlined />,
  playback: <PlayCircleOutlined />,
  hardware: <ExperimentOutlined />,
  input: <ControlOutlined />,
  advanced: <SettingOutlined />,
};

function valueAtPath(root: JsonObject, path: string): any {
  return path.split('.').reduce((value, key) => value?.[key], root);
}

function valueWithPath(root: JsonObject, path: string, value: any): JsonObject {
  const next = structuredClone(root);
  const parts = path.split('.');
  let cursor: JsonObject = next;
  parts.slice(0, -1).forEach((part) => {
    cursor[part] = { ...(cursor[part] ?? {}) };
    cursor = cursor[part];
  });
  cursor[parts[parts.length - 1]] = value;
  return next;
}

function patchForPath(values: JsonObject, path: string): JsonObject {
  const root = path.split('.')[0];
  return { [root]: values[root] };
}

export const PreferencesTab: React.FC<PreferencesTabProps> = ({ apiBaseUrl, locale, onConfigChange }) => {
  const [contract, setContract] = useState<PreferencesContract | null>(null);
  const [section, setSection] = useState('appearance');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [status, setStatus] = useState<{ kind: 'success' | 'error'; text: string } | null>(null);

  const load = async () => {
    setLoading(true);
    setStatus(null);
    try {
      const response = await fetch(`${apiBaseUrl}/api/preferences`);
      if (!response.ok) throw new Error(`Preferences request failed (${response.status})`);
      const next = await response.json() as PreferencesContract;
      if (next.format !== 'pealayer-preferences') throw new Error('Unsupported preferences response');
      setContract(next);
      setSection((current) => next.sections.some((item) => item.id === current) ? current : next.sections[0]?.id ?? 'appearance');
      onConfigChange?.(next.values);
    } catch (error) {
      setStatus({ kind: 'error', text: error instanceof Error ? error.message : String(error) });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void load(); }, [apiBaseUrl]);

  const controls = useMemo(
    () => contract?.controls.filter((control) => control.section === section) ?? [],
    [contract, section],
  );
  const groups = useMemo(() => Array.from(new Set(controls.map((control) => control.group))), [controls]);

  const update = async (control: PreferenceControl, displayedValue: any) => {
    if (!contract) return;
    const storedValue = control.inverted ? !displayedValue : displayedValue;
    const nextValues = valueWithPath(contract.values, control.key, storedValue === '' ? null : storedValue);
    setContract({ ...contract, values: nextValues });
    setSaving(control.key);
    setStatus(null);
    try {
      const response = await fetch(`${apiBaseUrl}/api/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(patchForPath(nextValues, control.key)),
      });
      if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body.error || `Save failed (${response.status})`);
      }
      onConfigChange?.(nextValues);
      setStatus({ kind: 'success', text: tr(locale, 'Preferences saved') });
    } catch (error) {
      setContract(contract);
      setStatus({ kind: 'error', text: error instanceof Error ? error.message : String(error) });
    } finally {
      setSaving(null);
    }
  };

  const renderControl = (control: PreferenceControl) => {
    const stored = valueAtPath(contract?.values ?? {}, control.key);
    const value = control.inverted ? !Boolean(stored) : stored;
    const commonLabel = <span className="preference-control__label">{tr(locale, control.label)}</span>;
    if (control.kind === 'boolean') {
      return (
        <label className="preference-control preference-control--boolean" key={control.key}>
          <span>{commonLabel}{control.description && <small>{tr(locale, control.description)}</small>}</span>
          <Switch checked={Boolean(value)} loading={saving === control.key} onChange={(next) => void update(control, next)} />
        </label>
      );
    }
    if (control.kind === 'select') {
      return (
        <label className="preference-control" key={control.key}>
          {commonLabel}
          <Select
            value={value}
            options={(control.options ?? []).map((option) => ({ value: option.value, label: tr(locale, option.label) }))}
            onChange={(next) => void update(control, next)}
          />
        </label>
      );
    }
    if (control.kind === 'number') {
      return (
        <label className="preference-control preference-control--number" key={control.key}>
          {commonLabel}
          <div className="preference-control__number">
            <Slider
              min={control.minimum}
              max={control.maximum}
              step={control.step}
              value={Number(value)}
              onChangeComplete={(next) => void update(control, next)}
            />
            <InputNumber
              min={control.minimum}
              max={control.maximum}
              step={control.step}
              value={Number(value)}
              onChange={(next) => next !== null && void update(control, next)}
            />
          </div>
        </label>
      );
    }
    return (
      <label className="preference-control" key={control.key}>
        {commonLabel}
        <Input
          defaultValue={value ?? ''}
          placeholder={control.placeholder}
          onPressEnter={(event) => void update(control, event.currentTarget.value.trim())}
          onBlur={(event) => event.currentTarget.value !== (value ?? '') && void update(control, event.currentTarget.value.trim())}
        />
      </label>
    );
  };

  return (
    <section className="preferences-page" aria-label={tr(locale, 'Preferences')}>
      <header className="preferences-page__header">
        <div>
          <Typography.Title level={2}>{tr(locale, 'Preferences')}</Typography.Title>
          <Typography.Text>{tr(locale, 'Application settings')}</Typography.Text>
        </div>
        <Button icon={<ReloadOutlined />} onClick={() => void load()} loading={loading}>{tr(locale, 'Reload')}</Button>
      </header>

      {status && <Alert showIcon type={status.kind} message={status.text} closable onClose={() => setStatus(null)} />}

      {loading && !contract ? <div className="preferences-page__loading"><Spin /></div> : contract && (
        <div className="preferences-layout">
          <nav className="preferences-rail" aria-label={tr(locale, 'Preference sections')}>
            {contract.sections.map((item) => (
              <button
                key={item.id}
                type="button"
                className={item.id === section ? 'is-active' : ''}
                onClick={() => setSection(item.id)}
              >
                {sectionIcons[item.id] ?? <SettingOutlined />}
                <span>{tr(locale, item.label)}</span>
              </button>
            ))}
          </nav>
          <main className="preferences-detail">
            {groups.map((group) => (
              <Card key={group} className="preferences-group" title={group} bordered>
                <div className="preferences-group__controls">
                  {controls.filter((control) => control.group === group).map(renderControl)}
                </div>
              </Card>
            ))}
            <div className="preferences-save-state" aria-live="polite">
              {saving ? <><SaveOutlined /> {tr(locale, 'Saving')}</> : <><CheckOutlined /> {tr(locale, 'Changes save automatically')}</>}
            </div>
          </main>
        </div>
      )}
    </section>
  );
};
