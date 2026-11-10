import React, { useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { tr, UiLocale } from './i18n';
import { Divider, Input, Select } from 'antd';
import type { RefSelectProps } from 'antd';
import {
  ApiOutlined,
  BellOutlined,
  BulbOutlined,
  CarOutlined,
  CustomerServiceOutlined,
  FireOutlined,
  FolderOpenOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
  RadarChartOutlined,
  SettingOutlined,
  SoundOutlined,
  ThunderboltOutlined,
  WifiOutlined,
  SearchOutlined,
  StarOutlined,
} from '@ant-design/icons';

const presets = [
  ['window','Window'], ['chandelier','Chandelier'], ['loostr','Loostr'], ['stage-light','Stage light'], ['garden','Garden'],
  ['sparkle', 'Sparkle'],
  ['folder', 'Folder'],
  ['plug', 'Plug'], ['lightning', 'Lightning'], ['lightbulb', 'Light bulb'],
  ['lamp', 'Lamp'], ['fan', 'Fan'], ['power', 'Power'], ['speaker', 'Speaker'],
  ['radio', 'Radio'], ['seat', 'Seat'], ['car', 'Car'], ['door', 'Door'],
  ['bell', 'Bell'], ['fire', 'Fire'], ['snowflake', 'Snowflake'],
  ['thermometer', 'Thermometer'], ['waveform', 'Waveform'],
  ['circuitry', 'Circuitry'], ['gear', 'Gear'],
] as const;

type CatalogIcon = {key:string;label:string;glyph:string};
let catalog: CatalogIcon[] = [];
let catalogRequest: Promise<void> | undefined;
const listeners = new Set<()=>void>();
const subscribe = (listener:()=>void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };
const catalogSnapshot = () => catalog;
function loadCatalog() {
  catalogRequest ??= fetch('/api/icons/catalog', {signal:AbortSignal.timeout(8000)}).then(response=>{
    if (!response.ok) throw new Error('Icon catalog'); return response.json();
  }).then((value:unknown)=>{
    if (!Array.isArray(value) || value.length>4096 || !value.every(item=>typeof item?.key==='string' && typeof item?.label==='string' && typeof item?.glyph==='string')) throw new Error('Icon catalog');
    catalog = value; listeners.forEach(listener=>listener());
  }).catch(()=>{}); // Retain the bundled common presets on an offline request.
}
function CatalogGlyph({icon}:{icon:string}) {
  const entries = useSyncExternalStore(subscribe,catalogSnapshot,catalogSnapshot);
  useEffect(loadCatalog,[]);
  const entry = entries.find(entry=>entry.key===icon.trim().toLowerCase());
  return entry ? <span className="phosphor-icon" aria-hidden="true">{entry.glyph}</span> : fallbackGlyph(icon);
}
export const effectGlyph = (icon:string) => <CatalogGlyph icon={icon} />;
const fallbackGlyph = (icon: string) => {
  switch (icon.trim().toLowerCase()) {
    case 'sparkle': return <StarOutlined />;
    case 'plug': return <ApiOutlined />;
    case 'folder': return <FolderOpenOutlined />;
    case 'lightning': return <ThunderboltOutlined />;
    case 'lightbulb':
    case 'chandelier': case 'loostr': case 'stage-light':
    case 'lamp': return <BulbOutlined />;
    case 'power': return <PoweroffOutlined />;
    case 'speaker': return <SoundOutlined />;
    case 'radio': return <WifiOutlined />;
    case 'car': return <CarOutlined />;
    case 'bell': return <BellOutlined />;
    case 'fire': return <FireOutlined />;
    case 'gear': return <SettingOutlined />;
    case 'circuitry': return <ApiOutlined />;
    case 'seat':
    case 'door': return <CustomerServiceOutlined />;
    case 'waveform':
    case 'fan':
    case 'snowflake':
    case 'thermometer': return <RadarChartOutlined />;
    case 'window': case 'garden': return <ApiOutlined />;
    default: return <PlayCircleOutlined />;
  }
};

export const effectIconOptions = presets.map(([value, label]) => ({
  value,
  label: <span className="effect-icon-option">{effectGlyph(value)}<span>{label}</span></span>,
}));

interface EffectIconPickerProps {
  value: string;
  onChange: (value: string) => void;
  searchPlaceholder: string;
  presetsLabel: string;
  emptyLabel: string;
  className?: string;
  compact?: boolean;
  locale?: UiLocale;
}

/**
 * One cohesive icon control for effect properties. Search is intentionally a
 * dedicated first row instead of a second field beside the selector, and the
 * divider is part of the popup structure rather than individual option data.
 */
export const EffectIconPicker: React.FC<EffectIconPickerProps> = ({
  value,
  onChange,
  searchPlaceholder,
  presetsLabel,
  emptyLabel,
  className,
  compact = false,
  locale = 'en',
}) => {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const select = useRef<RefSelectProps>(null);
  const entries = useSyncExternalStore(subscribe,catalogSnapshot,catalogSnapshot);
  useEffect(loadCatalog,[]);
  const filteredOptions = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    const available: ReadonlyArray<readonly [string,string]> = entries.length ? entries.map(entry=>[entry.key,entry.label] as const) : presets;
    return available
      .filter(([presetValue, label]) => !normalized
        || presetValue.toLocaleLowerCase().includes(normalized)
        || label.toLocaleLowerCase().includes(normalized) || tr(locale, label).toLocaleLowerCase().includes(normalized))
      .map(([presetValue, label]) => ({
        value: presetValue,
        label: <span className="effect-icon-option">{effectGlyph(presetValue)}<span>{tr(locale, label)}</span></span>,
      }));
  }, [query, locale, entries]);

  return <Select
    ref={select}
    className={className ? `effect-icon-picker ${className}` : 'effect-icon-picker'}
    value={value}
    labelRender={compact ? () => <span className="effect-icon-picker__compact">{effectGlyph(value)}</span> : undefined}
    open={open}
    onOpenChange={(nextOpen) => {
      setOpen(nextOpen);
      if (!nextOpen) setQuery('');
    }}
    onChange={onChange}
    options={[{ label: presetsLabel, options: filteredOptions }]}
    notFoundContent={<span className="effect-icon-picker__empty">{emptyLabel}</span>}
    popupRender={(menu) => <div className="effect-icon-picker__popup">
      <div
        className="effect-icon-picker__search"
        onMouseDown={(event) => event.stopPropagation()}
        onClick={(event) => event.stopPropagation()}
      >
        <Input
          autoFocus
          allowClear
          prefix={<SearchOutlined />}
          value={query}
          placeholder={searchPlaceholder}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); select.current?.focus(); }
            else if (event.key === 'Escape') { event.preventDefault(); setOpen(false); }
            event.stopPropagation();
          }}
        />
      </div>
      <Divider className="effect-icon-picker__divider" />
      {menu}
    </div>}
  />;
};
