import { Select } from 'antd';
import { SoundOutlined } from '@ant-design/icons';
import type { HardwareMelody } from '../melodyCatalog';
import { tr, type UiLocale } from '../i18n';

/** Authoritative catalog shared by live buzzer control and sequence authoring. */
export function MelodySelect({ melodies, locale, refresh, ...props }: {
  melodies: HardwareMelody[]; locale: UiLocale; refresh: () => void;
  value?: string; disabled?: boolean; className?: string;
  placeholder?: React.ReactNode; onChange: (name: string) => void;
}) {
  return <Select
    {...props}
    aria-label={tr(locale, 'Configured melody')}
    showSearch
    filterOption={(input, option) => String(option?.value ?? '').toLocaleLowerCase().includes(input.toLocaleLowerCase())}
    placeholder={props.placeholder ?? tr(locale, 'No configured melodies')}
    notFoundContent={tr(locale, 'No configured melodies')}
    options={melodies.map((melody) => ({
      value: melody.name,
      label: <span className="melody-option"><SoundOutlined /><span className="melody-option__name" title={melody.name}>{melody.name}</span></span>,
      detail: `${new Intl.NumberFormat(locale, { maximumFractionDigits: 3 }).format(melody.duration_ms / 1000)} ${tr(locale, 'sec')} · ${melody.notes.length} ${tr(locale, 'notes')}`,
    }))}
    optionRender={(option) => <div className="melody-option-row">{option.label}<span className="melody-option__detail">{option.data.detail}</span></div>}
    onOpenChange={(open) => { if (open) refresh(); }}
  />;
}
