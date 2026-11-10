import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { ColorPicker } from './ColorPicker';
import { ColorDropdown } from './ColorDropdown';
import { ShortcutInput } from './ShortcutInput';
import {
  Alert,
  Button,
  Card,
  Collapse,
  Input,
  Select,
  Slider,
  Switch,
  Typography,
  Tooltip,
} from 'antd';
import { FujiLoader } from './FujiLoader';
import {
  BgColorsOutlined,
  CheckOutlined,
  InfoCircleOutlined,
  CheckSquareOutlined,
  ControlOutlined,
  CodeOutlined,
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
  FileImageOutlined,
  FolderOpenOutlined,
  UndoOutlined,
  ClockCircleOutlined,
  TranslationOutlined,
  PushpinOutlined,
  ExpandOutlined,
  AimOutlined,
  PauseCircleOutlined,
  StopOutlined,
  SoundOutlined,
  FontSizeOutlined,
  LineChartOutlined,
  BorderOutlined,
  GatewayOutlined,
  MenuOutlined,
  KeyOutlined,
  DashboardOutlined,
  VideoCameraOutlined,
  ArrowUpOutlined,
  ArrowDownOutlined,
} from '@ant-design/icons';
import { tr } from '../i18n';
import { mergeAppearance, resolvedAppearanceTheme } from '../appearance';
import palettes from '../../../assets/themes/palettes.json';
import { NumericValueControl } from './NumericValueControl';
import { ServerFilePicker } from './ServerFilePicker';
import type { TimelineWheelPreferences } from '../timelineWheel';

type JsonObject = Record<string, any>;

interface PreferenceSection {
  id: string;
  label: string;
  icon: string;
  navigation_group: string;
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
  kind: 'action_list' | 'accent' | 'color' | 'boolean' | 'file' | 'multi_select' | 'number' | 'replacement_list' | 'select' | 'choice_slider' | 'text' | 'multiline_text';
  icon: string;
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
  file_extensions?: string[];
}

interface PreferenceGroup {
  section: string;
  name: string;
  collapsible: boolean;
  default_open: boolean;
}

interface PreferencesContract {
  format: 'pealayer-preferences';
  sections: PreferenceSection[];
  controls: PreferenceControl[];
  groups: PreferenceGroup[];
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
  timeline_toolbar: <ControlOutlined />,
  sidebar_rail: <MenuOutlined />,
  workspace_tools: <CodeOutlined />,
  web: <GlobalOutlined />,
  advanced: <SettingOutlined />,
};

const controlIcons: Record<string, React.ReactNode> = {
  'terminal-window': <CodeOutlined />,
  plug: <LinkOutlined />,
  'monitor-play': <VideoCameraOutlined />,
  'arrow-square-out': <ExpandOutlined />,
  'squares-four': <GatewayOutlined />,
  'plugs-connected': <LinkOutlined />,
  'circle-half': <GatewayOutlined />,
  gauge: <DashboardOutlined />,
  'film-strip': <VideoCameraOutlined />,
  palette: <BgColorsOutlined />,
  swatches: <BgColorsOutlined />,
  translate: <TranslationOutlined />,
  'frame-corners': <BorderOutlined />,
  'push-pin': <PushpinOutlined />,
  'arrows-out': <ExpandOutlined />,
  target: <AimOutlined />,
  clock: <ClockCircleOutlined />,
  image: <FileImageOutlined />,
  play: <PlayCircleOutlined />,
  pause: <PauseCircleOutlined />,
  'stop-circle': <StopOutlined />,
  'speaker-high': <SoundOutlined />,
  subtitles: <FontSizeOutlined />,
  waveform: <LineChartOutlined />,
  globe: <GlobalOutlined />,
  keyboard: <KeyOutlined />,
  'check-square': <CheckSquareOutlined />,
  'sliders-horizontal': <ControlOutlined />,
  'text-align-left': <SwapRightOutlined />,
  'text-t': <FontSizeOutlined />,
  'list-checks': <MenuOutlined />,
  'pencil-simple': <EditOutlined />,
};

