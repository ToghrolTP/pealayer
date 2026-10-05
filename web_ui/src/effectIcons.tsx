import React, { useMemo, useState } from 'react';
import { Divider, Input, Select } from 'antd';
import {
  ApiOutlined,
  BellOutlined,
  BulbOutlined,
  CarOutlined,
  CustomerServiceOutlined,
  FireOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
  RadarChartOutlined,
  SettingOutlined,
  SoundOutlined,
  ThunderboltOutlined,
  WifiOutlined,
  SearchOutlined,
} from '@ant-design/icons';

const presets = [
  ['sparkle', 'Sparkle'],
  ['plug', 'Plug'], ['lightning', 'Lightning'], ['lightbulb', 'Light bulb'],
  ['lamp', 'Lamp'], ['fan', 'Fan'], ['power', 'Power'], ['speaker', 'Speaker'],
  ['radio', 'Radio'], ['seat', 'Seat'], ['car', 'Car'], ['door', 'Door'],
  ['bell', 'Bell'], ['fire', 'Fire'], ['snowflake', 'Snowflake'],
  ['thermometer', 'Thermometer'], ['waveform', 'Waveform'],
  ['circuitry', 'Circuitry'], ['gear', 'Gear'],
] as const;

export const effectGlyph = (icon: string) => {
  switch (icon.trim().toLowerCase()) {
    case 'lightning': return <ThunderboltOutlined />;
    case 'lightbulb':
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
}) => {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const filteredOptions = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    return presets
      .filter(([presetValue, label]) => !normalized
        || presetValue.toLocaleLowerCase().includes(normalized)
        || label.toLocaleLowerCase().includes(normalized))
      .map(([presetValue, label]) => ({
        value: presetValue,
        label: <span className="effect-icon-option">{effectGlyph(presetValue)}<span>{label}</span></span>,
      }));
  }, [query]);

  return <Select
    className={className ? `effect-icon-picker ${className}` : 'effect-icon-picker'}
    value={value}
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
          onKeyDown={(event) => event.stopPropagation()}
        />
      </div>
      <Divider className="effect-icon-picker__divider" />
      {menu}
    </div>}
  />;
};
