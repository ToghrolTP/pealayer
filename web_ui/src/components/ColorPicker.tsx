import React, { useSyncExternalStore } from 'react';
import { ColorPicker as AntColorPicker } from 'antd';
import swatches from '../../../assets/themes/ui-colors.json';
import { tr, UiLocale } from '../i18n';

let recentColors: string[] = [];
const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };
export function rememberColor(hex: string) {
  recentColors = [hex.toUpperCase(), ...recentColors.filter(color => color !== hex.toUpperCase())].slice(0, 12);
  listeners.forEach(listener => listener());
}

/** Shared swatches complement, rather than replace, the selected theme. */
export const ColorPicker: React.FC<React.ComponentProps<typeof AntColorPicker> & { locale?: UiLocale }> = ({ locale = 'en', onChangeComplete, ...props }) => {
  const recent = useSyncExternalStore(subscribe, () => recentColors);
  return <AntColorPicker {...props} presets={props.presets ?? [
    ...(recent.length ? [{ label: tr(locale, 'Recent colors'), colors: recent }] : []),
    { label: 'Pealayer', colors: swatches.map(color => color.hex) },
  ]} onChangeComplete={color => { rememberColor(color.toHexString()); onChangeComplete?.(color); }} />;
};
