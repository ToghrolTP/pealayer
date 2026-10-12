import { ApiOutlined, BgColorsOutlined, ClockCircleOutlined, EditOutlined, FolderOutlined, SoundOutlined, ThunderboltOutlined, AimOutlined, SlidersOutlined, StarOutlined } from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

export function DesignerFieldLabel({ label, locale }: { label: string; locale: UiLocale }) {
  const Icon = ['Name','Description','Text','Label','LCD message'].includes(label) ? EditOutlined
    : label === 'Icon' ? StarOutlined : label === 'Group' ? FolderOutlined
    : ['Color','RGB','Brightness'].includes(label) ? BgColorsOutlined
    : ['Start','Length','Finish','Interval','Repeat step','Repeat effect'].includes(label) ? ClockCircleOutlined
    : ['Command','Value','State','Direction'].includes(label) ? ThunderboltOutlined
    : ['Target','Destination'].includes(label) ? AimOutlined
    : ['Frequency','Volume','Audio file / URL','Output device / backend'].includes(label) ? SoundOutlined
    : ['Easing','Curve quality'].includes(label) ? SlidersOutlined : ApiOutlined;
  return <span className="designer-field-label"><Icon aria-hidden />{tr(locale, label)}</span>;
}
