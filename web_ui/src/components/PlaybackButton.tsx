import { Button, Tooltip } from 'antd';
import { CaretRightFilled, PauseOutlined } from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

/** Render the next action from the authoritative player snapshot, not from
 * optimistic click state. Both Web workspaces share geometry and semantics. */
export function PlaybackButton({ playing, loaded, locale, className, onClick }: {
  playing?: boolean;
  loaded: boolean;
  locale: UiLocale;
  className: string;
  onClick: () => void;
}) {
  const label = tr(locale, playing ? 'Pause' : 'Play');
  return <Tooltip title={label}>
    <Button
      className={`${className} transport-action ${playing ? 'transport-action--pause' : 'transport-action--play'}`}
      aria-label={label}
      disabled={!loaded || playing === undefined}
      icon={playing ? <PauseOutlined /> : <CaretRightFilled />}
      onClick={onClick}
    />
  </Tooltip>;
}
