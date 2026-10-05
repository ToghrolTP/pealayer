import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  ColorPicker,
  Input,
  Select,
  Spin,
  Switch,
  Typography,
} from 'antd';
import {
  BgColorsOutlined,
  BarsOutlined,
  CheckOutlined,
  CheckSquareOutlined,
  ControlOutlined,
  EditOutlined,
  ExperimentOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  ReloadOutlined,
  SaveOutlined,
  SettingOutlined,
  SwapRightOutlined,
  DeleteOutlined,
  DesktopOutlined,
  GlobalOutlined,
  LinkOutlined,
  SafetyCertificateOutlined,
  SyncOutlined,
  WifiOutlined,
} from '@ant-design/icons';
import { tr } from '../i18n';
import { mergeAppearance } from '../appearance';
import { NumericValueControl } from './NumericValueControl';
import type { TimelineWheelPreferences } from '../timelineWheel';

type JsonObject = Record<string, any>;

interface PreferenceSection {
  id: string;
  label: string;
  icon: string;
}

interface PreferenceOption {
  value: string | number | boolean;
  label: string;
  color?: string;
  description?: string;
  icon?: string;
}

interface PreferenceControl {
  key: string;
  section: string;
  group: string;
  label: string;
  kind: 'accent' | 'boolean' | 'multi_select' | 'number' | 'replacement_list' | 'select' | 'text';
  description?: string;
  options?: PreferenceOption[];
  minimum?: number;
  maximum?: number;
  step?: number;
  integer?: boolean;
  logarithmic?: boolean;
  inverted?: boolean;
  placeholder?: string;
  custom_key?: string;
}

interface PreferencesContract {
  format: 'pealayer-preferences';
  sections: PreferenceSection[];
  controls: PreferenceControl[];
  values: JsonObject;
  defaults: JsonObject;
}

interface PreferencesTabProps {
  apiBaseUrl: string;
  locale: 'en' | 'fa';
  appearance?: import('../appearance').AppearanceState;
  timelineWheelPreferences?: TimelineWheelPreferences;
  onConfigChange?: (values: JsonObject) => void;
}

const sectionIcons: Record<string, React.ReactNode> = {
  appearance: <BgColorsOutlined />,
  playback: <PlayCircleOutlined />,
  hardware: <ExperimentOutlined />,
  input: <ControlOutlined />,
  web: <GlobalOutlined />,
  advanced: <SettingOutlined />,
};

const controlIcons: Record<PreferenceControl['kind'], React.ReactNode> = {
  accent: <BgColorsOutlined />,
  boolean: <CheckSquareOutlined />,
  multi_select: <GlobalOutlined />,
  number: <ControlOutlined />,
  replacement_list: <SwapRightOutlined />,
  select: <BarsOutlined />,
  text: <EditOutlined />,
};

