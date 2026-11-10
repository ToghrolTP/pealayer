import type {ReactElement} from 'react';
import {Tooltip} from 'antd';
import {CheckOutlined, DashboardOutlined, VideoCameraOutlined, SwapOutlined} from '@ant-design/icons';
import {tr, type UiLocale} from '../i18n';
import '../frame-rate-tooltip.css';

interface Props {
  mediaRate?: number | null; uiRate?: number | null;
  screenRate: boolean; automatic: boolean; locale: UiLocale;
  children: ReactElement; open?: boolean;
}

/** Display current snapshots only; never requests data or changes the source. */
export function FrameRateTooltip({children, open, ...props}: Props) {
  return <Tooltip open={open} trigger={['hover', 'focus']} placement="topLeft" color="var(--surface-1)" title={<FrameRateDetails {...props}/>}
    styles={{root: {maxWidth: 'calc(100vw - 24px)'}, container: {
      background: 'var(--surface-1)', color: 'var(--text)', padding: 12,
      border: '1px solid var(--line)', borderRadius: 10,
    }}}>{children}</Tooltip>;
}

export function FrameRateDetails({mediaRate, uiRate, screenRate, automatic, locale}: Omit<Props, 'children' | 'open'>) {
  const rows = [
    {key: 'media', label: 'Media frame rate', icon: <VideoCameraOutlined/>, rate: mediaRate, active: !screenRate},
    {key: 'screen', label: 'Screen render rate', icon: <DashboardOutlined/>, rate: uiRate, active: screenRate},
  ];
  return <section className="frame-rate-details" aria-label={tr(locale, 'Frame rate')}>
    <header><DashboardOutlined/><strong>{tr(locale, 'Frame rate')}</strong>{automatic && <span className="frame-rate-details__auto">{tr(locale, 'Auto')}</span>}</header>
    <hr/>
    <div className="frame-rate-details__grid">{rows.map(row => <div className={`frame-rate-details__row${row.active ? ' is-active' : ''}`} key={row.key}>
      <span className="frame-rate-details__label">{row.icon}{tr(locale, row.label)}</span>
      <span className="frame-rate-details__value">{row.rate != null && Number.isFinite(row.rate) && row.rate > 0 ? row.rate.toFixed(2) : '—'}</span>
      <span className="frame-rate-details__unit">fps</span>
      <span className="frame-rate-details__active" aria-label={row.active ? tr(locale, 'Active source') : undefined}>{row.active && <CheckOutlined/>}</span>
    </div>)}</div>
    <hr/>
    <footer><SwapOutlined/>{tr(locale, 'Click to switch frame rate source')}</footer>
  </section>;
}
