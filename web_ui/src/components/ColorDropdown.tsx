import { useRef, useState } from 'react';
import { DownOutlined } from '@ant-design/icons';
import { ColorPicker } from './ColorPicker';
import { tr, UiLocale } from '../i18n';

export interface ColorChoice { value: string; label: string; color: string }
/** A single popup owns presets, Custom expansion, swatches and the HEX editor. */
export function ColorDropdown({ value, customHex, options, onChange, locale, disabled = false }: {
  value: string; customHex: string; options: ColorChoice[];
  onChange: (value: string, hex?: string) => void; locale: UiLocale; disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const list = useRef<HTMLDivElement>(null);
  const keyboardOpen = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const [popup, setPopup] = useState<{ placement: 'topLeft' | 'bottomLeft'; height: number }>({ placement: 'bottomLeft', height: 520 });
  const show = (next: boolean) => {
    if (next && trigger.current) {
      const rect = trigger.current.getBoundingClientRect();
      const above = rect.top - 32, below = window.innerHeight - rect.bottom - 32;
      setPopup({ placement: above > below ? 'topLeft' : 'bottomLeft', height: Math.max(120, Math.min(560, Math.max(above, below))) });
    }
    setOpen(next); if (!next) setExpanded(false);
  };
  const selected = options.find(option => option.value === value);
  const color = selected?.color ?? customHex;
  return <ColorPicker locale={locale} value={expanded ? customHex : color} disabledAlpha disabled={disabled}
    open={open} placement={popup.placement} onOpenChange={show}
    onChangeComplete={next => onChange('custom', next.toHexString().toUpperCase())}
    panelRender={panel => <div className="color-dropdown-panel" style={{ maxHeight: popup.height, overflowY: 'auto' }} ref={node => {
      list.current = node;
      if (node && keyboardOpen.current) {
        keyboardOpen.current = false;
        requestAnimationFrame(() => (node.querySelector<HTMLButtonElement>('button[aria-selected="true"]') ?? node.querySelector<HTMLButtonElement>('button[role="option"]'))?.focus());
      }
    }} role="listbox" aria-label={tr(locale, 'Color')}
      onKeyDown={event => {
        if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setOpen(false); return; }
        if ((event.target as HTMLElement).closest('input,textarea,select')) return;
        const choices = [...(list.current?.querySelectorAll<HTMLButtonElement>('button[role="option"]') ?? [])];
        const current = choices.indexOf(event.target as HTMLButtonElement);
        if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
          event.preventDefault(); event.stopPropagation();
          const next = event.key === 'Home' ? 0 : event.key === 'End' ? choices.length - 1
            : (current + (event.key === 'ArrowDown' ? 1 : -1) + choices.length) % choices.length;
          choices[next]?.focus();
        }
      }}>
      {options.map(option => <button key={option.value} type="button" role="option" aria-selected={value === option.value}
        className="color-dropdown-choice" onClick={() => { onChange(option.value); setOpen(false); }}>
        <span className="recording-color-swatch" style={{ backgroundColor: option.color }} />{option.label}
      </button>)}
      <button type="button" role="option" aria-selected={value === 'custom'} aria-expanded={expanded}
        className="color-dropdown-choice" onClick={() => { setExpanded(!expanded); onChange('custom', customHex); }}>
        <span className="recording-color-swatch" style={{ backgroundColor: customHex }} />{tr(locale, 'Custom')}
        <span className="color-dropdown-hex-preview">{customHex.toUpperCase()}</span>
      </button>
      {expanded && <div className="color-dropdown-custom">{panel}</div>}
    </div>}>
    <button ref={trigger} type="button" className="color-dropdown-trigger" role="combobox" aria-expanded={open}
      aria-label={tr(locale, 'Color')} disabled={disabled} onKeyDown={event => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); keyboardOpen.current = true; show(true); }
      }}>
      <span className="recording-color-swatch" style={{ backgroundColor: color }} />
      <span>{selected?.label ?? tr(locale, 'Custom')}</span><DownOutlined />
    </button>
  </ColorPicker>;
}