const networkOptionIcon = (icon?: string) => {
  switch (icon) {
    case 'wifi': return <WifiOutlined />;
    case 'ethernet': return <DesktopOutlined />;
    case 'vpn': return <SafetyCertificateOutlined />;
    case 'virtual': return <SyncOutlined />;
    case 'loopback': return <SyncOutlined />;
    case 'globe': return <GlobalOutlined />;
    default: return <LinkOutlined />;
  }
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

export const PreferencesTab: React.FC<PreferencesTabProps> = ({ apiBaseUrl, locale, appearance, timelineWheelPreferences, onConfigChange }) => {
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

  // Keep open preference controls in step with native previews and other
  // browser clients, not just the surrounding page's colors.
  useEffect(() => {
    if (!appearance) return;
    setContract((current) => current
      ? { ...current, values: mergeAppearance(current.values, appearance) }
      : current);
  }, [appearance?.theme, appearance?.color_palette, appearance?.accent_color, appearance?.custom_accent_color]);

  useEffect(() => {
    if (!timelineWheelPreferences) return;
    setContract((current) => current ? { ...current, values: {
      ...current.values,
      timeline_plain_wheel_action: timelineWheelPreferences.plain,
      timeline_ctrl_wheel_action: timelineWheelPreferences.ctrl,
      timeline_shift_wheel_action: timelineWheelPreferences.shift,
      timeline_alt_wheel_action: timelineWheelPreferences.alt,
    } } : current);
  }, [timelineWheelPreferences?.plain, timelineWheelPreferences?.ctrl,
    timelineWheelPreferences?.shift, timelineWheelPreferences?.alt]);

  const controls = useMemo(
    () => contract?.controls.filter((control) => control.section === section) ?? [],
    [contract, section],
  );
  const groups = useMemo(() => Array.from(new Set(controls.map((control) => control.group))), [controls]);

  const updatePaths = async (updates: Array<{ key: string; value: any }>) => {
    if (!contract) return;
    const nextValues = updates.reduce(
      (values, update) => valueWithPath(values, update.key, update.value === '' ? null : update.value),
      contract.values,
    );
    setContract({ ...contract, values: nextValues });
    setSaving(updates[0]?.key ?? null);
    setStatus(null);
    try {
      const patch = updates.reduce(
        (body, update) => ({ ...body, ...patchForPath(nextValues, update.key) }),
        {} as JsonObject,
      );
      const response = await fetch(`${apiBaseUrl}/api/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(patch),
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

  const updatePath = async (key: string, storedValue: any) => {
    await updatePaths([{ key, value: storedValue }]);
  };

  const update = async (control: PreferenceControl, displayedValue: any) => {
    const storedValue = control.inverted ? !displayedValue : displayedValue;
    await updatePath(control.key, storedValue);
  };

  const renderControl = (control: PreferenceControl) => {
    const stored = valueAtPath(contract?.values ?? {}, control.key);
    const value = control.inverted ? !Boolean(stored) : stored;
    const commonLabel = (
      <span className="preference-control__label">
        {controlIcons[control.kind]}
        <span>{tr(locale, control.label)}</span>
      </span>
    );
    if (control.kind === 'accent') {
      const customKey = control.custom_key ?? 'custom_accent_color';
      const customHex = String(valueAtPath(contract?.values ?? {}, customKey) ?? '#0078d4');
      const accentOptions = (control.options ?? []).map((option) => ({
        value: option.value,
        label: tr(locale, option.label),
        color: option.value === 'custom' ? customHex : option.color,
      }));
      return (
        <label className="preference-control" key={control.key}>
          {commonLabel}
          <div className="preference-control__accent">
            <Select
              value={value}
              options={accentOptions}
              onChange={(next) => void update(control, next)}
              labelRender={({ label }) => <span>{label}</span>}
              optionRender={(option) => {
                const isCustom = option.value === 'custom';
                return (
                  <span className={`accent-option${isCustom ? ' accent-option--custom' : ''}`}>
                    {!isCustom && (
                      <span
                        className="accent-option__swatch"
                        style={{ backgroundColor: String(option.data.color || 'transparent') }}
                      />
                    )}
                    <span className="accent-option__label">{option.label}</span>
                    {isCustom && (
                      <span
                        className="accent-option__editor"
                        onMouseDown={(event) => event.stopPropagation()}
                        onClick={(event) => event.stopPropagation()}
                      >
                        <ColorPicker
                          value={customHex}
                          disabledAlpha
                          onChangeComplete={(color) => {
                            void updatePaths([
                              { key: control.key, value: 'custom' },
                              { key: customKey, value: color.toHexString().toUpperCase() },
                            ]);
                          }}
                        />
                        <Input
                          key={customHex}
                          className="preference-control__hex"
                          defaultValue={customHex.toUpperCase()}
                          maxLength={7}
                          aria-label={tr(locale, 'Custom accent')}
                          onPressEnter={(event) => {
                            void updatePaths([
                              { key: control.key, value: 'custom' },
                              { key: customKey, value: event.currentTarget.value.trim() },
                            ]);
                          }}
                          onBlur={(event) => {
                            if (event.currentTarget.value !== customHex) {
                              void updatePaths([
                                { key: control.key, value: 'custom' },
                                { key: customKey, value: event.currentTarget.value.trim() },
                              ]);
                            }
                          }}
                        />
                      </span>
                    )}
                  </span>
                );
              }}
            />
          </div>
        </label>
      );
    }
    if (control.kind === 'boolean') {
      return (
        <label className="preference-control preference-control--boolean" key={control.key}>
          <span>{commonLabel}{control.description && <small>{tr(locale, control.description)}</small>}</span>
          <Switch checked={Boolean(value)} loading={saving === control.key} onChange={(next) => void update(control, next)} />
        </label>
      );
    }
    if (control.kind === 'multi_select') {
      const selected = Array.isArray(value) ? value : [];
      const options = (control.options ?? []).map((option) => ({
        value: option.value,
        label: option.description || tr(locale, option.label),
        title: tr(locale, option.label),
        description: option.description,
        icon: option.icon,
      }));
      return (
        <label className="preference-control" key={control.key}>
          <span>{commonLabel}{control.description && <small>{tr(locale, control.description)}</small>}</span>
          <Select
            mode="multiple"
            value={selected}
            options={options}
            maxTagCount="responsive"
            placeholder={tr(locale, 'Choose interfaces')}
            optionRender={(option) => (
              <span className="preference-network-option">
                {networkOptionIcon(option.data.icon)}
                <span>
                  <strong>{option.data.title}</strong>
                  {option.data.description && <small>{option.data.description}</small>}
                </span>
              </span>
            )}
            onChange={(next) => void update(control, next)}
          />
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
          <NumericValueControl value={Number(value)} minimum={control.minimum ?? 0} maximum={control.maximum ?? 100}
            defaultStep={control.step ?? 1} defaultValue={Number(valueAtPath(contract?.defaults ?? {}, control.key) ?? control.minimum ?? 0)}
            integer={control.integer} label={tr(locale, control.label)} tr={(label) => tr(locale, label)}
            steps={contract?.values.numeric_input_steps?.[control.key]} onChange={(next) => void update(control, next)}
            onStepsChange={(steps) => void updatePath('numeric_input_steps', { ...contract?.values.numeric_input_steps, [control.key]: steps })} />
        </label>
      );
    }
    if (control.kind === 'replacement_list') {
      const replacements = Array.isArray(value) ? value as Array<{ from: string; to: string }> : [];
      const replaceEntry = (index: number, key: 'from' | 'to', next: string) => {
        const updated = replacements.map((entry, entryIndex) => (
          entryIndex === index ? { ...entry, [key]: next } : entry
        ));
        void update(control, updated);
      };
      return (
        <div className="preference-control preference-control--replacements" key={control.key}>
          <span>
            {commonLabel}
            {control.description && <small>{tr(locale, control.description)}</small>}
          </span>
          <div className="preference-replacements">
            {replacements.map((entry, index) => (
              <div className="preference-replacement" key={`${index}-${entry.from}-${entry.to}`}>
                <Input
                  defaultValue={entry.from}
                  placeholder={tr(locale, 'Source text')}
                  aria-label={tr(locale, 'Source text')}
                  onBlur={(event) => event.currentTarget.value !== entry.from && replaceEntry(index, 'from', event.currentTarget.value)}
                  onPressEnter={(event) => event.currentTarget.blur()}
                />
                <SwapRightOutlined aria-hidden />
                <Input
                  defaultValue={entry.to}
                  placeholder={tr(locale, 'Replacement')}
                  aria-label={tr(locale, 'Replacement')}
                  onBlur={(event) => event.currentTarget.value !== entry.to && replaceEntry(index, 'to', event.currentTarget.value)}
                  onPressEnter={(event) => event.currentTarget.blur()}
                />
                <Button
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  aria-label={tr(locale, 'Remove replacement')}
                  title={tr(locale, 'Remove replacement')}
                  onClick={() => void update(control, replacements.filter((_, entryIndex) => entryIndex !== index))}
                />
              </div>
            ))}
            <Button
              className="preference-replacements__add"
              icon={<PlusOutlined />}
              onClick={() => void update(control, [...replacements, { from: '', to: '' }])}
            >
              {tr(locale, 'Add replacement')}
            </Button>
          </div>
        </div>
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