function PlayerExecutableHint({ executable }: { executable: string }) {
  const [result, setResult] = useState<{ resolved?: string; error?: string }>();
  useEffect(() => {
    const controller = new AbortController();
    setResult(undefined);
    void fetch('/api/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'pealayer.player.executable.resolve', params: { executable } }),
      signal: controller.signal,
    }).then(response => response.json()).then(reply => setResult(reply.result ?? { error: reply.error?.message ?? 'Could not resolve executable' }))
      .catch(error => { if (error.name !== 'AbortError') setResult({ error: 'Could not resolve executable' }); });
    return () => controller.abort();
  }, [executable]);
  return result ? <small className="preference-player-path" role="status">
    {result.resolved ? <CheckOutlined /> : <InfoCircleOutlined />} {result.resolved ?? result.error}
  </small> : null;
}

const networkOptionIcon = (icon?: string) => {
  switch (icon) {
    case 'terminal-window': return <CodeOutlined />;
    case 'plug': return <LinkOutlined />;
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

export function ChoiceSlider({ options, value, label, locale, disabled, onCommit }: {
  options: PreferenceOption[]; value: unknown; label: string; locale: 'en' | 'fa';
  disabled: boolean; onCommit: (value: PreferenceOption['value']) => void;
}) {
  const selected = Math.max(0, options.findIndex(option => option.value === value));
  const [index, setIndex] = useState(selected);
  useEffect(() => { if (!disabled) setIndex(selected); }, [selected, disabled]);
  if (!options.length) return null;
  return <div className="preference-choice-slider" data-disabled={disabled}>
    <Slider min={0} max={options.length - 1} step={1} value={index} disabled={disabled}
      ariaLabelForHandle={tr(locale, label)}
      ariaValueTextFormatterForHandle={position => tr(locale, options[position]?.label ?? '')}
      marks={Object.fromEntries(options.map((_, position) => [position, '']))}
      tooltip={{ formatter: position => tr(locale, options[position ?? index]?.label ?? '') }}
      onChange={setIndex} onChangeComplete={position => {
        if (options[position].value !== value) onCommit(options[position].value);
      }} />
    <button type="button" className="preference-choice-slider__value" disabled={disabled}
      aria-label={tr(locale, label)} title={`${tr(locale, options[index]?.label ?? '')} → ${tr(locale, options[(index + 1) % options.length].label)}`}
      onClick={() => { const next = (index + 1) % options.length; setIndex(next); onCommit(options[next].value); }}>
      {options.map((option, position) => <span key={position} aria-hidden="true" className="preference-choice-slider__measure">{tr(locale, option.label)}</span>)}
      <span className="preference-choice-slider__caption">{tr(locale, options[index]?.label ?? '')}</span>
    </button>
  </div>;
}

export const PreferencesTab: React.FC<PreferencesTabProps> = ({ apiBaseUrl, locale, appearance, timelineWheelPreferences, onConfigChange }) => {
  const [contract, setContract] = useState<PreferencesContract | null>(null);
  const [section, setSection] = useState('appearance');
  const [schemaRevision, setSchemaRevision] = useState(0);
  const detail = useRef<HTMLElement>(null);
  const sectionScroll = useRef<Record<string, number>>({});
  const selectSection = (next: string) => {
    const scroller = detail.current?.closest('.app-content');
    if (scroller) sectionScroll.current[section] = scroller.scrollTop;
    setSection(next);
  };
  useLayoutEffect(() => {
    const scroller = detail.current?.closest('.app-content');
    if (scroller) scroller.scrollTop = sectionScroll.current[section] ?? 0;
  }, [section]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [status, setStatus] = useState<{ kind: 'success' | 'error'; text: string } | null>(null);
  const [filePicker, setFilePicker] = useState<PreferenceControl | null>(null);
  const refreshAudioOutputs = async () => {
    try {
      const response = await fetch(`${apiBaseUrl}/api/rpc`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: 'audio-outputs', method: 'audio.outputs.refresh', params: {} }) });
      const result = await response.json();
      if (!response.ok || result.error) throw new Error(result.error?.message || 'Audio output discovery failed');
      // Discovery is asynchronous. Refresh only the options, never overwrite
      // a user's draft or replace the form with its loading state.
      for (const delay of [400, 1000]) {
        await new Promise((resolve) => window.setTimeout(resolve, delay));
        const catalog = await fetch(`${apiBaseUrl}/api/preferences`);
        if (!catalog.ok) continue;
        const next = await catalog.json() as PreferencesContract;
        setContract((current) => current ? { ...current, controls: next.controls } : current);
      }
    } catch (error) { setStatus({ kind: 'error', text: String(error) }); }
  };

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
  }, [appearance?.theme, appearance?.color_palette, appearance?.accent_color, appearance?.custom_accent_color,
    appearance?.ui_animation_mode, appearance?.ui_control_density, appearance?.ui_font_family, appearance?.monospace_font_family,
    appearance?.ui_font_name, appearance?.monospace_font_name, appearance?.workspace_ui,
    appearance?.native_monitor_style, appearance?.native_timeline_style, appearance?.preferences_grouped_navigation]);

  // Installed-font controls are host-owned and conditional on the font choice.
  // Refresh on local changes and remote/native previews without overwriting drafts.
  useEffect(() => {
    if (!contract) return;
    const abort = new AbortController();
    void fetch(`${apiBaseUrl}/api/preferences`, { signal: abort.signal }).then(response => {
      if (!response.ok) throw new Error(`Preferences request failed (${response.status})`);
      return response.json() as Promise<PreferencesContract>;
    }).then(next => setContract(current => current ? { ...current, controls: next.controls, groups: next.groups } : current))
      .catch(error => { if (!abort.signal.aborted) setStatus({ kind: 'error', text: String(error) }); });
    return () => abort.abort();
  }, [apiBaseUrl, appearance?.ui_font_family, appearance?.monospace_font_family, schemaRevision]);

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
  const groups = useMemo(() => contract?.groups.filter((group) => group.section === section) ?? [], [contract, section]);

  const updatePaths = async (updates: Array<{ key: string; value: any }>) => {
    if (!contract) return;
    const nextValues = updates.reduce(
      (values, update) => valueWithPath(values, update.key,
        update.value === '' && !update.key.startsWith('ui.terminal.') ? null : update.value),
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
      if (updates.some(update => update.key === 'ui.terminal.target')) setSchemaRevision(revision => revision + 1);
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
        {controlIcons[control.icon]}
        <span>{tr(locale, control.label)}</span>
      </span>
    );
    if (control.kind === 'file') {
      const executable = control.key === 'external_mpv.executable';
      const terminalExecutable = control.key === 'ui.terminal.shell_program';
      const resetLabel = terminalExecutable ? 'Use default shell' : executable ? 'Use default player' : 'Use default icon';
      const normalize = (text: string) => executable || terminalExecutable ? text.trim().replace(/^"|"$/g, '') : text.trim();
      return <div className="preference-control" key={control.key}>
        {commonLabel}
        <div className="preference-control__file">
          <Input key={String(value ?? '')} defaultValue={value ?? ''}
            aria-label={tr(locale, control.label)} title={String(value ?? '')}
            placeholder={tr(locale, control.placeholder ?? '')}
            onPressEnter={(event) => void update(control, normalize(event.currentTarget.value))}
            onBlur={(event) => event.currentTarget.value !== (value ?? '') && void update(control, normalize(event.currentTarget.value))} />
          <Button icon={<FolderOpenOutlined />} onClick={() => setFilePicker(control)}>{tr(locale, 'Browse…')}</Button>
          <Tooltip title={tr(locale, resetLabel)}>
            <Button icon={<UndoOutlined />} aria-label={tr(locale, resetLabel)} disabled={!value}
              onClick={() => void update(control, executable ? 'mpv' : terminalExecutable ? '' : null)} />
          </Tooltip>
        </div>
        {executable && <PlayerExecutableHint executable={String(value ?? '')} />}
      </div>;
    }
    if (control.kind === 'accent') {
      const customKey = control.custom_key ?? 'custom_accent_color';
      const customHex = String(valueAtPath(contract?.values ?? {}, customKey) ?? '#0078d4');
      const options = (control.options ?? []).filter(option => option.value !== 'custom').map(option => ({
        value: String(option.value), label: tr(locale, option.label), color: String(option.color ?? '#0078d4'),
      }));
      return <label className="preference-control" key={control.key}>{commonLabel}
        <ColorDropdown value={String(value)} customHex={customHex} options={options} locale={locale}
          disabled={saving === control.key} onChange={(next, hex) => {
            if (next === 'custom') void updatePaths([{ key: control.key, value: next }, { key: customKey, value: hex ?? customHex }]);
            else void update(control, next);
          }} />
      </label>;
    }
    if (control.kind === 'color') {
      return <label className="preference-control" key={control.key}><span>{commonLabel}</span>
        <ColorPicker value={String(value)} disabledAlpha showText disabled={saving === control.key}
          onChangeComplete={(color) => void update(control, color.toHexString().toUpperCase())} />
      </label>;
    }
    if (control.kind === 'action_list') {
      const schema = contract;
      if (!schema) return null;
      const options = control.options ?? [];
      const order = [...new Set([...(Array.isArray(value) ? value : []), ...options.map(option => option.value)])]
        .filter(value => options.some(option => option.value === value));
      const hiddenKey = control.custom_key!;
      const hidden: unknown[] = valueAtPath(schema.values, hiddenKey) ?? [];
      const move = (index: number, next: number) => {
        const values = [...order]; [values[index], values[next]] = [values[next], values[index]];
        void update(control, values);
      };
      return <div key={control.key} className="preference-action-list">
        {order.map((value, index) => {
          const option = options.find(option => option.value === value)!;
          return <div key={String(value)} style={{display:'flex',alignItems:'center',gap:8,marginBlock:6}}>
            <Switch size="small" checked={!hidden.includes(value)} disabled={saving !== null}
              aria-label={tr(locale, option.label)} onChange={visible => void updatePaths([{key:hiddenKey,
                value:visible ? hidden.filter(item => item !== value) : [...hidden.filter(item => item !== value), value]}])} />
            <span className="phosphor-icon" aria-hidden="true">{option.icon}</span>
            <span style={{flex:1,minWidth:0}}>{tr(locale, option.label)}</span>
            <Button size="small" icon={<ArrowUpOutlined />} title={tr(locale,'Move up')}
              disabled={index === 0 || saving !== null} onClick={() => move(index,index-1)} />
            <Button size="small" icon={<ArrowDownOutlined />} title={tr(locale,'Move down')}
              disabled={index === order.length-1 || saving !== null} onClick={() => move(index,index+1)} />
          </div>;
        })}
        <Button icon={<UndoOutlined />} disabled={saving !== null} onClick={() => void updatePaths([
          {key:control.key,value:valueAtPath(schema.defaults,control.key)},
          {key:hiddenKey,value:valueAtPath(schema.defaults,hiddenKey)},
        ])}>{tr(locale,'Reset shortcuts')}</Button>
      </div>;
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
    if (control.kind === 'choice_slider') {
      return <div className="preference-control" key={control.key}>
        {commonLabel}
        <ChoiceSlider options={control.options ?? []} value={value} label={control.label} locale={locale}
          disabled={saving === control.key} onCommit={next => void update(control, next)} />
      </div>;
    }
    if (control.kind === 'select') {
      const scheme = resolvedAppearanceTheme(contract?.values.theme ?? 'system', window.matchMedia('(prefers-color-scheme: light)').matches, appearance);
      const paletteLabel = (value: unknown, label: string) => {
        const colors = palettes[value === 'studio' ? 'studio' : 'native'][scheme];
        return <span className="preference-palette-option"><span aria-hidden="true" className="preference-palette-preview"
          style={{ background: colors.canvas, borderColor: colors['line-strong'], color: colors.text }}>
          <span style={{ background: colors['surface-2'] }}><i /></span>
        </span><span>{label}</span></span>;
      };
      return (
        <label className="preference-control" key={control.key}>
          {commonLabel}
          <Select
            value={value}
            showSearch={['ui_font_name', 'monospace_font_name'].includes(control.key) ? { optionFilterProp: 'label' } : undefined}
            onOpenChange={(open) => { if (open && ['audio_device', 'sfx_audio_device'].includes(control.key)) void refreshAudioOutputs(); }}
            options={(control.options ?? []).map((option) => ({ value: option.value, label: control.key === 'external_mpv.mode' ? tr(locale, option.label) : option.description || tr(locale, option.label), description: option.description, icon: option.icon }))}
            optionRender={control.key === 'color_palette' ? option => paletteLabel(option.value, String(option.label))
              : control.key === 'external_mpv.mode' ? option => <span className="preference-player-option">
                <span aria-hidden="true">{controlIcons[option.data.icon ?? 'monitor-play']}</span>
                <span>{option.label}<small>{option.data.description}</small></span>
              </span> : undefined}
            labelRender={control.key === 'color_palette' ? option => paletteLabel(option.value, String(option.label)) : undefined}
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
    if (control.key.startsWith('application_shortcuts.')) {
      return <label className="preference-control" key={control.key}>
        {commonLabel}
        <ShortcutInput value={String(value ?? '')} label={tr(locale, control.label)}
          placeholder={control.placeholder} onCommit={next => void update(control, next)} />
      </label>;
    }
    if (control.kind === 'multiline_text') return (
      <label className="preference-control" key={control.key}>
        {commonLabel}
        <Input.TextArea defaultValue={value ?? ''} rows={3} placeholder={control.placeholder}
          onBlur={event => event.currentTarget.value !== (value ?? '') && void update(control, event.currentTarget.value)} />
      </label>
    );
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

      {loading && !contract ? <FujiLoader locale={locale} /> : contract && (
        <div className="preferences-layout">
          <nav className="preferences-rail" aria-label={tr(locale, 'Preference sections')}>
            {contract.sections.map((item, index) => (
              <React.Fragment key={item.id}>
              {contract.values.preferences_grouped_navigation && (index === 0 || contract.sections[index - 1].navigation_group !== item.navigation_group) &&
                <span className="preferences-rail-group">{tr(locale, item.navigation_group)}</span>}
              <button
                key={item.id}
                type="button"
                className={item.id === section ? 'is-active' : ''}
                aria-pressed={item.id === section}
                onClick={() => selectSection(item.id)}
              >
                {sectionIcons[item.id] ?? <SettingOutlined />}
                <span>{tr(locale, item.label)}</span>
              </button>
              </React.Fragment>
            ))}
          </nav>
          <main className="preferences-detail" ref={detail}>
            {groups.map((group) => {
              const children = <div className="preferences-group__controls">
                {controls.filter((control) => control.group === group.name).map(renderControl)}
              </div>;
              return group.collapsible
                ? <Collapse key={`${section}:${group.name}`} className="preferences-group preferences-group--optional"
                    defaultActiveKey={group.default_open ? [group.name] : []}
                    items={[{ key: group.name, label: <span><FileImageOutlined /> {tr(locale, group.name)}</span>, children }]} />
                : <Card key={group.name} className="preferences-group" title={tr(locale, group.name)} bordered>{children}</Card>;
            })}
            <div className="preferences-save-state" aria-live="polite">
              {saving ? <><SaveOutlined /> {tr(locale, 'Saving')}</> : <><CheckOutlined /> {tr(locale, 'Changes save automatically')}</>}
            </div>
          </main>
        </div>
      )}
      {filePicker && <ServerFilePicker apiBaseUrl={apiBaseUrl} locale={locale}
        allowAnyFile={filePicker.key.startsWith('ui.terminal.') && !filePicker.file_extensions?.length}
        extensions={filePicker.file_extensions ?? []} initialFile={valueAtPath(contract?.values ?? {}, filePicker.key) ?? undefined}
        title={tr(locale, filePicker.key.startsWith('ui.terminal.') ? 'Choose terminal executable' : filePicker.key === 'external_mpv.executable' ? 'Choose mpv executable' : 'Choose an image file')} onCancel={() => setFilePicker(null)}
        onSelect={(path) => { void update(filePicker, path); setFilePicker(null); }} />}
    </section>
  );
};
