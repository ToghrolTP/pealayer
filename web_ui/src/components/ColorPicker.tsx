import React from 'react';
import { ColorPicker as AntColorPicker } from 'antd';
import swatches from '../../../assets/themes/ui-colors.json';

/** Shared swatches complement, rather than replace, the selected theme. */
export const ColorPicker: React.FC<React.ComponentProps<typeof AntColorPicker>> = (props) => (
  <AntColorPicker {...props} presets={props.presets ?? [{ label: 'Pealayer', colors: swatches.map((color) => color.hex) }]} />
);
